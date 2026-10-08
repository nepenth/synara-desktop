//! Core command adapters for account data.

use super::*;
use crate::dto::WriteAck;

/// Exact React/Tauri envelope payload for `matrix_get_room_image_packs`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixGetRoomImagePacksRequest {
    pub room_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixSetImagePackContentRequest {
    pub content: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixSetRoomImagePackRequest {
    pub room_id: String,
    pub state_key: String,
    pub content: serde_json::Value,
}

/// Exact React/Tauri envelope payload for `matrix_mdirect_add`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixMDirectAddRequest {
    pub room_id: String,
    pub user_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_mdirect_remove`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixMDirectRemoveRequest {
    pub room_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixLaterUpsertRequest {
    pub item: SynaraLaterItem,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixLaterCompleteRequest {
    pub item_id: String,
    #[serde(default)]
    pub completed_at: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixLaterSnoozeRequest {
    pub item_id: String,
    pub due_ts: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixLaterMarkRemindedRequest {
    pub item_id: String,
    #[serde(default)]
    pub reminded_at: Option<f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomNotesUpsertRequest {
    pub item: SynaraRoomNoteItem,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomNotesItemRequest {
    pub room_id: String,
    pub item_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomNotesCompleteTodoRequest {
    pub room_id: String,
    pub item_id: String,
    pub completed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomNotesMoveTodoRequest {
    pub room_id: String,
    pub item_id: String,
    pub direction: RoomNoteMoveDirection,
}

/// Exact React/Tauri envelope payload for `matrix_room_directory_search`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomDirectorySearchRequest {
    pub session_generation: u64,
    pub request_id: u64,
    #[serde(default)]
    pub server_name: Option<String>,
    #[serde(default)]
    pub term: Option<String>,
    #[serde(default)]
    pub room_type: Option<DirectoryRoomTypeFilter>,
    #[serde(default)]
    pub third_party_instance_id: Option<String>,
    pub limit: u64,
    #[serde(default)]
    pub since: Option<String>,
}

/// Exact React/Tauri envelope payload for `matrix_room_directory_cancel`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomDirectoryCancelRequest {
    pub session_generation: u64,
    pub request_id: u64,
}

/// Exact React/Tauri envelope payload for `matrix_get_room_directory_visibility`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixGetRoomDirectoryVisibilityRequest {
    pub room_id: String,
    pub session_generation: u64,
}

/// Exact React/Tauri envelope payload for `matrix_set_room_directory_visibility`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixSetRoomDirectoryVisibilityRequest {
    pub room_id: String,
    pub session_generation: u64,
    pub visibility: String,
}

/// Typed `matrix_agent_approval_history_snapshot`.
pub(super) async fn agent_approval_history_snapshot(
    state: &Arc<CoreState>,
) -> Result<NativeAgentApprovalHistorySnapshot, MatrixIpcError> {
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
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_agent_approval_history_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("agent-approval-history-invalid-payload"));
        }
        let response = agent_approval_history_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("agent-approval-history-serialization-failed"))
    })
}

/// Typed `matrix_room_directory_protocols`.
pub(super) async fn room_directory_protocols(
    state: &Arc<CoreState>,
) -> Result<NativeRoomDirectoryProtocols, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-directory-protocols-no-session")
    })?;
    let snapshot: NativeRoomDirectoryProtocols = owner
        .directory_protocols()
        .await
        .map_err(directory_protocols_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
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
        let response = room_directory_protocols(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-directory-protocols-serialization-failed"))
    })
}

/// Typed `matrix_room_directory_search`.
pub(super) async fn room_directory_search(
    state: &Arc<CoreState>,
    payload: MatrixRoomDirectorySearchRequest,
) -> Result<NativeRoomDirectorySearchResponse, MatrixIpcError> {
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
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_room_directory_search(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomDirectorySearchRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-directory-search-invalid-payload"))?;
        let response = room_directory_search(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-directory-search-serialization-failed"))
    })
}

/// Typed `matrix_room_directory_cancel`.
pub(super) async fn room_directory_cancel(
    state: &Arc<CoreState>,
    payload: MatrixRoomDirectoryCancelRequest,
) -> Result<NativeRoomDirectorySearchResponse, MatrixIpcError> {
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
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_room_directory_cancel(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomDirectoryCancelRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-directory-cancel-invalid-payload"))?;
        let response = room_directory_cancel(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-directory-cancel-serialization-failed"))
    })
}

/// Typed `matrix_get_room_directory_visibility`.
pub(super) async fn get_room_directory_visibility(
    state: &Arc<CoreState>,
    payload: MatrixGetRoomDirectoryVisibilityRequest,
) -> Result<MatrixRoomDirectoryVisibilityResult, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-get-room-directory-visibility-no-session")
    })?;
    let result: MatrixRoomDirectoryVisibilityResult = owner
        .get_directory_visibility(&payload.room_id, payload.session_generation)
        .await
        .map_err(directory_visibility_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_get_room_directory_visibility(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixGetRoomDirectoryVisibilityRequest =
            serde_json::from_value(request.payload).map_err(|_| {
                core_state_error("p2-get-room-directory-visibility-invalid-payload")
            })?;
        let response = get_room_directory_visibility(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-get-room-directory-visibility-serialization-failed"))
    })
}

/// Typed `matrix_set_room_directory_visibility`.
pub(super) async fn set_room_directory_visibility(
    state: &Arc<CoreState>,
    payload: MatrixSetRoomDirectoryVisibilityRequest,
) -> Result<MatrixRoomDirectoryVisibilityWriteResult, MatrixIpcError> {
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
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_set_room_directory_visibility(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetRoomDirectoryVisibilityRequest =
            serde_json::from_value(request.payload).map_err(|_| {
                core_state_error("p2-set-room-directory-visibility-invalid-payload")
            })?;
        let response = set_room_directory_visibility(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-set-room-directory-visibility-serialization-failed"))
    })
}

/// Typed `matrix_get_global_image_packs`.
pub(super) async fn get_global_image_packs(
    state: &Arc<CoreState>,
) -> Result<NativeGlobalImagePacksSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-global-image-packs-no-session")
    })?;
    let snapshot: NativeGlobalImagePacksSnapshot = owner
        .snapshot_global()
        .await
        .map_err(image_pack_snapshot_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_get_global_image_packs(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-global-image-packs-invalid-payload"));
        }
        let response = get_global_image_packs(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-global-image-packs-serialization-failed"))
    })
}

/// Typed `matrix_get_user_image_pack`.
pub(super) async fn get_user_image_pack(
    state: &Arc<CoreState>,
) -> Result<NativeUserImagePackSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-user-image-pack-no-session")
    })?;
    let snapshot: NativeUserImagePackSnapshot = owner
        .snapshot_user()
        .await
        .map_err(image_pack_snapshot_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_get_user_image_pack(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-user-image-pack-invalid-payload"));
        }
        let response = get_user_image_pack(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-user-image-pack-serialization-failed"))
    })
}

/// Typed `matrix_get_room_image_packs`.
pub(super) async fn get_room_image_packs(
    state: &Arc<CoreState>,
    payload: MatrixGetRoomImagePacksRequest,
) -> Result<NativeRoomImagePacksSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-image-packs-no-session")
    })?;
    let snapshot: NativeRoomImagePacksSnapshot = owner
        .snapshot_room(&payload.room_id)
        .await
        .map_err(image_pack_snapshot_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_get_room_image_packs(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixGetRoomImagePacksRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-image-packs-invalid-payload"))?;
        let response = get_room_image_packs(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-image-packs-serialization-failed"))
    })
}

/// `{"status":"ok"}` readback of an account-data write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixStatusOk {
    /// Always `ok`.
    pub status: WriteAck,
}

impl MatrixStatusOk {
    pub(super) fn ok() -> Self {
        Self {
            status: crate::dto::WriteAck::Ok,
        }
    }
}

/// Typed `matrix_set_user_image_pack`.
pub(super) async fn set_user_image_pack(
    state: &Arc<CoreState>,
    payload: MatrixSetImagePackContentRequest,
) -> Result<MatrixStatusOk, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-set-user-image-pack-no-session")
    })?;
    owner
        .set_user(payload.content)
        .await
        .map_err(image_pack_write_owner_error)?;
    Ok(MatrixStatusOk::ok())
}

#[cfg(test)]
pub(super) fn matrix_set_user_image_pack(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetImagePackContentRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-user-image-pack-invalid-payload"))?;
        let response = set_user_image_pack(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-image-pack-write-serialization-failed"))
    })
}

/// Typed `matrix_set_global_image_packs`.
pub(super) async fn set_global_image_packs(
    state: &Arc<CoreState>,
    payload: MatrixSetImagePackContentRequest,
) -> Result<MatrixStatusOk, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-set-global-image-packs-no-session")
    })?;
    owner
        .set_global(payload.content)
        .await
        .map_err(image_pack_write_owner_error)?;
    Ok(MatrixStatusOk::ok())
}

#[cfg(test)]
pub(super) fn matrix_set_global_image_packs(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetImagePackContentRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-global-image-packs-invalid-payload"))?;
        let response = set_global_image_packs(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-image-pack-write-serialization-failed"))
    })
}

/// Typed `matrix_set_room_image_pack`.
pub(super) async fn set_room_image_pack(
    state: &Arc<CoreState>,
    payload: MatrixSetRoomImagePackRequest,
) -> Result<MatrixStatusOk, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-set-room-image-pack-no-session")
    })?;
    owner
        .set_room(&payload.room_id, &payload.state_key, payload.content)
        .await
        .map_err(image_pack_write_owner_error)?;
    Ok(MatrixStatusOk::ok())
}

#[cfg(test)]
pub(super) fn matrix_set_room_image_pack(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetRoomImagePackRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-room-image-pack-invalid-payload"))?;
        let response = set_room_image_pack(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-image-pack-write-serialization-failed"))
    })
}

/// Typed `matrix_later_snapshot`.
pub(super) async fn later_snapshot(
    state: &Arc<CoreState>,
) -> Result<NativeLaterSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-later-snapshot-no-session")
    })?;
    let snapshot: NativeLaterSnapshot = owner.later_snapshot().await.map_err(later_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_later_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-later-snapshot-invalid-payload"));
        }
        let response = later_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-later-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_later_upsert`.
pub(super) async fn later_upsert(
    state: &Arc<CoreState>,
    payload: MatrixLaterUpsertRequest,
) -> Result<NativeLaterSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-later-upsert-no-session")
    })?;
    let snapshot: NativeLaterSnapshot = owner
        .later_upsert(payload.item)
        .await
        .map_err(later_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_later_upsert(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLaterUpsertRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-later-upsert-invalid-payload"))?;
        let response = later_upsert(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-later-upsert-serialization-failed"))
    })
}

