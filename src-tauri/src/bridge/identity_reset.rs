//! Desktop bridge for identity reset and the post-sign-in "Verify this
//! device" prompt (S5).
//!
//! Core owns the reset handle through the attached device owner. This adapter
//! calls the typed Core methods and keeps closed diagnostic ids on the
//! existing Tauri error shape. The password stays a method argument.

use synara_core::app::cross_signing::{
    NativeDeviceVerificationPrompt, NativeIdentityResetState, IDENTITY_RESET_APPROVAL_PENDING,
    IDENTITY_RESET_AUTH_UNSUPPORTED, IDENTITY_RESET_CANCELLED, IDENTITY_RESET_FAILED,
    IDENTITY_RESET_NETWORK, IDENTITY_RESET_NOT_PENDING, IDENTITY_RESET_PASSWORD_EMPTY,
    IDENTITY_RESET_PASSWORD_REJECTED,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const REQUIRES_SESSION: &str = "v-crypto.2-identity-reset-requires-session";
const PROMPT_FAILED: &str = "v-crypto.2-verify-prompt-failed";

pub(crate) async fn identity_reset_start(
    core: &Core,
) -> Result<NativeIdentityResetState, MatrixAuthCommandError> {
    core.identity_reset_start()
        .await
        .map_err(map_identity_reset_core_error)
}

pub(crate) async fn identity_reset_password(
    core: &Core,
    password: &str,
) -> Result<NativeIdentityResetState, MatrixAuthCommandError> {
    core.identity_reset_password(password)
        .await
        .map_err(map_identity_reset_core_error)
}

pub(crate) async fn identity_reset_continue(
    core: &Core,
) -> Result<NativeIdentityResetState, MatrixAuthCommandError> {
    core.identity_reset_continue()
        .await
        .map_err(map_identity_reset_core_error)
}

pub(crate) async fn identity_reset_cancel(core: &Core) -> Result<bool, MatrixAuthCommandError> {
    core.identity_reset_cancel()
        .await
        .map_err(map_identity_reset_core_error)?;
    Ok(true)
}

pub(crate) async fn device_verification_prompt(
    core: &Core,
) -> Result<NativeDeviceVerificationPrompt, MatrixAuthCommandError> {
    core.device_verification_prompt()
        .await
        .map_err(map_verification_prompt_core_error)
}

pub(crate) fn device_verification_prompt_dismiss(
    core: &Core,
) -> Result<NativeDeviceVerificationPrompt, MatrixAuthCommandError> {
    core.dismiss_device_verification_prompt()
        .map_err(map_verification_prompt_core_error)
}

fn closed_diagnostic(error: &MatrixIpcError) -> &'static str {
    match error.diagnostic_id.as_deref() {
        Some(IDENTITY_RESET_NOT_PENDING) => IDENTITY_RESET_NOT_PENDING,
        Some(IDENTITY_RESET_PASSWORD_EMPTY) => IDENTITY_RESET_PASSWORD_EMPTY,
        Some(IDENTITY_RESET_PASSWORD_REJECTED) => IDENTITY_RESET_PASSWORD_REJECTED,
        Some(IDENTITY_RESET_APPROVAL_PENDING) => IDENTITY_RESET_APPROVAL_PENDING,
        Some(IDENTITY_RESET_NETWORK) => IDENTITY_RESET_NETWORK,
        Some(IDENTITY_RESET_AUTH_UNSUPPORTED) => IDENTITY_RESET_AUTH_UNSUPPORTED,
        Some(IDENTITY_RESET_CANCELLED) => IDENTITY_RESET_CANCELLED,
        Some("p2-identity-reset-no-session" | "v-crypto.2-cross-signing-user-missing") => {
            REQUIRES_SESSION
        }
        _ => IDENTITY_RESET_FAILED,
    }
}

fn map_identity_reset_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic_id = closed_diagnostic(&error);
    let (code, message) = match diagnostic_id {
        REQUIRES_SESSION => ("Forbidden", "No native Matrix session is active."),
        IDENTITY_RESET_PASSWORD_REJECTED => (
            "Forbidden",
            "Identity reset authentication failed. Check your password and try again.",
        ),
        IDENTITY_RESET_PASSWORD_EMPTY | IDENTITY_RESET_NOT_PENDING => (
            "InvalidRequest",
            "Start the identity reset before authenticating it.",
        ),
        IDENTITY_RESET_APPROVAL_PENDING => {
            ("Forbidden", "The identity reset has not been approved yet.")
        }
        IDENTITY_RESET_NETWORK => ("Unavailable", "The homeserver could not be reached."),
        IDENTITY_RESET_AUTH_UNSUPPORTED => (
            "Forbidden",
            "The homeserver requires an unsupported authentication step for identity reset.",
        ),
        _ => ("Unknown", "The identity reset could not be completed."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

fn map_verification_prompt_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    if error.category == MatrixIpcErrorCategory::Forbidden {
        return MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            REQUIRES_SESSION,
        );
    }
    MatrixAuthCommandError::new(
        "Unknown",
        "Device verification status is unavailable.",
        PROMPT_FAILED,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_errors_keep_closed_ids() {
        let rejected = map_identity_reset_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic(IDENTITY_RESET_PASSWORD_REJECTED),
        );
        assert_eq!(rejected.diagnostic_id, IDENTITY_RESET_PASSWORD_REJECTED);
        assert_eq!(rejected.code, "Forbidden");

        let no_session = map_identity_reset_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-identity-reset-no-session"),
        );
        assert_eq!(no_session.diagnostic_id, REQUIRES_SESSION);

        let pending = map_identity_reset_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::CryptoFailure)
                .with_diagnostic(IDENTITY_RESET_APPROVAL_PENDING),
        );
        assert_eq!(pending.diagnostic_id, IDENTITY_RESET_APPROVAL_PENDING);

        let unknown = map_identity_reset_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Unknown)
                .with_diagnostic("https://example.org/secret"),
        );
        assert_eq!(unknown.diagnostic_id, IDENTITY_RESET_FAILED);
    }

    #[test]
    fn prompt_errors_are_closed() {
        let failed = map_verification_prompt_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::CryptoFailure)
                .with_diagnostic("@alice:example.org"),
        );
        assert_eq!(failed.diagnostic_id, PROMPT_FAILED);
    }
}
