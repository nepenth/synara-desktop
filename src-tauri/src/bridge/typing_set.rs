//! Desktop bridge for `matrix_typing_set` through `Core::command`.

use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const TYPING_SET_COMMAND: &str = "matrix_typing_set";
const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn typing_set(
    core: &Core,
    room_id: String,
    typing: bool,
) -> Result<(), MatrixAuthCommandError> {
    core.command(CommandEnvelope {
        command: TYPING_SET_COMMAND.to_owned(),
        session_generation: READ_ONLY_SESSION_GENERATION,
        request_id: None,
        payload: serde_json::json!({ "roomId": room_id, "typing": typing }),
    })
    .await
    .map_err(map_typing_set_core_error)?;
    Ok(())
}

fn map_typing_set_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
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
        _ => match error.diagnostic_id.as_deref() {
            Some("v-rooms.4-typing-room-missing") => MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix typing room was not found.",
                "v-rooms.4-typing-room-missing",
            ),
            Some("v-rooms.4-typing-room-not-joined") => MatrixAuthCommandError::new(
                "NotFound",
                "The native Matrix typing room was not found.",
                "v-rooms.4-typing-room-not-joined",
            ),
            _ => MatrixAuthCommandError::new(
                "Unknown",
                "The native Matrix typing notice is unavailable.",
                "v-rooms.4-typing-notice-failed",
            ),
        },
    }
}
