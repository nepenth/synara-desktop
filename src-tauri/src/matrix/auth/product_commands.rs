use super::*;

/// V-AUTH.3 — privacy-safe login-flow DTO (no secrets; discovery only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixLoginFlowDto {
    /// Synara kind discriminator (`password`, `token`, `application_service`, `unknown`).
    pub kind: String,
    /// Original Matrix type string (`m.login.password`, custom types, …).
    pub matrix_type: String,
    /// Token flow: homeserver supports `get_login_token` (when known).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get_login_token: Option<bool>,
}

impl MatrixLoginFlowDto {
    pub(super) fn from_domain(flow: LoginFlow) -> Self {
        Self {
            kind: flow.kind.as_str().to_owned(),
            matrix_type: flow.matrix_type,
            get_login_token: flow.get_login_token,
        }
    }
}

/// V-AUTH.3 — login-flow discovery response for the product UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixLoginFlowsResponse {
    pub flows: Vec<MatrixLoginFlowDto>,
}

/// V-AUTH.3 — discover homeserver login flows (unauthenticated CS `GET /login`).
///
/// Fail-closed: transport/parse errors surface as privacy-safe command errors.
/// No credentials are submitted; DTO never contains tokens or passwords.
#[tauri::command]
pub async fn matrix_login_flows(
    homeserver_url: String,
) -> Result<MatrixLoginFlowsResponse, MatrixAuthCommandError> {
    let transport = HttpLoginFlowTransport::new().map_err(map_login_flows_auth_error)?;
    let result = discover_login_flows(&homeserver_url, &transport)
        .await
        .map_err(map_login_flows_auth_error)?;
    Ok(MatrixLoginFlowsResponse {
        flows: result
            .flows
            .into_iter()
            .map(MatrixLoginFlowDto::from_domain)
            .collect(),
    })
}

#[tauri::command]
pub async fn matrix_login_password(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
    homeserver_url: String,
    user: String,
    password: String,
) -> Result<MatrixLoginIdentity, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    if session.is_some() {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "A native Matrix session is already logged in.",
            "d0.1-session-already-active",
        ));
    }

    let homeserver_url = normalize_homeserver_url(&homeserver_url)
        .map_err(map_auth_error)?
        .into_string();
    let requested_identity = AccountIdentity::new(&user, &homeserver_url)
        .map_err(|_| MatrixAuthCommandError::invalid_input("d0.1-invalid-user-identity"))?;
    let app_data_root = app_data_root(&app)?;
    let client = build_client(&app_data_root, requested_identity.clone()).await?;

    let result = login_with_password(
        &client,
        requested_identity.user_id(),
        &password,
        &LoginOptions {
            request_refresh_token: true,
            ..LoginOptions::default()
        },
    )
    .await
    .map_err(map_auth_error)?;

    let live_identity = AccountIdentity::new(&result.user_id, &result.homeserver_url)
        .map_err(|_| MatrixAuthCommandError::invalid_input("d0.1-login-identity-invalid"))?;
    if live_identity != requested_identity {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "The authenticated Matrix identity did not match the requested account.",
            "d0.1-login-identity-mismatch",
        ));
    }

    ensure_crypto_ready(&client).await?;
    let session_generation = state.next_generation();
    let verification = NativeVerificationOwner::new(&client, session_generation);
    let devices = NativeDeviceOwner::start(&client, app.clone(), session_generation)
        .await
        .map_err(map_device_error)?;
    let image_packs = NativeImagePackOwner::start(&client, app.clone(), session_generation)
        .map_err(map_pack_read_subscribe_error)?;
    let typing = NativeTypingOwner::start(&client, session_generation).map_err(map_typing_error)?;
    let presence = NativePresenceOwner::start(&client, app.clone(), session_generation)
        .map_err(map_presence_error)?;
    let sync = start_sync_owner(&client, session_generation).await?;
    let session_vault = KeyringSessionMaterialVault::new();
    persist_session_after_login(&client, &live_identity, &session_vault)
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-session-persist-failed"))?;

    let identity = MatrixLoginIdentity {
        user_id: result.user_id,
        device_id: result.device_id,
        homeserver_url: result.homeserver_url,
    };
    if let Err(error) = write_active_identity(&app_data_root, &identity) {
        let _ = clear_session_material(&session_vault, &live_identity);
        return Err(error);
    }

    *session = Some(ManagedMatrixSession {
        client,
        identity: identity.clone(),
        sync,
        invite_avatars: InviteAvatarHandles::new(session_generation),
        timelines: NativeTimelineRegistry::new(session_generation),
        composer_drafts: ComposerDraftRegistry::new(),
        sends: SendQueue::new(session_generation),
        attachments: AttachmentSendQueue::new(session_generation),
        verification,
        _devices: devices,
        _image_packs: image_packs,
        typing,
        presence,
        pending_device_deletion: None,
        next_device_delete_operation_id: 0,
        pending_cross_signing_auth_session: None,
        room_key_transfer: Arc::new(Mutex::new(RoomKeyTransferFlow::new(session_generation))),
        selected_room_key_import: None,
        next_room_key_import_selection_id: 0,
    });
    Ok(identity)
}

