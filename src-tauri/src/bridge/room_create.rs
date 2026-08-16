//! Desktop bridge for room create through `Core::command`.

use synara_core::app::room_ops::MatrixRoomCreateRequest;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn room_create(
    core: &Core,
    request: MatrixRoomCreateRequest,
) -> Result<String, MatrixAuthCommandError> {
    let payload = serde_json::to_value(&request).map_err(|_| room_create_response_error())?;
    let response = core
        .command(CommandEnvelope {
            command: "matrix_room_create".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload,
        })
        .await
        .map_err(map_room_create_core_error)?;
    response
        .payload
        .as_str()
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(room_create_response_error)
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
