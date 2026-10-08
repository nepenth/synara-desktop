//! Desktop bridge for `matrix_verification_list` through `Core::command`.
//!
//! Core owns the live `NativeVerificationOwner` after the shell attaches it.
//! This adapter builds the envelope and maps closed Core categories onto the
//! existing Tauri error shape. React still invokes `matrix_verification_list`.

use synara_core::app::verification::NativeVerificationInbox;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn verification_list(
    core: &Core,
) -> Result<NativeVerificationInbox, MatrixAuthCommandError> {
    let response = core
        .verification_list()
        .await
        .map_err(map_verification_list_core_error)?;
    Ok(response)
}

fn map_verification_list_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-crypto.1-start-requires-session",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "Device verification could not be completed.",
            "v-crypto.1-list-unavailable",
        ),
    }
}
