use super::*;

/// V-AUTH.3 desktop compatibility re-exports for the shared command response.
pub use synara_core::app::auth::{MatrixLoginFlowDto, MatrixLoginFlowsResponse};

/// V-AUTH.3 / SNC-P3.1 — discover login flows through the managed shared Core.
///
/// The renderer input and DTO remain byte-compatible. This command owns no
/// transport: the Core uses the managed desktop Platform's established user
/// agent for its credential-free probe.
#[tauri::command]
pub async fn matrix_login_flows(
    core: State<'_, Arc<synara_core::Core>>,
    homeserver_url: String,
) -> Result<MatrixLoginFlowsResponse, MatrixAuthCommandError> {
    crate::bridge::auth_probes::login_flows(core.inner().as_ref(), homeserver_url).await
}

#[tauri::command]
pub async fn matrix_login_password(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
    core: State<'_, Arc<synara_core::Core>>,
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
    // A new ordinary login invalidates any abandoned process-local recovery
    // capability. It never invokes recovery or changes on-disk/key material.
    state.clear_store_recovery().await;

    let homeserver_url = normalize_homeserver_url(&homeserver_url)
        .map_err(map_auth_error)?
        .into_string();
    let requested_identity = AccountIdentity::new(&user, &homeserver_url)
        .map_err(|_| MatrixAuthCommandError::invalid_input("d0.1-invalid-user-identity"))?;
    let app_data_root = app_data_root(&app)?;
    let client = match build_client(&app_data_root, requested_identity.clone()).await {
        Ok(client) => client,
        Err(error) => {
            // The UI can only request archive-and-rebuild after one of these
            // fail-closed diagnostics. Normal login never calls the reset API.
            if is_recoverable_store_login_diagnostic(&error.diagnostic_id) {
                state.arm_store_recovery(requested_identity.clone()).await;
            }
            return Err(error);
        }
    };

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
    let verification = Arc::new(NativeVerificationOwner::new(&client, session_generation));
    let devices = Arc::new(
        crate::matrix::devices::start_device_owner(&client, app.clone(), session_generation)
            .await
            .map_err(map_device_error)?,
    );
    let image_packs = Arc::new(
        crate::matrix::account_data::start_image_pack_owner(
            &client,
            app.clone(),
            session_generation,
        )
        .map_err(map_pack_read_subscribe_error)?,
    );
    let typing =
        Arc::new(NativeTypingOwner::start(&client, session_generation).map_err(map_typing_error)?);
    let presence = Arc::new(
        crate::matrix::presence::start_presence_owner(&client, app.clone(), session_generation)
            .map_err(map_presence_error)?,
    );
    let join_rules = Arc::new(
        crate::matrix::room_profile::start_join_rule_owner(
            &client,
            app.clone(),
            session_generation,
        )
        .map_err(map_room_join_rule_owner_error)?,
    );
    let sync = Arc::new(start_sync_owner(&client, session_generation).await?);
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
    let timelines = Arc::new(NativeTimelineOwner::new(
        &client,
        crate::matrix::timeline::timeline_view_emit(app.clone()),
        session_generation,
    ));
    // A successfully installed session supersedes every pending/awaiting
    // recovery capability, including one prepared by an earlier failed login.
    state.clear_store_recovery().await;
    *session = Some(ManagedMatrixSession {
        client,
        identity: identity.clone(),
        sync: sync.clone(),
        invite_avatars: join_rules.invite_avatars(),
        timelines: timelines.clone(),
        sends: SendQueue::new(session_generation),
        attachments: AttachmentSendQueue::new(session_generation),
        verification: verification.clone(),
        devices: devices.clone(),
        _image_packs: image_packs.clone(),
        typing: typing.clone(),
        presence: presence.clone(),
        join_rules: join_rules.clone(),

        room_key_transfer: devices.room_key_transfer(),
        selected_room_key_import: None,
        next_room_key_import_selection_id: 0,
    });
    drop(session);
    crate::bridge::session_lifecycle::open_after_desktop_session_install(
        core.inner().as_ref(),
        &identity,
        session_generation,
    )
    .await?;
    core.inner()
        .attach_typing(typing)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-typing-attach-failed"))?;
    core.inner()
        .attach_presence(presence)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-presence-attach-failed"))?;
    core.inner()
        .attach_verification(verification)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-verification-attach-failed"))?;
    core.inner()
        .attach_devices(devices)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-device-attach-failed"))?;
    core.inner()
        .attach_join_rules(join_rules)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-join-rule-attach-failed"))?;
    core.inner()
        .attach_image_packs(image_packs)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-image-pack-attach-failed"))?;
    core.inner()
        .attach_timelines(timelines)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-timeline-attach-failed"))?;
    core.inner()
        .attach_sync(sync)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-sync-attach-failed"))?;
    Ok(identity)
}

/// Opaque process-local confirmation returned only after a failed native
/// store login. It contains no account identity, filesystem path, credential,
/// Matrix token, or encryption key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixStoreRecoveryChallenge {
    pub confirmation_id: String,
}

