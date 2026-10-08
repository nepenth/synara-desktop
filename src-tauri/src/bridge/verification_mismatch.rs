//! Desktop bridge for `matrix_verification_mismatch` through `Core::command`.
//!
//! Core owns the live `NativeVerificationOwner` after the shell attaches it.
//! This adapter builds the envelope and maps closed Core categories onto the
//! existing Tauri error shape. React still invokes `matrix_verification_mismatch`.

use synara_core::app::verification::NativeVerificationRequest;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;
use crate::matrix::verification::live::map_verification_error;

pub(crate) async fn verification_mismatch(
    core: &Core,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let response = core
        .verification_mismatch(synara_core::core_api::MatrixVerificationMismatchRequest { flow_id })
        .await
        .map_err(map_verification_mismatch_core_error)?;
    Ok(response)
}

fn map_verification_mismatch_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => {
            map_verification_error("v-crypto.1-start-requires-session")
        }
        MatrixIpcErrorCategory::SdkInvariant => match error.diagnostic_id.as_deref() {
            Some("v-crypto.1-flow-not-found") => {
                map_verification_error("v-crypto.1-flow-not-found")
            }
            Some("v-crypto.1-sas-unavailable") => {
                map_verification_error("v-crypto.1-sas-unavailable")
            }
            _ => MatrixAuthCommandError::new(
                "InvalidRequest",
                "The verification comparison is not ready.",
                "v-crypto.1-sas-unavailable",
            ),
        },
        _ => match error.diagnostic_id.as_deref() {
            Some("v-crypto.1-mismatch-failed") => {
                map_verification_error("v-crypto.1-mismatch-failed")
            }
            _ => map_verification_error("v-crypto.1-mismatch-failed"),
        },
    }
}
