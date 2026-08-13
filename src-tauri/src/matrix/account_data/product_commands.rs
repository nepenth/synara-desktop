use super::*;

#[tauri::command]
pub async fn matrix_mdirect_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeMDirectSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    snapshot_mdirect(&active.client, active.sync.session_generation())
        .await
        .map_err(map_mdirect_error)
}

/// V-SEND.R-PACK-READ: personal `im.ponies.user_emotes` account-data pack.
#[tauri::command]
pub async fn matrix_get_user_image_pack(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeUserImagePackSnapshot, MatrixAuthCommandError> {
    crate::bridge::user_image_pack::user_image_pack(core.inner().as_ref()).await
}

/// V-SEND.R-PACK-READ: `im.ponies.room_emotes` state packs for a room.
#[tauri::command]
pub async fn matrix_get_room_image_packs(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
) -> Result<NativeRoomImagePacksSnapshot, MatrixAuthCommandError> {
    crate::bridge::room_image_packs::room_image_packs(core.inner().as_ref(), room_id).await
}

/// V-SEND.R-PACK-READ: global packs enabled via `im.ponies.emote_rooms`.
#[tauri::command]
pub async fn matrix_get_global_image_packs(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeGlobalImagePacksSnapshot, MatrixAuthCommandError> {
    crate::bridge::global_image_packs::global_image_packs(core.inner().as_ref()).await
}

/// V-SEND.R-PACK-WRITE — replace the personal `im.ponies.user_emotes`
/// account-data pack content. Fail-closed: when a native session is live this
/// command is the only path; the JS `mx.setAccountData(PoniesUserEmotes)` must
/// not be used as a fallback.
#[tauri::command]
pub async fn matrix_set_user_image_pack(
    state: State<'_, MatrixAuthState>,
    content: serde_json::Value,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    let client = {
        let session = state.session.lock().await;
        let active = require_session(session.as_ref())?;
        active.client.clone()
    };
    set_user_image_pack(&client, content)
        .await
        .map_err(map_pack_write_error)?;
    Ok(MatrixProfileWriteResult { status: "ok" })
}

/// V-SEND.R-PACK-WRITE — replace the global `im.ponies.emote_rooms`
/// account-data content (add/remove/enable global pack references). Fail-closed:
/// when a native session is live this command is the only path; the JS
/// `mx.setAccountData(PoniesEmoteRooms)` must not be used as a fallback.
#[tauri::command]
pub async fn matrix_set_global_image_packs(
    state: State<'_, MatrixAuthState>,
    content: serde_json::Value,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    let client = {
        let session = state.session.lock().await;
        let active = require_session(session.as_ref())?;
        active.client.clone()
    };
    set_global_image_packs(&client, content)
        .await
        .map_err(map_pack_write_error)?;
    Ok(MatrixProfileWriteResult { status: "ok" })
}

/// V-SEND.R-PACK-WRITE — create/update/delete a `im.ponies.room_emotes` state
/// pack for a room. Empty `{}` content deletes the state event. Fail-closed:
/// when a native session is live this command is the only path; the JS
/// `mx.sendStateEvent(PoniesRoomEmotes)` must not be used as a fallback.
#[tauri::command]
pub async fn matrix_set_room_image_pack(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    state_key: String,
    content: serde_json::Value,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    let client = {
        let session = state.session.lock().await;
        let active = require_session(session.as_ref())?;
        active.client.clone()
    };
    set_room_image_pack(&client, &room_id, &state_key, content)
        .await
        .map_err(map_pack_write_error)?;
    Ok(MatrixProfileWriteResult { status: "ok" })
}

#[tauri::command]
pub async fn matrix_mdirect_add(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    user_id: String,
) -> Result<NativeMDirectMutationResult, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    add_room_to_mdirect(&active.client, &room_id, &user_id)
        .await
        .map_err(map_mdirect_error)
}

#[tauri::command]
pub async fn matrix_mdirect_remove(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeMDirectMutationResult, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    remove_room_from_mdirect(&active.client, &room_id)
        .await
        .map_err(map_mdirect_error)
}

#[tauri::command]
pub async fn matrix_later_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    snapshot_later(&active.client, active.sync.session_generation())
        .await
        .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_later_upsert(
    state: State<'_, MatrixAuthState>,
    item: SynaraLaterItem,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    upsert_later_item(&active.client, active.sync.session_generation(), item)
        .await
        .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_later_complete(
    state: State<'_, MatrixAuthState>,
    item_id: String,
    completed_at: Option<f64>,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    let completed_at = completed_at.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0)
    });
    complete_later_item_live(
        &active.client,
        active.sync.session_generation(),
        item_id,
        completed_at,
    )
    .await
    .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_later_snooze(
    state: State<'_, MatrixAuthState>,
    item_id: String,
    due_ts: f64,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    snooze_later_item_live(
        &active.client,
        active.sync.session_generation(),
        item_id,
        due_ts,
    )
    .await
    .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_later_clear_completed(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    clear_completed_later_live(&active.client, active.sync.session_generation())
        .await
        .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_later_mark_reminded(
    state: State<'_, MatrixAuthState>,
    item_id: String,
    reminded_at: Option<f64>,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    let reminded_at = reminded_at.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0)
    });
    mark_later_reminded_live(
        &active.client,
        active.sync.session_generation(),
        item_id,
        reminded_at,
    )
    .await
    .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_room_notes_snapshot(
    state: State<'_, MatrixAuthState>,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    snapshot_room_notes(&active.client, active.sync.session_generation())
        .await
        .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_room_notes_upsert(
    state: State<'_, MatrixAuthState>,
    item: SynaraRoomNoteItem,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    upsert_room_note_item(&active.client, active.sync.session_generation(), item)
        .await
        .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_room_notes_delete(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    item_id: String,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    delete_room_note_item_live(
        &active.client,
        active.sync.session_generation(),
        room_id,
        item_id,
    )
    .await
    .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_room_notes_complete_todo(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    item_id: String,
    completed: bool,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0);
    complete_room_todo_item_live(
        &active.client,
        active.sync.session_generation(),
        room_id,
        item_id,
        completed,
        now,
    )
    .await
    .map_err(map_later_notes_error)
}

#[tauri::command]
pub async fn matrix_room_notes_move_todo(
    state: State<'_, MatrixAuthState>,
    room_id: String,
    item_id: String,
    direction: RoomNoteMoveDirection,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0);
    move_room_todo_item_live(
        &active.client,
        active.sync.session_generation(),
        room_id,
        item_id,
        direction,
        now,
    )
    .await
    .map_err(map_later_notes_error)
}

pub(super) fn map_mdirect_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-rooms.5-mdirect-invalid-room" | "v-rooms.5-mdirect-invalid-user" => (
            "InvalidRequest",
            "The native Matrix direct-room request is invalid.",
        ),
        _ => (
            "Unknown",
            "The native Matrix direct-room map is unavailable.",
        ),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

