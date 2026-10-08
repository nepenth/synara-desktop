use super::*;
use crate::matrix::user_profile::NativeOwnProfileOwner;
use synara_core::app::media_cache::NativeMediaRetentionOwner;

/// V-AUTH.3 desktop compatibility re-exports for the shared command response.
pub use synara_core::app::auth::MatrixLoginFlowsResponse;

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

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn matrix_login_password(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
    core: State<'_, Arc<synara_core::Core>>,
    homeserver_url: String,
    user: String,
    password: String,
    indexed_message_search: Option<bool>,
) -> Result<MatrixLoginIdentity, MatrixAuthCommandError> {
    let password = zeroize::Zeroizing::new(password);
    let mut install = state.begin_install().await;
    settle_pending_logout_cleanup_for_install(&app, &install);
    if install.current().is_some() {
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
    let existing_device_id =
        match existing_login_device_id(&app_data_root, &requested_identity).await {
            Ok(device_id) => device_id,
            Err(error) => {
                if is_recoverable_store_login_diagnostic(&error.diagnostic_id) {
                    state.arm_store_recovery(requested_identity.clone()).await;
                }
                return Err(error);
            }
        };
    let indexed_message_search = indexed_message_search.unwrap_or(true);
    let (client, session_persistence) = match build_client(
        &app_data_root,
        requested_identity.clone(),
        indexed_message_search,
    )
    .await
    {
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

    let result = match finish_password_login_attempt(
        login_with_password(
            &client,
            requested_identity.user_id(),
            password.as_str(),
            &LoginOptions {
                request_refresh_token: true,
                device_id: existing_device_id,
                ..LoginOptions::default()
            },
        )
        .await,
        &client,
        &session_persistence.callback_lease(),
        || clear_persisted_logout_material(&KeyringSessionMaterialVault::new(), &app_data_root),
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            if is_recoverable_store_login_diagnostic(&error.diagnostic_id) {
                state.arm_store_recovery(requested_identity.clone()).await;
            }
            return Err(error);
        }
    };

    let live_identity = accept_authenticated_identity(
        authenticated_login_identity(&result.user_id, &result.homeserver_url, &requested_identity),
        || async {
            let _ = bounded_remote_logout(
                VOLUNTARY_REMOTE_LOGOUT_TIMEOUT,
                client.matrix_auth().logout(),
            )
            .await;
        },
    )
    .await?;

    let identity = MatrixLoginIdentity {
        user_id: result.user_id,
        device_id: result.device_id,
        homeserver_url: live_identity.homeserver_url().to_owned(),
    };
    let mut partial = SessionPreparationRollback::new(
        &client,
        &session_persistence,
        identity.clone(),
        SessionInstallOrigin::NewAuthentication,
    );
    let preparation = async {
        ensure_crypto_ready(&client).await?;
        let session_generation = state.next_generation();
        let owners = start_desktop_owners(
            &client,
            &app,
            session_generation,
            indexed_message_search,
            &mut partial,
        )
        .await?;
        persist_with_client_lease(
            session_persistence.lease(),
            || ensure_logout_retry_locator(&app_data_root, &identity),
            || {
                persist_session_after_login(
                    &client,
                    &live_identity,
                    &KeyringSessionMaterialVault::new(),
                )
                .map(|_| ())
                .map_err(|_| MatrixAuthCommandError::unavailable("d0.1-session-persist-failed"))
            },
            || Ok(()), // full preparation rollback performs credential cleanup once
        )?;
        crate::desktop_secret_store::clear_legacy_renderer_session_credentials();

        let (managed, notification_decisions) = owners.into_session(
            client,
            session_persistence,
            identity.clone(),
            session_generation,
            &app,
        );
        install.publish(managed).await;
        Ok(notification_decisions)
    }
    .await;
    let notification_decisions = finish_session_preparation(preparation, || {
        partial.rollback(core.inner().as_ref(), &app_data_root)
    })
    .await?;
    let wiring = wire_desktop_session(
        core.inner().as_ref(),
        install.current().expect("session installed above"),
        notification_decisions,
    )
    .await;
    finish_session_wiring(wiring, || {
        rollback_session_install(
            core.inner().as_ref(),
            install.slot_mut(),
            &app_data_root,
            SessionInstallOrigin::NewAuthentication,
        )
    })
    .await?;
    drop(install);
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
    let _transition = state.lock_transition().await;
    let session = state.session.lock().await;
    if session.is_some() {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "A native Matrix session is already logged in.",
            "d0.1-session-already-active",
        ));
    }
    let (app_data_root, identity) = confirmed_store_recovery_target(
        &state,
        app_data_root(&app),
        &confirmation_id,
        &confirmation_text,
    )
    .await?;
    archive_and_rebuild_store(&app_data_root, &identity)?;
    drop(session);

    Ok(MatrixStoreRecoveryResult {
        status: "archived_and_rebuilt",
    })
}

pub(super) async fn confirmed_store_recovery_target(
    state: &MatrixAuthState,
    root: Result<PathBuf, MatrixAuthCommandError>,
    confirmation_id: &str,
    confirmation_text: &str,
) -> Result<(PathBuf, AccountIdentity), MatrixAuthCommandError> {
    let root = root?;
    let identity = state
        .take_confirmed_store_recovery(confirmation_id, confirmation_text)
        .await?;
    // After this point the capability stays consumed, including partial archive
    // failure. Replaying it could apply a second archive to a changed layout.
    Ok((root, identity))
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
    let mut install = state.begin_install().await;
    settle_pending_logout_cleanup_for_install(&app, &install);
    if install.current().is_some() {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "A native Matrix session is already logged in.",
            "v-auth.4b-session-already-active",
        ));
    }

    let app_data_root = app_data_root(&app)?;
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
            let mut primary_armed = false;
            let installed = install_session_from_register_secrets(
                &app,
                &state,
                &mut install,
                &secrets,
                core.inner().as_ref(),
                &mut primary_armed,
            )
            .await;
            let (identity, _session_generation, notification_decisions) =
                finish_registration_install_attempt(installed, primary_armed, || async {
                    best_effort_revoke_uninstalled_registration(&secrets).await;
                })
                .await?;
            let wiring = wire_desktop_session(
                core.inner().as_ref(),
                install.current().expect("registration installed above"),
                notification_decisions,
            )
            .await;
            finish_session_wiring(wiring, || {
                rollback_session_install(
                    core.inner().as_ref(),
                    install.slot_mut(),
                    &app_data_root,
                    SessionInstallOrigin::NewAuthentication,
                )
            })
            .await?;
            drop(install);
            Ok(MatrixRegisterOutcome::Complete { identity })
        }
    }
}