/// Fixed success result for archive-and-rebuild store recovery. Store paths,
/// archive names, counters, and key material deliberately remain host-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixStoreRecoveryResult {
    pub status: &'static str,
}

/// Begin the explicit local-store recovery confirmation.
///
/// This has no filesystem or Keychain side effects. It is available only after
/// a failed normal login arms a matching host-local target, and returns a
/// CSPRNG-backed one-use confirmation capability rather than a guessable bool.
#[tauri::command]
pub async fn matrix_store_recovery_prepare(
    state: State<'_, MatrixAuthState>,
) -> Result<MatrixStoreRecoveryChallenge, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    if session.is_some() {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "A native Matrix session is already logged in.",
            "d0.1-session-already-active",
        ));
    }
    drop(session);

    let confirmation_id = state.prepare_store_recovery_confirmation().await?;
    Ok(MatrixStoreRecoveryChallenge { confirmation_id })
}

/// Explicitly archive the failed account's local state/crypto/cache/media
/// directories and rebuild an empty current layout.
///
/// This command is deliberately separate from normal login and requires both
/// an opaque CSPRNG confirmation capability and the exact typed `ARCHIVE`
/// acknowledgement. It never deletes/rotates Keychain material, sends network
/// requests, or returns raw paths, SDK errors, account identity, credentials,
/// tokens, or encryption keys.
#[tauri::command]
pub async fn matrix_store_recovery_confirm(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
    confirmation_id: String,
    confirmation_text: String,
) -> Result<MatrixStoreRecoveryResult, MatrixAuthCommandError> {
    // Keep the session gate while consuming the confirmation and touching the
    // local layout so a concurrent normal login cannot open the same store.
    let session = state.session.lock().await;
    if session.is_some() {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "A native Matrix session is already logged in.",
            "d0.1-session-already-active",
        ));
    }
    let identity = state
        .take_confirmed_store_recovery(&confirmation_id, &confirmation_text)
        .await?;
    let app_data_root = app_data_root(&app)?;
    archive_and_rebuild_store(&app_data_root, &identity)?;
    drop(session);

    Ok(MatrixStoreRecoveryResult {
        status: "archived_and_rebuilt",
    })
}

/// V-AUTH.4a — request a password-reset email token (unauthenticated CS API).
///
/// Does not create a product login session. Never logs email, client_secret, or sid.
#[tauri::command]
pub async fn matrix_password_reset_request_email_token(
    app: AppHandle,
    homeserver_url: String,
    email: String,
    client_secret: String,
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
    client_secret: String,
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

/// V-AUTH.4b / SNC-P3.1 — probe registration UIAA flows through the managed Core.
///
/// This is only the credential-free flow probe. Account creation, email-token,
/// and UIAA-continuation commands remain desktop-owned.
#[tauri::command]
pub async fn matrix_register_flows(
    core: State<'_, Arc<synara_core::Core>>,
    homeserver_url: String,
) -> Result<RegisterFlowsProbe, MatrixAuthCommandError> {
    crate::bridge::auth_probes::register_flows(core.inner().as_ref(), homeserver_url).await
}

/// V-AUTH.4b — request a registration email token (unauthenticated).
#[tauri::command]
pub async fn matrix_register_request_email_token(
    app: AppHandle,
    homeserver_url: String,
    email: String,
    client_secret: String,
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
#[allow(clippy::too_many_arguments)] // Stable Tauri IPC fields are intentionally explicit.
#[tauri::command]
pub async fn matrix_register(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
    core: State<'_, Arc<synara_core::Core>>,
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
            let (identity, session_generation) =
                install_session_from_register_secrets(&app, &state, &mut session, secrets).await?;
            let (typing, presence, verification, devices, join_rules, image_packs, timelines, sync) =
                session
                    .as_ref()
                    .map(|active| {
                        (
                            active.typing.clone(),
                            active.presence.clone(),
                            active.verification.clone(),
                            active.devices.clone(),
                            active.join_rules.clone(),
                            active._image_packs.clone(),
                            active.timelines.clone(),
                            active.sync.clone(),
                        )
                    })
                    .ok_or_else(|| MatrixAuthCommandError::unavailable("p2-typing-attach-failed"))?;
            drop(session);
            crate::bridge::session_lifecycle::open_after_desktop_session_install(
                core.inner().as_ref(),
                &identity,
                session_generation,
            )
            .await?;
            core.inner()
                .attach_typing(typing)
                .map_err(|_| MatrixAuthCommandError::unavailable("p2-typing-attach-failed"))?;
            core.inner()
                .attach_presence(presence)
                .map_err(|_| MatrixAuthCommandError::unavailable("p2-presence-attach-failed"))?;
            core.inner()
                .attach_verification(verification)
                .map_err(|_| {
                    MatrixAuthCommandError::unavailable("p2-verification-attach-failed")
                })?;
            core.inner()
                .attach_devices(devices)
                .map_err(|_| MatrixAuthCommandError::unavailable("p2-device-attach-failed"))?;
            core.inner()
                .attach_join_rules(join_rules)
                .map_err(|_| MatrixAuthCommandError::unavailable("p2-join-rule-attach-failed"))?;
            core.inner()
                .attach_image_packs(image_packs)
                .map_err(|_| MatrixAuthCommandError::unavailable("p2-image-pack-attach-failed"))?;
            core.inner()
                .attach_timelines(timelines)
                .map_err(|_| MatrixAuthCommandError::unavailable("p2-timeline-attach-failed"))?;
            core.inner()
                .attach_sync(sync)
                .map_err(|_| MatrixAuthCommandError::unavailable("p2-sync-attach-failed"))?;
            Ok(MatrixRegisterOutcome::Complete { identity })
        }
    }
}

