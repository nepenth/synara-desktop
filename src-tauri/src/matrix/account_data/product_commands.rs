use super::*;

#[tauri::command]
pub async fn matrix_mdirect_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeMDirectSnapshot, MatrixAuthCommandError> {
    crate::bridge::mdirect::mdirect_snapshot(core.inner().as_ref()).await
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
    core: State<'_, Arc<synara_core::Core>>,
    content: serde_json::Value,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    crate::bridge::image_pack_writes::set_user_image_pack(core.inner().as_ref(), content).await
}

/// V-SEND.R-PACK-WRITE — replace the global `im.ponies.emote_rooms`
/// account-data content (add/remove/enable global pack references). Fail-closed:
/// when a native session is live this command is the only path; the JS
/// `mx.setAccountData(PoniesEmoteRooms)` must not be used as a fallback.
#[tauri::command]
pub async fn matrix_set_global_image_packs(
    core: State<'_, Arc<synara_core::Core>>,
    content: serde_json::Value,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    crate::bridge::image_pack_writes::set_global_image_packs(core.inner().as_ref(), content).await
}

/// V-SEND.R-PACK-WRITE — create/update/delete a `im.ponies.room_emotes` state
/// pack for a room. Empty `{}` content deletes the state event. Fail-closed:
/// when a native session is live this command is the only path; the JS
/// `mx.sendStateEvent(PoniesRoomEmotes)` must not be used as a fallback.
#[tauri::command]
pub async fn matrix_set_room_image_pack(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    state_key: String,
    content: serde_json::Value,
) -> Result<MatrixProfileWriteResult, MatrixAuthCommandError> {
    crate::bridge::image_pack_writes::set_room_image_pack(
        core.inner().as_ref(),
        room_id,
        state_key,
        content,
    )
    .await
}

#[tauri::command]
pub async fn matrix_mdirect_add(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    user_id: String,
) -> Result<NativeMDirectMutationResult, MatrixAuthCommandError> {
    crate::bridge::mdirect::mdirect_add(core.inner().as_ref(), room_id, user_id).await
}

#[tauri::command]
pub async fn matrix_mdirect_remove(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
) -> Result<NativeMDirectMutationResult, MatrixAuthCommandError> {
    crate::bridge::mdirect::mdirect_remove(core.inner().as_ref(), room_id).await
}