pub(super) fn map_pack_read_subscribe_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    MatrixAuthCommandError::new(
        "Unknown",
        "The native Matrix image pack subscription could not be started.",
        diagnostic_id,
    )
}

pub(super) fn map_pack_read_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-send.r-pack-read-invalid-room" => (
            "InvalidRequest",
            "The native Matrix image-pack request is invalid.",
        ),
        "v-send.r-pack-read-room-missing" => (
            "NotFound",
            "The native Matrix image-pack room was not found.",
        ),
        "v-send.r-pack-read-no-user" => ("Forbidden", "No native Matrix session is active."),
        _ => (
            "Unknown",
            "The native Matrix image-pack projection is unavailable.",
        ),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

pub(super) fn map_pack_write_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-send.r-pack-write-invalid-content" => (
            "InvalidRequest",
            "The native Matrix image-pack write is invalid.",
        ),
        _ => (
            "Unknown",
            "The native Matrix image-pack write is unavailable.",
        ),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

pub(super) fn map_later_notes_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-timeline-later-invalid-item" | "v-timeline-room-notes-invalid-item" => (
            "InvalidRequest",
            "The native Matrix later/notes request is invalid.",
        ),
        _ => (
            "Unknown",
            "The native Matrix later/notes account data is unavailable.",
        ),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}