pub(super) async fn install_session_from_register_secrets(
    app: &AppHandle,
    state: &State<'_, MatrixAuthState>,
    session: &mut Option<ManagedMatrixSession>,
    secrets: super::super::RegisterCompleteSecrets,
) -> Result<(MatrixLoginIdentity, u64), MatrixAuthCommandError> {
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
    let verification = Arc::new(NativeVerificationOwner::new(&client, session_generation));
    let devices = Arc::new(
        crate::matrix::devices::start_device_owner(&client, app.clone(), session_generation)
            .await
            .map_err(map_device_error)?,
    );
    let image_packs = Arc::new(
        crate::matrix::account_data::start_image_pack_owner(
            &client,
            app.clone(),
            session_generation,
        )
        .map_err(map_pack_read_subscribe_error)?,
    );
    let typing =
        Arc::new(NativeTypingOwner::start(&client, session_generation).map_err(map_typing_error)?);
    let presence = Arc::new(
        crate::matrix::presence::start_presence_owner(&client, app.clone(), session_generation)
            .map_err(map_presence_error)?,
    );
    let join_rules = Arc::new(
        crate::matrix::room_profile::start_join_rule_owner(
            &client,
            app.clone(),
            session_generation,
        )
        .map_err(map_room_join_rule_owner_error)?,
    );
    let sync = Arc::new(start_sync_owner(&client, session_generation).await?);
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
    let timelines = Arc::new(NativeTimelineOwner::new(
        &client,
        crate::matrix::timeline::timeline_view_emit(app.clone()),
        session_generation,
    ));
    // Registration completed and is about to install a session, so no old
    // failed-login recovery capability may remain consumable.
    state.clear_store_recovery().await;
    *session = Some(ManagedMatrixSession {
        client,
        identity: identity.clone(),
        sync: sync.clone(),
        invite_avatars: join_rules.invite_avatars(),
        timelines,
        sends: SendQueue::new(session_generation),
        attachments: AttachmentSendQueue::new(session_generation),
        verification: verification.clone(),
        devices: devices.clone(),
        _image_packs: image_packs.clone(),
        typing: typing.clone(),
        presence: presence.clone(),
        join_rules: join_rules.clone(),

        room_key_transfer: devices.room_key_transfer(),
        selected_room_key_import: None,
        next_room_key_import_selection_id: 0,
    });
    Ok((identity, session_generation))
}

/// SNC-P3.2 — forward the existing read-only React session snapshot through
/// the managed Core. The desktop session owner remains private to every other
/// command; only this command consumes Core's stable envelope response.
#[tauri::command]
pub async fn matrix_session_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<MatrixSessionSnapshot, MatrixAuthCommandError> {
    crate::bridge::session_lifecycle::session_snapshot(core.inner().as_ref()).await
}

