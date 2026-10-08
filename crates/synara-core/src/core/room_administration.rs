//! Core command adapters for room administration.

use super::*;

/// Fixed public missing-secret label vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrixMissingSecretResponse {
    CrossSigningMaster,
    CrossSigningSelfSigning,
    CrossSigningUserSigning,
    EncryptionBackup,
}

/// Exact React/Tauri envelope payload for `matrix_room_join_rule_snapshot`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomJoinRuleSnapshotRequest {
    pub room_id: String,
    pub session_generation: u64,
}

/// Exact React/Tauri envelope payload for `matrix_room_set_join_rule`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomSetJoinRuleRequest {
    pub room_id: String,
    pub join_rule: String,
    #[serde(default)]
    pub allow_room_ids: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixInviteActionRequest {
    pub room_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomLeaveRequest {
    pub room_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_room_set_favorite`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomSetFavoriteRequest {
    pub room_id: String,
    pub favorite: bool,
}

/// Exact React/Tauri envelope payload for `matrix_room_set_read_state`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomSetReadStateRequest {
    pub room_id: String,
    pub action: NativeTimelineReadAction,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomJoinRequest {
    pub room_id_or_alias: String,
    #[serde(default)]
    pub via_servers: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomModerationRequest {
    pub room_id: String,
    pub user_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomUnbanRequest {
    pub room_id: String,
    pub user_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomSetPowerLevelRequest {
    pub room_id: String,
    pub user_id: String,
    pub power_level: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomSetPowerLevelStateRequest {
    pub room_id: String,
    pub content: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomMembersSnapshotRequest {
    pub room_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_space_hierarchy_snapshot`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixSpaceHierarchySnapshotRequest {
    pub room_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_space_child_set`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixSpaceChildSetRequest {
    pub parent_id: String,
    pub child_id: String,
    pub via: Vec<String>,
    #[serde(default)]
    pub order: Option<String>,
    #[serde(default)]
    pub suggested: Option<bool>,
}

/// Exact React/Tauri envelope payload for `matrix_space_child_remove`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixSpaceChildRemoveRequest {
    pub parent_id: String,
    pub child_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_restricted_join_reparent`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRestrictedJoinReparentRequest {
    pub room_id: String,
    #[serde(default)]
    pub remove_parent_id: Option<String>,
    pub add_parent_id: String,
}

/// Exact React/Tauri envelope payload for `matrix_set_room_name`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixSetRoomNameRequest {
    pub room_id: String,
    pub name: String,
}

/// Exact React/Tauri envelope payload for `matrix_set_room_topic`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixSetRoomTopicRequest {
    pub room_id: String,
    pub topic: String,
}

/// Exact React/Tauri envelope payload for `matrix_set_room_avatar`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixSetRoomAvatarRequest {
    pub room_id: String,
    pub mxc: String,
}

/// Exact React/Tauri envelope payload for leftover native state writes.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixSendStateEventRequest {
    pub room_id: String,
    pub event_type: String,
    #[serde(default)]
    pub state_key: String,
    pub content: serde_json::Value,
}

/// Exact React/Tauri envelope payload for room encryption enable / MSC4362 opt-in.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixEnableRoomEncryptedStateRequest {
    pub room_id: String,
    #[serde(default)]
    pub encrypt_state_events: bool,
}

/// Exact React/Tauri envelope payload for the create/opt-in account setting.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixSetEncryptedStateEventsSettingRequest {
    pub enabled: bool,
}

/// Exact React/Tauri envelope payload for `matrix_room_retention`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatrixRoomRetentionRequest {
    pub room_id: String,
    pub session_generation: u64,
}

/// Typed `matrix_room_list_snapshot`.
pub(super) async fn room_list_snapshot(
    state: &Arc<CoreState>,
) -> Result<NativeRoomListSnapshot, MatrixIpcError> {
    let owner = state.sync_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-list-snapshot-no-session")
    })?;
    let snapshot: NativeRoomListSnapshot = snapshot_from_sync_owner(&owner)
        .await
        .map_err(room_list_snapshot_owner_error)?;
    Ok(snapshot)
}

pub(super) fn matrix_room_list_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-room-list-snapshot-invalid-payload"));
        }
        let response = room_list_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-list-snapshot-serialization-failed"))
    })
}

pub(super) fn room_list_snapshot_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    MatrixIpcError::new(MatrixIpcErrorCategory::Unknown).with_diagnostic(diagnostic_id)
}

/// Typed `matrix_invites_snapshot`.
pub(super) async fn invites_snapshot(
    state: &Arc<CoreState>,
) -> Result<NativeInviteSnapshot, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-invites-snapshot-no-session")
    })?;
    let snapshot: NativeInviteSnapshot = owner
        .invites_snapshot()
        .await
        .map_err(invites_snapshot_owner_error)?;
    Ok(snapshot)
}

