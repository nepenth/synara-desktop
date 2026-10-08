//! Typed SharedCore operations and projections for rooms.

use super::*;

/// Privacy-safe room-list wake-up. No room ids, names, tokens, or password.
/// iOS re-fetches via the existing snapshot command.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomListUpdateDto {
    pub session_generation: u64,
}

/// Static fail-closed room-list update poll error.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum RoomListUpdateError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomListUpdateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomListUpdateError {}

pub(super) fn room_list_update_poll_failed(
    code: &'static str,
    description: &'static str,
) -> RoomListUpdateError {
    RoomListUpdateError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn push_room_list_update(
    queue: &Mutex<Vec<RoomListUpdateDto>>,
    session_generation: u64,
) {
    if let Ok(mut guard) = queue.lock() {
        if guard.len() >= ROOM_LIST_UPDATE_QUEUE_CAP {
            guard.remove(0);
        }
        guard.push(RoomListUpdateDto { session_generation });
    }
}

/// Privacy-safe room-list snapshot. Tokens and password never appear here.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomListSnapshotDto {
    pub session_generation: u64,
    pub ordered_room_ids: Vec<String>,
    pub rooms: Vec<RoomListRoomDto>,
}

/// One privacy-safe room-list row. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomListRoomDto {
    pub room_id: String,
    pub name: Option<String>,
    pub canonical_alias: Option<String>,
    pub avatar_url: Option<String>,
    pub membership: String,
    pub is_direct: bool,
    pub direct_user_id: Option<String>,
    pub is_space: bool,
    pub is_favorite: bool,
    pub is_call: bool,
    pub has_active_call: bool,
    pub active_call_participant_count: u32,
    pub unread_count: u32,
    pub highlight_count: u32,
    pub marked_unread: bool,
    pub last_activity_ts: Option<u64>,
    pub last_message_preview: Option<String>,
    pub last_message_is_agent_approval: bool,
    pub is_encrypted: bool,
    pub encryption_status: crate::dto::RoomEncryptionStatus,
    pub notification_mode: Option<String>,
}

/// Static fail-closed room-list error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum RoomListSnapshotError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomListSnapshotError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomListSnapshotError {}

pub(super) fn room_list_failed(
    code: &'static str,
    description: &'static str,
) -> RoomListSnapshotError {
    RoomListSnapshotError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_room_list_core_error(error: MatrixIpcError) -> RoomListSnapshotError {
    match error.diagnostic_id.as_deref() {
        Some("p2-room-list-snapshot-no-session") => {
            room_list_failed(ROOM_LIST_NO_SESSION_CODE, ROOM_LIST_NO_SESSION_DESCRIPTION)
        }
        Some(
            "d0.2-room-list-snapshot-timeout"
            | "d0.2-room-list-stream-ended"
            | "d0.2-room-list-reset-missing"
            | "d0.2-room-list-open-failed"
            | "d0.2-room-list-filter-failed",
        ) => room_list_failed(
            ROOM_LIST_SYNC_NOT_STARTED_CODE,
            ROOM_LIST_SYNC_NOT_STARTED_DESCRIPTION,
        ),
        _ => room_list_failed(ROOM_LIST_FAILED_CODE, ROOM_LIST_FAILED_DESCRIPTION),
    }
}

/// Privacy-safe invite snapshot. Tokens and password never appear here.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct InviteSnapshotDto {
    pub session_generation: u64,
    pub invites: Vec<InviteDto>,
}

/// One privacy-safe invite row. No tokens or password.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct InviteDto {
    pub room_id: String,
    pub room_name: String,
    pub avatar_handle_id: Option<String>,
    pub room_topic: Option<String>,
    pub room_alias: Option<String>,
    pub sender_id: String,
    pub sender_name: String,
    pub sender_ignored: bool,
    pub invite_ts: Option<u64>,
    pub reason: Option<String>,
    pub is_space: bool,
    pub is_direct: bool,
    pub is_encrypted: bool,
    pub triage: String,
}

/// Static fail-closed invite-snapshot error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum InviteSnapshotError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for InviteSnapshotError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for InviteSnapshotError {}

pub(super) fn invites_failed(code: &'static str, description: &'static str) -> InviteSnapshotError {
    InviteSnapshotError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_invites_core_error(error: MatrixIpcError) -> InviteSnapshotError {
    match error.diagnostic_id.as_deref() {
        Some(
            "p2-invites-snapshot-no-session"
            | "v-rooms.1-invites-requires-session"
            | "v-send.r-room-profile-join-rule-requires-session",
        ) => invites_failed(INVITES_NO_SESSION_CODE, INVITES_NO_SESSION_DESCRIPTION),
        _ => invites_failed(INVITES_FAILED_CODE, INVITES_FAILED_DESCRIPTION),
    }
}

pub(super) fn invite_dto(invite: NativeInvite) -> InviteDto {
    InviteDto {
        room_id: invite.room_id,
        room_name: invite.room_name,
        avatar_handle_id: invite.avatar_handle_id,
        room_topic: invite.room_topic,
        room_alias: invite.room_alias,
        sender_id: invite.sender_id,
        sender_name: invite.sender_name,
        sender_ignored: invite.sender_ignored,
        invite_ts: invite.invite_ts,
        reason: invite.reason,
        is_space: invite.is_space,
        is_direct: invite.is_direct,
        is_encrypted: invite.is_encrypted,
        triage: match invite.triage {
            NativeInviteTriage::Known => "known".to_owned(),
            NativeInviteTriage::Public => "public".to_owned(),
            NativeInviteTriage::Spam => "spam".to_owned(),
        },
    }
}

/// Privacy-safe join-rule snapshot. Closed vocabulary only; no allow-list or tokens.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomJoinRuleSnapshotDto {
    pub status: String,
    pub room_id: String,
    pub session_generation: u64,
    pub join_rule: String,
}

/// Privacy-safe join-rule write ack. Status only; no room id, join rule, or allow-list.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomJoinRuleWriteDto {
    pub status: String,
}

/// Static fail-closed join-rule error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum JoinRuleCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for JoinRuleCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for JoinRuleCommandError {}