/// SNC-P3.3 — keep the existing payload-free sync-status command while
/// routing its exact DTO through the managed Core registry. The desktop
/// Platform remains the sole owner of the live SDK sync owner.
#[tauri::command]
pub async fn matrix_sync_status(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<SyncReadinessSnapshot, MatrixAuthCommandError> {
    crate::bridge::session_lifecycle::sync_status(core.inner().as_ref()).await
}

/// SNC-P3.4 — retain the existing zero-argument crypto-status command while
/// routing envelope validation and exact response serialization through Core.
/// The desktop Platform still samples the live crypto owner under its auth
/// mutex; this command receives only the already validated public DTO.
#[tauri::command]
pub async fn matrix_crypto_status(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<MatrixCryptoStatus, MatrixAuthCommandError> {
    crate::bridge::session_lifecycle::crypto_status(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_logout(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<MatrixSessionSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    // Logout is also a security boundary when no session is installed: an old
    // Pending/AwaitingConfirmation recovery capability must never outlive it.
    state.clear_store_recovery().await;
    let Some(active) = session.as_ref() else {
        // A repeated logout must also clear any stale Core projection left by
        // a prior partial lifecycle failure. Release the desktop mutex first.
        drop(session);
        crate::bridge::session_lifecycle::close_after_desktop_session_removal(
            core.inner().as_ref(),
        )
        .await?;
        return Ok(MatrixSessionSnapshot::LoggedOut);
    };

    // Remote logout is best-effort. An expired/revoked token must never trap a
    // user in a locally authenticated state or prevent secure local cleanup.
    let _remote_logout_succeeded = active.client.matrix_auth().logout().await.is_ok();
    active.join_rules.retire();
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
    // The desktop session is now gone. Release its async mutex before Core's
    // await and close Core before reporting deferred non-session cleanup errors.
    drop(session);
    crate::bridge::session_lifecycle::close_after_desktop_session_removal(core.inner().as_ref())
        .await?;
    clear_result?;
    remove_result?;
    Ok(MatrixSessionSnapshot::LoggedOut)
}

#[tauri::command]
pub async fn matrix_restore_session(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
    core: State<'_, Arc<synara_core::Core>>,
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
    let verification = Arc::new(NativeVerificationOwner::new(&client, session_generation));
    let devices = Arc::new(
        crate::matrix::devices::start_device_owner(&client, app.clone(), session_generation)
            .await
            .map_err(map_device_error)?,
    );
    let image_packs = Arc::new(
        crate::matrix::account_data::start_image_pack_owner(
            &client,
            app.clone(),
            session_generation,
        )
        .map_err(map_pack_read_subscribe_error)?,
    );
    let typing =
        Arc::new(NativeTypingOwner::start(&client, session_generation).map_err(map_typing_error)?);
    let presence = Arc::new(
        crate::matrix::presence::start_presence_owner(&client, app.clone(), session_generation)
            .map_err(map_presence_error)?,
    );
    let join_rules = Arc::new(
        crate::matrix::room_profile::start_join_rule_owner(
            &client,
            app.clone(),
            session_generation,
        )
        .map_err(map_room_join_rule_owner_error)?,
    );
    let sync = Arc::new(start_sync_owner(&client, session_generation).await?);
    let timelines = Arc::new(NativeTimelineOwner::new(
        &client,
        crate::matrix::timeline::timeline_view_emit(app.clone()),
        session_generation,
    ));
    // Restoring persisted material installs a new live session and therefore
    // revokes any stale recovery capability from an earlier failed login.
    state.clear_store_recovery().await;
    *session = Some(ManagedMatrixSession {
        client,
        identity: identity.clone(),
        sync: sync.clone(),
        invite_avatars: join_rules.invite_avatars(),
        timelines: timelines.clone(),
        sends: SendQueue::new(session_generation),
        attachments: AttachmentSendQueue::new(session_generation),
        verification: verification.clone(),
        devices: devices.clone(),
        _image_packs: image_packs.clone(),
        typing: typing.clone(),
        presence: presence.clone(),
        join_rules: join_rules.clone(),

        room_key_transfer: devices.room_key_transfer(),
        selected_room_key_import: None,
        next_room_key_import_selection_id: 0,
    });
    drop(session);
    crate::bridge::session_lifecycle::open_after_desktop_session_install(
        core.inner().as_ref(),
        &identity,
        session_generation,
    )
    .await?;
    core.inner()
        .attach_typing(typing)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-typing-attach-failed"))?;
    core.inner()
        .attach_presence(presence)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-presence-attach-failed"))?;
    core.inner()
        .attach_verification(verification)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-verification-attach-failed"))?;
    core.inner()
        .attach_devices(devices)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-device-attach-failed"))?;
    core.inner()
        .attach_join_rules(join_rules)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-join-rule-attach-failed"))?;
    core.inner()
        .attach_image_packs(image_packs)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-image-pack-attach-failed"))?;
    core.inner()
        .attach_timelines(timelines)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-timeline-attach-failed"))?;
    core.inner()
        .attach_sync(sync)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-sync-attach-failed"))?;
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

pub(super) fn map_sync_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native Matrix sync is unavailable.",
        diagnostic_id,
    )
}

pub(super) fn map_room_join_rule_owner_error(
    diagnostic_id: &'static str,
) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native Matrix room join-rule updates are unavailable.",
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

/// The only normal-login failures that may arm the explicit archive action.
/// Other local failures (locked keychain, generic store open, I/O) remain
/// fail-closed and retry/support-only rather than being treated as corruption.
fn is_recoverable_store_login_diagnostic(diagnostic_id: &str) -> bool {
    matches!(
        diagnostic_id,
        "p3.2-login-store-reset-required" | "p3.2-login-store-migration-required"
    )
}

/// Archive-and-rebuild uses the existing non-destructive reset primitive. It
/// intentionally does not consult, create, replace, or delete Keychain keys:
/// #695's fresh-store-only generation policy remains untouched.
fn archive_and_rebuild_store(
    app_data_root: &Path,
    identity: &AccountIdentity,
) -> Result<(), MatrixAuthCommandError> {
    let paths = StorePaths::derive(app_data_root, identity).map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "Local Matrix store recovery could not be completed.",
            "p3.2-login-store-recovery-failed",
        )
    })?;
    reset_store_for_recovery(&paths).map(|_| ()).map_err(|_| {
        MatrixAuthCommandError::new(
            "Unknown",
            "Local Matrix store recovery could not be completed.",
            "p3.2-login-store-recovery-failed",
        )
    })
}

fn map_store_migration_error(error: StoreMigrationError) -> MatrixAuthCommandError {
    MatrixAuthCommandError::unavailable(error.diagnostic_id())
}