pub(super) async fn finish_registration_install_attempt<T, Compensate, CompensateFuture>(
    installed: Result<T, MatrixAuthCommandError>,
    primary_armed: bool,
    compensate: Compensate,
) -> Result<T, MatrixAuthCommandError>
where
    Compensate: FnOnce() -> CompensateFuture,
    CompensateFuture: std::future::Future<Output = ()>,
{
    if installed.is_err() && !primary_armed {
        compensate().await;
    }
    installed
}

async fn best_effort_revoke_uninstalled_registration(
    secrets: &super::super::RegisterCompleteSecrets,
) {
    // Raw /register returns tokens without applying them to its unauthenticated
    // synthetic client. Only this compensation path constructs a memory client;
    // no product/synthetic store or credential callback is opened or installed.
    let _ = revoke_uninstalled_registration(secrets).await;
}

pub(super) async fn revoke_uninstalled_registration(
    secrets: &super::super::RegisterCompleteSecrets,
) -> Result<(), MatrixAuthCommandError> {
    let homeserver_url =
        normalize_homeserver_url(&secrets.homeserver_url).map_err(map_register_auth_error)?;
    let account =
        AccountIdentity::new(&secrets.user_id, homeserver_url.as_str()).map_err(|_| {
            MatrixAuthCommandError::invalid_input("v-auth.4b-register-identity-invalid")
        })?;
    // Config path metadata is unused in memory mode. This absolute temp label
    // does not create a directory and never points at the product layout.
    let unused_root = std::env::temp_dir().join(format!(
        "synara-registration-compensation-metadata-{}",
        std::process::id()
    ));
    let config =
        ClientBuildConfig::product_default(&unused_root, account.clone(), None).map_err(|_| {
            MatrixAuthCommandError::unavailable("v-auth.4b-register-compensation-build-failed")
        })?;
    let client = synara_core::app::client_builder::build_memory_only_client(&config)
        .await
        .map_err(|_| {
            MatrixAuthCommandError::unavailable("v-auth.4b-register-compensation-build-failed")
        })?;
    let material = SessionMaterial::from_matrix_tokens(
        &account,
        secrets.device_id.as_str(),
        secrets.access_token.as_str(),
        secrets.refresh_token.as_ref().map(|token| token.as_str()),
    )
    .map_err(|_| {
        MatrixAuthCommandError::unavailable("v-auth.4b-register-compensation-restore-failed")
    })?;
    restore_session_onto_client(&client, &account, &material)
        .await
        .map_err(|_| {
            MatrixAuthCommandError::unavailable("v-auth.4b-register-compensation-restore-failed")
        })?;
    if bounded_remote_logout(
        VOLUNTARY_REMOTE_LOGOUT_TIMEOUT,
        client.matrix_auth().logout(),
    )
    .await
    {
        Ok(())
    } else {
        Err(MatrixAuthCommandError::unavailable(
            "v-auth.4b-register-compensation-logout-failed",
        ))
    }
}

pub(super) async fn install_session_from_register_secrets(
    app: &AppHandle,
    state: &State<'_, MatrixAuthState>,
    install: &mut session::SessionInstallGuard<'_, ManagedMatrixSession>,
    secrets: &super::super::RegisterCompleteSecrets,
    core: &synara_core::Core,
    primary_armed: &mut bool,
) -> Result<
    (
        MatrixLoginIdentity,
        u64,
        Arc<synara_core::app::notifications::NativeNotificationDecisionOwner>,
    ),
    MatrixAuthCommandError,
> {
    let homeserver_url = normalize_homeserver_url(&secrets.homeserver_url)
        .map_err(map_register_auth_error)?
        .into_string();
    let live_identity = AccountIdentity::new(&secrets.user_id, &homeserver_url).map_err(|_| {
        MatrixAuthCommandError::invalid_input("v-auth.4b-register-identity-invalid")
    })?;
    let app_data_root = app_data_root(app)?;
    let indexed_message_search = true;
    let (client, session_persistence) = build_client(
        &app_data_root,
        live_identity.clone(),
        indexed_message_search,
    )
    .await?;

    let identity = MatrixLoginIdentity {
        user_id: secrets.user_id.clone(),
        device_id: secrets.device_id.clone(),
        homeserver_url,
    };
    let mut partial = SessionPreparationRollback::new(
        &client,
        &session_persistence,
        identity.clone(),
        SessionInstallOrigin::NewAuthentication,
    );
    let preparation = async {
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
        let restored = restore_session_onto_client(&client, &live_identity, &material).await;
        *primary_armed = client.session().is_some();
        restored.map_err(|_| {
            MatrixAuthCommandError::new(
                "Unknown",
                "Failed to restore the native Matrix session after registration.",
                "v-auth.4b-register-restore-failed",
            )
        })?;

        ensure_crypto_ready(&client).await?;
        let session_generation = state.next_generation();
        let owners = start_desktop_owners(
            &client,
            app,
            session_generation,
            indexed_message_search,
            &mut partial,
        )
        .await?;
        persist_with_client_lease(
            session_persistence.lease(),
            || ensure_logout_retry_locator(&app_data_root, &identity),
            || {
                persist_session_after_login(
                    &client,
                    &live_identity,
                    &KeyringSessionMaterialVault::new(),
                )
                .map(|_| ())
                .map_err(|_| {
                    MatrixAuthCommandError::unavailable("v-auth.4b-session-persist-failed")
                })
            },
            || Ok(()), // full preparation rollback performs credential cleanup once
        )?;
        crate::desktop_secret_store::clear_legacy_renderer_session_credentials();

        let (managed, notification_decisions) = owners.into_session(
            client,
            session_persistence,
            identity.clone(),
            session_generation,
            app,
        );
        install.publish(managed).await;
        Ok((identity, session_generation, notification_decisions))
    }
    .await;
    finish_session_preparation(preparation, || partial.rollback(core, &app_data_root)).await
}

