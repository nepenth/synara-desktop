//! Desktop bridge for room identity-change warnings.
//!
//! Core owns the trust decision through the attached verification owner. This
//! adapter calls the typed Core methods and keeps closed diagnostic ids on the
//! existing Tauri error shape. No user ids or SDK text reach the error.

use synara_core::app::verification::{
    NativeIdentityWarningAction, NativeIdentityWarningResolveRequest, NativeRoomIdentityWarnings,
    NativeRoomIdentityWarningsRequest,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const REQUIRES_SESSION: &str = "v-crypto.identity-warning-requires-session";
const FALLBACK: &str = "v-crypto.identity-warning-failed";

pub(crate) async fn room_identity_warnings(
    core: &Core,
    room_id: String,
) -> Result<NativeRoomIdentityWarnings, MatrixAuthCommandError> {
    core.room_identity_warnings(NativeRoomIdentityWarningsRequest { room_id })
        .await
        .map_err(map_identity_warning_core_error)
}

pub(crate) async fn room_identity_warning_resolve(
    core: &Core,
    room_id: String,
    user_id: String,
    action: NativeIdentityWarningAction,
) -> Result<NativeRoomIdentityWarnings, MatrixAuthCommandError> {
    core.room_identity_warning_resolve(NativeIdentityWarningResolveRequest {
        room_id,
        user_id,
        action,
    })
    .await
    .map_err(map_identity_warning_core_error)
}

fn closed_diagnostic(error: &MatrixIpcError) -> &'static str {
    match error.diagnostic_id.as_deref() {
        Some("v-crypto.identity-warning-invalid-room") => "v-crypto.identity-warning-invalid-room",
        Some("v-crypto.identity-warning-invalid-user") => "v-crypto.identity-warning-invalid-user",
        Some("v-crypto.identity-warning-room-not-found") => {
            "v-crypto.identity-warning-room-not-found"
        }
        Some("v-crypto.identity-warning-identity-unavailable") => {
            "v-crypto.identity-warning-identity-unavailable"
        }
        Some("v-crypto.identity-warning-action-not-applicable") => {
            "v-crypto.identity-warning-action-not-applicable"
        }
        Some("v-crypto.identity-warning-store-failed") => "v-crypto.identity-warning-store-failed",
        _ => FALLBACK,
    }
}

fn map_identity_warning_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    if error.category == MatrixIpcErrorCategory::Forbidden {
        return MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            REQUIRES_SESSION,
        );
    }
    let diagnostic_id = closed_diagnostic(&error);
    let code = match diagnostic_id {
        "v-crypto.identity-warning-invalid-room" | "v-crypto.identity-warning-invalid-user" => {
            "InvalidRequest"
        }
        _ => "Unknown",
    };
    MatrixAuthCommandError::new(
        code,
        "The identity change could not be updated.",
        diagnostic_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_keep_closed_ids_and_no_session_is_forbidden() {
        let no_session = map_identity_warning_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic(REQUIRES_SESSION),
        );
        assert_eq!(no_session.diagnostic_id, REQUIRES_SESSION);

        let not_applicable = map_identity_warning_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("v-crypto.identity-warning-action-not-applicable"),
        );
        assert_eq!(
            not_applicable.diagnostic_id,
            "v-crypto.identity-warning-action-not-applicable"
        );

        let unknown = map_identity_warning_core_error(
            MatrixIpcError::new(MatrixIpcErrorCategory::Unknown)
                .with_diagnostic("https://example.org/secret"),
        );
        assert_eq!(unknown.diagnostic_id, FALLBACK);
    }
}
