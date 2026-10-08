//! Desktop bridge for `matrix_verification_dismiss` through `Core::command`.
//!
//! Core owns the live `NativeVerificationOwner` after the shell attaches it.
//! This adapter builds the envelope and maps closed Core categories onto the
//! existing Tauri error shape. React still invokes `matrix_verification_dismiss`.

use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;
use crate::matrix::verification::live::map_verification_error;

pub(crate) async fn verification_dismiss(
    core: &Core,
    flow_id: String,
) -> Result<(), MatrixAuthCommandError> {
    core.verification_dismiss(synara_core::core_api::MatrixVerificationDismissRequest { flow_id })
        .await
        .map_err(map_verification_dismiss_core_error)?;
    Ok(())
}

fn map_verification_dismiss_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => {
            map_verification_error("v-crypto.1-start-requires-session")
        }
        MatrixIpcErrorCategory::SdkInvariant => match error.diagnostic_id.as_deref() {
            Some("v-crypto.1-flow-not-found") => {
                map_verification_error("v-crypto.1-flow-not-found")
            }
            Some("v-crypto.1-dismiss-active-flow") => {
                map_verification_error("v-crypto.1-dismiss-active-flow")
            }
            _ => MatrixAuthCommandError::new(
                "InvalidRequest",
                "An active verification request must be cancelled before it is dismissed.",
                "v-crypto.1-dismiss-active-flow",
            ),
        },
        _ => map_verification_error("v-crypto.1-dismiss-active-flow"),
    }
}