/// SNC-P3.2 — forward the existing read-only React session snapshot through
/// the managed Core. The desktop session owner remains private to every other
/// command; only this command consumes Core's stable envelope response.
#[tauri::command]
pub async fn matrix_session_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<MatrixSessionSnapshot, MatrixAuthCommandError> {
    crate::bridge::session_lifecycle::session_snapshot(core.inner().as_ref())
}

/// Return only the persisted account identity used to decide whether the
/// native client route should mount. This deliberately does not restore the
/// SDK client, start sync, or expose session credentials.
#[tauri::command]
pub async fn matrix_session_identity(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
) -> Result<Option<MatrixLoginIdentity>, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    if let Some(active) = session.as_ref() {
        return Ok(Some(active.identity.clone()));
    }
    drop(session);
    // A retired identity whose credential delete failed is not a sign-in.
    if state.has_pending_cleanup() {
        return Ok(None);
    }

    let root = app_data_root(&app)?;
    if !active_identity_path(&root).is_file() {
        return Ok(None);
    }
    read_active_identity(&root).map(Some)
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

/// Restart the live SyncService after OS sleep / network resume. The SDK can
/// keep reporting Running on a dead long-poll; this command applies Resume
/// against the desktop-owned owner rather than only refreshing status.
#[tauri::command]
pub async fn matrix_sync_recover(
    state: State<'_, MatrixAuthState>,
) -> Result<SyncReadinessSnapshot, MatrixAuthCommandError> {
    state
        .recover_sync_after_wake()
        .await
        .map_err(|error| map_sync_error(error.diagnostic_id()))
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
    expected_session_generation: Option<u64>,
) -> Result<MatrixSessionSnapshot, MatrixAuthCommandError> {
    // The owner's transition gate serializes this teardown with every other
    // logout (including the rejection watcher), restore, login and register.
    // The session mutex is held only to validate and take the session.
    let root = app_data_root(&app);
    let orphan_root = root.clone();
    state
        .logout(
            |active| {
                if let Some(expected) = expected_session_generation {
                    if active.sync.session_generation() != expected
                        || active.sync.observe().failure_diagnostic_id
                            != Some(
                                synara_core::app::sync::SYNC_AUTHENTICATION_FAILURE_DIAGNOSTIC_ID,
                            )
                    {
                        return Err(MatrixAuthCommandError::new(
                            "InvalidRequest",
                            "The rejected session has already changed.",
                            "d0.1-session-rejection-stale",
                        ));
                    }
                }
                // Resolve the root while the live session still retains the
                // retry identity. Preflight failure leaves this session installed.
                let root = root.clone()?;
                ensure_logout_retry_locator(&root, &active.identity)?;
                Ok(LogoutPlan {
                    root,
                    remote_logout_allowed: remote_logout_allowed(
                        expected_session_generation,
                        active.sync.observe().failure_diagnostic_id,
                    ),
                })
            },
            // Even a path-resolution failure must not skip retirement of stale Core.
            || {
                finish_orphan_logout(
                    crate::bridge::session_lifecycle::close_after_desktop_session_removal(
                        core.inner().as_ref(),
                    ),
                    || {
                        orphan_root.and_then(|root| {
                            clear_persisted_logout_material(
                                &KeyringSessionMaterialVault::new(),
                                &root,
                            )
                            .inspect_err(|_| {
                                if let Ok(identity) = read_active_identity(&root) {
                                    state.record_pending_cleanup(identity);
                                }
                            })
                        })
                    },
                )
            },
            |active, plan| {
                if let Some(expected) = expected_session_generation {
                    let _ = app.emit("matrix-session-expired", expected);
                }
                let persistence_lease = active.session_persistence.callback_lease();
                let client = active.client.clone();
                let sync = Arc::clone(&active.sync);
                let identity = active.identity.clone();
                let join_rules = Arc::clone(&active.join_rules);
                let observations = Arc::clone(&active.notification_observations);
                let widgets = Arc::clone(&active.widgets);
                let root = plan.root;
                let state = &state;
                let core = &core;
                finish_taken_session_logout(
                    active,
                    VOLUNTARY_REMOTE_LOGOUT_TIMEOUT,
                    move || {
                        persistence_lease.revoke();
                    },
                    // Voluntary only: give pending room keys a bounded chance to
                    // reach the server backup, then one `/logout`, all inside the
                    // remote bound.
                    plan.remote_logout_allowed.then_some(move || async move {
                        let _ = session::wait_for_backup_steady_state(
                            &client,
                            session::LOGOUT_BACKUP_STEADY_STATE_TIMEOUT,
                        )
                        .await;
                        client.matrix_auth().logout().await
                    }),
                    || async move {
                        join_rules.retire();
                        observations.retire();
                        widgets.retire_and_close().await;
                        sync.stop()
                            .await
                            .map(|_| ())
                            .map_err(|error| map_sync_error(error.diagnostic_id()))
                    },
                    move || {
                        clear_native_logout_material(
                            &KeyringSessionMaterialVault::new(),
                            &identity,
                            &root,
                        )
                        .inspect_err(|_| state.record_pending_cleanup(identity.clone()))
                    },
                    move || {
                        crate::bridge::session_lifecycle::close_after_desktop_session_removal(
                            core.inner().as_ref(),
                        )
                    },
                )
            },
        )
        .await?;
    Ok(MatrixSessionSnapshot::LoggedOut)
}

pub(super) struct LogoutPlan {
    root: PathBuf,
    remote_logout_allowed: bool,
}

use synara_core::app::lifecycle::session;
pub(super) use synara_core::app::lifecycle::session::remote_logout_allowed;

