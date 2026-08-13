//! Live room-profile state owned by the managed native Matrix session.
//!
//! This module projects the bounded join-rule vocabulary needed by the
//! room-settings Publish-to-Directory gate and owns the room name/topic/avatar
//! writes. It does not own a join-rule writer and never sends SDK event
//! objects over the application event boundary.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use matrix_sdk::{
    deserialized_responses::RawSyncOrStrippedState,
    event_handler::EventHandlerDropGuard,
    ruma::{
        api::client::{room::Visibility, state::get_state_event_for_key},
        events::{
            room::join_rules::{RoomJoinRulesEventContent, SyncRoomJoinRulesEvent},
            StateEventType,
        },
        room::JoinRule,
        Int, OwnedMxcUri, OwnedRoomId, OwnedRoomOrAliasId, OwnedServerName, OwnedUserId,
    },
    Client, Room, RoomState,
};

use crate::app::members::{
    validate_power_level_tags_content, validate_room_power_levels_content,
    NativePowerLevelWriteResult, ROOM_POWER_LEVELS_EVENT_TYPE, ROOM_POWER_LEVEL_TAGS_EVENT_TYPE,
};
use crate::app::user_profile::MatrixProfileWriteResult;

use super::{
    MatrixRoomDirectoryVisibilityResult, MatrixRoomDirectoryVisibilityWriteResult,
    MatrixRoomJoinRuleSnapshot, NativeRoomJoinRuleUpdate,
};

/// Shell-supplied sink for join-rule updates. Desktop maps this to the
/// existing Tauri event; iOS can map it to a UniFFI callback later.
pub type JoinRuleUpdateEmit = Arc<dyn Fn(NativeRoomJoinRuleUpdate) + Send + Sync>;

/// Project the SDK/Ruma enum into the exact product union. Custom rules are
/// intentionally unavailable; in particular they must never become a
/// publishable rule through a string fallback.
pub fn project_join_rule(rule: &JoinRule) -> Option<&'static str> {
    match rule {
        JoinRule::Public => Some("public"),
        JoinRule::Knock => Some("knock"),
        JoinRule::Invite => Some("invite"),
        JoinRule::Restricted(_) => Some("restricted"),
        JoinRule::KnockRestricted(_) => Some("knock_restricted"),
        JoinRule::Private => Some("private"),
        _ => None,
    }
}

/// Owns the native m.room.join_rules update boundary for one managed session.
/// Dropping the guard removes the SDK handler; `retire` makes the lifecycle
/// boundary explicit and prevents a late event from the old generation from
/// reaching the webview during teardown.
pub struct NativeRoomJoinRuleOwner {
    client: Client,
    session_generation: u64,
    retired: Arc<AtomicBool>,
    _handler: EventHandlerDropGuard,
}

impl NativeRoomJoinRuleOwner {
    pub fn start(
        client: &Client,
        emit: JoinRuleUpdateEmit,
        session_generation: u64,
    ) -> Result<Self, &'static str> {
        if session_generation == 0 {
            return Err("v-send.r-room-profile-join-rule-owner-invalid-generation");
        }
        client
            .user_id()
            .ok_or("v-send.r-room-profile-join-rule-owner-no-user")?;

        let retired = Arc::new(AtomicBool::new(false));
        let retired_for_handler = retired.clone();
        let handler = client.add_event_handler(move |event: SyncRoomJoinRulesEvent, room: Room| {
            let emit = emit.clone();
            let retired = retired_for_handler.clone();
            async move {
                if retired.load(Ordering::Acquire) {
                    return;
                }

                let room_id = room.room_id().to_string();
                let join_rule = if room.state() == RoomState::Joined {
                    project_join_rule(event.join_rule())
                } else {
                    None
                };
                let update = match join_rule {
                    Some(join_rule) => NativeRoomJoinRuleUpdate::Ready {
                        room_id,
                        session_generation,
                        join_rule,
                    },
                    None => NativeRoomJoinRuleUpdate::Unavailable {
                        room_id,
                        session_generation,
                    },
                };
                emit(update);
            }
        });