pub(super) fn join_rule_failed(code: &str, description: &'static str) -> JoinRuleCommandError {
    JoinRuleCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_join_rule_core_error(error: MatrixIpcError) -> JoinRuleCommandError {
    map_join_rule_core_error_with_no_session(JOIN_RULE_SNAPSHOT_NO_SESSION_CODE, error)
}

pub(super) fn map_join_rule_core_error_with_no_session(
    no_session: &'static str,
    error: MatrixIpcError,
) -> JoinRuleCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            join_rule_failed(code, JOIN_RULE_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code == JOIN_RULE_SNAPSHOT_NO_SESSION_CODE
                || code == JOIN_RULE_SET_NO_SESSION_CODE =>
        {
            join_rule_failed(code, JOIN_RULE_NO_SESSION_DESCRIPTION)
        }
        Some(code) if code.starts_with("v-send.r-room-profile-join-rule-") => {
            join_rule_failed(code, JOIN_RULE_OWNER_DESCRIPTION)
        }
        _ => join_rule_failed(JOIN_RULE_FAILED_CODE, JOIN_RULE_FAILED_DESCRIPTION),
    }
}

pub(super) fn join_rule_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, JoinRuleCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(join_rule_failed(
            JOIN_RULE_FAILED_CODE,
            JOIN_RULE_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

/// Privacy-safe room-note item. Body/ids/timestamps may cross; no tokens.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct RoomNoteItemDto {
    pub id: String,
    pub kind: String,
    pub room_id: String,
    pub created_at: f64,
    pub updated_at: f64,
    pub body: Option<String>,
    pub completed_at: Option<f64>,
    pub order: Option<f64>,
    pub event_id: Option<String>,
    pub event_ts: Option<f64>,
    pub sender: Option<String>,
}

pub(super) fn room_note_item_from_dto(
    item: RoomNoteItemDto,
) -> Result<SynaraRoomNoteItem, RoomNotesCommandError> {
    let kind = match item.kind.as_str() {
        "note" => SynaraRoomNoteItemKind::Note,
        "todo" => SynaraRoomNoteItemKind::Todo,
        "message" => SynaraRoomNoteItemKind::Message,
        _ => {
            return Err(room_notes_failed(
                ROOM_NOTES_INVALID_ITEM_CODE,
                ROOM_NOTES_INVALID_ITEM_DESCRIPTION,
            ))
        }
    };
    if item.id.is_empty()
        || item.room_id.is_empty()
        || !item.created_at.is_finite()
        || !item.updated_at.is_finite()
    {
        return Err(room_notes_failed(
            ROOM_NOTES_INVALID_ITEM_CODE,
            ROOM_NOTES_INVALID_ITEM_DESCRIPTION,
        ));
    }
    Ok(SynaraRoomNoteItem {
        id: item.id,
        kind,
        room_id: item.room_id,
        created_at: item.created_at,
        updated_at: item.updated_at,
        body: item.body.filter(|value| !value.is_empty()),
        completed_at: item.completed_at.filter(|value| value.is_finite()),
        order: item.order.filter(|value| value.is_finite()),
        event_id: item.event_id.filter(|value| !value.is_empty()),
        event_ts: item.event_ts.filter(|value| value.is_finite()),
        sender: item.sender.filter(|value| !value.is_empty()),
    })
}

pub(super) fn room_note_move_direction_from_dto(
    direction: &str,
) -> Result<RoomNoteMoveDirection, RoomNotesCommandError> {
    match direction {
        "up" => Ok(RoomNoteMoveDirection::Up),
        "down" => Ok(RoomNoteMoveDirection::Down),
        _ => Err(room_notes_failed(
            ROOM_NOTES_INVALID_ITEM_CODE,
            ROOM_NOTES_INVALID_ITEM_DESCRIPTION,
        )),
    }
}

pub(super) fn room_note_item_dto(item: SynaraRoomNoteItem) -> RoomNoteItemDto {
    RoomNoteItemDto {
        id: item.id,
        kind: match item.kind {
            SynaraRoomNoteItemKind::Note => "note".to_owned(),
            SynaraRoomNoteItemKind::Todo => "todo".to_owned(),
            SynaraRoomNoteItemKind::Message => "message".to_owned(),
        },
        room_id: item.room_id,
        created_at: item.created_at,
        updated_at: item.updated_at,
        body: item.body,
        completed_at: item.completed_at,
        order: item.order,
        event_id: item.event_id,
        event_ts: item.event_ts,
        sender: item.sender,
    }
}

/// Privacy-safe room-profile write ack. Status only; no room id, name, topic, or mxc.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomProfileWriteDto {
    pub status: String,
}

/// Static fail-closed room-profile-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum RoomProfileCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomProfileCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomProfileCommandError {}

pub(super) fn room_profile_failed(
    code: &str,
    description: &'static str,
) -> RoomProfileCommandError {
    RoomProfileCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_room_profile_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RoomProfileCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            room_profile_failed(code, ROOM_PROFILE_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("v-send.r-room-profile-")
                || code == "v-send.r-avatar-invalid-mxc"
                || code == "d0.4-send-invalid-room-id" =>
        {
            room_profile_failed(code, ROOM_PROFILE_OWNER_DESCRIPTION)
        }
        _ => room_profile_failed(ROOM_PROFILE_FAILED_CODE, ROOM_PROFILE_FAILED_DESCRIPTION),
    }
}

pub(super) fn room_profile_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, RoomProfileCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_profile_failed(
            ROOM_PROFILE_FAILED_CODE,
            ROOM_PROFILE_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn room_profile_write_dto(
    payload: serde_json::Value,
) -> Result<RoomProfileWriteDto, RoomProfileCommandError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            room_profile_failed(ROOM_PROFILE_FAILED_CODE, ROOM_PROFILE_FAILED_DESCRIPTION)
        })?;
    Ok(RoomProfileWriteDto {
        status: status.to_owned(),
    })
}

/// Privacy-safe room leave/join write ack. Status only; no room id, alias, or via servers.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomMembershipWriteDto {
    pub status: String,
}

/// Static fail-closed room-membership-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum RoomMembershipCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomMembershipCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomMembershipCommandError {}