/// Typed `matrix_later_complete`.
pub(super) async fn later_complete(
    state: &Arc<CoreState>,
    payload: MatrixLaterCompleteRequest,
) -> Result<NativeLaterSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-later-complete-no-session")
    })?;
    let snapshot: NativeLaterSnapshot = owner
        .later_complete(payload.item_id, payload.completed_at)
        .await
        .map_err(later_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_later_complete(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLaterCompleteRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-later-complete-invalid-payload"))?;
        let response = later_complete(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-later-complete-serialization-failed"))
    })
}

/// Typed `matrix_later_snooze`.
pub(super) async fn later_snooze(
    state: &Arc<CoreState>,
    payload: MatrixLaterSnoozeRequest,
) -> Result<NativeLaterSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-later-snooze-no-session")
    })?;
    let snapshot: NativeLaterSnapshot = owner
        .later_snooze(payload.item_id, payload.due_ts)
        .await
        .map_err(later_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_later_snooze(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLaterSnoozeRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-later-snooze-invalid-payload"))?;
        let response = later_snooze(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-later-snooze-serialization-failed"))
    })
}

/// Typed `matrix_later_clear_completed`.
pub(super) async fn later_clear_completed(
    state: &Arc<CoreState>,
) -> Result<NativeLaterSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-later-clear-completed-no-session")
    })?;
    let snapshot: NativeLaterSnapshot = owner
        .later_clear_completed()
        .await
        .map_err(later_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_later_clear_completed(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-later-clear-completed-invalid-payload"));
        }
        let response = later_clear_completed(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-later-clear-completed-serialization-failed"))
    })
}