pub(super) use synara_core::app::lifecycle::session::{
    bounded_remote_logout, finish_orphan_logout, finish_taken_session_logout,
    VOLUNTARY_REMOTE_LOGOUT_TIMEOUT,
};
#[cfg(test)]
pub(super) use synara_core::app::lifecycle::session::{
    finish_active_logout, take_session_for_logout,
};

fn settle_pending_logout_cleanup_for_install(
    app: &AppHandle,
    install: &session::SessionInstallGuard<'_, ManagedMatrixSession>,
) {
    install.settle_pending_cleanup(|identity| {
        app_data_root(app).and_then(|root| {
            clear_native_logout_material(&KeyringSessionMaterialVault::new(), identity, &root)
        })
    });
}

#[tauri::command]
pub async fn matrix_restore_session(
    app: AppHandle,
    state: State<'_, MatrixAuthState>,
    core: State<'_, Arc<synara_core::Core>>,
    indexed_message_search: Option<bool>,
) -> Result<MatrixLoginIdentity, MatrixAuthCommandError> {
    // Waits for an in-flight logout, which then leaves no identity to restore.
    let mut install = state.begin_install().await;
    if let Some(active) = install.current() {
        return Ok(active.identity.clone());
    }

    let app_data_root = app_data_root(&app)?;
    // A retired session whose credential delete failed must not come back.
    // Finish that delete first; the identity read below then fails closed.
    install.retry_pending_cleanup(|identity| {
        clear_native_logout_material(
            &KeyringSessionMaterialVault::new(),
            identity,
            &app_data_root,
        )
    })?;
    let identity = read_active_identity(&app_data_root)?;
    ensure_logout_retry_locator(&app_data_root, &identity)?;
    let account = account_identity(&identity)?;
    let indexed_message_search = indexed_message_search.unwrap_or(true);
    let (client, session_persistence) =
        build_client(&app_data_root, account.clone(), indexed_message_search).await?;
    let mut partial = SessionPreparationRollback::new(
        &client,
        &session_persistence,
        identity.clone(),
        SessionInstallOrigin::Restored,
    );
    let preparation = async {
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
        crate::desktop_secret_store::clear_legacy_renderer_session_credentials();

        ensure_crypto_ready(&client).await?;
        let session_generation = state.next_generation();
        let owners = start_desktop_owners(
            &client,
            &app,
            session_generation,
            indexed_message_search,
            &mut partial,
        )
        .await?;
        // Publishing revokes any stale recovery capability from an earlier
        // failed login.
        let (managed, notification_decisions) = owners.into_session(
            client,
            session_persistence,
            identity.clone(),
            session_generation,
            &app,
        );
        install.publish(managed).await;
        Ok(notification_decisions)
    }
    .await;
    let notification_decisions = finish_session_preparation(preparation, || {
        partial.rollback(core.inner().as_ref(), &app_data_root)
    })
    .await?;
    let wiring = wire_desktop_session(
        core.inner().as_ref(),
        install.current().expect("session installed above"),
        notification_decisions,
    )
    .await;
    finish_session_wiring(wiring, || {
        rollback_session_install(
            core.inner().as_ref(),
            install.slot_mut(),
            &app_data_root,
            SessionInstallOrigin::Restored,
        )
    })
    .await?;
    drop(install);
    Ok(identity)
}

/// Called under the desktop auth gate. Core lifecycle only changes Core's
/// own projection/owner locks and never calls the Platform session adapter.
async fn wire_desktop_session(
    core: &synara_core::Core,
    active: &ManagedMatrixSession,
    notification_decisions: Arc<synara_core::app::notifications::NativeNotificationDecisionOwner>,
) -> Result<(), MatrixAuthCommandError> {
    crate::bridge::session_lifecycle::open_after_desktop_session_install(
        core,
        &active.identity,
        active.sync.session_generation(),
    )
    .await?;
    session::attach_owner_set(
        core,
        session::SessionOwnerSet {
            typing: active.typing.clone(),
            presence: active.presence.clone(),
            rtc_transports: active.rtc_transports.clone(),
            user_status: active.user_status.clone(),
            widgets: Some(active.widgets.clone()),
            verification: active.verification.clone(),
            devices: active.devices.clone(),
            dehydrated_devices: active.dehydrated_devices.clone(),
            join_rules: active.join_rules.clone(),
            image_packs: active._image_packs.clone(),
            http_pusher: None,
            timelines: active.timelines.clone(),
            notification_decisions: Some(notification_decisions),
            sync: active.sync.clone(),
        },
    )
    .map_err(|kind| MatrixAuthCommandError::unavailable(kind.attach_diagnostic_id()))?;
    Ok(())
}

struct SessionPreparationRollback {
    client: Client,
    persistence_lease: Arc<SessionPersistenceLease>,
    identity: MatrixLoginIdentity,
    origin: SessionInstallOrigin,
    sync: Option<Arc<SyncServiceOwner>>,
    widgets: Option<Arc<NativeWidgetOwner>>,
    join_rules: Option<Arc<NativeRoomJoinRuleOwner>>,
    observations: Option<Arc<NativeNotificationObservationOwner>>,
}

impl SessionPreparationRollback {
    fn new(
        client: &Client,
        persistence: &SessionPersistenceOwner,
        identity: MatrixLoginIdentity,
        origin: SessionInstallOrigin,
    ) -> Self {
        Self {
            client: client.clone(),
            persistence_lease: persistence.callback_lease(),
            identity,
            origin,
            sync: None,
            widgets: None,
            join_rules: None,
            observations: None,
        }
    }

    async fn rollback(
        &self,
        core: &synara_core::Core,
        root: &Path,
    ) -> Result<(), MatrixAuthCommandError> {
        finish_install_rollback(
            self.origin,
            &self.persistence_lease,
            || async {
                if self.client.session().is_some() {
                    let _ = bounded_remote_logout(
                        VOLUNTARY_REMOTE_LOGOUT_TIMEOUT,
                        self.client.matrix_auth().logout(),
                    )
                    .await;
                }
            },
            || async {
                if let Some(owner) = &self.join_rules {
                    owner.retire();
                }
                if let Some(owner) = &self.observations {
                    owner.retire();
                }
                if let Some(owner) = &self.widgets {
                    owner.retire_and_close().await;
                }
                if let Some(owner) = &self.sync {
                    owner
                        .stop()
                        .await
                        .map(|_| ())
                        .map_err(|error| map_sync_error(error.diagnostic_id()))?;
                }
                Ok(())
            },
            || {
                clear_native_logout_material(
                    &KeyringSessionMaterialVault::new(),
                    &self.identity,
                    root,
                )
            },
            || {}, // no live desktop session was published by failed preparation
            || crate::bridge::session_lifecycle::close_after_desktop_session_removal(core),
        )
        .await
    }
}