pub(super) fn room_membership_failed(
    code: &str,
    description: &'static str,
) -> RoomMembershipCommandError {
    RoomMembershipCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_room_membership_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RoomMembershipCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            room_membership_failed(code, ROOM_MEMBERSHIP_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("v-rooms-room-leave-")
                || code.starts_with("v-rooms-room-join-")
                || code.starts_with("v-rooms-room-favorite-")
                || code == "v-send.r-room-profile-join-rule-requires-session" =>
        {
            room_membership_failed(code, ROOM_MEMBERSHIP_OWNER_DESCRIPTION)
        }
        _ => room_membership_failed(
            ROOM_MEMBERSHIP_FAILED_CODE,
            ROOM_MEMBERSHIP_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn room_membership_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, RoomMembershipCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_membership_failed(
            ROOM_MEMBERSHIP_FAILED_CODE,
            ROOM_MEMBERSHIP_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn closed_room_membership_status(value: &str) -> Option<&'static str> {
    match value {
        "ok" => Some("ok"),
        _ => None,
    }
}

pub(super) fn room_membership_write_dto(
    payload: serde_json::Value,
) -> Result<RoomMembershipWriteDto, RoomMembershipCommandError> {
    if payload.is_null() {
        return Ok(RoomMembershipWriteDto {
            status: "ok".to_owned(),
        });
    }
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_room_membership_status)
        .ok_or_else(|| {
            room_membership_failed(
                ROOM_MEMBERSHIP_FAILED_CODE,
                ROOM_MEMBERSHIP_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomMembershipWriteDto {
        status: status.to_owned(),
    })
}

/// Privacy-safe room invite/kick/ban/unban write ack. Status only; no room id, user id, or reason.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomModerationWriteDto {
    pub status: String,
}

/// Static fail-closed room-moderation-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum RoomModerationCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomModerationCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomModerationCommandError {}

pub(super) fn room_moderation_failed(
    code: &str,
    description: &'static str,
) -> RoomModerationCommandError {
    RoomModerationCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_room_moderation_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RoomModerationCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            room_moderation_failed(code, ROOM_MODERATION_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("v-rooms-members-moderation-")
                || code == "v-send.r-room-profile-join-rule-requires-session" =>
        {
            room_moderation_failed(code, ROOM_MODERATION_OWNER_DESCRIPTION)
        }
        _ => room_moderation_failed(
            ROOM_MODERATION_FAILED_CODE,
            ROOM_MODERATION_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn room_moderation_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, RoomModerationCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_moderation_failed(
            ROOM_MODERATION_FAILED_CODE,
            ROOM_MODERATION_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn closed_room_moderation_status(value: &str) -> Option<&'static str> {
    match value {
        "ok" => Some("ok"),
        _ => None,
    }
}

pub(super) fn room_moderation_write_dto(
    payload: serde_json::Value,
) -> Result<RoomModerationWriteDto, RoomModerationCommandError> {
    if payload.is_null() {
        return Ok(RoomModerationWriteDto {
            status: "ok".to_owned(),
        });
    }
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_room_moderation_status)
        .ok_or_else(|| {
            room_moderation_failed(
                ROOM_MODERATION_FAILED_CODE,
                ROOM_MODERATION_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomModerationWriteDto {
        status: status.to_owned(),
    })
}

/// Privacy-safe room power-level write ack. Status only; no room id, user id, power level, or content.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomPowerLevelWriteDto {
    pub status: String,
}

/// Static fail-closed room-power-level-family error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum RoomPowerLevelCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomPowerLevelCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomPowerLevelCommandError {}

pub(super) fn room_power_level_failed(
    code: &str,
    description: &'static str,
) -> RoomPowerLevelCommandError {
    RoomPowerLevelCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_room_power_level_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RoomPowerLevelCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            room_power_level_failed(code, ROOM_POWER_LEVEL_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("v-rooms-members-moderation-")
                || code.starts_with("v-rooms-power-levels-")
                || code == "v-send.r-room-profile-join-rule-requires-session" =>
        {
            room_power_level_failed(code, ROOM_POWER_LEVEL_OWNER_DESCRIPTION)
        }
        _ => room_power_level_failed(
            ROOM_POWER_LEVEL_FAILED_CODE,
            ROOM_POWER_LEVEL_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn parse_power_level_content_json(
    content_json: &str,
) -> Result<serde_json::Value, RoomPowerLevelCommandError> {
    if content_json.len() > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_power_level_failed(
            ROOM_POWER_LEVEL_FAILED_CODE,
            ROOM_POWER_LEVEL_FAILED_DESCRIPTION,
        ));
    }
    serde_json::from_str(content_json).map_err(|_| {
        room_power_level_failed(
            ROOM_POWER_LEVEL_FAILED_CODE,
            ROOM_POWER_LEVEL_FAILED_DESCRIPTION,
        )
    })
}

pub(super) fn room_power_level_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, RoomPowerLevelCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_power_level_failed(
            ROOM_POWER_LEVEL_FAILED_CODE,
            ROOM_POWER_LEVEL_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn closed_room_power_level_status(value: &str) -> Option<&'static str> {
    match value {
        "ok" => Some("ok"),
        _ => None,
    }
}

pub(super) fn room_power_level_write_dto(
    payload: serde_json::Value,
) -> Result<RoomPowerLevelWriteDto, RoomPowerLevelCommandError> {
    if payload.is_null() {
        return Ok(RoomPowerLevelWriteDto {
            status: "ok".to_owned(),
        });
    }
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_room_power_level_status)
        .ok_or_else(|| {
            room_power_level_failed(
                ROOM_POWER_LEVEL_FAILED_CODE,
                ROOM_POWER_LEVEL_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomPowerLevelWriteDto {
        status: status.to_owned(),
    })
}

/// Typed room-create request. Core scalar fields only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomCreateRequestDto {
    pub name: Option<String>,
    pub topic: Option<String>,
    pub room_alias_name: Option<String>,
    pub visibility: Option<String>,
    pub preset: Option<String>,
    pub is_direct: bool,
    pub encryption: bool,
    pub invite: Vec<String>,
    pub room_version: Option<String>,
    pub join_rule: Option<String>,
    pub knock: bool,
    pub parent_room_id: Option<String>,
}

/// Privacy-safe room-create result. Created room id only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomCreateDto {
    pub room_id: String,
}

/// Static fail-closed room-create error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum RoomCreateCommandError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomCreateCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomCreateCommandError {}

pub(super) fn room_create_failed(code: &str, description: &'static str) -> RoomCreateCommandError {
    RoomCreateCommandError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_room_create_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RoomCreateCommandError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            room_create_failed(code, ROOM_CREATE_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("v-rooms-room-create-")
                || code == "p2-room-create-invalid-payload"
                || code == "v-send.r-room-profile-join-rule-requires-session" =>
        {
            room_create_failed(code, ROOM_CREATE_OWNER_DESCRIPTION)
        }
        _ => room_create_failed(ROOM_CREATE_FAILED_CODE, ROOM_CREATE_FAILED_DESCRIPTION),
    }
}

pub(super) fn closed_room_create_visibility(value: &str) -> Option<&'static str> {
    match value {
        "private" => Some("private"),
        "public" => Some("public"),
        _ => None,
    }
}

pub(super) fn closed_room_create_preset(value: &str) -> Option<&'static str> {
    match value {
        "private_chat" => Some("private_chat"),
        "public_chat" => Some("public_chat"),
        "trusted_private_chat" => Some("trusted_private_chat"),
        _ => None,
    }
}

pub(super) fn closed_created_room_id(value: &str) -> Option<String> {
    if value.starts_with('!')
        && !value.is_empty()
        && value.len() <= 512
        && value.trim() == value
        && !value.chars().any(char::is_whitespace)
    {
        Some(value.to_owned())
    } else {
        None
    }
}

pub(super) fn room_create_request_payload(
    request: RoomCreateRequestDto,
) -> Result<serde_json::Value, RoomCreateCommandError> {
    let visibility = request
        .visibility
        .as_deref()
        .map(|value| {
            closed_room_create_visibility(value).ok_or_else(|| {
                room_create_failed(ROOM_CREATE_FAILED_CODE, ROOM_CREATE_FAILED_DESCRIPTION)
            })
        })
        .transpose()?;
    let preset = request
        .preset
        .as_deref()
        .map(|value| {
            closed_room_create_preset(value).ok_or_else(|| {
                room_create_failed(ROOM_CREATE_FAILED_CODE, ROOM_CREATE_FAILED_DESCRIPTION)
            })
        })
        .transpose()?;
    Ok(serde_json::json!({
        "name": request.name,
        "topic": request.topic,
        "roomAliasName": request.room_alias_name,
        "visibility": visibility,
        "preset": preset,
        "isDirect": request.is_direct,
        "encryption": request.encryption,
        "invite": request.invite,
        "roomVersion": request.room_version,
        "joinRule": request.join_rule,
        "knock": request.knock,
        "parentRoomId": request.parent_room_id,
    }))
}

pub(super) fn room_create_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, RoomCreateCommandError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_create_failed(
            ROOM_CREATE_FAILED_CODE,
            ROOM_CREATE_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn room_create_dto(
    payload: serde_json::Value,
) -> Result<RoomCreateDto, RoomCreateCommandError> {
    payload
        .as_str()
        .and_then(closed_created_room_id)
        .map(|room_id| RoomCreateDto { room_id })
        .ok_or_else(|| room_create_failed(ROOM_CREATE_FAILED_CODE, ROOM_CREATE_FAILED_DESCRIPTION))
}

/// Privacy-safe room member row. Ids, display name, mxc, membership, and power only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomMemberDto {
    pub room_id: String,
    pub user_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub membership: String,
    pub power_level: i32,
    pub is_direct_target: Option<bool>,
}

/// Privacy-safe members snapshot. Member rows only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomMembersSnapshotDto {
    pub session_generation: u64,
    pub room_id: String,
    pub members: Vec<RoomMemberDto>,
}

/// Privacy-safe power-levels snapshot. Content is JSON text.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomPowerLevelsSnapshotDto {
    pub status: String,
    pub session_generation: u64,
    pub room_id: String,
    pub event_type: String,
    pub state_key: String,
    pub content_json: String,
}

/// Privacy-safe creators snapshot. Creator user ids only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomCreatorsSnapshotDto {
    pub status: String,
    pub session_generation: u64,
    pub room_id: String,
    pub event_type: String,
    pub state_key: String,
    pub creators: Vec<String>,
}

/// Privacy-safe power-level-tags snapshot. Content is JSON text.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RoomPowerLevelTagsSnapshotDto {
    pub status: String,
    pub session_generation: u64,
    pub room_id: String,
    pub event_type: String,
    pub state_key: String,
    pub content_json: String,
}

