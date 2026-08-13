//! Desktop bridge for `matrix_verification_begin_sas` through `Core::command`.
//!
//! Core owns the live `NativeVerificationOwner` after the shell attaches it.
//! This adapter builds the envelope and maps closed Core categories onto the
//! existing Tauri error shape. React still invokes `matrix_verification_begin_sas`.

use synara_core::app::verification::NativeVerificationRequest;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;
use crate::matrix::verification::live::map_verification_error;

const VERIFICATION_BEGIN_SAS_COMMAND: &str = "matrix_verification_begin_sas";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn verification_begin_sas(
    core: &Core,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: VERIFICATION_BEGIN_SAS_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "flowId": flow_id }),
        })
        .await
        .map_err(map_verification_begin_sas_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| verification_begin_sas_response_error())
}

fn map_verification_begin_sas_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => {
            map_verification_error("v-crypto.1-start-requires-session")
        }
        MatrixIpcErrorCategory::SdkInvariant => match error.diagnostic_id.as_deref() {
            Some("v-crypto.1-flow-not-found") => {
                map_verification_error("v-crypto.1-flow-not-found")
            }
            Some("v-crypto.1-sas-invalid-state") => {
                map_verification_error("v-crypto.1-sas-invalid-state")
            }
            _ => MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix verification request is invalid.",
                "v-crypto.1-sas-invalid-state",
            ),
        },
        _ => match error.diagnostic_id.as_deref() {
            Some("v-crypto.1-sas-start-failed") => {
                map_verification_error("v-crypto.1-sas-start-failed")
            }
            Some("v-crypto.1-sas-start-unavailable") => {
                map_verification_error("v-crypto.1-sas-start-unavailable")
            }
            Some("v-crypto.1-sas-accept-failed") => {
                map_verification_error("v-crypto.1-sas-accept-failed")
            }
            _ => map_verification_error("v-crypto.1-sas-start-failed"),
        },
    }
}

fn verification_begin_sas_response_error() -> MatrixAuthCommandError {
    map_verification_error("v-crypto.1-sas-start-failed")
}
