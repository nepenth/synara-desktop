use std::sync::Arc;

use synara_core::app::cross_signing::{NativeDeviceVerificationPrompt, NativeIdentityResetState};

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
    // Core owns the password stage (shared with iOS): it re-checks that no
    // identity was published meanwhile and refuses a partial private identity.
    active
        .devices
        .authenticate_cross_signing_password(password)
        .await
        .map_err(|diagnostic_id| match diagnostic_id {
            "v-crypto.2-cross-signing-password-rejected" => MatrixAuthCommandError::new(
                "Forbidden",
                "Cross-signing setup authentication failed. Check your password and try again.",
                diagnostic_id,
            ),
            _ => map_cross_signing_setup_owner_error(diagnostic_id),
        })
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

/// S5 — start "Reset identity" for an account that lost its recovery key and
/// every other device. Core keeps the SDK reset handle for the session.
#[tauri::command]
pub async fn matrix_identity_reset_start(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeIdentityResetState, MatrixAuthCommandError> {
    crate::bridge::identity_reset::identity_reset_start(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_identity_reset_password(
    core: State<'_, Arc<synara_core::Core>>,
    mut password: String,
) -> Result<NativeIdentityResetState, MatrixAuthCommandError> {
    let result =
        crate::bridge::identity_reset::identity_reset_password(core.inner().as_ref(), &password)
            .await;
    password.zeroize();
    result
}

#[tauri::command]
pub async fn matrix_identity_reset_continue(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeIdentityResetState, MatrixAuthCommandError> {
    crate::bridge::identity_reset::identity_reset_continue(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_identity_reset_cancel(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<bool, MatrixAuthCommandError> {
    crate::bridge::identity_reset::identity_reset_cancel(core.inner().as_ref()).await
}

/// S5 — whether to show "Verify This Device" once after sign-in.
#[tauri::command]
pub async fn matrix_device_verification_prompt(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeDeviceVerificationPrompt, MatrixAuthCommandError> {
    crate::bridge::identity_reset::device_verification_prompt(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_device_verification_prompt_dismiss(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeDeviceVerificationPrompt, MatrixAuthCommandError> {
    crate::bridge::identity_reset::device_verification_prompt_dismiss(core.inner().as_ref())
}
