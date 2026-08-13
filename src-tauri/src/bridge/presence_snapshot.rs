//! Desktop bridge for `matrix_presence_snapshot` through `Core::command`.
//!
//! Core owns the live `NativePresenceOwner` after the shell attaches it. This
//! adapter builds the envelope and maps closed Core categories onto the
//! existing Tauri error shape. React still invokes `matrix_presence_snapshot`.

use synara_core::app::presence::NativePresenceSnapshotResult;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const PRESENCE_SNAPSHOT_COMMAND: &str = "matrix_presence_snapshot";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn presence_snapshot(
    core: &Core,
    user_id: String,
) -> Result<NativePresenceSnapshotResult, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: PRESENCE_SNAPSHOT_COMMAND.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::json!({ "userId": user_id }),
        })
        .await
        .map_err(map_presence_snapshot_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| presence_snapshot_response_error())
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

fn presence_snapshot_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "Native Matrix presence is unavailable.",
        "v-presence-store-read-failed",
    )
}
