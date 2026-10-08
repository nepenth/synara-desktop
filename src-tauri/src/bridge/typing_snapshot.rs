//! Desktop bridge for `matrix_typing_snapshot` through `Core::command`.
//!
//! Core owns the live `NativeTypingOwner` after the shell attaches it. This
//! adapter builds the envelope and maps closed Core categories onto the
//! existing Tauri error shape. React still invokes `matrix_typing_snapshot`.

use synara_core::app::typing::NativeTypingSnapshot;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn typing_snapshot(
    core: &Core,
) -> Result<NativeTypingSnapshot, MatrixAuthCommandError> {
    let response = core
        .typing_snapshot()
        .await
        .map_err(map_typing_snapshot_core_error)?;
    Ok(response)
}

fn map_typing_snapshot_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-rooms.4-typing-owner-user-missing",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix typing request is invalid.",
            "v-rooms.4-typing-invalid-room",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix typing notice is unavailable.",
            "v-rooms.4-typing-notice-failed",
        ),
    }
}