fn map_store_key_vault_error(error: StoreKeyVaultError) -> MatrixAuthCommandError {
    let diagnostic_id = match error {
        StoreKeyVaultError::BackendUnavailable { .. } => "p3.2-login-store-locked",
        StoreKeyVaultError::MissingKeyForExistingStore | StoreKeyVaultError::CorruptPayload => {
            "p3.2-login-store-reset-required"
        }
        StoreKeyVaultError::NotFound | StoreKeyVaultError::Encoding => {
            "p3.2-login-store-open-failed"
        }
    };
    MatrixAuthCommandError::unavailable(diagnostic_id)
}

fn map_store_client_build_error(error: ClientBuilderError) -> MatrixAuthCommandError {
    let diagnostic_id = match error.to_factory_error().category {
        crate::matrix::ipc::MatrixIpcErrorCategory::StoreLocked => "p3.2-login-store-locked",
        crate::matrix::ipc::MatrixIpcErrorCategory::StoreUnavailable
        | crate::matrix::ipc::MatrixIpcErrorCategory::StoreCorrupt => {
            "p3.2-login-store-open-failed"
        }
        _ => "p3.2-login-store-open-failed",
    };
    MatrixAuthCommandError::unavailable(diagnostic_id)
}

pub(super) async fn build_client(
    app_data_root: &Path,
    identity: AccountIdentity,
) -> Result<Client, MatrixAuthCommandError> {
    // Probe before migration creates the account layout or revision manifest.
    // Once an account root already exists, a Keychain miss must fail closed:
    // generating a replacement key could make encrypted SQLite data
    // unrecoverable. The probe is read-only and surfaces only a static error.
    let store_paths = StorePaths::derive(app_data_root, &identity)
        .map_err(|_| MatrixAuthCommandError::unavailable("p3.2-login-store-migration-failed"))?;
    let key_creation_policy = store_paths
        .key_creation_policy()
        .map_err(|_| MatrixAuthCommandError::unavailable("p3.2-login-store-migration-failed"))?;

    // Revision-aware Keychain/Secret-Service lookup copies a valid legacy key
    // forward before creation. Only a genuinely fresh account root may receive
    // a newly generated key; unavailable or corrupt Keychain data never does.
    let store_key =
        get_or_migrate_store_key(&KeyringStoreKeyVault::new(), &identity, key_creation_policy)
            .map_err(map_store_key_vault_error)?;

    // Run deterministic revision migrations before the SDK opens encrypted
    // SQLite. A corrupt/ahead/missing migration chain is a reset *decision*,
    // never an automatic wipe; only the static safe diagnostic crosses IPC.
    migrate_store_to_current(&store_paths).map_err(map_store_migration_error)?;
    let config =
        ClientBuildConfig::product_default(app_data_root, identity.clone(), Some(store_key))
            .map_err(|_| {
                MatrixAuthCommandError::unavailable("p3.2-login-store-migration-failed")
            })?;
    let client = build_unauthenticated_client(&config)
        .await
        .map_err(map_store_client_build_error)?;
    install_session_rotation_callbacks(&client, identity)?;
    Ok(client)
}

