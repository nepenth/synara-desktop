//! Desktop bridges for `in.synara.room_notes` through `Core::command`.

use synara_core::app::account_data::{
    NativeRoomNotesSnapshot, RoomNoteMoveDirection, SynaraRoomNoteItem,
};
use synara_core::transport::{CommandEnvelope, MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

const READ_ONLY_SESSION_GENERATION: u64 = 0;

pub(crate) async fn room_notes_snapshot(
    core: &Core,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    dispatch(core, "matrix_room_notes_snapshot", serde_json::Value::Null).await
}

pub(crate) async fn room_notes_upsert(
    core: &Core,
    item: SynaraRoomNoteItem,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    dispatch(
        core,
        "matrix_room_notes_upsert",
        serde_json::json!({ "item": item }),
    )
    .await
}

pub(crate) async fn room_notes_delete(
    core: &Core,
    room_id: String,
    item_id: String,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    dispatch(
        core,
        "matrix_room_notes_delete",
        serde_json::json!({ "roomId": room_id, "itemId": item_id }),
    )
    .await
}

pub(crate) async fn room_notes_complete_todo(
    core: &Core,
    room_id: String,
    item_id: String,
    completed: bool,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    dispatch(
        core,
        "matrix_room_notes_complete_todo",
        serde_json::json!({
            "roomId": room_id,
            "itemId": item_id,
            "completed": completed,
        }),
    )
    .await
}

pub(crate) async fn room_notes_move_todo(
    core: &Core,
    room_id: String,
    item_id: String,
    direction: RoomNoteMoveDirection,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    dispatch(
        core,
        "matrix_room_notes_move_todo",
        serde_json::json!({
            "roomId": room_id,
            "itemId": item_id,
            "direction": direction,
        }),
    )
    .await
}

async fn dispatch(
    core: &Core,
    command: &str,
    payload: serde_json::Value,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    let response = core
        .command(CommandEnvelope {
            command: command.to_owned(),
            session_generation: READ_ONLY_SESSION_GENERATION,
            request_id: None,
            payload,
        })
        .await
        .map_err(map_room_notes_core_error)?;
    serde_json::from_value(response.payload).map_err(|_| room_notes_response_error())
}

fn map_room_notes_core_error(error: MatrixIpcError) -> MatrixAuthCommandError {
    match error.category {
        MatrixIpcErrorCategory::Forbidden => MatrixAuthCommandError::new(
            "Forbidden",
            "No native Matrix session is active.",
            "d0.4-send-requires-session",
        ),
        MatrixIpcErrorCategory::SdkInvariant => MatrixAuthCommandError::new(
            "InvalidRequest",
            "The native Matrix later/notes request is invalid.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-timeline-room-notes-invalid-item"),
        ),
        _ => MatrixAuthCommandError::new(
            "Unknown",
            "The native Matrix later/notes account data is unavailable.",
            error
                .diagnostic_id
                .as_deref()
                .unwrap_or("v-timeline-room-notes-fetch-failed"),
        ),
    }
}

fn room_notes_response_error() -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix later/notes account data is unavailable.",
        "v-timeline-room-notes-fetch-failed",
    )
}