pub(super) fn matrix_invites_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error("p2-invites-snapshot-invalid-payload"));
        }
        let response = invites_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-invites-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_invites_accept`.
pub(super) async fn invites_accept(
    state: &Arc<CoreState>,
    payload: MatrixInviteActionRequest,
) -> Result<NativeInviteSnapshot, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-invites-accept-no-session")
    })?;
    let snapshot: NativeInviteSnapshot = owner
        .invite_accept(&payload.room_id)
        .await
        .map_err(invite_action_owner_error)?;
    Ok(snapshot)
}

pub(super) fn matrix_invites_accept(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixInviteActionRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-invites-accept-invalid-payload"))?;
        let response = invites_accept(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-invites-accept-serialization-failed"))
    })
}

/// Typed `matrix_invites_decline`.
pub(super) async fn invites_decline(
    state: &Arc<CoreState>,
    payload: MatrixInviteActionRequest,
) -> Result<NativeInviteSnapshot, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-invites-decline-no-session")
    })?;
    let snapshot: NativeInviteSnapshot = owner
        .invite_decline(&payload.room_id)
        .await
        .map_err(invite_action_owner_error)?;
    Ok(snapshot)
}

pub(super) fn matrix_invites_decline(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixInviteActionRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-invites-decline-invalid-payload"))?;
        let response = invites_decline(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-invites-decline-serialization-failed"))
    })
}

/// Typed `matrix_invites_report_spam`.
pub(super) async fn invites_report_spam(
    state: &Arc<CoreState>,
    payload: MatrixInviteActionRequest,
) -> Result<NativeInviteSnapshot, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-invites-report-spam-no-session")
    })?;
    let snapshot: NativeInviteSnapshot = owner
        .invite_report_spam(&payload.room_id)
        .await
        .map_err(invite_action_owner_error)?;
    Ok(snapshot)
}

pub(super) fn matrix_invites_report_spam(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixInviteActionRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-invites-report-spam-invalid-payload"))?;
        let response = invites_report_spam(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-invites-report-spam-serialization-failed"))
    })
}

/// Typed `matrix_invites_block_sender`.
pub(super) async fn invites_block_sender(
    state: &Arc<CoreState>,
    payload: MatrixInviteActionRequest,
) -> Result<NativeInviteSnapshot, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-invites-block-sender-no-session")
    })?;
    let snapshot: NativeInviteSnapshot = owner
        .invite_block_sender(&payload.room_id)
        .await
        .map_err(invite_action_owner_error)?;
    Ok(snapshot)
}

pub(super) fn matrix_invites_block_sender(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixInviteActionRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-invites-block-sender-invalid-payload"))?;
        let response = invites_block_sender(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-invites-block-sender-serialization-failed"))
    })
}

pub(super) fn invites_snapshot_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms.1-invites-requires-session"
        | "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn invite_action_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms.1-invite-invalid-room"
        | "v-rooms.1-invite-invalid-sender"
        | "v-rooms.1-invite-not-found"
        | "v-rooms.1-invite-member-missing" => MatrixIpcErrorCategory::SdkInvariant,
        "v-rooms.1-invites-requires-session"
        | "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn directory_protocols_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms.directory-protocol-id-cap"
        | "v-rooms.directory-protocol-instance-invalid"
        | "v-rooms.directory-protocol-instance-cap" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn directory_correlation_invalid(session_generation: u64, request_id: u64) -> bool {
    session_generation == 0
        || request_id == 0
        || session_generation > MAX_WIRE_COUNTER
        || request_id > MAX_WIRE_COUNTER
}

pub(super) fn directory_search_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms.directory-invalid-correlation"
        | "v-rooms.directory-invalid-limit"
        | "v-rooms.directory-invalid-server"
        | "v-rooms.directory-invalid-term"
        | "v-rooms.directory-invalid-instance"
        | "v-rooms.directory-invalid-since" => MatrixIpcErrorCategory::SdkInvariant,
        "v-rooms.directory-stale-generation-before-request"
        | "v-rooms.directory-stale-generation-after-request"
        | "v-rooms.directory-cancel-stale-generation" => {
            MatrixIpcErrorCategory::StaleSessionGeneration
        }
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        "v-rooms.directory-federation-forbidden" => MatrixIpcErrorCategory::Forbidden,
        "v-rooms.directory-network-failed" => MatrixIpcErrorCategory::Connectivity,
        "v-rooms.directory-server-not-found" => MatrixIpcErrorCategory::HomeserverUnavailable,
        "v-rooms.directory-rate-limited" => MatrixIpcErrorCategory::RateLimited,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

/// Typed `matrix_room_join_rule_snapshot`.
pub(super) async fn room_join_rule_snapshot(
    state: &Arc<CoreState>,
    payload: MatrixRoomJoinRuleSnapshotRequest,
) -> Result<MatrixRoomJoinRuleSnapshot, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-join-rule-snapshot-no-session")
    })?;
    let snapshot: MatrixRoomJoinRuleSnapshot = owner
        .snapshot(&payload.room_id, payload.session_generation)
        .await
        .map_err(join_rule_snapshot_owner_error)?;
    Ok(snapshot)
}

pub(super) fn matrix_room_join_rule_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomJoinRuleSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-join-rule-snapshot-invalid-payload"))?;
        let response = room_join_rule_snapshot(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-join-rule-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_room_set_join_rule`.