        Ok(Self {
            client: client.clone(),
            session_generation,
            retired,
            _handler: client.event_handler_drop_guard(handler),
        })
    }

    pub fn session_generation(&self) -> u64 {
        self.session_generation
    }

    pub fn retire(&self) {
        self.retired.store(true, Ordering::Release);
    }

    pub async fn snapshot(
        &self,
        room_id: &str,
        session_generation: u64,
    ) -> Result<MatrixRoomJoinRuleSnapshot, &'static str> {
        if self.retired.load(Ordering::Acquire) {
            return Err("v-send.r-room-profile-join-rule-requires-session");
        }
        if session_generation == 0 || session_generation != self.session_generation {
            return Err("v-send.r-room-profile-join-rule-stale-generation");
        }
        let room_id = parse_join_rule_room_id(room_id)?;
        let room = self
            .client
            .get_room(&room_id)
            .ok_or("v-send.r-room-profile-join-rule-room-not-found")?;
        if room.state() != RoomState::Joined {
            return Err("v-send.r-room-profile-join-rule-room-state-unavailable");
        }
        let raw = room
            .get_state_event_static::<RoomJoinRulesEventContent>()
            .await
            .map_err(|_| "v-send.r-room-profile-join-rule-read-sdk-failed")?
            .ok_or("v-send.r-room-profile-join-rule-room-state-unavailable")?;
        let event = match raw {
            RawSyncOrStrippedState::Sync(raw) => raw
                .deserialize()
                .map_err(|_| "v-send.r-room-profile-join-rule-deserialize-failed")?,
            RawSyncOrStrippedState::Stripped(_) => {
                return Err("v-send.r-room-profile-join-rule-room-state-unavailable");
            }
        };
        let original = event
            .as_original()
            .ok_or("v-send.r-room-profile-join-rule-room-state-unavailable")?;
        let join_rule = project_join_rule(&original.content.join_rule)
            .ok_or("v-send.r-room-profile-join-rule-unsupported")?;
        Ok(MatrixRoomJoinRuleSnapshot {
            status: "ok".to_owned(),
            room_id: room_id.to_string(),
            session_generation: self.session_generation,
            join_rule: join_rule.to_owned(),
        })
    }

    pub async fn set_name(
        &self,
        room_id: &str,
        name: &str,
    ) -> Result<MatrixProfileWriteResult, &'static str> {
        if self.retired.load(Ordering::Acquire) {
            return Err("v-send.r-room-profile-join-rule-requires-session");
        }
        let name = parse_room_name(name)?;
        let room = self.profile_room(room_id)?;
        room.set_name(name)
            .await
            .map_err(|_| "v-send.r-room-profile-name-sdk-failed")?;
        Ok(MatrixProfileWriteResult { status: "ok" })
    }

    pub async fn set_topic(
        &self,
        room_id: &str,
        topic: &str,
    ) -> Result<MatrixProfileWriteResult, &'static str> {
        if self.retired.load(Ordering::Acquire) {
            return Err("v-send.r-room-profile-join-rule-requires-session");
        }
        let topic = parse_room_topic(topic)?;
        let room = self.profile_room(room_id)?;
        room.set_room_topic(&topic)
            .await
            .map_err(|_| "v-send.r-room-profile-topic-sdk-failed")?;
        Ok(MatrixProfileWriteResult { status: "ok" })
    }

    pub async fn set_avatar(
        &self,
        room_id: &str,
        mxc: &str,
    ) -> Result<MatrixProfileWriteResult, &'static str> {
        if self.retired.load(Ordering::Acquire) {
            return Err("v-send.r-room-profile-join-rule-requires-session");
        }
        let mxc = parse_avatar_mxc(mxc)?;
        let room = self.profile_room(room_id)?;
        match mxc {
            Some(url) => {
                room.set_avatar_url(&url, None)
                    .await
                    .map_err(|_| "v-send.r-room-profile-avatar-set-sdk-failed")?;
            }
            None => {
                room.remove_avatar()
                    .await
                    .map_err(|_| "v-send.r-room-profile-avatar-remove-sdk-failed")?;
            }
        }
        Ok(MatrixProfileWriteResult { status: "ok" })
    }

    pub async fn leave(&self, room_id: &str) -> Result<(), &'static str> {
        if self.retired.load(Ordering::Acquire) {
            return Err("v-send.r-room-profile-join-rule-requires-session");
        }
        let room_id = parse_room_leave_id(room_id)?;
        let room = self
            .client
            .get_room(&room_id)
            .ok_or("v-rooms-room-leave-room-not-found")?;
        room.leave().await.map_err(|_| "v-rooms-room-leave-failed")
    }

    pub async fn join(
        &self,
        room_id_or_alias: &str,
        via_servers: Option<Vec<String>>,
    ) -> Result<(), &'static str> {
        if self.retired.load(Ordering::Acquire) {
            return Err("v-send.r-room-profile-join-rule-requires-session");
        }
        let target = parse_room_join_target(room_id_or_alias)?;
        let via_servers = parse_room_join_via_servers(via_servers.as_deref())?;
        self.client
            .join_room_by_id_or_alias(&target, &via_servers)
            .await
            .map(|_| ())
            .map_err(|_| "v-rooms-room-join-failed")
    }

    pub async fn invite(
        &self,
        room_id: &str,
        user_id: &str,
        reason: Option<String>,
    ) -> Result<(), &'static str> {
        // matrix-sdk 0.18's invite_user_by_id API does not expose a reason field.
        let _reason = normalize_moderation_reason(reason);
        let room = self.moderation_room(room_id)?;
        let user_id = parse_room_moderation_user_id(user_id)?;
        room.invite_user_by_id(&user_id)
            .await
            .map_err(|_| "v-rooms-members-moderation-invite-failed")
    }

    pub async fn kick(
        &self,
        room_id: &str,
        user_id: &str,
        reason: Option<String>,
    ) -> Result<(), &'static str> {
        let room = self.moderation_room(room_id)?;
        let user_id = parse_room_moderation_user_id(user_id)?;
        let reason = normalize_moderation_reason(reason);
        room.kick_user(&user_id, reason.as_deref())
            .await
            .map_err(|_| "v-rooms-members-moderation-kick-failed")
    }

    pub async fn ban(
        &self,
        room_id: &str,
        user_id: &str,
        reason: Option<String>,
    ) -> Result<(), &'static str> {
        let room = self.moderation_room(room_id)?;
        let user_id = parse_room_moderation_user_id(user_id)?;
        let reason = normalize_moderation_reason(reason);
        room.ban_user(&user_id, reason.as_deref())
            .await
            .map_err(|_| "v-rooms-members-moderation-ban-failed")
    }

    pub async fn unban(&self, room_id: &str, user_id: &str) -> Result<(), &'static str> {
        let room = self.moderation_room(room_id)?;
        let user_id = parse_room_moderation_user_id(user_id)?;
        room.unban_user(&user_id, None)
            .await
            .map_err(|_| "v-rooms-members-moderation-unban-failed")
    }

    pub async fn set_power_level(
        &self,
        room_id: &str,
        user_id: &str,
        power_level: i64,
    ) -> Result<(), &'static str> {
        let room = self.moderation_room(room_id)?;
        let user_id = parse_room_moderation_user_id(user_id)?;
        let power_level = parse_room_moderation_power_level(power_level)?;
        room.update_power_levels(vec![(&user_id, power_level)])
            .await
            .map(|_| ())
            .map_err(|_| "v-rooms-members-moderation-power-level-failed")
    }

    pub async fn set_power_level_state(
        &self,
        room_id: &str,
        content: serde_json::Value,
        event_type: &str,
    ) -> Result<NativePowerLevelWriteResult, &'static str> {
        if self.retired.load(Ordering::Acquire) {
            return Err("v-send.r-room-profile-join-rule-requires-session");
        }
        let event_type = match event_type {
            ROOM_POWER_LEVELS_EVENT_TYPE => {
                validate_room_power_levels_content(&content)?;
                ROOM_POWER_LEVELS_EVENT_TYPE
            }
            ROOM_POWER_LEVEL_TAGS_EVENT_TYPE => {
                validate_power_level_tags_content(&content)?;
                ROOM_POWER_LEVEL_TAGS_EVENT_TYPE
            }
            _ => return Err("v-rooms-power-levels-invalid-content"),
        };
        let room_id = parse_power_level_room_id(room_id)?;
        let room = self
            .client
            .get_room(&room_id)
            .ok_or("v-rooms-power-levels-room-not-found")?;
        room.send_state_event_raw(event_type, "", content.clone())
            .await
            .map_err(|_| "v-rooms-power-levels-send-failed")?;
        let readback = self
            .client
            .send(get_state_event_for_key::v3::Request::new(
                room_id.clone(),
                StateEventType::from(event_type),
                String::new(),
            ))
            .await
            .map_err(|_| "v-rooms-power-levels-readback-failed")?
            .into_content()
            .deserialize_as_unchecked::<serde_json::Value>()
            .map_err(|_| "v-rooms-power-levels-readback-malformed")?;
        if self.retired.load(Ordering::Acquire) {
            return Err("v-rooms-power-levels-stale-session-generation");
        }
        if readback != content {
            return Err("v-rooms-power-levels-readback-mismatch");
        }
        Ok(NativePowerLevelWriteResult {
            status: "ok",
            room_id: room_id.to_string(),
            event_type,
            state_key: "",
            session_generation: self.session_generation,
            content: readback,
        })
    }

    pub async fn get_directory_visibility(
        &self,
        room_id: &str,
        session_generation: u64,
    ) -> Result<MatrixRoomDirectoryVisibilityResult, &'static str> {
        if self.retired.load(Ordering::Acquire) {
            return Err("v-send.r-room-profile-directory-visibility-requires-session");
        }
        if session_generation == 0 || session_generation != self.session_generation {
            return Err("v-send.r-room-profile-directory-visibility-stale-generation");
        }
        let room_id = parse_directory_visibility_room_id(room_id)?;
        let room = self
            .client
            .get_room(&room_id)
            .ok_or("v-send.r-room-profile-directory-visibility-room-not-found")?;
        let visibility = room
            .privacy_settings()
            .get_room_visibility()
            .await
            .map_err(|_| "v-send.r-room-profile-directory-visibility-get-sdk-failed")?;
        let visibility = match visibility {
            Visibility::Public => "public",
            Visibility::Private => "private",
            _ => return Err("v-send.r-room-profile-directory-visibility-get-sdk-failed"),
        };
        Ok(MatrixRoomDirectoryVisibilityResult {
            status: "ok",
            room_id: room_id.to_string(),
            session_generation: self.session_generation,
            visibility,
        })
    }

    pub async fn set_directory_visibility(
        &self,
        room_id: &str,
        session_generation: u64,
        visibility: &str,
    ) -> Result<MatrixRoomDirectoryVisibilityWriteResult, &'static str> {
        if self.retired.load(Ordering::Acquire) {
            return Err("v-send.r-room-profile-directory-visibility-requires-session");
        }
        if session_generation == 0 || session_generation != self.session_generation {
            return Err("v-send.r-room-profile-directory-visibility-stale-generation");
        }
        let room_id = parse_directory_visibility_room_id(room_id)?;
        let (native_visibility, requested_visibility) = parse_directory_visibility(visibility)?;
        let room = self
            .client
            .get_room(&room_id)
            .ok_or("v-send.r-room-profile-directory-visibility-room-not-found")?;
        let Some(room_version) = room.version() else {
            return Err("v-send.r-room-profile-directory-visibility-permission-state-unavailable");
        };
        if room_version.rules().is_none() {
            return Err("v-send.r-room-profile-directory-visibility-permission-state-unavailable");
        }
        let power_levels = room.power_levels().await.map_err(|_| {
            "v-send.r-room-profile-directory-visibility-permission-state-unavailable"
        })?;
        let user_id = self
            .client
            .user_id()
            .ok_or("v-send.r-room-profile-directory-visibility-permission-state-unavailable")?;
        if !power_levels.user_can_send_state(user_id, StateEventType::RoomCanonicalAlias) {
            return Err("v-send.r-room-profile-directory-visibility-permission-denied");
        }
        room.privacy_settings()
            .update_room_visibility(native_visibility)
            .await
            .map_err(|_| "v-send.r-room-profile-directory-visibility-set-sdk-failed")?;
        Ok(MatrixRoomDirectoryVisibilityWriteResult {
            status: "ok",
            room_id: room_id.to_string(),
            session_generation: self.session_generation,
            requested_visibility,
        })
    }

    fn profile_room(&self, room_id: &str) -> Result<Room, &'static str> {
        let room_id = parse_profile_room_id(room_id)?;
        self.client
            .get_room(&room_id)
            .ok_or("v-send.r-room-profile-room-not-found")
    }

    fn moderation_room(&self, room_id: &str) -> Result<Room, &'static str> {
        if self.retired.load(Ordering::Acquire) {
            return Err("v-send.r-room-profile-join-rule-requires-session");
        }
        let room_id = parse_room_moderation_room_id(room_id)?;
        self.client
            .get_room(&room_id)
            .ok_or("v-rooms-members-moderation-room-not-found")
    }
}