#[derive(Debug)]
struct SessionRotationCallbackError(&'static str);

impl std::fmt::Display for SessionRotationCallbackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for SessionRotationCallbackError {}

fn install_session_rotation_callbacks(
    client: &Client,
    identity: AccountIdentity,
) -> Result<(), MatrixAuthCommandError> {
    let reload_identity = identity.clone();
    let save_identity = identity;
    client
        .set_session_callbacks(
            Box::new(move |_| {
                let material =
                    load_session_material(&KeyringSessionMaterialVault::new(), &reload_identity)
                        .map_err(|_| {
                            SessionRotationCallbackError("d0.1-session-reload-read-failed")
                        })?
                        .ok_or(SessionRotationCallbackError(
                            "d0.1-session-reload-material-missing",
                        ))?;
                let secrets = material.decode_host_secrets().map_err(|_| {
                    SessionRotationCallbackError("d0.1-session-reload-decode-failed")
                })?;
                let session = matrix_session_from_host_secrets(&reload_identity, &secrets)
                    .map_err(|_| SessionRotationCallbackError("d0.1-session-reload-invalid"))?;
                Ok(session.tokens)
            }),
            Box::new(move |client| {
                persist_session_after_login(
                    &client,
                    &save_identity,
                    &KeyringSessionMaterialVault::new(),
                )
                .map_err(|_| {
                    SessionRotationCallbackError("d0.1-session-rotation-persist-failed")
                })?;
                Ok(())
            }),
        )
        .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-session-callback-install-failed"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn store_recovery_requires_exact_typed_confirmation_and_one_use_csprng_id() {
        let state = MatrixAuthState::new();
        let identity = AccountIdentity::new("@alice:example.org", "https://matrix.example.org")
            .expect("test identity");
        state.arm_store_recovery(identity.clone()).await;

        let confirmation_id = state
            .prepare_store_recovery_confirmation()
            .await
            .expect("failed login may prepare recovery confirmation");
        assert_eq!(
            confirmation_id.len(),
            STORE_RECOVERY_CONFIRMATION_ID_BYTES * 2
        );
        assert!(is_store_recovery_confirmation_id(&confirmation_id));
        assert!(confirmation_id.bytes().all(|byte| byte.is_ascii_hexdigit()));

        for malformed_or_wrong_text in ["", "archive", "ARCHIVE ", "ARCHIVE\0"] {
            let wrong = state
                .take_confirmed_store_recovery(&confirmation_id, malformed_or_wrong_text)
                .await
                .expect_err("wrong typed acknowledgement must not authorize archive");
            assert_eq!(
                wrong.diagnostic_id,
                "p3.2-login-store-recovery-confirmation-required"
            );
        }
        let wrong_id = state
            .take_confirmed_store_recovery(
                &"0".repeat(STORE_RECOVERY_CONFIRMATION_ID_BYTES * 2),
                STORE_RECOVERY_TYPED_CONFIRMATION_TEXT,
            )
            .await
            .expect_err("a guessable/fixed confirmation must not archive a store");
        assert_eq!(
            wrong_id.diagnostic_id,
            "p3.2-login-store-recovery-confirmation-required"
        );
        assert_eq!(
            state
                .take_confirmed_store_recovery(
                    &confirmation_id,
                    STORE_RECOVERY_TYPED_CONFIRMATION_TEXT,
                )
                .await
                .expect("both exact confirmations must resolve the native target"),
            identity
        );
        let replay = state
            .take_confirmed_store_recovery(&confirmation_id, STORE_RECOVERY_TYPED_CONFIRMATION_TEXT)
            .await
            .expect_err("confirmation capability must not replay");
        assert_eq!(
            replay.diagnostic_id,
            "p3.2-login-store-recovery-confirmation-required"
        );
    }

    #[tokio::test]
    async fn invalid_typed_recovery_confirmation_leaves_store_unarchived() {
        let root = std::env::temp_dir().join(format!(
            "synara-store-recovery-typed-confirmation-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temporary root");
        let identity = AccountIdentity::new("@alice:example.org", "https://matrix.example.org")
            .expect("test identity");
        let paths = StorePaths::derive(&root, &identity).expect("store paths");
        paths.ensure_dirs().expect("initial layout");
        let state_file = paths.state_dir().join("state.sqlite");
        fs::write(&state_file, b"must-not-archive").expect("state fixture");

        let state = MatrixAuthState::new();
        state.arm_store_recovery(identity).await;
        let confirmation_id = state
            .prepare_store_recovery_confirmation()
            .await
            .expect("failed login may prepare recovery confirmation");
        let error = state
            .take_confirmed_store_recovery(&confirmation_id, "ARCHIVE ")
            .await
            .expect_err("wrong typed acknowledgement must fail before archive");
        assert_eq!(
            error.diagnostic_id,
            "p3.2-login-store-recovery-confirmation-required"
        );
        assert_eq!(
            fs::read(&state_file).expect("live state remains"),
            b"must-not-archive"
        );
        assert!(
            !paths
                .account_root()
                .join(crate::matrix::store::STORE_RECOVERY_ARCHIVE_SEGMENT)
                .exists(),
            "a rejected typed acknowledgement must not create an archive"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn store_recovery_revocation_invalidates_pending_and_awaiting_capabilities() {
        let state = MatrixAuthState::new();
        let identity = AccountIdentity::new("@alice:example.org", "https://matrix.example.org")
            .expect("test identity");

        state.arm_store_recovery(identity.clone()).await;
        state.clear_store_recovery().await;
        let pending = state
            .prepare_store_recovery_confirmation()
            .await
            .expect_err("a session transition must revoke pending recovery");
        assert_eq!(
            pending.diagnostic_id,
            "p3.2-login-store-recovery-not-pending"
        );

        state.arm_store_recovery(identity).await;
        let confirmation_id = state
            .prepare_store_recovery_confirmation()
            .await
            .expect("recovery may be prepared before a session transition");
        state.clear_store_recovery().await;
        let awaiting = state
            .take_confirmed_store_recovery(&confirmation_id, STORE_RECOVERY_TYPED_CONFIRMATION_TEXT)
            .await
            .expect_err("an old recovery confirmation must not survive a session transition");
        assert_eq!(
            awaiting.diagnostic_id,
            "p3.2-login-store-recovery-confirmation-required"
        );
    }

    #[test]
    fn archive_and_rebuild_store_moves_all_local_components_without_key_operations() {
        let root = std::env::temp_dir().join(format!(
            "synara-store-recovery-command-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temporary root");
        let identity = AccountIdentity::new("@alice:example.org", "https://matrix.example.org")
            .expect("test identity");
        let paths = StorePaths::derive(&root, &identity).expect("store paths");
        paths.ensure_dirs().expect("initial layout");
        fs::write(paths.state_dir().join("state.sqlite"), b"state").expect("state fixture");
        fs::write(paths.crypto_dir().join("crypto.sqlite"), b"crypto").expect("crypto fixture");
        fs::write(paths.cache_dir().join("cache.sqlite"), b"cache").expect("cache fixture");
        fs::write(paths.media_dir().join("media.bin"), b"media").expect("media fixture");

        archive_and_rebuild_store(&root, &identity).expect("archive-and-rebuild should succeed");

        for directory in [
            paths.state_dir(),
            paths.crypto_dir(),
            paths.cache_dir(),
            paths.media_dir(),
        ] {
            assert!(directory.is_dir(), "current layout is rebuilt");
            assert_eq!(
                fs::read_dir(directory)
                    .expect("rebuilt directory is readable")
                    .count(),
                0,
                "rebuild has no copied live files"
            );
        }
        let archive = paths
            .account_root()
            .join(crate::matrix::store::STORE_RECOVERY_ARCHIVE_SEGMENT);
        let archived = fs::read_dir(&archive)
            .expect("recovery archive exists")
            .next()
            .expect("one archive is created")
            .expect("archive entry")
            .path();
        for name in ["state", "crypto", "cache", "media"] {
            assert!(
                archived.join(name).is_dir(),
                "{name} is archived, not deleted"
            );
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn store_recovery_ipc_is_explicit_registered_and_privacy_limited() {
        let commands = include_str!("product_commands.rs");
        let production = commands
            .split("#[cfg(test)]")
            .next()
            .expect("production command source");
        let lib = include_str!("../../lib.rs");
        let build = include_str!("../../../build.rs");
        let capability = include_str!("../../../capabilities/main.json");
        let schemas = [
            include_str!("../../../gen/schemas/desktop-schema.json"),
            include_str!("../../../gen/schemas/linux-schema.json"),
            include_str!("../../../gen/schemas/macOS-schema.json"),
        ];
        for command in [
            "matrix_store_recovery_prepare",
            "matrix_store_recovery_confirm",
        ] {
            let permission = command.replace('_', "-");
            assert!(production.contains(&format!("pub async fn {command}")));
            assert!(lib.contains(command));
            assert!(build.contains(&format!("\"{command}\"")));
            assert!(capability.contains(&format!("allow-{permission}")));
            assert!(include_str!(
                "../../../permissions/autogenerated/matrix_store_recovery_prepare.toml"
            )
            .contains("matrix_store_recovery_prepare"));
            assert!(include_str!(
                "../../../permissions/autogenerated/matrix_store_recovery_confirm.toml"
            )
            .contains("matrix_store_recovery_confirm"));
            for schema in schemas {
                assert!(schema.contains(command));
            }
        }

        let login = production
            .split("pub async fn matrix_login_password")
            .nth(1)
            .and_then(|rest| rest.split("pub async fn ").next())
            .expect("normal login command body");
        assert!(login.contains("arm_store_recovery"));
        assert!(
            !login.contains("reset_store_for_recovery"),
            "normal login must only arm recovery; it must never archive/reset automatically"
        );
        let recovery = production
            .split("pub async fn matrix_store_recovery_confirm")
            .nth(1)
            .and_then(|rest| rest.split("pub async fn ").next())
            .expect("explicit confirmation command body");
        assert!(recovery.contains("confirmation_text: String"));
        assert!(recovery
            .contains("take_confirmed_store_recovery(&confirmation_id, &confirmation_text)"));
        assert!(recovery.contains("take_confirmed_store_recovery"));
        assert!(recovery.contains("archive_and_rebuild_store"));
        assert!(
            recovery.find("take_confirmed_store_recovery")
                < recovery.find("archive_and_rebuild_store"),
            "the host must validate both confirmations before filesystem recovery"
        );
        assert_eq!(STORE_RECOVERY_TYPED_CONFIRMATION_TEXT, "ARCHIVE");
        let archive = production
            .split("fn archive_and_rebuild_store")
            .nth(1)
            .and_then(|rest| rest.split("fn map_store_migration_error").next())
            .expect("archive helper body");
        for forbidden in [
            "get_or_migrate_store_key",
            "get_or_create_store_key",
            ".delete(",
        ] {
            assert!(
                !archive.contains(forbidden),
                "explicit recovery must not change #695 key-generation policy ({forbidden})"
            );
        }
        // Examine executable source only: adjacent API documentation may
        // legitimately mention a password-reset command without making it a
        // recovery IPC input or operation.
        let recovery_implementation = recovery
            .lines()
            .filter(|line| !line.trim_start().starts_with("///"))
            .collect::<Vec<_>>()
            .join("\n");
        for forbidden in ["password", "access_token", "refresh_token", "StoreKey"] {
            assert!(
                !recovery_implementation.contains(forbidden),
                "recovery IPC must not expose or operate on {forbidden}"
            );
        }
        let challenge = serde_json::to_string(&MatrixStoreRecoveryChallenge {
            confirmation_id: "a".repeat(STORE_RECOVERY_CONFIRMATION_ID_BYTES * 2),
        })
        .expect("recovery challenge serializes");
        let result = serde_json::to_string(&MatrixStoreRecoveryResult {
            status: "archived_and_rebuilt",
        })
        .expect("recovery result serializes");
        for wire in [challenge, result] {
            for forbidden in [
                "accessToken",
                "refreshToken",
                "password",
                "userId",
                "homeserver",
                "path",
                "key",
            ] {
                assert!(
                    !wire
                        .to_ascii_lowercase()
                        .contains(&forbidden.to_ascii_lowercase()),
                    "recovery IPC result must not expose {forbidden}"
                );
            }
        }
    }

    #[test]
    fn session_install_and_every_logout_path_revoke_store_recovery() {
        let production = include_str!("product_commands.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("production command source");
        let password_install = production
            .split("pub async fn matrix_login_password")
            .nth(1)
            .and_then(|rest| rest.split("pub async fn ").next())
            .expect("password login body");
        let register_install = production
            .split("pub(super) async fn install_session_from_register_secrets")
            .nth(1)
            .and_then(|rest| rest.split("#[tauri::command]").next())
            .expect("register install body");
        let restore_install = production
            .split("pub async fn matrix_restore_session")
            .nth(1)
            .and_then(|rest| rest.split("/// Build the hybrid desktop").next())
            .expect("restore session body");
        for (label, install) in [
            ("password", password_install),
            ("register", register_install),
            ("restore", restore_install),
        ] {
            assert!(
                install.contains("state.clear_store_recovery().await"),
                "{label} session installation must revoke stale recovery"
            );
        }

        let logout = production
            .split("pub async fn matrix_logout")
            .nth(1)
            .and_then(|rest| rest.split("pub async fn ").next())
            .expect("logout body");
        let revocation = logout
            .find("state.clear_store_recovery().await")
            .expect("logout must revoke recovery even when already logged out");
        let logged_out = logout
            .find("let Some(active) = session.as_ref() else")
            .expect("logout's logged-out branch");
        assert!(
            revocation < logged_out,
            "recovery must be revoked before the already-logged-out return"
        );
    }

    #[test]
    fn store_recovery_diagnostics_are_fixed_allowlisted_ids() {
        assert!(is_recoverable_store_login_diagnostic(
            "p3.2-login-store-reset-required"
        ));
        assert!(is_recoverable_store_login_diagnostic(
            "p3.2-login-store-migration-required"
        ));
        assert!(!is_recoverable_store_login_diagnostic(
            "p3.2-login-store-open-failed"
        ));
        let failed = archive_and_rebuild_store(
            std::path::Path::new("relative-root-is-refused"),
            &AccountIdentity::new("@alice:example.org", "https://matrix.example.org")
                .expect("test identity"),
        )
        .expect_err("relative roots fail closed");
        assert_eq!(failed.diagnostic_id, "p3.2-login-store-recovery-failed");
        assert!(
            !failed.message.contains("relative-root-is-refused"),
            "raw paths never reach a recovery command result"
        );
    }

    #[test]
    fn store_recovery_maps_only_static_login_diagnostics() {
        let reset = map_store_migration_error(StoreMigrationError::CorruptManifest);
        assert_eq!(reset.diagnostic_id, "p3.2-login-store-reset-required");
        let migration = map_store_migration_error(StoreMigrationError::RevisionAhead {
            observed: 2,
            known: 1,
        });
        assert_eq!(
            migration.diagnostic_id,
            "p3.2-login-store-migration-required"
        );
        let failed =
            map_store_migration_error(StoreMigrationError::StepFailed { step_id: "r2-test" });
        assert_eq!(failed.diagnostic_id, "p3.2-login-store-migration-failed");

        let locked = map_store_key_vault_error(StoreKeyVaultError::BackendUnavailable {
            diagnostic_id: "r0.4-keyring-platform-failure",
        });
        assert_eq!(locked.diagnostic_id, "p3.2-login-store-locked");
        let corrupt = map_store_key_vault_error(StoreKeyVaultError::CorruptPayload);
        assert_eq!(corrupt.diagnostic_id, "p3.2-login-store-reset-required");
        let missing_existing =
            map_store_key_vault_error(StoreKeyVaultError::MissingKeyForExistingStore);
        assert_eq!(
            missing_existing.diagnostic_id,
            "p3.2-login-store-reset-required"
        );
    }

    #[test]
    fn sdk_store_build_errors_preserve_locked_vs_open_failed_boundary() {
        let locked = map_store_client_build_error(ClientBuilderError::SdkBuild {
            category: crate::matrix::ipc::MatrixIpcErrorCategory::StoreLocked,
            diagnostic_id: "p2.3-sdk-build-store-locked",
            message: "store is locked".into(),
        });
        assert_eq!(locked.diagnostic_id, "p3.2-login-store-locked");
        let unavailable = map_store_client_build_error(ClientBuilderError::SdkBuild {
            category: crate::matrix::ipc::MatrixIpcErrorCategory::StoreUnavailable,
            diagnostic_id: "p2.3-sdk-build-store",
            message: "store initialization failed".into(),
        });
        assert_eq!(unavailable.diagnostic_id, "p3.2-login-store-open-failed");
    }
}