/// V-AUTH.4a — request a password-reset email token (unauthenticated CS API).
///
/// Does not create a product login session. Never logs email, client_secret, or sid.
#[tauri::command]
pub async fn matrix_password_reset_request_email_token(
    app: AppHandle,
    homeserver_url: String,
    email: String,
    client_secret: <SET_IN_CONFIG>
    send_attempt: u32,
) -> Result<PasswordEmailTokenResult, MatrixAuthCommandError> {
    let client_secret = zeroize::Zeroizing::new(client_secret);
    let client = build_password_reset_client(&app, &homeserver_url).await?;
    request_password_email_token(&client, &email, client_secret.as_str(), send_attempt)
        .await
        .map_err(map_password_reset_auth_error)
}

/// V-AUTH.4a — complete password reset with email-identity (+ optional password) UIAA.
///
/// Host owns the stages required by the retained desktop flow. Unsupported UIAA
/// stages fail closed. Never logs password, client_secret, or sid.
#[tauri::command]
pub async fn matrix_password_reset_complete(
    app: AppHandle,
    homeserver_url: String,
    email: String,
    new_password: String,
    client_secret: <SET_IN_CONFIG>
    sid: String,
) -> Result<PasswordResetOutcome, MatrixAuthCommandError> {
    let new_password = zeroize::Zeroizing::new(new_password);
    let client_secret = zeroize::Zeroizing::new(client_secret);
    let client = build_password_reset_client(&app, &homeserver_url).await?;
    complete_password_reset(
        &client,
        &email,
        new_password.as_str(),
        client_secret.as_str(),
        &sid,
    )
    .await
    .map_err(map_password_reset_auth_error)
}

/// V-AUTH.4b — probe registration UIAA flows (unauthenticated).
#[tauri::command]
pub async fn matrix_register_flows(
    app: AppHandle,
    homeserver_url: String,
) -> Result<RegisterFlowsProbe, MatrixAuthCommandError> {
    let client = build_register_ephemeral_client(&app, &homeserver_url).await?;
    probe_register_flows(&client)
        .await
        .map_err(map_register_auth_error)
}

/// V-AUTH.4b — request a registration email token (unauthenticated).
#[tauri::command]
pub async fn matrix_register_request_email_token(
    app: AppHandle,
    homeserver_url: String,
    email: String,
    client_secret: <SET_IN_CONFIG>
    send_attempt: u32,
) -> Result<PasswordEmailTokenResult, MatrixAuthCommandError> {
    let client_secret = zeroize::Zeroizing::new(client_secret);
    let client = build_register_ephemeral_client(&app, &homeserver_url).await?;
    request_register_email_token(&client, &email, client_secret.as_str(), send_attempt)
        .await
        .map_err(map_register_auth_error)
}

