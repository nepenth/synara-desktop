//! Desktop bridge for `matrix_room_list_snapshot` through `Core::command`.

use synara_core::app::room_list::NativeRoomListSnapshot;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn room_list_snapshot(
    core: &Core,
) -> Result<NativeRoomListSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: "matrix_room_list_snapshot".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .map_err(map_room_list_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| room_list_response_error())
}

fn map_room_list_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.2-room-list-requires-session",
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix room list is unavailable.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("d0.2-room-list-open-failed"),
        ),
    }
}

fn room_list_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix room list is unavailable.",
        "d0.2-room-list-open-failed",
    )
}
