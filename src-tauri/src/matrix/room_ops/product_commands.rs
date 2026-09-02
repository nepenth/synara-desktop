use super::*;

#[tauri::command]
pub async fn matrix_invites_accept(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    crate::bridge::invites_snapshot::invites_accept(core.inner().as_ref(), room_id).await
}

#[tauri::command]
pub async fn matrix_invites_decline(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    crate::bridge::invites_snapshot::invites_decline(core.inner().as_ref(), room_id).await
}

/// V-ROOMS room creation: create the room through the live native Matrix SDK.
/// Fail-closed: desktop create call sites must not use `mx.createRoom` when a
/// native Matrix session owns room lifecycle mutations.
#[tauri::command]
pub async fn matrix_room_create(
    core: State<'_, Arc<synara_core::Core>>,
    request: MatrixRoomCreateRequest,
) -> Result<String, MatrixAuthCommandError> {
    crate::bridge::room_create::room_create(core.inner().as_ref(), request).await
}

/// V-ROOMS room membership: leave the selected room through the native SDK.
/// Fail-closed: the desktop product must not use `mx.leave` when a native
/// Matrix session owns the room lifecycle.
#[tauri::command]
pub async fn matrix_room_leave(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
) -> Result<(), MatrixAuthCommandError> {
    crate::bridge::room_leave_join::room_leave(core.inner().as_ref(), room_id).await
}

/// V-ROOMS room membership: join a room or room alias through the native SDK.
/// Fail-closed: the desktop product must not use `mx.joinRoom` when a native
/// Matrix session owns the room lifecycle.
#[tauri::command]
pub async fn matrix_room_join(
    core: State<'_, Arc<synara_core::Core>>,
    room_id_or_alias: String,
    via_servers: Option<Vec<String>>,
) -> Result<(), MatrixAuthCommandError> {
    crate::bridge::room_leave_join::room_join(core.inner().as_ref(), room_id_or_alias, via_servers)
        .await
}

/// Persist `m.favourite` through the native Matrix SDK tag write.
/// Fail-closed: the desktop product must not fake favorites in localStorage.
#[tauri::command]
pub async fn matrix_room_set_favorite(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    favorite: bool,
) -> Result<(), MatrixAuthCommandError> {
    crate::bridge::room_leave_join::room_set_favorite(core.inner().as_ref(), room_id, favorite)
        .await
}

/// Persist an explicit user's private receipt or unread flag through the native
/// Matrix SDK. Automatic visibility acknowledgements use the timeline command
/// with an exact observed-tail precondition instead.
#[tauri::command]
pub async fn matrix_room_set_read_state(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    action: NativeTimelineReadAction,
) -> Result<(), MatrixAuthCommandError> {
    crate::bridge::room_read_state::room_set_read_state(core.inner().as_ref(), room_id, action)
        .await
}

/// V-ROOMS members moderation: invite a user through the live native Matrix SDK.
/// Fail-closed: desktop moderation must not use the JS SDK membership methods.
#[tauri::command]
pub async fn matrix_room_invite(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    user_id: String,
    reason: Option<String>,
) -> Result<(), MatrixAuthCommandError> {
    crate::bridge::room_moderation::room_invite(core.inner().as_ref(), room_id, user_id, reason)
        .await
}

/// V-ROOMS members moderation: kick a user through the live native Matrix SDK.
#[tauri::command]
pub async fn matrix_room_kick(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    user_id: String,
    reason: Option<String>,
) -> Result<(), MatrixAuthCommandError> {
    crate::bridge::room_moderation::room_kick(core.inner().as_ref(), room_id, user_id, reason).await
}

/// V-ROOMS members moderation: ban a user through the live native Matrix SDK.
#[tauri::command]
pub async fn matrix_room_ban(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    user_id: String,
    reason: Option<String>,
) -> Result<(), MatrixAuthCommandError> {
    crate::bridge::room_moderation::room_ban(core.inner().as_ref(), room_id, user_id, reason).await
}

/// V-ROOMS members moderation: unban a user through the live native Matrix SDK.
#[tauri::command]
pub async fn matrix_room_unban(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    user_id: String,
) -> Result<(), MatrixAuthCommandError> {
    crate::bridge::room_moderation::room_unban(core.inner().as_ref(), room_id, user_id).await
}

/// V-ROOMS members moderation: set one user's power level through the live SDK.
#[tauri::command]
pub async fn matrix_room_set_power_level(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    user_id: String,
    power_level: i64,
) -> Result<(), MatrixAuthCommandError> {
    crate::bridge::room_moderation::room_set_power_level(
        core.inner().as_ref(),
        room_id,
        user_id,
        power_level,
    )
    .await
}

/// V-ROOMS.R-POWERS-BULK — replace the complete `m.room.power_levels` state
/// event through the one managed native Matrix SDK client. This is deliberately
/// not implemented as repeated `matrix_room_set_power_level` calls.
#[tauri::command]
pub async fn matrix_room_set_power_levels(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    content: serde_json::Value,
) -> Result<NativePowerLevelWriteResult, MatrixAuthCommandError> {
    crate::bridge::room_power_levels::room_set_power_levels(core.inner().as_ref(), room_id, content)
        .await
}

/// V-ROOMS.R-POWERS-BULK — replace the complete
/// `in.synara.room.power_level_tags` state event through the one managed native
/// Matrix SDK client. Empty `{}` is a valid tag state and represents deletion
/// of all custom tags.
#[tauri::command]
pub async fn matrix_room_set_power_level_tags(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
    content: serde_json::Value,
) -> Result<NativePowerLevelWriteResult, MatrixAuthCommandError> {
    crate::bridge::room_power_levels::room_set_power_level_tags(
        core.inner().as_ref(),
        room_id,
        content,
    )
    .await
}