/// Serializable product outcome for register submit (no tokens).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum MatrixRegisterOutcome {
    /// Registration completed and a native product session was installed.
    Complete { identity: MatrixLoginIdentity },
    /// UIAA stage still required.
    #[serde(rename_all = "camelCase")]
    UiaRequired {
        session: Option<String>,
        flows: Vec<RegisterUiaFlow>,
        completed: Vec<String>,
        params: Option<serde_json::Value>,
        error_code: Option<String>,
        error_message: Option<&'static str>,
    },
}

/// V-AUTH.4b — submit registration (+ UIAA stage). On complete, installs native session.
///
/// Access/refresh tokens never leave the host. Unsupported UIAA stages fail closed.
#[tauri::command]
pub async fn matrix_register(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
    homeserver_url: String,
    username: String,
    password: String,
    device_display_name: Option<String>,
    auth: RegisterAuthStage,
) -> Result<MatrixRegisterOutcome, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    if session.is_some() {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "A native Matrix session is already logged in.",
            "v-auth.4b-session-already-active",
        ));
    }

    let password = zeroize::Zeroizing::new(password);
    let device_display_name = device_display_name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| super::super::platform_device_display_name().to_owned());

    let client = build_register_ephemeral_client(&app, &homeserver_url).await?;
    let outcome = register_submit(
        &client,
        &username,
        password.as_str(),
        &device_display_name,
        auth,
    )
    .await
    .map_err(map_register_auth_error)?;

    match outcome {
        RegisterSubmitOutcome::UiaRequired(challenge) => Ok(MatrixRegisterOutcome::UiaRequired {
            session: challenge.session,
            flows: challenge.flows,
            completed: challenge.completed,
            params: challenge.params,
            error_code: challenge.error_code,
            error_message: challenge.error_message,
        }),
        RegisterSubmitOutcome::Complete(secrets) => {
            let identity =
                install_session_from_register_secrets(&app, &state, &mut session, secrets).await?;
            Ok(MatrixRegisterOutcome::Complete { identity })
        }
    }
}