fn parse_join_rule_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    if room_id.is_empty()
        || room_id.len() > 512
        || room_id.trim() != room_id
        || room_id.chars().any(char::is_whitespace)
        || !room_id.starts_with('!')
    {
        return Err("v-send.r-room-profile-join-rule-invalid");
    }
    room_id
        .parse()
        .map_err(|_| "v-send.r-room-profile-join-rule-invalid")
}

fn parse_profile_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    OwnedRoomId::try_from(room_id.trim()).map_err(|_| "d0.4-send-invalid-room-id")
}

fn parse_room_leave_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    room_id
        .trim()
        .parse()
        .map_err(|_| "v-rooms-room-leave-invalid-room")
}

fn parse_room_join_target(room_id_or_alias: &str) -> Result<OwnedRoomOrAliasId, &'static str> {
    room_id_or_alias
        .trim()
        .parse()
        .map_err(|_| "v-rooms-room-join-invalid-room")
}

fn parse_room_moderation_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    room_id
        .trim()
        .parse()
        .map_err(|_| "v-rooms-members-moderation-invalid-room")
}

fn parse_room_moderation_user_id(user_id: &str) -> Result<OwnedUserId, &'static str> {
    user_id
        .trim()
        .parse()
        .map_err(|_| "v-rooms-members-moderation-invalid-user")
}

