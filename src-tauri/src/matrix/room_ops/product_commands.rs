use super::*;

#[tauri::command]
pub async fn matrix_invites_accept(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    let invite = native_invite_target(active, &room_id).await?;
    let room = native_invite_room(active, &invite)?;
    room.join()
        .await
        .map_err(|_| map_invite_error("v-rooms.1-invite-accept-failed"))?;
    if invite.is_direct {
        let sender_id = OwnedUserId::try_from(invite.sender_id.as_str())
            .map_err(|_| map_invite_error("v-rooms.1-invite-invalid-sender"))?;
        active
            .client
            .account()
            .mark_as_dm(room.room_id(), &[sender_id])
            .await
            .map_err(|_| map_invite_error("v-rooms.1-invite-direct-mark-failed"))?;
    }
    active.invite_avatars.revoke_room(&invite.room_id);
    snapshot_invites(
        &active.client,
        active.sync.session_generation(),
        &mut active.invite_avatars,
    )
    .await
    .map_err(map_invite_error)
}

#[tauri::command]
pub async fn matrix_invites_decline(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    let invite = native_invite_target(active, &room_id).await?;
    native_invite_room(active, &invite)?
        .leave()
        .await
        .map_err(|_| map_invite_error("v-rooms.1-invite-decline-failed"))?;
    active.invite_avatars.revoke_room(&invite.room_id);
    snapshot_invites(
        &active.client,
        active.sync.session_generation(),
        &mut active.invite_avatars,
    )
    .await
    .map_err(map_invite_error)
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

pub(super) const ROOM_POWER_LEVELS_EVENT_TYPE: &str = "m.room.power_levels";
pub(super) const POWER_LEVEL_TAGS_EVENT_TYPE: &str = "in.synara.room.power_level_tags";

#[tauri::command]
pub async fn matrix_invites_report_spam(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    let invite = native_invite_target(active, &room_id).await?;
    native_invite_room(active, &invite)?
        .report_room("Spam Invite".to_owned())
        .await
        .map_err(|_| map_invite_error("v-rooms.1-invite-report-failed"))?;
    active.invite_avatars.revoke_room(&invite.room_id);
    snapshot_invites(
        &active.client,
        active.sync.session_generation(),
        &mut active.invite_avatars,
    )
    .await
    .map_err(map_invite_error)
}

#[tauri::command]
pub async fn matrix_invites_block_sender(
    state: State<'_, MatrixAuthState>,
    room_id: String,
) -> Result<NativeInviteSnapshot, MatrixAuthCommandError> {
    let mut session = state.session.lock().await;
    let active = require_session_mut(session.as_mut())?;
    let invite = native_invite_target(active, &room_id).await?;
    let sender_id = OwnedUserId::try_from(invite.sender_id.as_str())
        .map_err(|_| map_invite_error("v-rooms.1-invite-invalid-sender"))?;
    active
        .client
        .account()
        .ignore_user(&sender_id)
        .await
        .map_err(|_| map_invite_error("v-rooms.1-invite-block-failed"))?;
    active.invite_avatars.revoke_room(&invite.room_id);
    snapshot_invites(
        &active.client,
        active.sync.session_generation(),
        &mut active.invite_avatars,
    )
    .await
    .map_err(map_invite_error)
}

pub(super) fn map_invite_error(diagnostic_id: &'static str) -> MatrixAuthCommandError {
    let (code, message) = match diagnostic_id {
        "v-rooms.1-invite-invalid-room" | "v-rooms.1-invite-invalid-sender" => (
            "InvalidRequest",
            "The native Matrix invite request is invalid.",
        ),
        "v-rooms.1-invite-not-found" | "v-rooms.1-invite-member-missing" => (
            "NotFound",
            "The native Matrix invitation is no longer available.",
        ),
        _ => (
            "Unknown",
            "The native Matrix invite operation could not be completed.",
        ),
    };
    MatrixAuthCommandError::new(code, message, diagnostic_id)
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
    room_id
        .trim()
        .parse()
        .map_err(|_| "v-rooms-members-read-invalid-room")
}

pub(super) fn project_room_member(
    room_id: &OwnedRoomId,
    member: &matrix_sdk::room::RoomMember,
    is_two_party_direct: bool,
    current_user: Option<&matrix_sdk::ruma::UserId>,
) -> Result<ProductRoomMember, &'static str> {
    let membership = match member.membership() {
        MembershipState::Ban => ProductMembership::Ban,
        MembershipState::Invite => ProductMembership::Invite,
        MembershipState::Join => ProductMembership::Join,
        MembershipState::Knock => ProductMembership::Knock,
        MembershipState::Leave => ProductMembership::Leave,
        _ => return Err("v-rooms-members-read-unsupported-membership"),
    };
    let power_level = match member.power_level() {
        UserPowerLevel::Infinite => i32::MAX,
        UserPowerLevel::Int(value) => {
            i32::try_from(value).map_err(|_| "v-rooms-members-read-power-level-invalid")?
        }
        _ => return Err("v-rooms-members-read-power-level-invalid"),
    };

    Ok(ProductRoomMember {
        room_id: room_id.to_string(),
        user_id: member.user_id().to_string(),
        display_name: member.display_name().map(ToOwned::to_owned),
        avatar_url: member.avatar_url().map(ToString::to_string),
        membership,
        power_level,
        is_direct_target: is_two_party_direct
            .then(|| current_user.is_some_and(|current_user| current_user != member.user_id())),
    })
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

pub(super) async fn native_invite_target(
    active: &mut ManagedMatrixSession,
    room_id: &str,
) -> Result<NativeInvite, MatrixAuthCommandError> {
    let normalized_room_id = room_id.trim();
    if normalized_room_id.is_empty() {
        return Err(map_invite_error("v-rooms.1-invite-invalid-room"));
    }
    let snapshot = snapshot_invites(
        &active.client,
        active.sync.session_generation(),
        &mut active.invite_avatars,
    )
    .await
    .map_err(map_invite_error)?;
    snapshot
        .invites
        .into_iter()
        .find(|invite| invite.room_id == normalized_room_id)
        .ok_or_else(|| map_invite_error("v-rooms.1-invite-not-found"))
}

pub(super) fn native_invite_room(
    active: &ManagedMatrixSession,
    invite: &NativeInvite,
) -> Result<Room, MatrixAuthCommandError> {
    let room_id = OwnedRoomId::try_from(invite.room_id.as_str())
        .map_err(|_| map_invite_error("v-rooms.1-invite-invalid-room"))?;
    active
        .client
        .get_room(&room_id)
        .ok_or_else(|| map_invite_error("v-rooms.1-invite-not-found"))
}