pub(super) async fn install_session_from_register_secrets(
    app: &AppHandle,
    state: &State<'_, MatrixAuthState>,
    session: &mut Option<ManagedMatrixSession>,
    secrets: super::super::register::RegisterCompleteSecrets,
) -> Result<MatrixLoginIdentity, MatrixAuthCommandError> {
    let homeserver_url = normalize_homeserver_url(&secrets.homeserver_url)
        .map_err(map_register_auth_error)?
        .into_string();
    let live_identity = AccountIdentity::new(&secrets.user_id, &homeserver_url).map_err(|_| {
        MatrixAuthCommandError::invalid_input("v-auth.4b-register-identity-invalid")
    })?;
    let app_data_root = app_data_root(app)?;
    let client = build_client(&app_data_root, live_identity.clone()).await?;

    // Session install must go through lifecycle (guardrail: no Client::restore_session under matrix/auth/).
    let material = SessionMaterial::from_matrix_tokens(
        &live_identity,
        secrets.device_id.as_str(),
        secrets.access_token.as_str(),
        secrets.refresh_token.as_ref().map(|t| t.as_str()),
    )
    .map_err(|_| {
        MatrixAuthCommandError::invalid_input("v-auth.4b-register-session-material-invalid")
    })?;
    restore_session_onto_client(&client, &live_identity, &material)
        .await
        .map_err(|_| {
            MatrixAuthCommandError::new(
                "Unknown",
                "Failed to restore the native Matrix session after registration.",
                "v-auth.4b-register-restore-failed",
            )
        })?;

    ensure_crypto_ready(&client).await?;
    let session_generation = state.next_generation();
    let verification = NativeVerificationOwner::new(&client, session_generation);
    let devices = NativeDeviceOwner::start(&client, app.clone(), session_generation)
        .await
        .map_err(map_device_error)?;
    let image_packs = NativeImagePackOwner::start(&client, app.clone(), session_generation)
        .map_err(map_pack_read_subscribe_error)?;
    let typing = NativeTypingOwner::start(&client, session_generation).map_err(map_typing_error)?;
    let presence = NativePresenceOwner::start(&client, app.clone(), session_generation)
        .map_err(map_presence_error)?;
    let sync = start_sync_owner(&client, session_generation).await?;
    let session_vault = KeyringSessionMaterialVault::new();
    persist_session_after_login(&client, &live_identity, &session_vault)
        .map_err(|_| MatrixAuthCommandError::unavailable("v-auth.4b-session-persist-failed"))?;

    let identity = MatrixLoginIdentity {
        user_id: secrets.user_id.clone(),
        device_id: secrets.device_id.clone(),
        homeserver_url,
    };
    if let Err(error) = write_active_identity(&app_data_root, &identity) {
        let _ = clear_session_material(&session_vault, &live_identity);
        return Err(error);
    }

    *session = Some(ManagedMatrixSession {
        client,
        identity: identity.clone(),
        sync,
        invite_avatars: InviteAvatarHandles::new(session_generation),
        timelines: NativeTimelineRegistry::new(session_generation),
        composer_drafts: ComposerDraftRegistry::new(),
        sends: SendQueue::new(session_generation),
        attachments: AttachmentSendQueue::new(session_generation),
        verification,
        _devices: devices,
        _image_packs: image_packs,
        typing,
        presence,
        pending_device_deletion: None,
        next_device_delete_operation_id: 0,
        pending_cross_signing_auth_session: None,
        room_key_transfer: Arc::new(Mutex::new(RoomKeyTransferFlow::new(session_generation))),
        selected_room_key_import: None,
        next_room_key_import_selection_id: 0,
    });
    Ok(identity)
}

#[tauri::command]
pub async fn matrix_session_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<MatrixSessionSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    Ok(snapshot(session.as_ref()))
}

#[tauri::command]
pub async fn matrix_sync_status(
    state: State<'_, MatrixAuthState>,
) -> Result<SyncReadinessSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    Ok(match session.as_ref() {
        Some(active) => active.sync.observe(),
        None => unconfigured_snapshot(state.current_generation()),
    })
}

#[tauri::command]
pub async fn matrix_crypto_status(
    state: State<'_, MatrixAuthState>,
) -> Result<MatrixCryptoStatus, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let Some(active) = session.as_ref() else {
        return Ok(crypto_status(state.current_generation(), None));
    };
    let cross_signing = active.client.encryption().cross_signing_status().await;
    Ok(crypto_status(
        active.sync.session_generation(),
        cross_signing,
    ))
}

#[tauri::command]
pub async fn matrix_logout(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
) -> Result<MatrixSessionSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let Some(active) = session.as_ref() else {
        return Ok(MatrixSessionSnapshot::LoggedOut);
    };

    active.client.matrix_auth().logout().await.map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "The Matrix homeserver rejected logout.",
            "d0.1-remote-logout-failed",
        )
    })?;
    active
        .sync
        .stop()
        .await
        .map_err(|error| map_sync_error(error.diagnostic_id()))?;

    let identity = account_identity(&active.identity)?;
    let clear_result = clear_session_material(&KeyringSessionMaterialVault::new(), &identity)
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-session-clear-failed"));
    let remove_result = remove_active_identity(&app_data_root(&app)?);
    *session = None;
    clear_result?;
    remove_result?;
    Ok(MatrixSessionSnapshot::LoggedOut)
}

