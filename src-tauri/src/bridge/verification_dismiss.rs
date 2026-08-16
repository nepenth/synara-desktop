//! Desktop bridge for `matrix_verification_dismiss` through `Core::command`.
//!
//! Core owns the live `NativeVerificationOwner` after the shell attaches it.
//! This adapter builds the envelope and maps closed Core categories onto the
//! existing Tauri error shape. React still invokes `matrix_verification_dismiss`.

use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;
use crate::matrix::verification::live::map_verification_error;

const VERIFICATION_DISMISS_COMMAND: &str = "matrix_verification_dismiss";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn verification_dismiss(
    core: &Core,
    flow_id: String,
) -> Result<(), MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: VERIFICATION_DISMISS_COMMAND.to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload: serde_json::json!({ "flowId": flow_id }),
    })
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