/// Typed `matrix_later_mark_reminded`.
pub(super) async fn later_mark_reminded(
    state: &Arc<CoreState>,
    payload: MatrixLaterMarkRemindedRequest,
) -> Result<NativeLaterSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-later-mark-reminded-no-session")
    })?;
    let snapshot: NativeLaterSnapshot = owner
        .later_mark_reminded(payload.item_id, payload.reminded_at)
        .await
        .map_err(later_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_later_mark_reminded(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixLaterMarkRemindedRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-later-mark-reminded-invalid-payload"))?;
        let response = later_mark_reminded(&state, payload).await?;
        serde_json::to_value(response)
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

/// Typed `matrix_room_notes_snapshot`.
pub(super) async fn room_notes_snapshot(
    state: &Arc<CoreState>,
) -> Result<NativeRoomNotesSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-notes-snapshot-no-session")
    })?;
    let snapshot: NativeRoomNotesSnapshot = owner
        .room_notes_snapshot()
        .await
        .map_err(room_notes_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_room_notes_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-room-notes-snapshot-invalid-payload"));
        }
        let response = room_notes_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-notes-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_room_notes_upsert`.
pub(super) async fn room_notes_upsert(
    state: &Arc<CoreState>,
    payload: MatrixRoomNotesUpsertRequest,
) -> Result<NativeRoomNotesSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-notes-upsert-no-session")
    })?;
    let snapshot: NativeRoomNotesSnapshot = owner
        .room_notes_upsert(payload.item)
        .await
        .map_err(room_notes_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_room_notes_upsert(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomNotesUpsertRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-notes-upsert-invalid-payload"))?;
        let response = room_notes_upsert(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-notes-upsert-serialization-failed"))
    })
}

/// Typed `matrix_room_notes_delete`.
pub(super) async fn room_notes_delete(
    state: &Arc<CoreState>,
    payload: MatrixRoomNotesItemRequest,
) -> Result<NativeRoomNotesSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-notes-delete-no-session")
    })?;
    let snapshot: NativeRoomNotesSnapshot = owner
        .room_notes_delete(payload.room_id, payload.item_id)
        .await
        .map_err(room_notes_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_room_notes_delete(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomNotesItemRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-notes-delete-invalid-payload"))?;
        let response = room_notes_delete(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-notes-delete-serialization-failed"))
    })
}

/// Typed `matrix_room_notes_complete_todo`.
pub(super) async fn room_notes_complete_todo(
    state: &Arc<CoreState>,
    payload: MatrixRoomNotesCompleteTodoRequest,
) -> Result<NativeRoomNotesSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-notes-complete-todo-no-session")
    })?;
    let snapshot: NativeRoomNotesSnapshot = owner
        .room_notes_complete_todo(payload.room_id, payload.item_id, payload.completed)
        .await
        .map_err(room_notes_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_room_notes_complete_todo(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomNotesCompleteTodoRequest =
            serde_json::from_value(request.payload)
                .map_err(|_| core_state_error("p2-room-notes-complete-todo-invalid-payload"))?;
        let response = room_notes_complete_todo(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-notes-complete-todo-serialization-failed"))
    })
}

/// Typed `matrix_room_notes_move_todo`.
pub(super) async fn room_notes_move_todo(
    state: &Arc<CoreState>,
    payload: MatrixRoomNotesMoveTodoRequest,
) -> Result<NativeRoomNotesSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-notes-move-todo-no-session")
    })?;
    let snapshot: NativeRoomNotesSnapshot = owner
        .room_notes_move_todo(payload.room_id, payload.item_id, payload.direction)
        .await
        .map_err(room_notes_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_room_notes_move_todo(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomNotesMoveTodoRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-notes-move-todo-invalid-payload"))?;
        let response = room_notes_move_todo(&state, payload).await?;
        serde_json::to_value(response)
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

/// Typed `matrix_mdirect_snapshot`.
pub(super) async fn mdirect_snapshot(
    state: &Arc<CoreState>,
) -> Result<NativeMDirectSnapshot, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-mdirect-snapshot-no-session")
    })?;
    let snapshot: NativeMDirectSnapshot = owner
        .mdirect_snapshot()
        .await
        .map_err(mdirect_owner_error)?;
    Ok(snapshot)
}

#[cfg(test)]
pub(super) fn matrix_mdirect_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-mdirect-snapshot-invalid-payload"));
        }
        let response = mdirect_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-mdirect-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_mdirect_add`.
