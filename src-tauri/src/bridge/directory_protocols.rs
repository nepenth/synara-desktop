//! Desktop bridge for `matrix_room_directory_protocols` through `Core::command`.

use synara_core::app::room_directory::NativeRoomDirectoryProtocols;
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn room_directory_protocols(
    core: &Core,
) -> Result<NativeRoomDirectoryProtocols, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: "matrix_room_directory_protocols".to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload: serde_json::Value::Null,
        })
        .await
        .map_err(map_directory_protocols_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| directory_protocols_response_error())
}

fn map_directory_protocols_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "v-rooms.directory-protocols-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix directory protocol list is invalid.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-rooms.directory-protocol-instance-invalid"),
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix directory protocol list is unavailable.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-rooms.directory-protocols-sdk-failed"),
        ),
    }
}

fn directory_protocols_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix directory protocol list is unavailable.",
        "v-rooms.directory-protocols-sdk-failed",
    )
}
