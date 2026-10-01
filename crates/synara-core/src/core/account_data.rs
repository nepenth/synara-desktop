//! Core command adapters for account data.

use super::*;

/// Exact React/Tauri envelope payload for `matrix_get_room_image_packs`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixGetRoomImagePacksRequest {
    pub(super) room_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixSetImagePackContentRequest {
    pub(super) content: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixSetRoomImagePackRequest {
    pub(super) room_id: String,
    pub(super) state_key: String,
    pub(super) content: serde_json::Value,
}

/// Exact React/Tauri envelope payload for `matrix_mdirect_add`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixMDirectAddRequest {
    pub(super) room_id: String,
    pub(super) user_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_mdirect_remove`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixMDirectRemoveRequest {
    pub(super) room_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixLaterUpsertRequest {
    pub(super) item: SynaraLaterItem,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixLaterCompleteRequest {
    pub(super) item_id: String,
    #[serde(default)]
    pub(super) completed_at: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixLaterSnoozeRequest {
    pub(super) item_id: String,
    pub(super) due_ts: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixLaterMarkRemindedRequest {
    pub(super) item_id: String,
    #[serde(default)]
    pub(super) reminded_at: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixRoomNotesUpsertRequest {
    pub(super) item: SynaraRoomNoteItem,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixRoomNotesItemRequest {
    pub(super) room_id: String,
    pub(super) item_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixRoomNotesCompleteTodoRequest {
    pub(super) room_id: String,
    pub(super) item_id: String,
    pub(super) completed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixRoomNotesMoveTodoRequest {
    pub(super) room_id: String,
    pub(super) item_id: String,
    pub(super) direction: RoomNoteMoveDirection,
}

/// Exact React/Tauri envelope payload for `matrix_room_directory_search`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixRoomDirectorySearchRequest {
    pub(super) session_generation: u64,
    pub(super) request_id: u64,
    #[serde(default)]
    pub(super) server_name: Option<String>,
    #[serde(default)]
    pub(super) term: Option<String>,
    #[serde(default)]
    pub(super) room_type: Option<DirectoryRoomTypeFilter>,
    #[serde(default)]
    pub(super) third_party_instance_id: Option<String>,
    pub(super) limit: u64,
    #[serde(default)]
    pub(super) since: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_room_directory_cancel`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixRoomDirectoryCancelRequest {
    pub(super) session_generation: u64,
    pub(super) request_id: u64,
}

/// Exact React/Tauri envelope payload for `matrix_get_room_directory_visibility`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixGetRoomDirectoryVisibilityRequest {
    pub(super) room_id: String,
    pub(super) session_generation: u64,
}

/// Exact React/Tauri envelope payload for `matrix_set_room_directory_visibility`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MatrixSetRoomDirectoryVisibilityRequest {
    pub(super) room_id: String,
    pub(super) session_generation: u64,
    pub(super) visibility: String,
}

pub(super) fn matrix_agent_approval_history_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("agent-approval-history-invalid-payload"));
        }
        let owner = state.timeline_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("agent-approval-history-no-session")
        })?;
        let snapshot: NativeAgentApprovalHistorySnapshot = owner
            .agent_approval_history_snapshot()
            .await
            .map_err(|diagnostic| {
                MatrixIpcError::new(MatrixIpcErrorCategory::Unknown).with_diagnostic(diagnostic)
            })?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("agent-approval-history-serialization-failed"))
    })
}

pub(super) fn matrix_room_directory_protocols(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error(
                "p2-room-directory-protocols-invalid-payload",
            ));
        }
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-directory-protocols-no-session")
        })?;
        let snapshot: NativeRoomDirectoryProtocols = owner
            .directory_protocols()
            .await
            .map_err(directory_protocols_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-directory-protocols-serialization-failed"))
    })
}

pub(super) fn matrix_room_directory_search(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomDirectorySearchRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-directory-search-invalid-payload"))?;
        if directory_correlation_invalid(payload.session_generation, payload.request_id) {
            return Err(MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("v-rooms.directory-invalid-correlation"));
        }
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-directory-search-no-session")
        })?;
        let result: NativeRoomDirectorySearchResponse = owner
            .directory_search(
                payload.session_generation,
                payload.request_id,
                DirectorySearchInput {
                    server_name: payload.server_name,
                    term: payload.term,
                    room_type: payload.room_type,
                    third_party_instance_id: payload.third_party_instance_id,
                    limit: payload.limit,
                    since: payload.since,
                },
            )
            .await
            .map_err(directory_search_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-room-directory-search-serialization-failed"))
    })
}