#[tauri::command]
pub async fn matrix_restore_session(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
) -> Result<MatrixLoginIdentity, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    if let Some(active) = session.as_ref() {
        return Ok(active.identity.clone());
    }

    let app_data_root = app_data_root(&app)?;
    let identity = read_active_identity(&app_data_root)?;
    let account = account_identity(&identity)?;
    let client = build_client(&app_data_root, account.clone()).await?;
    let restored =
        restore_session_from_vault(&client, &account, &KeyringSessionMaterialVault::new())
            .await
            .map_err(|_| {
                MatrixAuthCommandError::new(
                    "Forbidden",
                    "No restorable native Matrix session is available.",
                    "d0.1-session-restore-failed",
                )
            })?;

    if restored.meta.device_id != identity.device_id {
        return Err(MatrixAuthCommandError::new(
            "Forbidden",
            "The persisted native Matrix session identity is inconsistent.",
            "d0.1-restored-device-mismatch",
        ));
    }

    ensure_crypto_ready(&client).await?;
    let session_generation = state.next_generation();
    let verification = NativeVerificationOwner::new(&client, session_generation);
    let devices = NativeDeviceOwner::start(&client, app.clone(), session_generation)
        .await
        .map_err(map_device_error)?;
    let image_packs = NativeImagePackOwner::start(&client, app.clone(), session_generation)
        .map_err(map_pack_read_subscribe_error)?;
    let typing = NativeTypingOwner::start(&client, session_generation).map_err(map_typing_error)?;
    let presence = NativePresenceOwner::start(&client, app.clone(), session_generation)
        .map_err(map_presence_error)?;
    let sync = start_sync_owner(&client, session_generation).await?;
    *session = Some(ManagedMatrixSession {
        client,
        identity: identity.clone(),
        sync,
        invite_avatars: InviteAvatarHandles::new(session_generation),
        timelines: NativeTimelineRegistry::new(session_generation),
        composer_drafts: ComposerDraftRegistry::new(),
        sends: SendQueue::new(session_generation),
        attachments: AttachmentSendQueue::new(session_generation),
        verification,
        _devices: devices,
        _image_packs: image_packs,
        typing,
        presence,
        pending_device_deletion: None,
        next_device_delete_operation_id: 0,
        pending_cross_signing_auth_session: None,
        room_key_transfer: Arc::new(Mutex::new(RoomKeyTransferFlow::new(session_generation))),
        selected_room_key_import: None,
        next_room_key_import_selection_id: 0,
    });
    Ok(identity)
}

impl MatrixAuthState {
    pub(super) fn next_generation(&self) -> u64 {
        self.next_session_generation
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1)
    }

    pub(super) fn current_generation(&self) -> u64 {
        self.next_session_generation.load(Ordering::Relaxed)
    }
}

pub(super) async fn start_sync_owner(
    client: &Client,
    session_generation: u64,
) -> Result<SyncServiceOwner, MatrixAuthCommandError> {
    let owner = build_sync_service(client, session_generation, SyncServiceConfig::default())
        .await
        .map_err(|error| map_sync_error(error.diagnostic_id()))?;
    owner
        .start()
        .await
        .map_err(|error| map_sync_error(error.diagnostic_id()))?;
    Ok(owner)
}

pub(super) async fn ensure_crypto_ready(client: &Client) -> Result<(), MatrixAuthCommandError> {
    if client.encryption().cross_signing_status().await.is_none() {
        return Err(MatrixAuthCommandError::new(
            "Unknown",
            "Native Matrix encryption is unavailable.",
            "d0.5-crypto-machine-unavailable",
        ));
    }
    Ok(())
}

pub(super) fn crypto_status(
    session_generation: u64,
    cross_signing: Option<CrossSigningStatus>,
) -> MatrixCryptoStatus {
    MatrixCryptoStatus {
        session_generation,
        encryption_enabled: cross_signing.is_some(),
        cross_signing_state: cross_signing_state(cross_signing.as_ref()),
    }
}

pub(super) fn cross_signing_state(status: Option<&CrossSigningStatus>) -> MatrixCrossSigningState {
    match status {
        None => MatrixCrossSigningState::Unavailable,
        Some(status) if status.is_complete() => MatrixCrossSigningState::Ready,
        Some(status) if status.has_master || status.has_self_signing || status.has_user_signing => {
            MatrixCrossSigningState::Partial
        }
        Some(_) => MatrixCrossSigningState::NotSetUp,
    }
}