fn parse_room_moderation_power_level(power_level: i64) -> Result<Int, &'static str> {
    power_level
        .try_into()
        .map_err(|_| "v-rooms-members-moderation-invalid-power-level")
}

fn parse_power_level_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    if room_id.is_empty() || room_id.trim() != room_id {
        return Err("v-rooms-power-levels-invalid-room");
    }
    room_id
        .parse()
        .map_err(|_| "v-rooms-power-levels-invalid-room")
}

fn normalize_moderation_reason(reason: Option<String>) -> Option<String> {
    reason
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn parse_room_join_via_servers(
    via_servers: Option<&[String]>,
) -> Result<Vec<OwnedServerName>, &'static str> {
    via_servers
        .unwrap_or_default()
        .iter()
        .map(|server| {
            server
                .trim()
                .parse()
                .map_err(|_| "v-rooms-room-join-invalid-via-server")
        })
        .collect()
}

fn parse_directory_visibility_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    room_id
        .parse()
        .map_err(|_| "v-send.r-room-profile-directory-visibility-invalid")
}

fn parse_directory_visibility(
    visibility: &str,
) -> Result<(Visibility, &'static str), &'static str> {
    match visibility {
        "public" => Ok((Visibility::Public, "public")),
        "private" => Ok((Visibility::Private, "private")),
        _ => Err("v-send.r-room-profile-directory-visibility-invalid"),
    }
}

fn parse_room_name(name: &str) -> Result<String, &'static str> {
    let trimmed = name.trim();
    if trimmed.chars().count() > 255 {
        return Err("v-send.r-room-profile-name-too-long");
    }
    Ok(trimmed.to_owned())
}