pub(super) async fn room_set_join_rule(
    state: &Arc<CoreState>,
    payload: MatrixRoomSetJoinRuleRequest,
) -> Result<MatrixProfileWriteResult, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-set-join-rule-no-session")
    })?;
    let result: MatrixProfileWriteResult = owner
        .set_join_rule(
            &payload.room_id,
            &payload.join_rule,
            payload.allow_room_ids.as_deref(),
        )
        .await
        .map_err(join_rule_snapshot_owner_error)?;
    Ok(result)
}

pub(super) fn matrix_room_set_join_rule(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomSetJoinRuleRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-set-join-rule-invalid-payload"))?;
        let response = room_set_join_rule(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-set-join-rule-serialization-failed"))
    })
}

/// Typed `matrix_room_leave`.
pub(super) async fn room_leave(
    state: &Arc<CoreState>,
    payload: MatrixRoomLeaveRequest,
) -> Result<(), MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-leave-no-session")
    })?;
    owner
        .leave(&payload.room_id)
        .await
        .map_err(room_leave_join_owner_error)?;
    Ok(())
}

pub(super) fn matrix_room_leave(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomLeaveRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-leave-invalid-payload"))?;
        room_leave(&state, payload).await?;
        Ok(serde_json::Value::Null)
    })
}

/// Typed `matrix_room_join`.
pub(super) async fn room_join(
    state: &Arc<CoreState>,
    payload: MatrixRoomJoinRequest,
) -> Result<(), MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-join-no-session")
    })?;
    owner
        .join(&payload.room_id_or_alias, payload.via_servers)
        .await
        .map_err(room_leave_join_owner_error)?;
    Ok(())
}

pub(super) fn matrix_room_join(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomJoinRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-join-invalid-payload"))?;
        room_join(&state, payload).await?;
        Ok(serde_json::Value::Null)
    })
}

/// Typed `matrix_room_set_favorite`.
pub(super) async fn room_set_favorite(
    state: &Arc<CoreState>,
    payload: MatrixRoomSetFavoriteRequest,
) -> Result<(), MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-set-favorite-no-session")
    })?;
    owner
        .set_favorite(&payload.room_id, payload.favorite)
        .await
        .map_err(room_leave_join_owner_error)?;
    Ok(())
}

pub(super) fn matrix_room_set_favorite(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomSetFavoriteRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-set-favorite-invalid-payload"))?;
        room_set_favorite(&state, payload).await?;
        Ok(serde_json::Value::Null)
    })
}

/// Typed `matrix_room_set_read_state`.
pub(super) async fn room_set_read_state(
    state: &Arc<CoreState>,
    payload: MatrixRoomSetReadStateRequest,
) -> Result<crate::app::timeline::NativeRoomReadStateReadback, MatrixIpcError> {
    let owner = state.timeline_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-set-read-state-no-session")
    })?;
    let readback = owner
        .set_room_read_state(&payload.room_id, payload.action)
        .await
        .map_err(room_read_state_owner_error)?;
    Ok(readback)
}

pub(super) fn matrix_room_set_read_state(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomSetReadStateRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-set-read-state-invalid-payload"))?;
        let response = room_set_read_state(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-set-read-state-serialization-failed"))
    })
}

pub(super) fn room_read_state_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "d0.3-timeline-invalid-room-id" | "v-rooms-room-read-state-room-not-found" => {
            MatrixIpcErrorCategory::SdkInvariant
        }
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn room_leave_join_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms-room-leave-invalid-room"
        | "v-rooms-room-leave-room-not-found"
        | "v-rooms-room-join-invalid-room"
        | "v-rooms-room-join-invalid-via-server"
        | "v-rooms-room-favorite-invalid-room"
        | "v-rooms-room-favorite-room-not-found" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

/// Typed `matrix_room_invite`.
pub(super) async fn room_invite(
    state: &Arc<CoreState>,
    payload: MatrixRoomModerationRequest,
) -> Result<(), MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-invite-no-session")
    })?;
    owner
        .invite(&payload.room_id, &payload.user_id, payload.reason)
        .await
        .map_err(room_moderation_owner_error)?;
    Ok(())
}