pub(super) fn map_sync_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native Matrix sync is unavailable.",
        diagnostic_id,
    )
}

pub(super) fn snapshot(session: Option<&ManagedMatrixSession>) -> MatrixSessionSnapshot {
    match session {
        None => MatrixSessionSnapshot::LoggedOut,
        Some(active) => MatrixSessionSnapshot::LoggedIn {
            user_id: active.identity.user_id.clone(),
            device_id: active.identity.device_id.clone(),
            homeserver_url: active.identity.homeserver_url.clone(),
            session_generation: active.sync.session_generation(),
        },
    }
}

pub(super) async fn build_client(
    app_data_root: &Path,
    identity: AccountIdentity,
) -> Result<Client, MatrixAuthCommandError> {
    let store_key = get_or_create_store_key(
        &KeyringStoreKeyVault::new(),
        &StoreKeyId::from_identity(&identity),
    )
    .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-store-key-unavailable"))?;
    let config = ClientBuildConfig::product_default(app_data_root, identity, Some(store_key))
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-client-config-failed"))?;
    build_unauthenticated_client(&config)
        .await
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-client-build-failed"))
}

/// Ephemeral unauthenticated client for password-reset (no product session, no keyring key).
pub(super) async fn build_password_reset_client(
    app: &AppHandle,
    homeserver_url: &str,
) -> Result<Client, MatrixAuthCommandError> {
    let homeserver_url = normalize_homeserver_url(homeserver_url)
        .map_err(map_password_reset_auth_error)?
        .into_string();
    let user_id =
        password_reset_ephemeral_user_id(&homeserver_url).map_err(map_password_reset_auth_error)?;
    let identity = AccountIdentity::new(&user_id, &homeserver_url).map_err(|_| {
        MatrixAuthCommandError::invalid_input("v-auth.4-password-reset-identity-invalid")
    })?;
    let app_data_root = app_data_root(app)?;
    // Process-local store key — never persisted to the OS credential store.
    let store_key = StoreKeyMaterial::generate().map_err(|_| {
        MatrixAuthCommandError::unavailable("v-auth.4-password-reset-store-key-unavailable")
    })?;
    let config = ClientBuildConfig::product_default(&app_data_root, identity, Some(store_key))
        .map_err(|_| {
            MatrixAuthCommandError::unavailable("v-auth.4-password-reset-client-config-failed")
        })?;
    build_unauthenticated_client(&config).await.map_err(|_| {
        MatrixAuthCommandError::unavailable("v-auth.4-password-reset-client-build-failed")
    })
}

/// Ephemeral unauthenticated client for registration probe/submit/email (no product session).
pub(super) async fn build_register_ephemeral_client(
    app: &AppHandle,
    homeserver_url: &str,
) -> Result<Client, MatrixAuthCommandError> {
    let homeserver_url = normalize_homeserver_url(homeserver_url)
        .map_err(map_register_auth_error)?
        .into_string();
    let user_id = register_ephemeral_user_id(&homeserver_url).map_err(map_register_auth_error)?;
    let identity = AccountIdentity::new(&user_id, &homeserver_url).map_err(|_| {
        MatrixAuthCommandError::invalid_input("v-auth.4b-register-identity-invalid")
    })?;
    let app_data_root = app_data_root(app)?;
    let store_key = StoreKeyMaterial::generate().map_err(|_| {
        MatrixAuthCommandError::unavailable("v-auth.4b-register-store-key-unavailable")
    })?;
    let config = ClientBuildConfig::product_default(&app_data_root, identity, Some(store_key))
        .map_err(|_| {
            MatrixAuthCommandError::unavailable("v-auth.4b-register-client-config-failed")
        })?;
    build_unauthenticated_client(&config)
        .await
        .map_err(|_| MatrixAuthCommandError::unavailable("v-auth.4b-register-client-build-failed"))
}