#[tauri::command]
pub async fn matrix_later_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    crate::bridge::later::later_snapshot(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_push_rules_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<synara_core::app::notifications::MatrixPushRulesSnapshot, MatrixAuthCommandError> {
    crate::bridge::push_rules::push_rules_snapshot(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_push_rules_set_default(
    core: State<'_, Arc<synara_core::Core>>,
    encrypted: bool,
    one_to_one: bool,
    mode: String,
) -> Result<synara_core::app::notifications::MatrixPushRulesWriteResult, MatrixAuthCommandError> {
    crate::bridge::push_rules::push_rules_set_default(
        core.inner().as_ref(),
        encrypted,
        one_to_one,
        mode,
    )
    .await
}

#[tauri::command]
pub async fn matrix_push_rules_set_mention(
    core: State<'_, Arc<synara_core::Core>>,
    rule_id: String,
    enabled: bool,
) -> Result<synara_core::app::notifications::MatrixPushRulesWriteResult, MatrixAuthCommandError> {
    crate::bridge::push_rules::push_rules_set_mention(core.inner().as_ref(), rule_id, enabled).await
}

#[tauri::command]
pub async fn matrix_push_rules_add_keyword(
    core: State<'_, Arc<synara_core::Core>>,
    keyword: String,
) -> Result<synara_core::app::notifications::MatrixPushRulesWriteResult, MatrixAuthCommandError> {
    crate::bridge::push_rules::push_rules_add_keyword(core.inner().as_ref(), keyword).await
}

#[tauri::command]
pub async fn matrix_push_rules_remove_keyword(
    core: State<'_, Arc<synara_core::Core>>,
    keyword: String,
) -> Result<synara_core::app::notifications::MatrixPushRulesWriteResult, MatrixAuthCommandError> {
    crate::bridge::push_rules::push_rules_remove_keyword(core.inner().as_ref(), keyword).await
}

#[tauri::command]
pub async fn matrix_room_notification_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
) -> Result<synara_core::app::notifications::MatrixRoomNotificationSnapshot, MatrixAuthCommandError>
{
    crate::bridge::room_notification::room_notification_snapshot(core.inner().as_ref(), room_id)
        .await
}

#[tauri::command]
pub async fn matrix_room_notification_set(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    mode: String,
) -> Result<
    synara_core::app::notifications::MatrixRoomNotificationWriteResult,
    MatrixAuthCommandError,
> {
    crate::bridge::room_notification::room_notification_set(core.inner().as_ref(), room_id, mode)
        .await
}

#[tauri::command]
pub async fn matrix_room_notifications_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<synara_core::app::notifications::MatrixRoomNotificationsSnapshot, MatrixAuthCommandError>
{
    crate::bridge::room_notification::room_notifications_snapshot(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_later_upsert(
    core: State<'_, Arc<synara_core::Core>>,
    item: SynaraLaterItem,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    crate::bridge::later::later_upsert(core.inner().as_ref(), item).await
}

#[tauri::command]
pub async fn matrix_later_complete(
    core: State<'_, Arc<synara_core::Core>>,
    item_id: String,
    completed_at: Option<f64>,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    crate::bridge::later::later_complete(core.inner().as_ref(), item_id, completed_at).await
}

#[tauri::command]
pub async fn matrix_later_snooze(
    core: State<'_, Arc<synara_core::Core>>,
    item_id: String,
    due_ts: f64,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    crate::bridge::later::later_snooze(core.inner().as_ref(), item_id, due_ts).await
}

#[tauri::command]
pub async fn matrix_later_clear_completed(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    crate::bridge::later::later_clear_completed(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_later_mark_reminded(
    core: State<'_, Arc<synara_core::Core>>,
    item_id: String,
    reminded_at: Option<f64>,
) -> Result<NativeLaterSnapshot, MatrixAuthCommandError> {
    crate::bridge::later::later_mark_reminded(core.inner().as_ref(), item_id, reminded_at).await
}

#[tauri::command]
pub async fn matrix_room_notes_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    crate::bridge::room_notes::room_notes_snapshot(core.inner().as_ref()).await
}

#[tauri::command]
pub async fn matrix_room_notes_upsert(
    core: State<'_, Arc<synara_core::Core>>,
    item: SynaraRoomNoteItem,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    crate::bridge::room_notes::room_notes_upsert(core.inner().as_ref(), item).await
}

#[tauri::command]
pub async fn matrix_room_notes_delete(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    item_id: String,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    crate::bridge::room_notes::room_notes_delete(core.inner().as_ref(), room_id, item_id).await
}

#[tauri::command]
pub async fn matrix_room_notes_complete_todo(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    item_id: String,
    completed: bool,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    crate::bridge::room_notes::room_notes_complete_todo(
        core.inner().as_ref(),
        room_id,
        item_id,
        completed,
    )
    .await
}

#[tauri::command]
pub async fn matrix_room_notes_move_todo(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    item_id: String,
    direction: RoomNoteMoveDirection,
) -> Result<NativeRoomNotesSnapshot, MatrixAuthCommandError> {
    crate::bridge::room_notes::room_notes_move_todo(
        core.inner().as_ref(),
        room_id,
        item_id,
        direction,
    )
    .await
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

/// A9 decision stream: record the platform-observed focused room in Core.
/// `room_id` is None when no room has focus. Unknown rooms fail closed.
#[tauri::command]
pub async fn matrix_notification_focus_set(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: Option<String>,
) -> Result<(), MatrixAuthCommandError> {
    crate::bridge::notification_decision::notification_focus_set(core.inner().as_ref(), room_id)
        .await
}

/// A9 decision stream: apply the Core suppress/show policy to one observed
/// event. Returns the closed show/suppress readback; the renderer delivers
/// shown candidates through the existing platform notification facade.
#[tauri::command]
pub async fn matrix_notification_decide(
    core: State<'_, Arc<synara_core::Core>>,
    request: synara_core::app::notifications::NativeNotificationDecideRequest,
) -> Result<synara_core::app::notifications::NotificationDecisionReadback, MatrixAuthCommandError> {
    crate::bridge::notification_decision::notification_decide(core.inner().as_ref(), request).await
}

/// A9 decision stream: acknowledge a delivered or dismissed candidate.
/// Dedup memory is retained so the same event never re-notifies.
#[tauri::command]
pub async fn matrix_notification_dismiss(
    core: State<'_, Arc<synara_core::Core>>,
    candidate_id: String,
) -> Result<bool, MatrixAuthCommandError> {
    crate::bridge::notification_decision::notification_dismiss(core.inner().as_ref(), candidate_id)
        .await
}

/// A9 decision stream: pending Core-decided candidates in insertion order.
#[tauri::command]
pub async fn matrix_notification_pending_snapshot(
    core: State<'_, Arc<synara_core::Core>>,
) -> Result<Vec<synara_core::dto::NotificationCandidate>, MatrixAuthCommandError> {
    crate::bridge::notification_decision::notification_pending_snapshot(core.inner().as_ref()).await
}
