//! Desktop bridge for `matrix_presence_snapshot` through `Core::command`.
//!
//! Core owns the live `NativePresenceOwner` after the shell attaches it. This
//! adapter builds the envelope and maps closed Core categories onto the
//! existing Tauri error shape. React still invokes `matrix_presence_snapshot`.

use synara_core::app::presence::NativePresenceSnapshotResult;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn presence_snapshot(
    core: &Core,
    user_id: String,
) -> Result<NativePresenceSnapshotResult, MatrixAuthCommandError> {
    let response = core
        .presence_snapshot(synara_core::core_api::MatrixPresenceSnapshotRequest { user_id })
        .await
        .map_err(map_presence_snapshot_core_error)?;
    Ok(response)
}

fn map_presence_snapshot_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-presence-user-owner-missing",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix presence request is invalid.",
            "v-presence-invalid-user-id",
        ),
        MatrixIpcErrorCategory::StaleSessionGeneration => MatrixAuthCommandError::new(
            "StaleSessionGeneration",
            "The native Matrix presence session changed.",
            "v-presence-stale-session-generation",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "Native Matrix presence is unavailable.",
            "v-presence-store-read-failed",
        ),
    }
}