pub(super) fn matrix_room_invite(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomModerationRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-invite-invalid-payload"))?;
        room_invite(&state, payload).await?;
        Ok(serde_json::Value::Null)
    })
}

/// Typed `matrix_room_kick`.
pub(super) async fn room_kick(
    state: &Arc<CoreState>,
    payload: MatrixRoomModerationRequest,
) -> Result<(), MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-kick-no-session")
    })?;
    owner
        .kick(&payload.room_id, &payload.user_id, payload.reason)
        .await
        .map_err(room_moderation_owner_error)?;
    Ok(())
}

pub(super) fn matrix_room_kick(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomModerationRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-kick-invalid-payload"))?;
        room_kick(&state, payload).await?;
        Ok(serde_json::Value::Null)
    })
}

/// Typed `matrix_room_ban`.
pub(super) async fn room_ban(
    state: &Arc<CoreState>,
    payload: MatrixRoomModerationRequest,
) -> Result<(), MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-ban-no-session")
    })?;
    owner
        .ban(&payload.room_id, &payload.user_id, payload.reason)
        .await
        .map_err(room_moderation_owner_error)?;
    Ok(())
}

pub(super) fn matrix_room_ban(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomModerationRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-ban-invalid-payload"))?;
        room_ban(&state, payload).await?;
        Ok(serde_json::Value::Null)
    })
}

/// Typed `matrix_room_unban`.
pub(super) async fn room_unban(
    state: &Arc<CoreState>,
    payload: MatrixRoomUnbanRequest,
) -> Result<(), MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-unban-no-session")
    })?;
    owner
        .unban(&payload.room_id, &payload.user_id)
        .await
        .map_err(room_moderation_owner_error)?;
    Ok(())
}

pub(super) fn matrix_room_unban(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomUnbanRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-unban-invalid-payload"))?;
        room_unban(&state, payload).await?;
        Ok(serde_json::Value::Null)
    })
}

/// Typed `matrix_room_create`.
pub(super) async fn room_create(
    state: &Arc<CoreState>,
    payload: MatrixRoomCreateRequest,
) -> Result<String, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-create-no-session")
    })?;
    let room_id = owner
        .create_room(payload)
        .await
        .map_err(room_create_owner_error)?;
    Ok(room_id)
}

pub(super) fn matrix_room_create(state: Arc<CoreState>, request: CommandEnvelope) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomCreateRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-create-invalid-payload"))?;
        let response = room_create(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-create-serialization-failed"))
    })
}

/// Typed `matrix_room_members_snapshot`.
pub(super) async fn room_members_snapshot(
    state: &Arc<CoreState>,
    payload: MatrixRoomMembersSnapshotRequest,
) -> Result<NativeRoomMembersSnapshot, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-members-snapshot-no-session")
    })?;
    let snapshot: NativeRoomMembersSnapshot = owner
        .members_snapshot(&payload.room_id)
        .await
        .map_err(room_members_snapshot_owner_error)?;
    Ok(snapshot)
}

pub(super) fn matrix_room_members_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomMembersSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-members-snapshot-invalid-payload"))?;
        let response = room_members_snapshot(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-members-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_room_power_levels_snapshot`.
pub(super) async fn room_power_levels_snapshot(
    state: &Arc<CoreState>,
    payload: MatrixRoomMembersSnapshotRequest,
) -> Result<NativeRoomPowerLevelsSnapshot, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-power-levels-snapshot-no-session")
    })?;
    let snapshot: NativeRoomPowerLevelsSnapshot = owner
        .power_levels_snapshot(&payload.room_id)
        .await
        .map_err(room_members_snapshot_owner_error)?;
    Ok(snapshot)
}

pub(super) fn matrix_room_power_levels_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomMembersSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-power-levels-snapshot-invalid-payload"))?;
        let response = room_power_levels_snapshot(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-power-levels-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_room_creators_snapshot`.
pub(super) async fn room_creators_snapshot(
    state: &Arc<CoreState>,
    payload: MatrixRoomMembersSnapshotRequest,
) -> Result<NativeRoomCreatorsSnapshot, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-creators-snapshot-no-session")
    })?;
    let snapshot: NativeRoomCreatorsSnapshot = owner
        .creators_snapshot(&payload.room_id)
        .await
        .map_err(room_members_snapshot_owner_error)?;
    Ok(snapshot)
}

