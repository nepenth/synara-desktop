use std::sync::Arc;

use super::*;

/// SNC-P3.6 — retain the existing payload-free cross-signing status wire DTO
/// while Core owns command registration, validation, and truth-table output.
/// The desktop Platform remains the only Matrix SDK/client/crypto/store owner.
#[tauri::command]
pub async fn matrix_cross_signing_status(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeCrossSigningStatus, MatrixAuthCommandError> {
    crate::bridge::cross_signing_status::cross_signing_status(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_cross_signing_setup(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeCrossSigningSetupResult, MatrixAuthCommandError> {
    crate::bridge::cross_signing_setup::cross_signing_setup(core.inner().as_ref()).await
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
        .devices
        .pending_cross_signing_auth()
        .map_err(map_cross_signing_setup_owner_error)?;
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
                active
                    .devices
                    .set_pending_cross_signing(Some(auth_session))
                    .map_err(map_cross_signing_setup_owner_error)?;
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

    active
        .devices
        .finish_cross_signing_setup()
        .await
        .map_err(map_cross_signing_setup_owner_error)
}

fn map_cross_signing_setup_owner_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    match diagnostic_id {
        "v-crypto.2-cross-signing-auth-not-pending" | "v-crypto.2-cross-signing-password-empty" => {
            MatrixAuthCommandError::new(
                "InvalidRequest",
                "Start native cross-signing setup before authenticating it.",
                diagnostic_id,
            )
        }
        "v-crypto.2-cross-signing-user-missing" => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            diagnostic_id,
        ),
        _ => cross_signing_setup_error(diagnostic_id),
    }
}

pub(super) fn cross_signing_setup_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native cross-signing setup could not be completed.",
        diagnostic_id,
    )
}
