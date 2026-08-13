use std::sync::Arc;

use super::*;

#[tauri::command]
pub async fn matrix_typing_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeTypingSnapshot, MatrixAuthCommandError> {
    crate::bridge::typing_snapshot::typing_snapshot(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_typing_set(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    typing: bool,
) -> Result<(), MatrixAuthCommandError> {
    crate::bridge::typing_set::typing_set(core.inner().as_ref(), room_id, typing).await
}

pub(super) fn map_typing_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-rooms.4-typing-invalid-room" => (
            "InvalidRequest",
            "The native Matrix typing request is invalid.",
        ),
        "v-rooms.4-typing-room-missing" | "v-rooms.4-typing-room-not-joined" => {
            ("NotFound", "The native Matrix typing room was not found.")
        }
        "v-rooms.4-typing-owner-user-missing" => {
            ("Forbidden", "No native Matrix session is active.")
        }
        _ => ("Unknown", "The native Matrix typing notice is unavailable."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}