pub(super) fn matrix_room_directory_cancel(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomDirectoryCancelRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-directory-cancel-invalid-payload"))?;
        if directory_correlation_invalid(payload.session_generation, payload.request_id) {
            return Err(MatrixIpcError::new(MatrixIpcErrorCategory::SdkInvariant)
                .with_diagnostic("v-rooms.directory-invalid-correlation"));
        }
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-directory-cancel-no-session")
        })?;
        let result: NativeRoomDirectorySearchResponse = owner
            .directory_cancel(payload.session_generation, payload.request_id)
            .map_err(directory_search_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-room-directory-cancel-serialization-failed"))
    })
}

pub(super) fn matrix_get_room_directory_visibility(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixGetRoomDirectoryVisibilityRequest =
            serde_json::from_value(request.payload).map_err(|_| {
                core_state_error("p2-get-room-directory-visibility-invalid-payload")
            })?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-get-room-directory-visibility-no-session")
        })?;
        let result: MatrixRoomDirectoryVisibilityResult = owner
            .get_directory_visibility(&payload.room_id, payload.session_generation)
            .await
            .map_err(directory_visibility_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-get-room-directory-visibility-serialization-failed"))
    })
}

pub(super) fn matrix_set_room_directory_visibility(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetRoomDirectoryVisibilityRequest =
            serde_json::from_value(request.payload).map_err(|_| {
                core_state_error("p2-set-room-directory-visibility-invalid-payload")
            })?;
        let owner = state.join_rule_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-set-room-directory-visibility-no-session")
        })?;
        let result: MatrixRoomDirectoryVisibilityWriteResult = owner
            .set_directory_visibility(
                &payload.room_id,
                payload.session_generation,
                &payload.visibility,
            )
            .await
            .map_err(directory_visibility_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-set-room-directory-visibility-serialization-failed"))
    })
}

pub(super) fn matrix_get_global_image_packs(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-global-image-packs-invalid-payload"));
        }
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-global-image-packs-no-session")
        })?;
        let snapshot: NativeGlobalImagePacksSnapshot = owner
            .snapshot_global()
            .await
            .map_err(image_pack_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-global-image-packs-serialization-failed"))
    })
}

pub(super) fn matrix_get_user_image_pack(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-user-image-pack-invalid-payload"));
        }
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-user-image-pack-no-session")
        })?;
        let snapshot: NativeUserImagePackSnapshot = owner
            .snapshot_user()
            .await
            .map_err(image_pack_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-user-image-pack-serialization-failed"))
    })
}

pub(super) fn matrix_get_room_image_packs(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixGetRoomImagePacksRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-image-packs-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-image-packs-no-session")
        })?;
        let snapshot: NativeRoomImagePacksSnapshot = owner
            .snapshot_room(&payload.room_id)
            .await
            .map_err(image_pack_snapshot_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-image-packs-serialization-failed"))
    })
}

pub(super) fn matrix_set_user_image_pack(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetImagePackContentRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-user-image-pack-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-set-user-image-pack-no-session")
        })?;
        owner
            .set_user(payload.content)
            .await
            .map_err(image_pack_write_owner_error)?;
        Ok(serde_json::json!({"status":"ok"}))
    })
}

pub(super) fn matrix_set_global_image_packs(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetImagePackContentRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-global-image-packs-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-set-global-image-packs-no-session")
        })?;
        owner
            .set_global(payload.content)
            .await
            .map_err(image_pack_write_owner_error)?;
        Ok(serde_json::json!({"status":"ok"}))
    })
}

pub(super) fn matrix_set_room_image_pack(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetRoomImagePackRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-room-image-pack-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-set-room-image-pack-no-session")
        })?;
        owner
            .set_room(&payload.room_id, &payload.state_key, payload.content)
            .await
            .map_err(image_pack_write_owner_error)?;
        Ok(serde_json::json!({"status":"ok"}))
    })
}

pub(super) fn matrix_later_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-later-snapshot-invalid-payload"));
        }
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-later-snapshot-no-session")
        })?;
        let snapshot: NativeLaterSnapshot =
            owner.later_snapshot().await.map_err(later_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-later-snapshot-serialization-failed"))
    })
}

pub(super) fn matrix_later_upsert(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLaterUpsertRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-later-upsert-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-later-upsert-no-session")
        })?;
        let snapshot: NativeLaterSnapshot = owner
            .later_upsert(payload.item)
            .await
            .map_err(later_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-later-upsert-serialization-failed"))
    })
}

pub(super) fn matrix_later_complete(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLaterCompleteRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-later-complete-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-later-complete-no-session")
        })?;
        let snapshot: NativeLaterSnapshot = owner
            .later_complete(payload.item_id, payload.completed_at)
            .await
            .map_err(later_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-later-complete-serialization-failed"))
    })
}

pub(super) fn matrix_later_snooze(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLaterSnoozeRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-later-snooze-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-later-snooze-no-session")
        })?;
        let snapshot: NativeLaterSnapshot = owner
            .later_snooze(payload.item_id, payload.due_ts)
            .await
            .map_err(later_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-later-snooze-serialization-failed"))
    })
}

