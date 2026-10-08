//! Desktop bridge for `matrix_verification_confirm` through `Core::command`.
//!
//! Core owns the live `NativeVerificationOwner` after the shell attaches it.
//! This adapter builds the envelope and maps closed Core categories onto the
//! existing Tauri error shape. React still invokes `matrix_verification_confirm`.

use synara_core::app::verification::NativeVerificationRequest;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;
use crate::matrix::verification::live::map_verification_error;

pub(crate) async fn verification_confirm(
    core: &Core,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let response = core
        .verification_confirm(synara_core::core_api::MatrixVerificationConfirmRequest { flow_id })
        .await
        .map_err(map_verification_confirm_core_error)?;
    Ok(response)
}

fn map_verification_confirm_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => {
            map_verification_error("v-crypto.1-start-requires-session")
        }
        MatrixIpcErrorCategory::SdkInvariant => match error.diagnostic_id.as_deref() {
            Some("v-crypto.1-flow-not-found") => {
                map_verification_error("v-crypto.1-flow-not-found")
            }
            Some("v-crypto.1-confirm-before-sas") => {
                map_verification_error("v-crypto.1-confirm-before-sas")
            }
            Some("v-crypto.1-sas-unavailable") => {
                map_verification_error("v-crypto.1-sas-unavailable")
            }
            _ => MatrixAuthCommandError::new(
                "InvalidRequest",
                "The verification comparison is not ready.",
                "v-crypto.1-confirm-before-sas",
            ),
        },
        _ => match error.diagnostic_id.as_deref() {
            Some("v-crypto.1-confirm-failed") => {
                map_verification_error("v-crypto.1-confirm-failed")
            }
            _ => map_verification_error("v-crypto.1-confirm-failed"),
        },
    }
}
