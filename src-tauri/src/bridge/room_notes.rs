//! Desktop bridges for `in.synara.room_notes` through `Core::command`.

use synara_core::app::account_data::{
    NativeRoomNotesSnapshot, RoomNoteMoveDirection, SynaraRoomNoteItem,
};
use synara_core::transport::{MatrixIpcError, MatrixIpcErrorCategory};
use synara_core::Core;

use crate::matrix::auth::product::MatrixAuthCommandError;

pub(crate) async fn room_notes_snapshot(
    core: &Core,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    core.room_notes_snapshot()
        .await
        .map_err(map_room_notes_core_error)
}

pub(crate) async fn room_notes_upsert(
    core: &Core,
    item: SynaraRoomNoteItem,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    core.room_notes_upsert(synara_core::core_api::MatrixRoomNotesUpsertRequest { item })
        .await
        .map_err(map_room_notes_core_error)
}

pub(crate) async fn room_notes_delete(
    core: &Core,
    room_id: String,
    item_id: String,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    core.room_notes_delete(synara_core::core_api::MatrixRoomNotesItemRequest { room_id, item_id })
        .await
        .map_err(map_room_notes_core_error)
}

pub(crate) async fn room_notes_complete_todo(
    core: &Core,
    room_id: String,
    item_id: String,
    completed: bool,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    core.room_notes_complete_todo(synara_core::core_api::MatrixRoomNotesCompleteTodoRequest {
        room_id,
        item_id,
        completed,
    })
    .await
    .map_err(map_room_notes_core_error)
}

pub(crate) async fn room_notes_move_todo(
    core: &Core,
    room_id: String,
    item_id: String,
    direction: RoomNoteMoveDirection,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    core.room_notes_move_todo(synara_core::core_api::MatrixRoomNotesMoveTodoRequest {
        room_id,
        item_id,
        direction,
    })
    .await
    .map_err(map_room_notes_core_error)
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
