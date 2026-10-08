//! Desktop bridge for `matrix_verification_accept` through `Core::command`.
//!
//! Core owns the live `NativeVerificationOwner` after the shell attaches it.
//! This adapter builds the envelope and maps closed Core categories onto the
//! existing Tauri error shape. React still invokes `matrix_verification_accept`.

use synara_core::app::verification::NativeVerificationRequest;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;
use crate::matrix::verification::live::map_verification_error;

pub(crate) async fn verification_accept(
    core: &Core,
    flow_id: String,
) -> Result<NativeVerificationRequest, MatrixAuthCommandError> {
    let response = core
        .verification_accept(synara_core::core_api::MatrixVerificationAcceptRequest { flow_id })
        .await
        .map_err(map_verification_accept_core_error)?;
    Ok(response)
}

fn map_verification_accept_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
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
                "v-crypto.1-flow-not-found",
            ),
        },
        _ => match error.diagnostic_id.as_deref() {
            Some("v-crypto.1-accept-failed") => map_verification_error("v-crypto.1-accept-failed"),
            _ => map_verification_error("v-crypto.1-accept-failed"),
        },
    }
}
