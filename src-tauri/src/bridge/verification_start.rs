//! Desktop bridge for `matrix_verification_start` through `Core::command`.
//!
//! Core owns the live `NativeVerificationOwner` after the shell attaches it.
//! This adapter builds the envelope and maps closed Core categories onto the
//! existing Tauri error shape. React still invokes `matrix_verification_start`.

use synara_core::app::verification::NativeVerificationRequest;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;
use crate::matrix::verification::live::map_verification_error;

const VERIFICATION_START_COMMAND: &str = "matrix_verification_start";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn verification_start(
    core: &Core,
    device_id: Option<String>,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: VERIFICATION_START_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "deviceId": device_id }),
        })
        .await
        .map_err(map_verification_start_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| verification_start_response_error())
}

fn map_verification_start_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => {
            map_verification_error("v-crypto.1-start-requires-session")
        }
        MatrixIpcErrorCategory::UnsupportedCapability => {
            map_verification_error("v-crypto.1-own-identity-unavailable")
        }
        MatrixIpcErrorCategory::SdkInvariant => match error.diagnostic_id.as_deref() {
            Some("v-crypto.1-device-not-found") => {
                map_verification_error("v-crypto.1-device-not-found")
            }
            _ => MatrixAuthCommandError::new(
                "InvalidRequest",
                "The native Matrix verification request is invalid.",
                "v-crypto.1-device-not-found",
            ),
        },
        _ => match error.diagnostic_id.as_deref() {
            Some("v-crypto.1-device-query-failed") => {
                map_verification_error("v-crypto.1-device-query-failed")
            }
            Some("v-crypto.1-device-request-failed") => {
                map_verification_error("v-crypto.1-device-request-failed")
            }
            Some("v-crypto.1-identity-query-failed") => {
                map_verification_error("v-crypto.1-identity-query-failed")
            }
            Some("v-crypto.1-own-request-failed") => {
                map_verification_error("v-crypto.1-own-request-failed")
            }
            _ => map_verification_error("v-crypto.1-own-request-failed"),
        },
    }
}

fn verification_start_response_error() -> MatrixAuthCommandError {
    map_verification_error("v-crypto.1-own-request-failed")
}