pub(super) async fn mdirect_add(
    state: &Arc<CoreState>,
    payload: MatrixMDirectAddRequest,
) -> Result<NativeMDirectMutationResult, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-mdirect-add-no-session")
    })?;
    let result: NativeMDirectMutationResult = owner
        .mdirect_add(&payload.room_id, &payload.user_id)
        .await
        .map_err(mdirect_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_mdirect_add(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixMDirectAddRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-mdirect-add-invalid-payload"))?;
        let response = mdirect_add(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-mdirect-add-serialization-failed"))
    })
}

/// Typed `matrix_mdirect_remove`.
pub(super) async fn mdirect_remove(
    state: &Arc<CoreState>,
    payload: MatrixMDirectRemoveRequest,
) -> Result<NativeMDirectMutationResult, MatrixIpcError> {
    let owner = state.image_pack_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-mdirect-remove-no-session")
    })?;
    let result: NativeMDirectMutationResult = owner
        .mdirect_remove(&payload.room_id)
        .await
        .map_err(mdirect_owner_error)?;
    Ok(result)
}

#[cfg(test)]
pub(super) fn matrix_mdirect_remove(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixMDirectRemoveRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-mdirect-remove-invalid-payload"))?;
        let response = mdirect_remove(&state, payload).await?;
        serde_json::to_value(response)
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

/// Registers this domain's JSON adapters with the test-only command registry.
#[cfg(test)]
pub(super) fn register_commands(registry: &mut CommandRegistry) {
    registry
        .register(
            "matrix_room_directory_protocols",
            matrix_room_directory_protocols,
        )
        .expect("built-in matrix_room_directory_protocols must remain in the command census");
    registry
        .register("matrix_room_directory_search", matrix_room_directory_search)
        .expect("built-in matrix_room_directory_search must remain in the command census");
    registry
        .register("matrix_room_directory_cancel", matrix_room_directory_cancel)
        .expect("built-in matrix_room_directory_cancel must remain in the command census");
    registry
        .register(
            "matrix_get_room_directory_visibility",
            matrix_get_room_directory_visibility,
        )
        .expect("built-in matrix_get_room_directory_visibility must remain in the command census");
    registry
        .register(
            "matrix_set_room_directory_visibility",
            matrix_set_room_directory_visibility,
        )
        .expect("built-in matrix_set_room_directory_visibility must remain in the command census");
    registry
        .register(
            "matrix_get_global_image_packs",
            matrix_get_global_image_packs,
        )
        .expect("built-in matrix_get_global_image_packs must remain in the command census");
    registry
        .register("matrix_get_user_image_pack", matrix_get_user_image_pack)
        .expect("built-in matrix_get_user_image_pack must remain in the command census");
    registry
        .register("matrix_get_room_image_packs", matrix_get_room_image_packs)
        .expect("built-in matrix_get_room_image_packs must remain in the command census");
    registry
        .register("matrix_set_user_image_pack", matrix_set_user_image_pack)
        .expect("built-in matrix_set_user_image_pack must remain in the command census");
    registry
        .register(
            "matrix_set_global_image_packs",
            matrix_set_global_image_packs,
        )
        .expect("built-in matrix_set_global_image_packs must remain in the command census");
    registry
        .register("matrix_set_room_image_pack", matrix_set_room_image_pack)
        .expect("built-in matrix_set_room_image_pack must remain in the command census");
    registry
        .register("matrix_later_snapshot", matrix_later_snapshot)
        .expect("built-in matrix_later_snapshot must remain in the command census");
    registry
        .register("matrix_later_upsert", matrix_later_upsert)
        .expect("built-in matrix_later_upsert must remain in the command census");
    registry
        .register("matrix_later_complete", matrix_later_complete)
        .expect("built-in matrix_later_complete must remain in the command census");
    registry
        .register("matrix_later_snooze", matrix_later_snooze)
        .expect("built-in matrix_later_snooze must remain in the command census");
    registry
        .register("matrix_later_clear_completed", matrix_later_clear_completed)
        .expect("built-in matrix_later_clear_completed must remain in the command census");
    registry
        .register("matrix_later_mark_reminded", matrix_later_mark_reminded)
        .expect("built-in matrix_later_mark_reminded must remain in the command census");
    registry
        .register("matrix_room_notes_snapshot", matrix_room_notes_snapshot)
        .expect("built-in matrix_room_notes_snapshot must remain in the command census");
    registry
        .register("matrix_room_notes_upsert", matrix_room_notes_upsert)
        .expect("built-in matrix_room_notes_upsert must remain in the command census");
    registry
        .register("matrix_room_notes_delete", matrix_room_notes_delete)
        .expect("built-in matrix_room_notes_delete must remain in the command census");
    registry
        .register(
            "matrix_room_notes_complete_todo",
            matrix_room_notes_complete_todo,
        )
        .expect("built-in matrix_room_notes_complete_todo must remain in the command census");
    registry
        .register("matrix_room_notes_move_todo", matrix_room_notes_move_todo)
        .expect("built-in matrix_room_notes_move_todo must remain in the command census");
    registry
        .register("matrix_mdirect_snapshot", matrix_mdirect_snapshot)
        .expect("built-in matrix_mdirect_snapshot must remain in the command census");
    registry
        .register("matrix_mdirect_add", matrix_mdirect_add)
        .expect("built-in matrix_mdirect_add must remain in the command census");
    registry
        .register("matrix_mdirect_remove", matrix_mdirect_remove)
        .expect("built-in matrix_mdirect_remove must remain in the command census");
    registry
        .register(
            "matrix_agent_approval_history_snapshot",
            matrix_agent_approval_history_snapshot,
        )
        .expect(
            "built-in matrix_agent_approval_history_snapshot must remain in the command census",
        );
}