pub(super) fn matrix_room_creators_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomMembersSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-creators-snapshot-invalid-payload"))?;
        let response = room_creators_snapshot(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-creators-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_room_power_level_tags_snapshot`.
pub(super) async fn room_power_level_tags_snapshot(
    state: &Arc<CoreState>,
    payload: MatrixRoomMembersSnapshotRequest,
) -> Result<NativeRoomPowerLevelTagsSnapshot, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-power-level-tags-snapshot-no-session")
    })?;
    let snapshot: NativeRoomPowerLevelTagsSnapshot = owner
        .power_level_tags_snapshot(&payload.room_id)
        .await
        .map_err(room_members_snapshot_owner_error)?;
    Ok(snapshot)
}

pub(super) fn matrix_room_power_level_tags_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomMembersSnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-power-level-tags-snapshot-invalid-payload"))?;
        let response = room_power_level_tags_snapshot(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-power-level-tags-snapshot-serialization-failed"))
    })
}

pub(super) fn room_members_snapshot_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms-members-read-invalid-room" | "v-rooms-members-read-room-not-found" => {
            MatrixIpcErrorCategory::SdkInvariant
        }
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn room_create_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms-room-create-invalid-name"
        | "v-rooms-room-create-invalid-topic"
        | "v-rooms-room-create-invalid-room-version"
        | "v-rooms-room-create-invalid-alias"
        | "v-rooms-room-create-invalid-invite"
        | "v-rooms-room-create-invalid-creation-content"
        | "v-rooms-room-create-invalid-additional-creator"
        | "v-rooms-room-create-invalid-parent"
        | "v-rooms-room-create-invalid-join-rule"
        | "v-rooms-room-create-missing-restricted-parent"
        | "v-rooms-room-create-invalid-power-level" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

/// Typed `matrix_space_parents_snapshot`.
pub(super) async fn space_parents_snapshot(
    state: &Arc<CoreState>,
) -> Result<NativeSpaceParentsSnapshot, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-space-parents-snapshot-no-session")
    })?;
    let snapshot: NativeSpaceParentsSnapshot = owner
        .space_parents_snapshot()
        .await
        .map_err(space_owner_error)?;
    Ok(snapshot)
}

pub(super) fn matrix_space_parents_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error(
                "p2-space-parents-snapshot-invalid-payload",
            ));
        }
        let response = space_parents_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-space-parents-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_space_hierarchy_snapshot`.
pub(super) async fn space_hierarchy_snapshot(
    state: &Arc<CoreState>,
    payload: MatrixSpaceHierarchySnapshotRequest,
) -> Result<NativeSpaceHierarchySnapshot, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-space-hierarchy-snapshot-no-session")
    })?;
    let snapshot: NativeSpaceHierarchySnapshot = owner
        .space_hierarchy_snapshot(&payload.room_id)
        .await
        .map_err(space_owner_error)?;
    Ok(snapshot)
}