/// Static fail-closed members-snapshot error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum RoomMembersSnapshotError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for RoomMembersSnapshotError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for RoomMembersSnapshotError {}

pub(super) fn room_members_snapshot_failed(
    code: &str,
    description: &'static str,
) -> RoomMembersSnapshotError {
    RoomMembersSnapshotError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_room_members_snapshot_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> RoomMembersSnapshotError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            room_members_snapshot_failed(code, ROOM_MEMBERS_SNAPSHOT_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("v-rooms-members-read-")
                || code == "v-send.r-room-profile-join-rule-requires-session" =>
        {
            room_members_snapshot_failed(code, ROOM_MEMBERS_SNAPSHOT_OWNER_DESCRIPTION)
        }
        _ => room_members_snapshot_failed(
            ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
            ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
        ),
    }
}

pub(super) fn closed_room_member_membership(value: &str) -> Option<&'static str> {
    match value {
        "invite" => Some("invite"),
        "join" => Some("join"),
        "knock" => Some("knock"),
        "leave" => Some("leave"),
        "ban" => Some("ban"),
        _ => None,
    }
}

pub(super) fn closed_members_snapshot_status(value: &str) -> Option<&'static str> {
    match value {
        "ok" => Some("ok"),
        _ => None,
    }
}

pub(super) fn closed_power_levels_event_type(value: &str) -> Option<&'static str> {
    match value {
        "m.room.power_levels" => Some("m.room.power_levels"),
        _ => None,
    }
}

pub(super) fn closed_power_level_tags_event_type(value: &str) -> Option<&'static str> {
    match value {
        "in.synara.room.power_level_tags" => Some("in.synara.room.power_level_tags"),
        _ => None,
    }
}