#[tauri::command]
pub async fn matrix_invites_report_spam(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    crate::bridge::invites_snapshot::invites_report_spam(core.inner().as_ref(), room_id).await
}

#[tauri::command]
pub async fn matrix_invites_block_sender(
    core: State<'_, Arc<synara_core::Core>>,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    crate::bridge::invites_snapshot::invites_block_sender(core.inner().as_ref(), room_id).await
}

pub(super) fn map_room_leave_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-rooms-room-leave-invalid-room" => (
            "InvalidRequest",
            "The native Matrix room leave request is invalid.",
        ),
        "v-rooms-room-leave-room-not-found" => {
            ("NotFound", "The native Matrix room is not available.")
        }
        _ => ("Unknown", "The native Matrix room could not be left."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

pub(super) fn map_room_moderation_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-rooms-members-moderation-invalid-room"
        | "v-rooms-members-moderation-invalid-user"
        | "v-rooms-members-moderation-invalid-power-level" => (
            "InvalidRequest",
            "The native Matrix member moderation request is invalid.",
        ),
        "v-rooms-members-moderation-room-not-found" => (
            "NotFound",
            "The native Matrix moderation room is not available.",
        ),
        _ => (
            "Unknown",
            "The native Matrix member moderation operation could not be completed.",
        ),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

pub(super) fn map_power_level_write_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
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
        | "v-rooms-power-levels-content-too-large" => (
            "InvalidRequest",
            "The native Matrix power-level write request is invalid.",
        ),
        "v-rooms-power-levels-room-not-found" => (
            "NotFound",
            "The native Matrix power-level room is not available.",
        ),
        "v-rooms-power-levels-stale-session-generation" => (
            "StaleSessionGeneration",
            "The native Matrix session changed during the power-level write.",
        ),
        _ => (
            "Unknown",
            "The native Matrix power-level write could not be completed.",
        ),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

pub(super) fn validate_room_power_levels_content(
    content: &serde_json::Value,
) -> Result<(), MatrixAuthCommandError> {
    synara_core::app::members::validate_room_power_levels_content(content)
        .map_err(map_power_level_write_error)
}

pub(super) fn validate_power_level_tags_content(
    content: &serde_json::Value,
) -> Result<(), MatrixAuthCommandError> {
    synara_core::app::members::validate_power_level_tags_content(content)
        .map_err(map_power_level_write_error)
}

pub(super) fn map_room_create_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
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
        | "v-rooms-room-create-invalid-power-level" => (
            "InvalidRequest",
            "The native Matrix room create request is invalid.",
        ),
        _ => ("Unknown", "The native Matrix room could not be created."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

pub(super) fn build_room_create_request(
    input: MatrixRoomCreateRequest,
) -> Result<create_room::v3::Request, MatrixAuthCommandError> {
    synara_core::app::room_ops::build_room_create_request(input).map_err(map_room_create_error)
}

pub(super) fn parse_room_leave_id(room_id: &str) -> Result<OwnedRoomId, MatrixAuthCommandError> {
    room_id
        .trim()
        .parse()
        .map_err(|_| map_room_leave_error("v-rooms-room-leave-invalid-room"))
}

pub(super) fn parse_room_members_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    synara_core::app::members::parse_room_members_room_id(room_id)
}

pub(super) fn parse_room_moderation_room_id(
    room_id: &str,
) -> Result<OwnedRoomId, MatrixAuthCommandError> {
    room_id
        .trim()
        .parse()
        .map_err(|_| map_room_moderation_error("v-rooms-members-moderation-invalid-room"))
}

pub(super) fn parse_room_moderation_user_id(
    user_id: &str,
) -> Result<OwnedUserId, MatrixAuthCommandError> {
    user_id
        .trim()
        .parse()
        .map_err(|_| map_room_moderation_error("v-rooms-members-moderation-invalid-user"))
}

pub(super) fn parse_room_moderation_power_level(
    power_level: i64,
) -> Result<Int, MatrixAuthCommandError> {
    power_level
        .try_into()
        .map_err(|_| map_room_moderation_error("v-rooms-members-moderation-invalid-power-level"))
}

pub(super) fn normalize_moderation_reason(reason: Option<String>) -> Option<String> {
    reason
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub(super) fn map_room_join_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-rooms-room-join-invalid-room" | "v-rooms-room-join-invalid-via-server" => (
            "InvalidRequest",
            "The native Matrix room join request is invalid.",
        ),
        _ => ("Unknown", "The native Matrix room could not be joined."),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
}

pub(super) fn parse_room_join_target(
    room_id_or_alias: &str,
) -> Result<OwnedRoomOrAliasId, MatrixAuthCommandError> {
    room_id_or_alias
        .trim()
        .parse()
        .map_err(|_| map_room_join_error("v-rooms-room-join-invalid-room"))
}

pub(super) fn parse_room_join_via_servers(
    via_servers: Option<&[String]>,
) -> Result<Vec<OwnedServerName>, MatrixAuthCommandError> {
    via_servers
        .unwrap_or_default()
        .iter()
        .map(|server| {
            server
                .trim()
                .parse()
                .map_err(|_| map_room_join_error("v-rooms-room-join-invalid-via-server"))
        })
        .collect()
}
