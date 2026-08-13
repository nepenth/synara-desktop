//! Desktop bridge for `matrix_invites_snapshot` through `Core::command`.

use synara_core::app::room_list::NativeInviteSnapshot;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn invites_snapshot(
    core: &Core,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: "matrix_invites_snapshot".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .map_err(map_invites_snapshot_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| invites_snapshot_response_error())
}

fn map_invites_snapshot_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-rooms.1-invites-requires-session",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix invite inbox is unavailable.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-rooms.1-invite-member-read-failed"),
        ),
    }
}

fn invites_snapshot_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix invite inbox is unavailable.",
        "v-rooms.1-invite-member-read-failed",
    )
}
