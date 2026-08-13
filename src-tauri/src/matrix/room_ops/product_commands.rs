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
    state: State<'_, MatrixAuthState>,
    request: MatrixRoomCreateRequest,
) -> Result<String, MatrixAuthCommandError> {
    let request = build_room_create_request(request)?;
    let session = state.session.lock().await;
    let active = require_session(session.as_ref())?;
    let room = active
        .client
        .create_room(request)
        .await
        .map_err(|_| map_room_create_error("v-rooms-room-create-failed"))?;
    Ok(room.room_id().to_string())
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
    let MatrixRoomCreateRequest {
        name,
        topic,
        room_version,
        room_alias_name,
        is_direct,
        invite,
        visibility,
        preset,
        creation_content,
        encryption,
        join_rule,
        knock,
        parent_room_id,
        power_level_content_override,
    } = input;

    let name = name
        .map(|value| {
            let value = value.trim();
            if value.is_empty() || value.chars().count() > 255 {
                return Err(map_room_create_error("v-rooms-room-create-invalid-name"));
            }
            Ok(value.to_owned())
        })
        .transpose()?;
    let topic = topic
        .map(|value| {
            let value = value.trim();
            if value.chars().count() > 2_048 {
                return Err(map_room_create_error("v-rooms-room-create-invalid-topic"));
            }
            if value.is_empty() {
                Ok(None)
            } else {
                Ok(Some(value.to_owned()))
            }
        })
        .transpose()?
        .flatten();
    let room_version = room_version
        .map(|value| {
            value
                .trim()
                .parse::<RoomVersionId>()
                .map_err(|_| map_room_create_error("v-rooms-room-create-invalid-room-version"))
        })
        .transpose()?;
    let room_alias_name = room_alias_name
        .map(|value| {
            let value = value.trim();
            if value.is_empty() || value.chars().count() > 255 {
                return Err(map_room_create_error("v-rooms-room-create-invalid-alias"));
            }
            Ok(value.to_owned())
        })
        .transpose()?;
    let invite = invite
        .into_iter()
        .map(|value| {
            value
                .trim()
                .parse::<OwnedUserId>()
                .map_err(|_| map_room_create_error("v-rooms-room-create-invalid-invite"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let parent_room_id = parent_room_id
        .map(|value| {
            value
                .trim()
                .parse::<OwnedRoomId>()
                .map_err(|_| map_room_create_error("v-rooms-room-create-invalid-parent"))
        })
        .transpose()?;

    let room_type = creation_content
        .as_ref()
        .and_then(|content| content.room_type.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    if creation_content
        .as_ref()
        .and_then(|content| content.room_type.as_deref())
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(map_room_create_error(
            "v-rooms-room-create-invalid-creation-content",
        ));
    }

    let creation_content = creation_content
        .map(build_room_create_creation_content)
        .transpose()?;
    let power_level_content_override = power_level_content_override
        .map(build_room_create_power_levels)
        .transpose()?;

    let mut initial_state = Vec::new();
    if encryption {
        initial_state.push(raw_room_create_state(
            "m.room.encryption",
            "",
            serde_json::json!({ "algorithm": "m.megolm.v1.aes-sha2" }),
        )?);
    }
    if room_type.as_deref() == Some("org.matrix.msc3417.call") {
        initial_state.push(raw_room_create_state(
            "org.matrix.msc3401.call",
            "",
            serde_json::json!({}),
        )?);
    }
    if let Some(join_rules) =
        build_room_create_join_rules(join_rule.as_deref(), knock, parent_room_id.as_ref())?
    {
        initial_state.push(join_rules);
    }

    let mut request = create_room::v3::Request::new();
    request.name = name;
    request.topic = topic;
    request.room_version = room_version;
    request.room_alias_name = room_alias_name;
    request.is_direct = is_direct;
    request.invite = invite;
    request.visibility = match visibility {
        Some(MatrixRoomCreateVisibility::Public) => Visibility::Public,
        Some(MatrixRoomCreateVisibility::Private) | None => Visibility::Private,
    };
    request.preset = preset.map(|preset| match preset {
        MatrixRoomCreatePreset::Private => RoomPreset::PrivateChat,
        MatrixRoomCreatePreset::Public => RoomPreset::PublicChat,
        MatrixRoomCreatePreset::TrustedPrivate => RoomPreset::TrustedPrivateChat,
    });
    request.creation_content = creation_content;
    request.initial_state = initial_state;
    request.power_level_content_override = power_level_content_override;
    Ok(request)
}

pub(super) fn build_room_create_creation_content(
    content: MatrixRoomCreateContent,
) -> Result<Raw<create_room::v3::CreationContent>, MatrixAuthCommandError> {
    let mut value = serde_json::Map::new();
    if let Some(room_type) = content.room_type {
        value.insert("type".to_owned(), serde_json::Value::String(room_type));
    }
    if let Some(federate) = content.federate {
        value.insert("m.federate".to_owned(), serde_json::json!(federate));
    }
    if let Some(additional_creators) = content.additional_creators {
        let additional_creators = additional_creators
            .into_iter()
            .map(|value| {
                value.trim().parse::<OwnedUserId>().map_err(|_| {
                    map_room_create_error("v-rooms-room-create-invalid-additional-creator")
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        value.insert(
            "additional_creators".to_owned(),
            serde_json::to_value(additional_creators).expect("user IDs serialize"),
        );
    }
    raw_room_create(
        serde_json::Value::Object(value),
        "v-rooms-room-create-invalid-creation-content",
    )
}

pub(super) fn build_room_create_power_levels(
    power_levels: MatrixRoomCreatePowerLevels,
) -> Result<Raw<create_room::RoomPowerLevelsContentOverride>, MatrixAuthCommandError> {
    if power_levels.events_default.is_none() && power_levels.events.is_empty() {
        return Err(map_room_create_error(
            "v-rooms-room-create-invalid-power-level",
        ));
    }
    let mut value = serde_json::Map::new();
    if let Some(events_default) = power_levels.events_default {
        value.insert(
            "events_default".to_owned(),
            serde_json::json!(events_default),
        );
    }
    if !power_levels.events.is_empty() {
        value.insert(
            "events".to_owned(),
            serde_json::to_value(power_levels.events).expect("power level map serializes"),
        );
    }
    raw_room_create(
        serde_json::Value::Object(value),
        "v-rooms-room-create-invalid-power-level",
    )
}

pub(super) fn build_room_create_join_rules(
    join_rule: Option<&str>,
    knock: bool,
    parent_room_id: Option<&OwnedRoomId>,
) -> Result<Option<Raw<AnyInitialStateEvent>>, MatrixAuthCommandError> {
    let Some(join_rule) = join_rule else {
        if knock {
            return Err(map_room_create_error(
                "v-rooms-room-create-invalid-join-rule",
            ));
        }
        return Ok(None);
    };

    let join_rule = join_rule.trim();
    let join_rule = match join_rule {
        "invite" | "knock" => {
            if join_rule == "knock" || knock {
                "knock"
            } else {
                "invite"
            }
        }
        "restricted" | "knock_restricted" => {
            if join_rule == "knock_restricted" || knock {
                "knock_restricted"
            } else {
                "restricted"
            }
        }
        "public" if !knock => "public",
        _ => {
            return Err(map_room_create_error(
                "v-rooms-room-create-invalid-join-rule",
            ));
        }
    };

    let restricted = matches!(join_rule, "restricted" | "knock_restricted");
    if restricted && parent_room_id.is_none() {
        return Err(map_room_create_error(
            "v-rooms-room-create-missing-restricted-parent",
        ));
    }

    let mut content = serde_json::json!({ "join_rule": join_rule });
    if restricted {
        content["allow"] = serde_json::json!([{
            "type": "m.room_membership",
            "room_id": parent_room_id.expect("restricted parent checked").to_string(),
        }]);
    }
    Ok(Some(raw_room_create_state(
        "m.room.join_rules",
        "",
        content,
    )?))
}

pub(super) fn raw_room_create_state(
    event_type: &str,
    state_key: &str,
    content: serde_json::Value,
) -> Result<Raw<AnyInitialStateEvent>, MatrixAuthCommandError> {
    raw_room_create(
        serde_json::json!({
            "type": event_type,
            "state_key": state_key,
            "content": content,
        }),
        "v-rooms-room-create-invalid-creation-content",
    )
}

pub(super) fn raw_room_create<T>(
    value: serde_json::Value,
    diagnostic_id: &'static str,
) -> Result<Raw<T>, MatrixAuthCommandError> {
    serde_json::value::to_raw_value(&value)
        .map(Raw::<T>::from_json)
        .map_err(|_| map_room_create_error(diagnostic_id))
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