/// SDK login may install tokens before local activation fails and before
/// session metadata exists. Retire the same lease and origin coordinator even
/// on this earlier error; a validation/HTTP failure has no token to revoke.
pub(super) async fn finish_password_login_attempt<T>(
    attempt: Result<T, AuthError>,
    client: &Client,
    persistence_lease: &SessionPersistenceLease,
    cleanup: impl FnOnce() -> Result<(), MatrixAuthCommandError>,
) -> Result<T, MatrixAuthCommandError> {
    match attempt {
        Ok(result) => Ok(result),
        Err(error) => {
            finish_install_rollback(
                SessionInstallOrigin::NewAuthentication,
                persistence_lease,
                || async {
                    // access_token() returns an owned secret; zeroize this
                    // presence-check copy immediately and never expose it.
                    if client.access_token().map(zeroize::Zeroizing::new).is_some() {
                        let _ = bounded_remote_logout(
                            VOLUNTARY_REMOTE_LOGOUT_TIMEOUT,
                            client.matrix_auth().logout(),
                        )
                        .await;
                    }
                },
                || async { Ok(()) },
                cleanup,
                || {}, // no local owners or live/Core session were published
                || async { Ok(()) },
            )
            .await?;
            Err(map_auth_error(error))
        }
    }
}

pub(super) use synara_core::app::lifecycle::session::{
    finish_install_rollback, finish_session_preparation, finish_session_wiring,
    SessionInstallOrigin,
};

async fn rollback_session_install(
    core: &synara_core::Core,
    session: &mut Option<ManagedMatrixSession>,
    root: &Path,
    origin: SessionInstallOrigin,
) -> Result<(), MatrixAuthCommandError> {
    let active = session
        .as_ref()
        .expect("failed tentative install is present");
    let persistence_lease = active.session_persistence.callback_lease();
    let client = active.client.clone();
    let sync = active.sync.clone();
    let identity = active.identity.clone();
    let join_rules = active.join_rules.clone();
    let observations = active.notification_observations.clone();
    let widgets = active.widgets.clone();
    finish_install_rollback(
        origin,
        &persistence_lease,
        || async {
            let _ = bounded_remote_logout(
                VOLUNTARY_REMOTE_LOGOUT_TIMEOUT,
                client.matrix_auth().logout(),
            )
            .await;
        },
        || async move {
            join_rules.retire();
            observations.retire();
            widgets.retire_and_close().await;
            sync.stop()
                .await
                .map(|_| ())
                .map_err(|error| map_sync_error(error.diagnostic_id()))
        },
        || clear_native_logout_material(&KeyringSessionMaterialVault::new(), &identity, root),
        || {
            *session = None;
        },
        || crate::bridge::session_lifecycle::close_after_desktop_session_removal(core),
    )
    .await
}

pub(super) use synara_core::app::lifecycle::session::{
    persist_with_client_lease, SessionPersistenceLease, SessionPersistenceOwner,
};

pub(super) fn authenticated_login_identity(
    user_id: &str,
    homeserver_url: &str,
    requested: &AccountIdentity,
) -> Result<AccountIdentity, MatrixAuthCommandError> {
    let normalized = normalize_homeserver_url(homeserver_url)
        .map_err(|_| MatrixAuthCommandError::invalid_input("d0.1-login-identity-invalid"))?;
    let identity = AccountIdentity::new(user_id, normalized.as_str())
        .map_err(|_| MatrixAuthCommandError::invalid_input("d0.1-login-identity-invalid"))?;
    let live_url = url::Url::parse(identity.homeserver_url())
        .map_err(|_| MatrixAuthCommandError::invalid_input("d0.1-login-identity-invalid"))?;
    let requested_url = url::Url::parse(requested.homeserver_url())
        .map_err(|_| MatrixAuthCommandError::invalid_input("d0.1-login-identity-invalid"))?;
    if identity.user_id() != requested.user_id() || live_url != requested_url {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "The authenticated Matrix identity did not match the requested account.",
            "d0.1-login-identity-mismatch",
        ));
    }
    // URL equivalence must not silently rekey an existing account layout or
    // Keyring lookup: keep the exact requested account identity on success.
    Ok(requested.clone())
}

pub(super) use synara_core::app::lifecycle::session::accept_authenticated_identity;
#[cfg(test)]
pub(super) use synara_core::app::lifecycle::session::with_generation_bound_acceptance;

/// Every desktop owner a newly installed session needs before Core wiring.
/// Login, registration and restore all start them through
/// [`start_desktop_owners`], in one order.
struct DesktopSessionOwners {
    verification: Arc<NativeVerificationOwner>,
    devices: Arc<NativeDeviceOwner>,
    dehydrated_devices: Arc<NativeDehydratedDevicesOwner>,
    image_packs: Arc<NativeImagePackOwner>,
    typing: Arc<NativeTypingOwner>,
    presence: Arc<NativePresenceOwner>,
    rtc_transports: Arc<NativeRtcTransportsOwner>,
    user_status: Arc<NativeUserStatusOwner>,
    widgets: Arc<NativeWidgetOwner>,
    notification_observations: Arc<NativeNotificationObservationOwner>,
    join_rules: Arc<NativeRoomJoinRuleOwner>,
    sync: Arc<SyncServiceOwner>,
    own_profile: NativeOwnProfileOwner,
    media_retention: NativeMediaRetentionOwner,
    notification_decisions: Arc<synara_core::app::notifications::NativeNotificationDecisionOwner>,
}