pub(super) fn matrix_space_hierarchy_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSpaceHierarchySnapshotRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-space-hierarchy-snapshot-invalid-payload"))?;
        let response = space_hierarchy_snapshot(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-space-hierarchy-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_space_children_snapshot`.
pub(super) async fn space_children_snapshot(
    state: &Arc<CoreState>,
) -> Result<NativeSpaceChildrenSnapshot, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-space-children-snapshot-no-session")
    })?;
    let snapshot: NativeSpaceChildrenSnapshot = owner
        .space_children_snapshot()
        .await
        .map_err(space_owner_error)?;
    Ok(snapshot)
}

pub(super) fn matrix_space_children_snapshot(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        if !request.payload.is_null() {
            return Err(core_state_error(
                "p2-space-children-snapshot-invalid-payload",
            ));
        }
        let response = space_children_snapshot(&state).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-space-children-snapshot-serialization-failed"))
    })
}

/// Typed `matrix_space_child_set`.
pub(super) async fn space_child_set(
    state: &Arc<CoreState>,
    payload: MatrixSpaceChildSetRequest,
) -> Result<NativeSpaceChildMutationResult, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-space-child-set-no-session")
    })?;
    let result: NativeSpaceChildMutationResult = owner
        .space_child_set(
            &payload.parent_id,
            &payload.child_id,
            &payload.via,
            payload.order.as_deref(),
            payload.suggested,
        )
        .await
        .map_err(space_owner_error)?;
    Ok(result)
}

pub(super) fn matrix_space_child_set(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSpaceChildSetRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-space-child-set-invalid-payload"))?;
        let response = space_child_set(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-space-child-set-serialization-failed"))
    })
}

/// Typed `matrix_space_child_remove`.
pub(super) async fn space_child_remove(
    state: &Arc<CoreState>,
    payload: MatrixSpaceChildRemoveRequest,
) -> Result<NativeSpaceChildMutationResult, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-space-child-remove-no-session")
    })?;
    let result: NativeSpaceChildMutationResult = owner
        .space_child_remove(&payload.parent_id, &payload.child_id)
        .await
        .map_err(space_owner_error)?;
    Ok(result)
}

pub(super) fn matrix_space_child_remove(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSpaceChildRemoveRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-space-child-remove-invalid-payload"))?;
        let response = space_child_remove(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-space-child-remove-serialization-failed"))
    })
}

/// Typed `matrix_restricted_join_reparent`.
pub(super) async fn restricted_join_reparent(
    state: &Arc<CoreState>,
    payload: MatrixRestrictedJoinReparentRequest,
) -> Result<NativeRestrictedJoinReparentResult, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-restricted-join-reparent-no-session")
    })?;
    let result: NativeRestrictedJoinReparentResult = owner
        .restricted_join_reparent(
            &payload.room_id,
            payload.remove_parent_id.as_deref(),
            &payload.add_parent_id,
        )
        .await
        .map_err(space_owner_error)?;
    Ok(result)
}

pub(super) fn matrix_restricted_join_reparent(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRestrictedJoinReparentRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-restricted-join-reparent-invalid-payload"))?;
        let response = restricted_join_reparent(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-restricted-join-reparent-serialization-failed"))
    })
}

pub(super) fn space_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms.2c-invalid-parent"
        | "v-rooms.2c-invalid-child"
        | "v-rooms.2c-invalid-room"
        | "v-rooms.2c-invalid-via"
        | "v-rooms.2c-invalid-order"
        | "v-rooms.2b-space-hierarchy-invalid-room"
        | "v-rooms.2c-room-missing"
        | "v-rooms.2c-room-not-joined" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn room_moderation_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms-members-moderation-invalid-room"
        | "v-rooms-members-moderation-invalid-user"
        | "v-rooms-members-moderation-invalid-power-level"
        | "v-rooms-members-moderation-room-not-found" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

/// Typed `matrix_room_set_power_level`.
pub(super) async fn room_set_power_level(
    state: &Arc<CoreState>,
    payload: MatrixRoomSetPowerLevelRequest,
) -> Result<(), MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-set-power-level-no-session")
    })?;
    owner
        .set_power_level(&payload.room_id, &payload.user_id, payload.power_level)
        .await
        .map_err(room_moderation_owner_error)?;
    Ok(())
}

pub(super) fn matrix_room_set_power_level(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomSetPowerLevelRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-set-power-level-invalid-payload"))?;
        room_set_power_level(&state, payload).await?;
        Ok(serde_json::Value::Null)
    })
}

/// Typed `matrix_room_set_power_levels`.
pub(super) async fn room_set_power_levels(
    state: &Arc<CoreState>,
    payload: MatrixRoomSetPowerLevelStateRequest,
) -> Result<NativePowerLevelWriteResult, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-set-power-levels-no-session")
    })?;
    let result: NativePowerLevelWriteResult = owner
        .set_power_level_state(
            &payload.room_id,
            payload.content,
            ROOM_POWER_LEVELS_EVENT_TYPE,
        )
        .await
        .map_err(room_power_level_state_owner_error)?;
    Ok(result)
}

pub(super) fn matrix_room_set_power_levels(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomSetPowerLevelStateRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-set-power-levels-invalid-payload"))?;
        let response = room_set_power_levels(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-set-power-levels-serialization-failed"))
    })
}

/// Typed `matrix_room_set_power_level_tags`.
pub(super) async fn room_set_power_level_tags(
    state: &Arc<CoreState>,
    payload: MatrixRoomSetPowerLevelStateRequest,
) -> Result<NativePowerLevelWriteResult, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-set-power-level-tags-no-session")
    })?;
    let result: NativePowerLevelWriteResult = owner
        .set_power_level_state(
            &payload.room_id,
            payload.content,
            ROOM_POWER_LEVEL_TAGS_EVENT_TYPE,
        )
        .await
        .map_err(room_power_level_state_owner_error)?;
    Ok(result)
}

pub(super) fn matrix_room_set_power_level_tags(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomSetPowerLevelStateRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-set-power-level-tags-invalid-payload"))?;
        let response = room_set_power_level_tags(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-set-power-level-tags-serialization-failed"))
    })
}

pub(super) fn room_power_level_state_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms-power-levels-invalid-room"
        | "v-rooms-power-levels-invalid-content"
        | "v-rooms-power-levels-invalid-power"
        | "v-rooms-power-levels-invalid-power-map"
        | "v-rooms-power-levels-invalid-tag-key"
        | "v-rooms-power-levels-invalid-tag"
        | "v-rooms-power-levels-invalid-tag-name"
        | "v-rooms-power-levels-invalid-tag-color"
        | "v-rooms-power-levels-invalid-icon"
        | "v-rooms-power-levels-invalid-icon-info"
        | "v-rooms-power-levels-invalid-icon-field"
        | "v-rooms-power-levels-content-too-large"
        | "v-rooms-power-levels-room-not-found" => MatrixIpcErrorCategory::SdkInvariant,
        "v-rooms-power-levels-stale-session-generation" => {
            MatrixIpcErrorCategory::StaleSessionGeneration
        }
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

/// Typed `matrix_set_room_name`.
pub(super) async fn set_room_name(
    state: &Arc<CoreState>,
    payload: MatrixSetRoomNameRequest,
) -> Result<MatrixProfileWriteResult, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-set-room-name-no-session")
    })?;
    let result: MatrixProfileWriteResult = owner
        .set_name(&payload.room_id, &payload.name)
        .await
        .map_err(room_profile_write_owner_error)?;
    Ok(result)
}

pub(super) fn matrix_set_room_name(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetRoomNameRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-room-name-invalid-payload"))?;
        let response = set_room_name(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-set-room-name-serialization-failed"))
    })
}

/// Typed `matrix_set_room_topic`.
pub(super) async fn set_room_topic(
    state: &Arc<CoreState>,
    payload: MatrixSetRoomTopicRequest,
) -> Result<MatrixProfileWriteResult, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-set-room-topic-no-session")
    })?;
    let result: MatrixProfileWriteResult = owner
        .set_topic(&payload.room_id, &payload.topic)
        .await
        .map_err(room_profile_write_owner_error)?;
    Ok(result)
}

pub(super) fn matrix_set_room_topic(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetRoomTopicRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-room-topic-invalid-payload"))?;
        let response = set_room_topic(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-set-room-topic-serialization-failed"))
    })
}

/// Typed `matrix_set_room_avatar`.
pub(super) async fn set_room_avatar(
    state: &Arc<CoreState>,
    payload: MatrixSetRoomAvatarRequest,
) -> Result<MatrixProfileWriteResult, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-set-room-avatar-no-session")
    })?;
    let result: MatrixProfileWriteResult = owner
        .set_avatar(&payload.room_id, &payload.mxc)
        .await
        .map_err(room_profile_write_owner_error)?;
    Ok(result)
}

pub(super) fn matrix_set_room_avatar(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetRoomAvatarRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-set-room-avatar-invalid-payload"))?;
        let response = set_room_avatar(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-set-room-avatar-serialization-failed"))
    })
}

/// Typed `matrix_send_state_event`.
pub(super) async fn send_state_event(
    state: &Arc<CoreState>,
    payload: MatrixSendStateEventRequest,
) -> Result<MatrixProfileWriteResult, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-send-state-event-no-session")
    })?;
    let result: MatrixProfileWriteResult = owner
        .send_state_event(
            &payload.room_id,
            &payload.event_type,
            &payload.state_key,
            payload.content,
        )
        .await
        .map_err(room_state_event_owner_error)?;
    Ok(result)
}

pub(super) fn matrix_send_state_event(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSendStateEventRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-send-state-event-invalid-payload"))?;
        let response = send_state_event(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-send-state-event-serialization-failed"))
    })
}

/// Typed `matrix_enable_room_encrypted_state`.
pub(super) async fn enable_room_encrypted_state(
    state: &Arc<CoreState>,
    payload: MatrixEnableRoomEncryptedStateRequest,
) -> Result<MatrixProfileWriteResult, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-enable-room-encrypted-state-no-session")
    })?;
    let result: MatrixProfileWriteResult = owner
        .enable_room_encrypted_state(&payload.room_id, payload.encrypt_state_events)
        .await
        .map_err(room_state_event_owner_error)?;
    Ok(result)
}

pub(super) fn matrix_enable_room_encrypted_state(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixEnableRoomEncryptedStateRequest =
            serde_json::from_value(request.payload)
                .map_err(|_| core_state_error("p2-enable-room-encrypted-state-invalid-payload"))?;
        let response = enable_room_encrypted_state(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-enable-room-encrypted-state-serialization-failed"))
    })
}

/// Readback of `matrix_set_encrypted_state_events_setting`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixEncryptedStateEventsSettingResult {
    /// Always `ok`.
    pub status: String,
    pub enabled: bool,
}

/// Typed `matrix_set_encrypted_state_events_setting`.
pub(super) fn set_encrypted_state_events_setting(
    payload: MatrixSetEncryptedStateEventsSettingRequest,
) -> MatrixEncryptedStateEventsSettingResult {
    set_encrypted_state_events_setting_enabled(payload.enabled);
    MatrixEncryptedStateEventsSettingResult {
        status: "ok".to_owned(),
        enabled: payload.enabled,
    }
}

pub(super) fn matrix_set_encrypted_state_events_setting(
    _state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixSetEncryptedStateEventsSettingRequest =
            serde_json::from_value(request.payload).map_err(|_| {
                core_state_error("p2-set-encrypted-state-events-setting-invalid-payload")
            })?;
        serde_json::to_value(set_encrypted_state_events_setting(payload)).map_err(|_| {
            core_state_error("p2-set-encrypted-state-events-setting-serialization-failed")
        })
    })
}

/// Typed `matrix_room_retention`.
pub(super) async fn room_retention(
    state: &Arc<CoreState>,
    payload: MatrixRoomRetentionRequest,
) -> Result<MatrixRoomRetentionSnapshot, MatrixIpcError> {
    let owner = state.join_rule_owner()?.ok_or_else(|| {
        MatrixIpcError::new(MatrixIpcErrorCategory::Forbidden)
            .with_diagnostic("p2-room-retention-no-session")
    })?;
    let result: MatrixRoomRetentionSnapshot = owner
        .get_retention(&payload.room_id, payload.session_generation)
        .await
        .map_err(room_retention_owner_error)?;
    Ok(result)
}

pub(super) fn matrix_room_retention(
    state: Arc<CoreState>,
    request: CommandEnvelope,
) -> CommandFuture {
    Box::pin(async move {
        let payload: MatrixRoomRetentionRequest = serde_json::from_value(request.payload)
            .map_err(|_| core_state_error("p2-room-retention-invalid-payload"))?;
        let response = room_retention(&state, payload).await?;
        serde_json::to_value(response)
            .map_err(|_| core_state_error("p2-room-retention-serialization-failed"))
    })
}

pub(super) fn room_retention_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-send.r-room-profile-retention-invalid"
        | "v-send.r-room-profile-retention-room-not-found" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-room-profile-retention-requires-session" => MatrixIpcErrorCategory::Forbidden,
        "v-send.r-room-profile-retention-stale-generation" => {
            MatrixIpcErrorCategory::StaleSessionGeneration
        }
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn directory_visibility_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-send.r-room-profile-directory-visibility-invalid"
        | "v-send.r-room-profile-directory-visibility-room-not-found" => {
            MatrixIpcErrorCategory::SdkInvariant
        }
        "v-send.r-room-profile-directory-visibility-requires-session" => {
            MatrixIpcErrorCategory::Forbidden
        }
        "v-send.r-room-profile-directory-visibility-stale-generation" => {
            MatrixIpcErrorCategory::StaleSessionGeneration
        }
        "v-send.r-room-profile-directory-visibility-permission-denied" => {
            MatrixIpcErrorCategory::Forbidden
        }
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn room_profile_write_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "d0.4-send-invalid-room-id"
        | "v-send.r-room-profile-name-too-long"
        | "v-send.r-room-profile-topic-too-long"
        | "v-send.r-avatar-invalid-mxc"
        | "v-send.r-room-profile-room-not-found" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn room_state_event_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-rooms-state-event-invalid-type"
        | "v-rooms-state-event-invalid-key"
        | "v-rooms-state-event-invalid-content"
        | "d0.4-send-invalid-room-id"
        | "v-send.r-room-profile-room-not-found" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn content_upload_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-send.r-content-upload-empty"
        | "v-send.r-content-upload-invalid-mime"
        | "v-send.r-content-upload-invalid-filename"
        | "v-send.r-content-upload-too-large" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-content-upload-no-session" => MatrixIpcErrorCategory::Forbidden,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn send_room_attachment_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-send.1-attachment-empty"
        | "v-send.1-attachment-formatted-caption-without-caption"
        | "v-send.1-attachment-invalid-filename"
        | "v-send.1-attachment-invalid-mime"
        | "v-send.1-attachment-invalid-reply"
        | "v-send.1-attachment-invalid-room"
        | "v-send.1-attachment-invalid-thread-root"
        | "v-send.1-attachment-invalid-transaction-id"
        | "v-send.1-attachment-too-large"
        | "d0.4-send-text-payload-too-large"
        | "v-send.4-invalid-mention-user-id"
        | "v-send.4-mention-user-id-too-long"
        | "v-send.4-too-many-mentions" => MatrixIpcErrorCategory::SdkInvariant,
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}

pub(super) fn join_rule_snapshot_owner_error(diagnostic_id: &'static str) -> MatrixIpcError {
    let category = match diagnostic_id {
        "v-send.r-room-profile-join-rule-invalid" => MatrixIpcErrorCategory::SdkInvariant,
        "v-send.r-room-profile-join-rule-requires-session" => MatrixIpcErrorCategory::Forbidden,
        "v-send.r-room-profile-join-rule-stale-generation" => {
            MatrixIpcErrorCategory::StaleSessionGeneration
        }
        _ => MatrixIpcErrorCategory::Unknown,
    };
    MatrixIpcError::new(category).with_diagnostic(diagnostic_id)
}
