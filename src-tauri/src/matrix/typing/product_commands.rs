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
    state: State<'_, MatrixAuthState>,
    room_id: String,
    typing: bool,
) -> Result<(), MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    set_typing_notice(&active.client, &room_id, typing)
        .await
        .map_err(map_typing_error)
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