/// Start the owners in their product order, recording each one that needs
/// retirement on rollback in `partial` as soon as it exists.
async fn start_desktop_owners(
    client: &Client,
    app: &AppHandle,
    session_generation: u64,
    indexed_message_search: bool,
    partial: &mut SessionPreparationRollback,
) -> Result<DesktopSessionOwners, MatrixAuthCommandError> {
    let verification = Arc::new(crate::matrix::verification::start_verification_owner(
        client,
        app.clone(),
        session_generation,
    ));
    let devices = Arc::new(
        crate::matrix::devices::start_device_owner(client, app.clone(), session_generation)
            .await
            .map_err(map_device_error)?,
    );
    let dehydrated_devices = Arc::new(
        crate::matrix::dehydrated_devices::start_dehydrated_devices_owner(
            client,
            app.clone(),
            session_generation,
        )
        .await,
    );
    let image_packs = Arc::new(
        crate::matrix::account_data::start_image_pack_owner(
            client,
            app.clone(),
            session_generation,
        )
        .map_err(map_pack_read_subscribe_error)?,
    );
    image_packs.set_indexed_message_search(indexed_message_search);
    let typing =
        Arc::new(NativeTypingOwner::start(client, session_generation).map_err(map_typing_error)?);
    let presence = Arc::new(
        crate::matrix::presence::start_presence_owner(client, app.clone(), session_generation)
            .map_err(map_presence_error)?,
    );
    let rtc_transports = Arc::new(
        crate::matrix::rtc_transports::NativeRtcTransportsOwner::start(client, session_generation),
    );
    let user_status = Arc::new(crate::matrix::user_status::NativeUserStatusOwner::start(
        client,
        session_generation,
    ));
    let widgets = Arc::new(
        crate::matrix::widgets::start_widget_owner(client, app.clone(), session_generation)
            .map_err(map_widget_error)?,
    );
    partial.widgets = Some(widgets.clone());
    // A9 observation stream: Core pushes each live message-like event to the
    // renderer, which hands the identity back to the decision owner. The
    // renderer no longer scans timelines to discover notifiable events.
    let notification_observations = Arc::new(
        crate::matrix::notifications::start_notification_observation_owner(
            client,
            app.clone(),
            session_generation,
        )
        .map_err(|_| {
            MatrixAuthCommandError::unavailable("p2-notification-observation-attach-failed")
        })?,
    );
    partial.observations = Some(notification_observations.clone());
    let join_rules = Arc::new(
        crate::matrix::room_profile::start_join_rule_owner(client, app.clone(), session_generation)
            .map_err(map_room_join_rule_owner_error)?,
    );
    partial.join_rules = Some(join_rules.clone());
    let sync = Arc::new(start_sync_owner(client, session_generation).await?);
    partial.sync = Some(sync.clone());
    let (own_profile, media_retention) =
        start_room_surface_owners(client, app.clone(), session_generation)?;
    // A9 decision stream: account-bound Core policy owner. The shell keeps no
    // handle; Core drops it on logout alongside the timeline registry.
    let notification_decisions = Arc::new(
        synara_core::app::notifications::NativeNotificationDecisionOwner::new(
            client,
            session_generation,
        )
        .map_err(|_| {
            MatrixAuthCommandError::unavailable("p2-notification-decision-attach-failed")
        })?,
    );
    Ok(DesktopSessionOwners {
        verification,
        devices,
        dehydrated_devices,
        image_packs,
        typing,
        presence,
        rtc_transports,
        user_status,
        widgets,
        notification_observations,
        join_rules,
        sync,
        own_profile,
        media_retention,
        notification_decisions,
    })
}

impl DesktopSessionOwners {
    /// Finish the session record: timeline registry and room-list events are
    /// created last, after any credential persistence the caller performed.
    fn into_session(
        self,
        client: Client,
        session_persistence: SessionPersistenceOwner,
        identity: MatrixLoginIdentity,
        session_generation: u64,
        app: &AppHandle,
    ) -> (
        ManagedMatrixSession,
        Arc<synara_core::app::notifications::NativeNotificationDecisionOwner>,
    ) {
        let timelines = Arc::new(NativeTimelineOwner::new(
            &client,
            crate::matrix::timeline::timeline_view_emit(app.clone()),
            session_generation,
        ));
        let room_list_live =
            crate::matrix::room_list::start_room_list_live(&self.sync, app.clone());
        let room_key_transfer = self.devices.room_key_transfer();
        (
            ManagedMatrixSession {
                client,
                session_persistence,
                identity,
                sync: self.sync,
                invite_avatars: self.join_rules.invite_avatars(),
                timelines,
                attachments: AttachmentSendQueue::new(session_generation),
                verification: self.verification,
                devices: self.devices,
                dehydrated_devices: self.dehydrated_devices,
                _image_packs: self.image_packs,
                typing: self.typing,
                presence: self.presence,
                rtc_transports: self.rtc_transports,
                user_status: self.user_status,
                widgets: self.widgets,
                join_rules: self.join_rules,
                _own_profile: self.own_profile,
                _media_retention: self.media_retention,
                _room_list_live: room_list_live,
                notification_observations: self.notification_observations,
                room_key_transfer,
                selected_room_key_import: None,
                next_room_key_import_selection_id: 0,
            },
            self.notification_decisions,
        )
    }
}

pub(super) async fn start_sync_owner(
    client: &Client,
    session_generation: u64,
) -> Result<SyncServiceOwner, MatrixAuthCommandError> {
    let owner = build_sync_service(client, session_generation, SyncServiceConfig::default())
        .await
        .map_err(|error| map_sync_error(error.diagnostic_id()))?;
    if let Err(error) = owner.start().await {
        let _ = owner.stop().await;
        return Err(map_sync_error(error.diagnostic_id()));
    }
    Ok(owner)
}

