use super::*;

#[tauri::command]
pub async fn matrix_cross_signing_status(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeCrossSigningStatus, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_cross_signing_session(session.as_ref())?;
    live_cross_signing_status(active).await
}

#[tauri::command]
pub async fn matrix_cross_signing_setup(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeCrossSigningSetupResult, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_cross_signing_session_mut(session.as_mut())?;
    let before = live_cross_signing_status(active).await?;
    if before.bootstrap != crate::matrix::cross_signing::live::NativeCrossSigningBootstrap::Needed {
        active.pending_cross_signing_auth_session = None;
        return Ok(NativeCrossSigningSetupResult {
            outcome: NativeCrossSigningSetupOutcome::AlreadyConfigured,
            status: before,
        });
    }

    match active
        .client
        .encryption()
        .bootstrap_cross_signing_if_needed(None)
        .await
    {
        Ok(()) => cross_signing_setup_complete(active).await,
        Err(error) => {
            let Some(info) = error.as_uiaa_response() else {
                return Err(cross_signing_setup_error(
                    "v-crypto.2-cross-signing-bootstrap-failed",
                ));
            };
            match supported_authentication(info) {
                Some(SupportedBootstrapAuthentication::Dummy) => {
                    let mut dummy = uiaa::Dummy::new();
                    dummy.session = info.session.clone();
                    active
                        .client
                        .encryption()
                        .bootstrap_cross_signing(Some(uiaa::AuthData::Dummy(dummy)))
                        .await
                        .map_err(|_| {
                            cross_signing_setup_error(
                                "v-crypto.2-cross-signing-dummy-auth-failed",
                            )
                        })?;
                    cross_signing_setup_complete(active).await
                }
                Some(SupportedBootstrapAuthentication::Password) => {
                    let auth_session = info.session.clone().ok_or_else(|| {
                        cross_signing_setup_error(
                            "v-crypto.2-cross-signing-auth-session-missing",
                        )
                    })?;
                    active.pending_cross_signing_auth_session = Some(auth_session);
                    Ok(NativeCrossSigningSetupResult {
                        outcome: NativeCrossSigningSetupOutcome::AuthenticationRequired,
                        status: live_cross_signing_status(active).await?,
                    })
                }
                None => Err(MatrixAuthCommandError::new(
                    "Forbidden",
                    "The homeserver requires an unsupported authentication step for cross-signing setup.",
                    "v-crypto.2-cross-signing-auth-unsupported",
                )),
            }
        }
    }
}

#[tauri::command]
pub async fn matrix_cross_signing_setup_password(
    state: State<'_, MatrixAuthState>,
    mut password: String,
) -> Result<NativeCrossSigningSetupResult, MatrixAuthCommandError> {
    let result = matrix_cross_signing_setup_password_inner(&state, &password).await;
    password.zeroize();
    result
}

pub(super) async fn matrix_cross_signing_setup_password_inner(
    state: &State<'_, MatrixAuthState>,
    password: &str,
) -> Result<NativeCrossSigningSetupResult, MatrixAuthCommandError> {
    if password.is_empty() {
        return Err(MatrixAuthCommandError::new(
            "InvalidRequest",
            "Your account password is required to finish cross-signing setup.",
            "v-crypto.2-cross-signing-password-empty",
        ));
    }

    let mut session = state.session.lock().await;
    let active = require_cross_signing_session_mut(session.as_mut())?;
    let auth_session = active
        .pending_cross_signing_auth_session
        .clone()
        .ok_or_else(|| {
            MatrixAuthCommandError::new(
                "InvalidRequest",
                "Start native cross-signing setup before authenticating it.",
                "v-crypto.2-cross-signing-auth-not-pending",
            )
        })?;
    let user_id = active.client.user_id().ok_or_else(|| {
        MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.2-cross-signing-user-missing",
        )
    })?;
    let mut auth = uiaa::Password::new(user_id.to_owned().into(), password.to_owned());
    auth.session = Some(auth_session);

    if let Err(error) = active
        .client
        .encryption()
        .bootstrap_cross_signing(Some(uiaa::AuthData::Password(auth)))
        .await
    {
        if let Some(info) = error.as_uiaa_response() {
            if let Some(auth_session) = info.session.clone() {
                active.pending_cross_signing_auth_session = Some(auth_session);
            }
            return Err(MatrixAuthCommandError::new(
                "Forbidden",
                "Cross-signing setup authentication failed. Check your password and try again.",
                "v-crypto.2-cross-signing-password-rejected",
            ));
        }
        return Err(cross_signing_setup_error(
            "v-crypto.2-cross-signing-auth-failed",
        ));
    }

    cross_signing_setup_complete(active).await
}

pub(super) async fn live_cross_signing_status(
    active: &ManagedMatrixSession,
) -> Result<NativeCrossSigningStatus, MatrixAuthCommandError> {
    let encryption = active.client.encryption();
    let private_status = encryption.cross_signing_status().await;
    let Some(user_id) = active.client.user_id() else {
        return Err(MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.2-cross-signing-user-missing",
        ));
    };
    let own_identity = encryption
        .request_user_identity(user_id)
        .await
        .map_err(|_| {
            MatrixAuthCommandError::new(
                "Unknown",
                "Native cross-signing status is unavailable.",
                "v-crypto.2-cross-signing-identity-query-failed",
            )
        })?;

    Ok(project_status(
        active.sync.session_generation(),
        private_status.as_ref(),
        own_identity.is_some(),
        own_identity
            .as_ref()
            .is_some_and(|identity| identity.is_verified()),
    ))
}

pub(super) async fn cross_signing_setup_complete(
    active: &mut ManagedMatrixSession,
) -> Result<NativeCrossSigningSetupResult, MatrixAuthCommandError> {
    active.pending_cross_signing_auth_session = None;
    let status = live_cross_signing_status(active).await?;
    if status.bootstrap == crate::matrix::cross_signing::live::NativeCrossSigningBootstrap::Needed {
        return Err(cross_signing_setup_error(
            "v-crypto.2-cross-signing-bootstrap-incomplete",
        ));
    }
    Ok(NativeCrossSigningSetupResult {
        outcome: NativeCrossSigningSetupOutcome::Complete,
        status,
    })
}

pub(super) fn cross_signing_setup_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native cross-signing setup could not be completed.",
        diagnostic_id,
    )
}