pub(super) fn map_register_auth_error(error: AuthError) -> MatrixAuthCommandError {
    let diagnostic = error.diagnostic_id();
    let code = match diagnostic {
        "v-auth.4b-register-user-taken" => "UserTaken",
        "v-auth.4b-register-user-invalid"
        | "v-auth.4b-empty-username"
        | "v-auth.4b-invalid-username" => "UserInvalid",
        "v-auth.4b-register-user-exclusive" => "UserExclusive",
        "v-auth.4b-register-password-weak" => "PasswordWeak",
        "v-auth.4b-register-password-short" => "PasswordShort",
        "v-auth.4b-register-forbidden" => "Forbidden",
        id if id.contains("rate-limited") => "RateLimited",
        id if id.contains("unsupported") => "Unsupported",
        _ => match &error {
            AuthError::InvalidInput { .. } => "InvalidRequest",
            AuthError::AuthenticationRejected { .. } => "Forbidden",
            AuthError::RateLimited { .. } => "RateLimited",
            AuthError::Connectivity { .. }
            | AuthError::HomeserverUnavailable { .. }
            | AuthError::WellKnownNotFound { .. } => "InvalidServer",
            AuthError::UnsupportedCapability { .. } => "Unsupported",
            AuthError::InteractiveAuthRequired { .. } => "Unauthorized",
            _ => "Unknown",
        },
    };
    let message = match code {
        "UserTaken" => "This username is already taken.",
        "UserInvalid" => "This username contains invalid characters.",
        "UserExclusive" => "This username is reserved.",
        "PasswordWeak" => "Password rejected as too weak.",
        "PasswordShort" => "Password rejected as too short.",
        "RateLimited" => "The registration request was rate limited.",
        "Forbidden" => "The homeserver does not permit registration.",
        "InvalidRequest" => "The registration request is invalid.",
        "InvalidServer" => "The Matrix homeserver is unavailable.",
        "Unsupported" => "The homeserver requires an unsupported registration stage.",
        "Unauthorized" => "Additional authentication is required to register.",
        _ => "Native registration failed.",
    };
    MatrixAuthCommandError::new(code, message, diagnostic)
}

pub(super) fn map_password_reset_auth_error(error: AuthError) -> MatrixAuthCommandError {
    let code = match error {
        AuthError::AuthenticationRejected { .. } => "Forbidden",
        AuthError::UserDeactivated { .. } => "UserDeactivated",
        AuthError::RateLimited { .. } => "RateLimited",
        AuthError::InvalidInput { .. } => "InvalidRequest",
        AuthError::Connectivity { .. }
        | AuthError::HomeserverUnavailable { .. }
        | AuthError::WellKnownNotFound { .. } => "InvalidServer",
        AuthError::UnsupportedCapability { .. } => "Unsupported",
        AuthError::InteractiveAuthRequired { .. } => "Unauthorized",
        _ => "Unknown",
    };
    let message = match code {
        "Forbidden" => "The password reset request was rejected.",
        "UserDeactivated" => "The Matrix account is deactivated.",
        "RateLimited" => "The password reset request was rate limited.",
        "InvalidRequest" => "The password reset request is invalid.",
        "InvalidServer" => "The Matrix homeserver is unavailable.",
        "Unsupported" => "The homeserver requires an unsupported authentication stage.",
        "Unauthorized" => "Additional authentication is required to reset the password.",
        _ => "Native password reset failed.",
    };
    MatrixAuthCommandError::new(code, message, error.diagnostic_id())
}

pub(super) fn account_identity(
    identity: &MatrixLoginIdentity,
) -> Result<AccountIdentity, MatrixAuthCommandError> {
    AccountIdentity::new(&identity.user_id, &identity.homeserver_url)
        .map_err(|_| MatrixAuthCommandError::invalid_input("d0.1-persisted-identity-invalid"))
}