fn start_room_surface_owners(
    client: &Client,
    app: AppHandle,
    session_generation: u64,
) -> Result<(NativeOwnProfileOwner, NativeMediaRetentionOwner), MatrixAuthCommandError> {
    let own_profile =
        crate::matrix::user_profile::start_own_profile_owner(client, app, session_generation)
            .map_err(|_| MatrixAuthCommandError::unavailable("p2-own-profile-attach-failed"))?;
    let media_retention = NativeMediaRetentionOwner::start(client, session_generation)
        .map_err(|_| MatrixAuthCommandError::unavailable("p2-media-retention-attach-failed"))?;
    Ok((own_profile, media_retention))
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

use synara_core::app::lifecycle::session::is_recoverable_store_login_diagnostic;

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

async fn existing_login_device_id(
    app_data_root: &Path,
    identity: &AccountIdentity,
) -> Result<Option<String>, MatrixAuthCommandError> {
    let store_paths = StorePaths::derive(app_data_root, identity)
        .map_err(|_| MatrixAuthCommandError::unavailable("p3.2-login-store-migration-failed"))?;
    if !store_paths
        .state_dir()
        .join("matrix-sdk-crypto.sqlite3")
        .is_file()
    {
        return Ok(None);
    }
    let key_creation_policy = store_paths
        .key_creation_policy()
        .map_err(|_| MatrixAuthCommandError::unavailable("p3.2-login-store-migration-failed"))?;
    let store_key =
        get_or_migrate_store_key(&KeyringStoreKeyVault::new(), identity, key_creation_policy)
            .map_err(map_store_key_vault_error)?;
    let config =
        ClientBuildConfig::product_default(app_data_root, identity.clone(), Some(store_key))
            .map_err(|_| {
                MatrixAuthCommandError::unavailable("p3.2-login-store-migration-failed")
            })?;
    existing_sqlite_crypto_device_id(
        config.state_store_path(),
        config.store_passphrase_hex().as_deref(),
    )
    .await
    .map_err(map_auth_error)
}

fn map_store_client_build_error(error: ClientBuilderError) -> MatrixAuthCommandError {
    let diagnostic_id = match error.to_factory_error().category {
        crate::matrix::ipc::MatrixIpcErrorCategory::StoreLocked => "p3.2-login-store-locked",
        crate::matrix::ipc::MatrixIpcErrorCategory::StoreCorrupt => {
            "p3.2-login-store-reset-required"
        }
        crate::matrix::ipc::MatrixIpcErrorCategory::StoreUnavailable => {
            "p3.2-login-store-open-failed"
        }
        _ => "p3.2-login-store-open-failed",
    };
    MatrixAuthCommandError::unavailable(diagnostic_id)
}

pub(super) async fn build_client(
    app_data_root: &Path,
    identity: AccountIdentity,
    indexed_message_search: bool,
) -> Result<(Client, SessionPersistenceOwner), MatrixAuthCommandError> {
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
            .map_err(|_| MatrixAuthCommandError::unavailable("p3.2-login-store-migration-failed"))?
            .with_indexed_message_search(indexed_message_search);
    let client = build_unauthenticated_client(&config)
        .await
        .map_err(map_store_client_build_error)?;
    let session_persistence = SessionPersistenceOwner::new();
    install_session_rotation_callbacks(
        &client,
        identity,
        app_data_root,
        session_persistence.callback_lease(),
    )?;
    Ok((client, session_persistence))
}

fn install_session_rotation_callbacks(
    client: &Client,
    identity: AccountIdentity,
    root: &Path,
    persistence_lease: Arc<SessionPersistenceLease>,
) -> Result<(), MatrixAuthCommandError> {
    use synara_core::app::lifecycle::session::{RotationDiagnostics, RotationHooks};
    let check_identity = identity.clone();
    let preflight_root = root.to_path_buf();
    let journal_root = root.to_path_buf();
    let hooks = RotationHooks {
        check_identity: Box::new(move |client| {
            let user_id = client
                .user_id()
                .ok_or("d0.1-session-rotation-identity-missing")?;
            let device_id = client
                .device_id()
                .ok_or("d0.1-session-rotation-identity-missing")?;
            let account = authenticated_login_identity(
                user_id.as_str(),
                client.homeserver().as_str(),
                &check_identity,
            )
            .map_err(|_| "d0.1-session-rotation-identity-mismatch")?;
            Ok(Some(MatrixLoginIdentity {
                user_id: account.user_id().to_owned(),
                device_id: device_id.to_string(),
                homeserver_url: account.homeserver_url().to_owned(),
            }))
        }),
        preflight: Box::new(move |locator| ensure_logout_retry_locator(&preflight_root, locator)),
        map_persist_error: map_session_rotation_persist_error,
        record_outcome: Box::new(move |result| {
            record_session_rotation_outcome(&journal_root, result)
        }),
    };
    synara_core::app::lifecycle::session::install_session_rotation_callbacks(
        client,
        identity,
        Arc::new(KeyringSessionMaterialVault::new()),
        persistence_lease,
        RotationDiagnostics::DESKTOP,
        hooks,
    )
    .map_err(MatrixAuthCommandError::from)
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
    session::active_locator_path(app_data_root)
}

#[cfg(test)]
pub(super) fn write_active_identity(
    app_data_root: &Path,
    identity: &MatrixLoginIdentity,
) -> Result<(), MatrixAuthCommandError> {
    Ok(session::write_active_locator(app_data_root, identity)?)
}

pub(super) fn read_active_identity(
    app_data_root: &Path,
) -> Result<MatrixLoginIdentity, MatrixAuthCommandError> {
    Ok(session::read_active_locator(app_data_root)?)
}

/// Establish the non-secret retry locator before any logout side effect.
pub(super) fn ensure_logout_retry_locator(
    root: &Path,
    identity: &MatrixLoginIdentity,
) -> Result<(), MatrixAuthCommandError> {
    Ok(session::ensure_logout_retry_locator(root, identity)?)
}

// Narrow I/O fault seam: production readback and file sync remain mandatory.
#[cfg(test)]
pub(super) fn ensure_logout_retry_locator_with_directory_sync(
    root: &Path,
    identity: &MatrixLoginIdentity,
    sync_directory: impl FnMut(&Path) -> std::io::Result<()>,
) -> Result<(), MatrixAuthCommandError> {
    Ok(session::ensure_logout_retry_locator_with_directory_sync(
        root,
        identity,
        sync_directory,
    )?)
}

/// Delete vault material, then the locator that names it.
pub(super) fn clear_native_logout_material<
    V: crate::matrix::lifecycle::SessionMaterialVault + ?Sized,
>(
    vault: &V,
    identity: &MatrixLoginIdentity,
    root: &Path,
) -> Result<(), MatrixAuthCommandError> {
    Ok(session::clear_native_logout_material(
        vault, identity, root,
    )?)
}

pub(super) fn clear_persisted_logout_material<
    V: crate::matrix::lifecycle::SessionMaterialVault + ?Sized,
>(
    vault: &V,
    root: &Path,
) -> Result<(), MatrixAuthCommandError> {
    Ok(session::clear_persisted_logout_material(vault, root)?)
}

#[cfg(test)]
pub(super) fn remove_active_identity(app_data_root: &Path) -> Result<(), MatrixAuthCommandError> {
    Ok(session::remove_active_locator(app_data_root)?)
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

pub(super) fn map_session_rotation_persist_error(
    error: crate::matrix::lifecycle::LifecycleError,
) -> MatrixAuthCommandError {
    MatrixAuthCommandError::unavailable(session::rotation_persist_diagnostic(&error))
}

/// Append the save outcome to the native session-lifecycle journal.
pub(super) fn record_session_rotation_outcome(
    root: &Path,
    result: &Result<(), MatrixAuthCommandError>,
) {
    session::record_session_rotation_outcome(
        root,
        result
            .as_ref()
            .err()
            .map(|error| error.diagnostic_id.as_str()),
    );
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
        assert_eq!(confirmation_id.len(), 64);
        assert!(confirmation_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
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
            .take_confirmed_store_recovery(&"0".repeat(64), STORE_RECOVERY_TYPED_CONFIRMATION_TEXT)
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
        let (production, _) = commands
            .split_once("\n#[cfg(test)]\nmod tests {")
            .expect("auth command test module boundary");
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
        assert!(login.contains("existing_login_device_id"));
        assert!(login.contains("device_id: existing_device_id"));
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
        assert!(recovery.contains("confirmed_store_recovery_target"));
        assert!(recovery.contains("archive_and_rebuild_store"));
        assert!(
            recovery.find("confirmed_store_recovery_target")
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
            confirmation_id: "a".repeat(64),
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
    fn every_remote_logout_is_bounded() {
        // Rollback, identity-mismatch, and registration-compensation logouts run
        // under the transition/session locks; an unbounded SDK logout (30 s ×
        // retries plus a refresh) would stall restore, login, and logout.
        let (production, _) = include_str!("product_commands.rs")
            .split_once("\n#[cfg(test)]\nmod tests {")
            .expect("auth command test module boundary");
        let calls = production.matches("matrix_auth().logout()").count();
        let bounded = production.matches("bounded_remote_logout(\n").count();
        // The voluntary path hands its closure to `finish_taken_session_logout`,
        // which applies `bounded_remote_logout` itself.
        assert_eq!(calls, bounded + 1, "every remote logout must be bounded");
        let voluntary = production
            .split("plan.remote_logout_allowed.then_some(move || async move {")
            .nth(1)
            .and_then(|rest| rest.split("}),").next())
            .expect("voluntary remote logout closure");
        // The backup wait runs first and is itself bounded, inside the remote bound.
        let backup = voluntary
            .find("wait_for_backup_steady_state(")
            .expect("voluntary logout waits for the key backup");
        let logout = voluntary
            .find("client.matrix_auth().logout().await")
            .expect("voluntary logout POSTs /logout");
        assert!(backup < logout);
        assert!(voluntary.contains("LOGOUT_BACKUP_STEADY_STATE_TIMEOUT"));
        // Core's `finish_taken_session_logout` stops sync before the remote
        // `/logout`; its own test pins that order.
    }

    #[test]
    fn session_install_and_every_logout_path_revoke_store_recovery() {
        let (production, _) = include_str!("product_commands.rs")
            .split_once("\n#[cfg(test)]\nmod tests {")
            .expect("auth command test module boundary");
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
        // Publishing through the install guard revokes stale recovery (Core
        // owner test `publish_revokes_store_recovery`).
        for (label, install) in [
            ("password", password_install),
            ("register", register_install),
            ("restore", restore_install),
        ] {
            assert!(
                install.contains("install.publish(managed).await"),
                "{label} session installation must publish through the guard"
            );
        }

        let logout = production
            .split("pub async fn matrix_logout")
            .nth(1)
            .and_then(|rest| rest.split("pub async fn ").next())
            .expect("logout body");
        // The owner's logout revokes recovery on both the orphan and the
        // active path, after taking the session and before any teardown.
        assert!(logout.contains("state\n        .logout("));
        let owner =
            include_str!("../../../../crates/synara-core/src/app/lifecycle/session/owner.rs");
        let owner_logout = owner
            .split("pub async fn logout<")
            .nth(1)
            .and_then(|rest| rest.split("pub fn save_retry_due").next())
            .expect("owner logout body");
        let take = owner_logout.find("take_session_for_logout(").unwrap();
        let revoke = owner_logout
            .find("self.clear_store_recovery().await")
            .unwrap();
        let orphan = owner_logout.find("orphan().await").unwrap();
        let teardown = owner_logout.find("teardown(session, plan)").unwrap();
        assert!(take < revoke && revoke < orphan && revoke < teardown);
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
        assert!(!is_recoverable_store_login_diagnostic(
            "p3.2-login-olm-unavailable"
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
        let corrupt = map_store_client_build_error(ClientBuilderError::SdkBuild {
            category: crate::matrix::ipc::MatrixIpcErrorCategory::StoreCorrupt,
            diagnostic_id: "test-confirmed-store-corrupt",
            message: "confirmed corruption".into(),
        });
        assert_eq!(corrupt.diagnostic_id, "p3.2-login-store-reset-required");
        assert!(is_recoverable_store_login_diagnostic(
            &corrupt.diagnostic_id
        ));
        assert!(!is_recoverable_store_login_diagnostic(
            &locked.diagnostic_id
        ));
        assert!(!is_recoverable_store_login_diagnostic(
            &unavailable.diagnostic_id
        ));
    }
}
