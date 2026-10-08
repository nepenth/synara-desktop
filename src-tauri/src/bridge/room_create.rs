//! Desktop bridge for room create through `Core::command`.

use synara_core::app::room_ops::MatrixRoomCreateRequest;
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn room_create(
    core: &Core,
    request: MatrixRoomCreateRequest,
) -> Result<String, MatrixAuthCommandError> {
    core.room_create(request)
        .await
        .map_err(map_room_create_core_error)
        .and_then(|room_id| {
            (!room_id.is_empty())
                .then_some(room_id)
                .ok_or_else(room_create_response_error)
        })
}

fn map_room_create_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    let diagnostic = error
        .diagnostic_id
        .as_deref()
        .unwrap_or("v-rooms-room-create-failed");
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix room create request is invalid.",
            diagnostic,
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix room could not be created.",
            diagnostic,
        ),
    }
}

fn room_create_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix room could not be created.",
        "v-rooms-room-create-failed",
    )
}