pub(super) fn app_data_root(app: &AppHandle) -> Result<PathBuf, MatrixAuthCommandError> {
    app.path()
        .app_data_dir()
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-app-data-dir-unavailable"))
}

pub(super) fn active_identity_path(app_data_root: &Path) -> PathBuf {
    app_data_root
        .join(MATRIX_DATA_DIR)
        .join(ACTIVE_SESSION_FILE)
}

pub(super) fn write_active_identity(
    app_data_root: &Path,
    identity: &MatrixLoginIdentity,
) -> Result<(), MatrixAuthCommandError> {
    let path = active_identity_path(app_data_root);
    let parent = path
        .parent()
        .ok_or_else(|| MatrixAuthCommandError::unavailable("d0.1-active-session-path-invalid"))?;
    fs::create_dir_all(parent)
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-active-session-dir-failed"))?;
    let bytes = serde_json::to_vec(identity)
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-active-session-encode-failed"))?;
    fs::write(path, bytes)
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-active-session-write-failed"))
}

pub(super) fn read_active_identity(
    app_data_root: &Path,
) -> Result<MatrixLoginIdentity, MatrixAuthCommandError> {
    let path = active_identity_path(app_data_root);
    if !path.is_file() {
        return Err(MatrixAuthCommandError::new(
            "Forbidden",
            "No persisted native Matrix session was found.",
            "d0.1-active-session-missing",
        ));
    }
    let bytes = fs::read(path)
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-active-session-read-failed"))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-active-session-invalid"))
}

pub(super) fn remove_active_identity(app_data_root: &Path) -> Result<(), MatrixAuthCommandError> {
    let path = active_identity_path(app_data_root);
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(MatrixAuthCommandError::unavailable(
            "d0.1-active-session-remove-failed",
        )),
    }
}

pub(super) fn map_auth_error(error: AuthError) -> MatrixAuthCommandError {
    let code = match error {
        AuthError::AuthenticationRejected { .. } => "Forbidden",
        AuthError::UserDeactivated { .. } => "UserDeactivated",
        AuthError::RateLimited { .. } => "RateLimited",
        AuthError::InvalidInput { .. } => "InvalidRequest",
        AuthError::Connectivity { .. }
        | AuthError::HomeserverUnavailable { .. }
        | AuthError::WellKnownNotFound { .. } => "InvalidServer",
        _ => "Unknown",
    };
    let message = match code {
        "Forbidden" => "The Matrix login credentials were rejected.",
        "UserDeactivated" => "The Matrix account is deactivated.",
        "RateLimited" => "The Matrix login request was rate limited.",
        "InvalidRequest" => "The native Matrix login request is invalid.",
        "InvalidServer" => "The Matrix homeserver is unavailable.",
        _ => "Native Matrix login failed.",
    };
    MatrixAuthCommandError::new(code, message, error.diagnostic_id())
}

/// Map login-flow discovery errors (V-AUTH.3). Privacy-safe; no secrets in message.
pub(super) fn map_login_flows_auth_error(error: AuthError) -> MatrixAuthCommandError {
    let code = match error {
        AuthError::InvalidInput { .. } => "InvalidRequest",
        AuthError::RateLimited { .. } => "RateLimited",
        AuthError::Connectivity { .. }
        | AuthError::HomeserverUnavailable { .. }
        | AuthError::WellKnownNotFound { .. } => "InvalidServer",
        AuthError::UnsupportedCapability { .. } => "Unsupported",
        _ => "Unknown",
    };
    let message = match code {
        "InvalidRequest" => "The login-flow discovery request is invalid.",
        "RateLimited" => "Login-flow discovery was rate limited.",
        "InvalidServer" => "The Matrix homeserver is unavailable.",
        "Unsupported" => "The homeserver returned unsupported login-flow data.",
        _ => "Native login-flow discovery failed.",
    };
    MatrixAuthCommandError::new(code, message, error.diagnostic_id())
}