pub(super) fn room_members_snapshot_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, RoomMembersSnapshotError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(room_members_snapshot_failed(
            ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
            ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn room_member_dto(
    value: &serde_json::Value,
) -> Result<RoomMemberDto, RoomMembersSnapshotError> {
    let membership = value
        .get("membership")
        .and_then(|item| item.as_str())
        .and_then(closed_room_member_membership)
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let power_level = value
        .get("powerLevel")
        .and_then(|item| item.as_i64())
        .and_then(|item| i32::try_from(item).ok())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let room_id = value
        .get("roomId")
        .and_then(|item| item.as_str())
        .filter(|item| !item.is_empty())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let user_id = value
        .get("userId")
        .and_then(|item| item.as_str())
        .filter(|item| !item.is_empty())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    Ok(RoomMemberDto {
        room_id: room_id.to_owned(),
        user_id: user_id.to_owned(),
        display_name: value
            .get("displayName")
            .and_then(|item| item.as_str())
            .map(ToOwned::to_owned),
        avatar_url: value
            .get("avatarUrl")
            .and_then(|item| item.as_str())
            .map(ToOwned::to_owned),
        membership: membership.to_owned(),
        power_level,
        is_direct_target: value.get("isDirectTarget").and_then(|item| item.as_bool()),
    })
}

pub(super) fn room_members_snapshot_dto(
    payload: serde_json::Value,
) -> Result<RoomMembersSnapshotDto, RoomMembersSnapshotError> {
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let room_id = payload
        .get("roomId")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let members = payload
        .get("members")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?
        .iter()
        .map(room_member_dto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(RoomMembersSnapshotDto {
        session_generation,
        room_id: room_id.to_owned(),
        members,
    })
}

pub(super) fn room_power_levels_snapshot_dto(
    payload: serde_json::Value,
) -> Result<RoomPowerLevelsSnapshotDto, RoomMembersSnapshotError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_members_snapshot_status)
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let room_id = payload
        .get("roomId")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let event_type = payload
        .get("eventType")
        .and_then(|value| value.as_str())
        .and_then(closed_power_levels_event_type)
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let state_key = payload
        .get("stateKey")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let content = payload.get("content").ok_or_else(|| {
        room_members_snapshot_failed(
            ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
            ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
        )
    })?;
    Ok(RoomPowerLevelsSnapshotDto {
        status: status.to_owned(),
        session_generation,
        room_id: room_id.to_owned(),
        event_type: event_type.to_owned(),
        state_key: state_key.to_owned(),
        content_json: snapshot_content_json(content)?,
    })
}

pub(super) fn room_creators_snapshot_dto(
    payload: serde_json::Value,
) -> Result<RoomCreatorsSnapshotDto, RoomMembersSnapshotError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_members_snapshot_status)
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let room_id = payload
        .get("roomId")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let event_type = payload
        .get("eventType")
        .and_then(|value| value.as_str())
        .and_then(closed_creators_event_type)
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let state_key = payload
        .get("stateKey")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let creators = payload
        .get("creators")
        .and_then(|value| value.as_array())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .ok_or_else(|| {
                    room_members_snapshot_failed(
                        ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                        ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(RoomCreatorsSnapshotDto {
        status: status.to_owned(),
        session_generation,
        room_id: room_id.to_owned(),
        event_type: event_type.to_owned(),
        state_key: state_key.to_owned(),
        creators,
    })
}

pub(super) fn room_power_level_tags_snapshot_dto(
    payload: serde_json::Value,
) -> Result<RoomPowerLevelTagsSnapshotDto, RoomMembersSnapshotError> {
    let status = payload
        .get("status")
        .and_then(|value| value.as_str())
        .and_then(closed_members_snapshot_status)
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let session_generation = payload
        .get("sessionGeneration")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let room_id = payload
        .get("roomId")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let event_type = payload
        .get("eventType")
        .and_then(|value| value.as_str())
        .and_then(closed_power_level_tags_event_type)
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let state_key = payload
        .get("stateKey")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            room_members_snapshot_failed(
                ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
                ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
            )
        })?;
    let content = payload.get("content").ok_or_else(|| {
        room_members_snapshot_failed(
            ROOM_MEMBERS_SNAPSHOT_FAILED_CODE,
            ROOM_MEMBERS_SNAPSHOT_FAILED_DESCRIPTION,
        )
    })?;
    Ok(RoomPowerLevelTagsSnapshotDto {
        status: status.to_owned(),
        session_generation,
        room_id: room_id.to_owned(),
        event_type: event_type.to_owned(),
        state_key: state_key.to_owned(),
        content_json: snapshot_content_json(content)?,
    })
}

/// Static fail-closed invite-action error. Fields are source constants only.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum InviteActionError {
    Failed { code: String, description: String },
}

impl std::fmt::Display for InviteActionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed { description, .. } => formatter.write_str(description),
        }
    }
}

impl std::error::Error for InviteActionError {}

pub(super) fn invite_action_failed(code: &str, description: &'static str) -> InviteActionError {
    InviteActionError::Failed {
        code: code.to_owned(),
        description: description.to_owned(),
    }
}

pub(super) fn map_invite_action_core_error(
    no_session: &'static str,
    error: MatrixIpcError,
) -> InviteActionError {
    match error.diagnostic_id.as_deref() {
        Some(code) if code == no_session => {
            invite_action_failed(code, INVITE_ACTION_NO_SESSION_DESCRIPTION)
        }
        Some(code)
            if code.starts_with("v-rooms.1-invite")
                || code.starts_with("p2-invites-")
                || code == "v-send.r-room-profile-join-rule-requires-session" =>
        {
            invite_action_failed(code, INVITE_ACTION_OWNER_DESCRIPTION)
        }
        _ => invite_action_failed(INVITE_ACTION_FAILED_CODE, INVITE_ACTION_FAILED_DESCRIPTION),
    }
}

pub(super) fn invite_action_envelope_payload(
    payload: serde_json::Value,
) -> Result<serde_json::Value, InviteActionError> {
    let size = serde_json::to_vec(&payload)
        .map(|bytes| bytes.len())
        .unwrap_or(usize::MAX);
    if size > MAX_ENVELOPE_PAYLOAD_JSON_BYTES {
        return Err(invite_action_failed(
            INVITE_ACTION_FAILED_CODE,
            INVITE_ACTION_FAILED_DESCRIPTION,
        ));
    }
    Ok(payload)
}