pub(super) fn matrix_later_clear_completed(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-later-clear-completed-invalid-payload"));
        }
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-later-clear-completed-no-session")
        })?;
        let snapshot: NativeLaterSnapshot = owner
            .later_clear_completed()
            .await
            .map_err(later_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-later-clear-completed-serialization-failed"))
    })
}

pub(super) fn matrix_later_mark_reminded(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLaterMarkRemindedRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-later-mark-reminded-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-later-mark-reminded-no-session")
        })?;
        let snapshot: NativeLaterSnapshot = owner
            .later_mark_reminded(payload.item_id, payload.reminded_at)
            .await
            .map_err(later_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-later-mark-reminded-serialization-failed"))
    })
}

pub(super) fn later_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-timeline-later-invalid-item" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn matrix_room_notes_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-room-notes-snapshot-invalid-payload"));
        }
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-notes-snapshot-no-session")
        })?;
        let snapshot: NativeRoomNotesSnapshot = owner
            .room_notes_snapshot()
            .await
            .map_err(room_notes_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-notes-snapshot-serialization-failed"))
    })
}

pub(super) fn matrix_room_notes_upsert(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomNotesUpsertRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-notes-upsert-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-notes-upsert-no-session")
        })?;
        let snapshot: NativeRoomNotesSnapshot = owner
            .room_notes_upsert(payload.item)
            .await
            .map_err(room_notes_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-notes-upsert-serialization-failed"))
    })
}

pub(super) fn matrix_room_notes_delete(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomNotesItemRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-notes-delete-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-notes-delete-no-session")
        })?;
        let snapshot: NativeRoomNotesSnapshot = owner
            .room_notes_delete(payload.room_id, payload.item_id)
            .await
            .map_err(room_notes_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-notes-delete-serialization-failed"))
    })
}

pub(super) fn matrix_room_notes_complete_todo(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomNotesCompleteTodoRequest =
            serde_json::from_value(request.payload)
                .map_err(|_| core_state_error("p2-room-notes-complete-todo-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-notes-complete-todo-no-session")
        })?;
        let snapshot: NativeRoomNotesSnapshot = owner
            .room_notes_complete_todo(payload.room_id, payload.item_id, payload.completed)
            .await
            .map_err(room_notes_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-notes-complete-todo-serialization-failed"))
    })
}

pub(super) fn matrix_room_notes_move_todo(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomNotesMoveTodoRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-notes-move-todo-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-room-notes-move-todo-no-session")
        })?;
        let snapshot: NativeRoomNotesSnapshot = owner
            .room_notes_move_todo(payload.room_id, payload.item_id, payload.direction)
            .await
            .map_err(room_notes_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-room-notes-move-todo-serialization-failed"))
    })
}

pub(super) fn room_notes_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-timeline-room-notes-invalid-item" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn matrix_mdirect_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-mdirect-snapshot-invalid-payload"));
        }
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-mdirect-snapshot-no-session")
        })?;
        let snapshot: NativeMDirectSnapshot = owner
            .mdirect_snapshot()
            .await
            .map_err(mdirect_owner_error)?;
        serde_json::to_value(snapshot)
            .map_err(|_| core_state_error("p2-mdirect-snapshot-serialization-failed"))
    })
}

pub(super) fn matrix_mdirect_add(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixMDirectAddRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-mdirect-add-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-mdirect-add-no-session")
        })?;
        let result: NativeMDirectMutationResult = owner
            .mdirect_add(&payload.room_id, &payload.user_id)
            .await
            .map_err(mdirect_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-mdirect-add-serialization-failed"))
    })
}

pub(super) fn matrix_mdirect_remove(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixMDirectRemoveRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-mdirect-remove-invalid-payload"))?;
        let owner = state.image_pack_owner()?.ok_or_else(|| {
            MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
                .with_diagnostic("p2-mdirect-remove-no-session")
        })?;
        let result: NativeMDirectMutationResult = owner
            .mdirect_remove(&payload.room_id)
            .await
            .map_err(mdirect_owner_error)?;
        serde_json::to_value(result)
            .map_err(|_| core_state_error("p2-mdirect-remove-serialization-failed"))
    })
}

pub(super) fn mdirect_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms.5-mdirect-invalid-room" | "v-rooms.5-mdirect-invalid-user" => {
            MatrixIpcErrorCategory::SdkInvariant
        }
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn image_pack_write_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-send.r-pack-write-invalid-content"
        | "v-send.r-pack-read-invalid-room"
        | "v-send.r-pack-write-room-missing" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn image_pack_snapshot_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-send.r-pack-read-invalid-room" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-pack-read-no-user" | "v-send.r-pack-read-subscribe-no-user" => {
            MatrixIpcErrorCategory::Forbidden
        }
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}