fn parse_room_topic(topic: &str) -> Result<String, &'static str> {
    let trimmed = topic.trim();
    if trimmed.chars().count() > 2_048 {
        return Err("v-send.r-room-profile-topic-too-long");
    }
    Ok(trimmed.to_owned())
}

fn parse_avatar_mxc(mxc: &str) -> Result<Option<OwnedMxcUri>, &'static str> {
    let trimmed = mxc.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if !trimmed.starts_with("mxc://") {
        return Err("v-send.r-avatar-invalid-mxc");
    }
    let owned = OwnedMxcUri::from(trimmed);
    if owned.as_str().matches('/').count() < 3 {
        return Err("v-send.r-avatar-invalid-mxc");
    }
    Ok(Some(owned))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn project_join_rule_is_closed_and_fail_closed() {
        let cases = [
            (JoinRule::Public, Some("public")),
            (JoinRule::Knock, Some("knock")),
            (JoinRule::Invite, Some("invite")),
            (JoinRule::Restricted(Default::default()), Some("restricted")),
            (
                JoinRule::KnockRestricted(Default::default()),
                Some("knock_restricted"),
            ),
            (JoinRule::Private, Some("private")),
        ];
        for (rule, expected) in cases {
            assert_eq!(project_join_rule(&rule), expected);
        }

        let custom = serde_json::from_value::<JoinRule>(json!({
            "join_rule": "org.example.custom"
        }))
        .expect("custom join rule is representable by Ruma");
        assert_eq!(project_join_rule(&custom), None);
    }
}