pub(super) fn invite_action_snapshot_dto(
    payload: serde_json::Value,
) -> Result<InviteSnapshotDto, InviteActionError> {
    let snapshot: NativeInviteSnapshot = serde_json::from_value(payload).map_err(|_| {
        invite_action_failed(INVITE_ACTION_FAILED_CODE, INVITE_ACTION_FAILED_DESCRIPTION)
    })?;
    Ok(InviteSnapshotDto {
        session_generation: snapshot.session_generation,
        invites: snapshot.invites.into_iter().map(invite_dto).collect(),
    })
}

impl SharedCore {
    pub(super) fn spawn_room_list_live(&self) {
        let Some(owner) = self.core.attached_sync_owner() else {
            return;
        };
        let queue = Arc::clone(&self.room_list_updates);
        let emit = Arc::new(move |update: NativeRoomListUpdateSignal| {
            push_room_list_update(&queue, update.session_generation);
        });
        let live = NativeRoomListOwner::start(&owner, emit);
        if let Ok(mut guard) = self.room_list_live.lock() {
            *guard = Some(live);
        }
    }

    pub(super) fn spawn_room_surface_owners(&self) {
        let Ok(client) = self.retained_client() else {
            return;
        };
        let Ok(Some(snapshot)) = self.core.session_snapshot() else {
            return;
        };
        let generation = snapshot.session_generation;
        if generation == 0 {
            return;
        }
        let emit: OwnProfileUpdateEmit = Arc::new(|_| {});
        if let Ok(owner) = NativeOwnProfileOwner::start(&client, emit, generation) {
            if let Ok(mut guard) = self.own_profile_live.lock() {
                *guard = Some(owner);
            }
        }
        if let Ok(owner) = NativeMediaRetentionOwner::start(&client, generation) {
            if let Ok(mut guard) = self.media_retention_live.lock() {
                *guard = Some(owner);
            }
        }
    }

    /// Test-only enqueue onto the room-list emit queue. Not on UDL.
    #[doc(hidden)]
    pub fn enqueue_room_list_update_for_test(&self, session_generation: u64) {
        push_room_list_update(&self.room_list_updates, session_generation);
    }

    pub(super) async fn invite_action_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        room_id: String,
    ) -> Result<InviteSnapshotDto, InviteActionError> {
        let payload = invite_action_envelope_payload(serde_json::json!({
            "roomId": room_id,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: INVITE_ACTION_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_invite_action_core_error(no_session, error))?;
        invite_action_snapshot_dto(response.payload)
    }

    pub(super) async fn room_members_snapshot_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        room_id: String,
    ) -> Result<serde_json::Value, RoomMembersSnapshotError> {
        let payload = room_members_snapshot_envelope_payload(serde_json::json!({
            "roomId": room_id,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: ROOM_MEMBERS_SNAPSHOT_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_room_members_snapshot_core_error(no_session, error))?;
        Ok(response.payload)
    }

    pub(super) async fn room_profile_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<RoomProfileWriteDto, RoomProfileCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: ROOM_PROFILE_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_room_profile_core_error(no_session, error))?;
        room_profile_write_dto(response.payload)
    }

    pub(super) async fn room_membership_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<RoomMembershipWriteDto, RoomMembershipCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: ROOM_MEMBERSHIP_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_room_membership_core_error(no_session, error))?;
        room_membership_write_dto(response.payload)
    }

    pub(super) async fn room_moderation_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<RoomModerationWriteDto, RoomModerationCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: ROOM_MODERATION_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_room_moderation_core_error(no_session, error))?;
        room_moderation_write_dto(response.payload)
    }

    pub(super) async fn room_power_level_command(
        &self,
        command: &'static str,
        no_session: &'static str,
        payload: serde_json::Value,
    ) -> Result<RoomPowerLevelWriteDto, RoomPowerLevelCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: command.to_owned(),
                session_generation: ROOM_POWER_LEVEL_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_room_power_level_core_error(no_session, error))?;
        room_power_level_write_dto(response.payload)
    }
}

#[uniffi::export(async_runtime = "tokio")]
impl SharedCore {
    /// Drain queued room-list wake-ups. Not `Core.command`.
    ///
    /// NSE forbids this. An empty queue returns an empty list. Room ids
    /// and names are never included. This is not Platform::emit.
    pub async fn poll_room_list_updates(
        &self,
    ) -> Result<Vec<RoomListUpdateDto>, RoomListUpdateError> {
        let mut guard = self.room_list_updates.lock().map_err(|_| {
            room_list_update_poll_failed(
                ROOM_LIST_UPDATE_POLL_FAILED_CODE,
                ROOM_LIST_UPDATE_POLL_FAILED_DESCRIPTION,
            )
        })?;
        Ok(guard.drain(..).collect())
    }

    /// Typed consume of the already-registered `matrix_room_list_snapshot`.
    ///
    /// Uses `Core::command` with the same null camelCase payload desktop
    /// sends. Does not start SyncService (no dual live sync); an unstarted
    /// owner yields the handler's empty snapshot. Does not expose a generic
    /// command FFI.
    pub async fn room_list_snapshot(&self) -> Result<RoomListSnapshotDto, RoomListSnapshotError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: ROOM_LIST_COMMAND.to_owned(),
                session_generation: ROOM_LIST_READ_ONLY_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(map_room_list_core_error)?;
        let snapshot: NativeRoomListSnapshot = serde_json::from_value(response.payload)
            .map_err(|_| room_list_failed(ROOM_LIST_FAILED_CODE, ROOM_LIST_FAILED_DESCRIPTION))?;
        Ok(RoomListSnapshotDto {
            session_generation: snapshot.session_generation,
            ordered_room_ids: snapshot.ordered_room_ids,
            rooms: snapshot
                .rooms
                .into_iter()
                .map(|room| RoomListRoomDto {
                    room_id: room.room_id,
                    name: room.name,
                    canonical_alias: room.canonical_alias,
                    avatar_url: room.avatar_url,
                    membership: room.membership.as_str().to_owned(),
                    is_direct: room.is_direct,
                    direct_user_id: room.direct_user_id,
                    is_space: room.is_space,
                    is_favorite: room.is_favorite,
                    is_call: room.is_call,
                    has_active_call: room.has_active_call,
                    active_call_participant_count: room.active_call_participant_count,
                    unread_count: room.unread_count,
                    highlight_count: room.highlight_count,
                    marked_unread: room.marked_unread,
                    last_activity_ts: room.last_activity_ts,
                    last_message_preview: room.last_message_preview,
                    last_message_is_agent_approval: room.last_message_is_agent_approval,
                    is_encrypted: room.encryption_status.is_encrypted(),
                    encryption_status: room.encryption_status,
                    notification_mode: room.notification_mode.map(|mode| mode.as_str().to_owned()),
                })
                .collect(),
        })
    }

    pub async fn invites_snapshot(&self) -> Result<InviteSnapshotDto, InviteSnapshotError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: INVITES_COMMAND.to_owned(),
                session_generation: INVITES_READ_ONLY_GENERATION,
                request_id: None,
                payload: serde_json::Value::Null,
            })
            .await
            .map_err(map_invites_core_error)?;
        let snapshot: NativeInviteSnapshot = serde_json::from_value(response.payload)
            .map_err(|_| invites_failed(INVITES_FAILED_CODE, INVITES_FAILED_DESCRIPTION))?;
        Ok(InviteSnapshotDto {
            session_generation: snapshot.session_generation,
            invites: snapshot.invites.into_iter().map(invite_dto).collect(),
        })
    }

    pub async fn room_join_rule_snapshot(
        &self,
        room_id: String,
        session_generation: u64,
    ) -> Result<RoomJoinRuleSnapshotDto, JoinRuleCommandError> {
        let response = self
            .core
            .command(CommandEnvelope {
                command: JOIN_RULE_SNAPSHOT_COMMAND.to_owned(),
                session_generation,
                request_id: None,
                payload: serde_json::json!({
                    "roomId": room_id,
                    "sessionGeneration": session_generation,
                }),
            })
            .await
            .map_err(map_join_rule_core_error)?;
        let snapshot: MatrixRoomJoinRuleSnapshot = serde_json::from_value(response.payload)
            .map_err(|_| join_rule_failed(JOIN_RULE_FAILED_CODE, JOIN_RULE_FAILED_DESCRIPTION))?;
        Ok(RoomJoinRuleSnapshotDto {
            status: snapshot.status,
            room_id: snapshot.room_id,
            session_generation: snapshot.session_generation,
            join_rule: snapshot.join_rule,
        })
    }

    pub async fn room_set_join_rule(
        &self,
        room_id: String,
        join_rule: String,
        allow_room_ids: Option<Vec<String>>,
    ) -> Result<RoomJoinRuleWriteDto, JoinRuleCommandError> {
        let payload = join_rule_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "joinRule": join_rule,
            "allowRoomIds": allow_room_ids,
        }))?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: JOIN_RULE_SET_COMMAND.to_owned(),
                session_generation: 0,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| {
                map_join_rule_core_error_with_no_session(JOIN_RULE_SET_NO_SESSION_CODE, error)
            })?;
        let status = response
            .payload
            .get("status")
            .and_then(|value| value.as_str())
            .ok_or_else(|| join_rule_failed(JOIN_RULE_FAILED_CODE, JOIN_RULE_FAILED_DESCRIPTION))?;
        Ok(RoomJoinRuleWriteDto {
            status: status.to_owned(),
        })
    }

    pub async fn set_room_name(
        &self,
        room_id: String,
        name: String,
    ) -> Result<RoomProfileWriteDto, RoomProfileCommandError> {
        let payload = room_profile_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "name": name,
        }))?;
        self.room_profile_command(
            SET_ROOM_NAME_COMMAND,
            SET_ROOM_NAME_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn set_room_topic(
        &self,
        room_id: String,
        topic: String,
    ) -> Result<RoomProfileWriteDto, RoomProfileCommandError> {
        let payload = room_profile_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "topic": topic,
        }))?;
        self.room_profile_command(
            SET_ROOM_TOPIC_COMMAND,
            SET_ROOM_TOPIC_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn set_room_avatar(
        &self,
        room_id: String,
        mxc: String,
    ) -> Result<RoomProfileWriteDto, RoomProfileCommandError> {
        let payload = room_profile_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "mxc": mxc,
        }))?;
        self.room_profile_command(
            SET_ROOM_AVATAR_COMMAND,
            SET_ROOM_AVATAR_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn room_leave(
        &self,
        room_id: String,
    ) -> Result<RoomMembershipWriteDto, RoomMembershipCommandError> {
        let payload = room_membership_envelope_payload(serde_json::json!({
            "roomId": room_id,
        }))?;
        self.room_membership_command(ROOM_LEAVE_COMMAND, ROOM_LEAVE_NO_SESSION_CODE, payload)
            .await
    }

    pub async fn room_join(
        &self,
        room_id_or_alias: String,
        via_servers: Option<Vec<String>>,
    ) -> Result<RoomMembershipWriteDto, RoomMembershipCommandError> {
        let payload = room_membership_envelope_payload(serde_json::json!({
            "roomIdOrAlias": room_id_or_alias,
            "viaServers": via_servers,
        }))?;
        self.room_membership_command(ROOM_JOIN_COMMAND, ROOM_JOIN_NO_SESSION_CODE, payload)
            .await
    }

    pub async fn room_set_favorite(
        &self,
        room_id: String,
        favorite: bool,
    ) -> Result<RoomMembershipWriteDto, RoomMembershipCommandError> {
        let payload = room_membership_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "favorite": favorite,
        }))?;
        self.room_membership_command(
            ROOM_SET_FAVORITE_COMMAND,
            ROOM_SET_FAVORITE_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn room_invite(
        &self,
        room_id: String,
        user_id: String,
        reason: Option<String>,
    ) -> Result<RoomModerationWriteDto, RoomModerationCommandError> {
        let payload = room_moderation_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
            "reason": reason,
        }))?;
        self.room_moderation_command(ROOM_INVITE_COMMAND, ROOM_INVITE_NO_SESSION_CODE, payload)
            .await
    }

    pub async fn room_kick(
        &self,
        room_id: String,
        user_id: String,
        reason: Option<String>,
    ) -> Result<RoomModerationWriteDto, RoomModerationCommandError> {
        let payload = room_moderation_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
            "reason": reason,
        }))?;
        self.room_moderation_command(ROOM_KICK_COMMAND, ROOM_KICK_NO_SESSION_CODE, payload)
            .await
    }

    pub async fn room_ban(
        &self,
        room_id: String,
        user_id: String,
        reason: Option<String>,
    ) -> Result<RoomModerationWriteDto, RoomModerationCommandError> {
        let payload = room_moderation_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
            "reason": reason,
        }))?;
        self.room_moderation_command(ROOM_BAN_COMMAND, ROOM_BAN_NO_SESSION_CODE, payload)
            .await
    }

    pub async fn room_unban(
        &self,
        room_id: String,
        user_id: String,
    ) -> Result<RoomModerationWriteDto, RoomModerationCommandError> {
        let payload = room_moderation_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
        }))?;
        self.room_moderation_command(ROOM_UNBAN_COMMAND, ROOM_UNBAN_NO_SESSION_CODE, payload)
            .await
    }

    pub async fn room_set_power_level(
        &self,
        room_id: String,
        user_id: String,
        power_level: i64,
    ) -> Result<RoomPowerLevelWriteDto, RoomPowerLevelCommandError> {
        let payload = room_power_level_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "userId": user_id,
            "powerLevel": power_level,
        }))?;
        self.room_power_level_command(
            ROOM_SET_POWER_LEVEL_COMMAND,
            ROOM_SET_POWER_LEVEL_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn room_set_power_levels(
        &self,
        room_id: String,
        content_json: String,
    ) -> Result<RoomPowerLevelWriteDto, RoomPowerLevelCommandError> {
        let content = parse_power_level_content_json(&content_json)?;
        let payload = room_power_level_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "content": content,
        }))?;
        self.room_power_level_command(
            ROOM_SET_POWER_LEVELS_COMMAND,
            ROOM_SET_POWER_LEVELS_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn room_set_power_level_tags(
        &self,
        room_id: String,
        content_json: String,
    ) -> Result<RoomPowerLevelWriteDto, RoomPowerLevelCommandError> {
        let content = parse_power_level_content_json(&content_json)?;
        let payload = room_power_level_envelope_payload(serde_json::json!({
            "roomId": room_id,
            "content": content,
        }))?;
        self.room_power_level_command(
            ROOM_SET_POWER_LEVEL_TAGS_COMMAND,
            ROOM_SET_POWER_LEVEL_TAGS_NO_SESSION_CODE,
            payload,
        )
        .await
    }

    pub async fn room_create(
        &self,
        request: RoomCreateRequestDto,
    ) -> Result<RoomCreateDto, RoomCreateCommandError> {
        let payload = room_create_envelope_payload(room_create_request_payload(request)?)?;
        let response = self
            .core
            .command(CommandEnvelope {
                command: ROOM_CREATE_COMMAND.to_owned(),
                session_generation: ROOM_CREATE_COMMAND_GENERATION,
                request_id: None,
                payload,
            })
            .await
            .map_err(|error| map_room_create_core_error(ROOM_CREATE_NO_SESSION_CODE, error))?;
        room_create_dto(response.payload)
    }

    pub async fn room_members_snapshot(
        &self,
        room_id: String,
    ) -> Result<RoomMembersSnapshotDto, RoomMembersSnapshotError> {
        let payload = self
            .room_members_snapshot_command(
                ROOM_MEMBERS_SNAPSHOT_COMMAND,
                ROOM_MEMBERS_SNAPSHOT_NO_SESSION_CODE,
                room_id,
            )
            .await?;
        room_members_snapshot_dto(payload)
    }

    pub async fn room_power_levels_snapshot(
        &self,
        room_id: String,
    ) -> Result<RoomPowerLevelsSnapshotDto, RoomMembersSnapshotError> {
        let payload = self
            .room_members_snapshot_command(
                ROOM_POWER_LEVELS_SNAPSHOT_COMMAND,
                ROOM_POWER_LEVELS_SNAPSHOT_NO_SESSION_CODE,
                room_id,
            )
            .await?;
        room_power_levels_snapshot_dto(payload)
    }

    pub async fn room_creators_snapshot(
        &self,
        room_id: String,
    ) -> Result<RoomCreatorsSnapshotDto, RoomMembersSnapshotError> {
        let payload = self
            .room_members_snapshot_command(
                ROOM_CREATORS_SNAPSHOT_COMMAND,
                ROOM_CREATORS_SNAPSHOT_NO_SESSION_CODE,
                room_id,
            )
            .await?;
        room_creators_snapshot_dto(payload)
    }

    pub async fn room_power_level_tags_snapshot(
        &self,
        room_id: String,
    ) -> Result<RoomPowerLevelTagsSnapshotDto, RoomMembersSnapshotError> {
        let payload = self
            .room_members_snapshot_command(
                ROOM_POWER_LEVEL_TAGS_SNAPSHOT_COMMAND,
                ROOM_POWER_LEVEL_TAGS_SNAPSHOT_NO_SESSION_CODE,
                room_id,
            )
            .await?;
        room_power_level_tags_snapshot_dto(payload)
    }

    pub async fn invites_accept(
        &self,
        room_id: String,
    ) -> Result<InviteSnapshotDto, InviteActionError> {
        self.invite_action_command(
            INVITES_ACCEPT_COMMAND,
            INVITES_ACCEPT_NO_SESSION_CODE,
            room_id,
        )
        .await
    }

    pub async fn invites_decline(
        &self,
        room_id: String,
    ) -> Result<InviteSnapshotDto, InviteActionError> {
        self.invite_action_command(
            INVITES_DECLINE_COMMAND,
            INVITES_DECLINE_NO_SESSION_CODE,
            room_id,
        )
        .await
    }

    pub async fn invites_report_spam(
        &self,
        room_id: String,
    ) -> Result<InviteSnapshotDto, InviteActionError> {
        self.invite_action_command(
            INVITES_REPORT_SPAM_COMMAND,
            INVITES_REPORT_SPAM_NO_SESSION_CODE,
            room_id,
        )
        .await
    }

    pub async fn invites_block_sender(
        &self,
        room_id: String,
    ) -> Result<InviteSnapshotDto, InviteActionError> {
        self.invite_action_command(
            INVITES_BLOCK_SENDER_COMMAND,
            INVITES_BLOCK_SENDER_NO_SESSION_CODE,
            room_id,
        )
        .await
    }
}
