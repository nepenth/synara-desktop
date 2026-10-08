//! Room aliases, mutual rooms, room upgrade and bulk redaction.
//!
//! These back renderer flows that previously had no native path: the local
//! alias list in room settings, alias availability while creating a room, the
//! shared-rooms chip on a user profile, room upgrade, and the `/delete` slash
//! command. Errors are closed diagnostic ids.

use std::collections::HashSet;

use matrix_sdk::{
    room::MessagesOptions,
    ruma::{
        api::client::{
            alias::{create_alias, delete_alias},
            room::{aliases, upgrade_room},
        },
        api::error::ErrorKind,
        MilliSecondsSinceUnixEpoch, OwnedEventId, OwnedRoomAliasId, OwnedRoomId, OwnedUserId,
        RoomVersionId, UInt,
    },
    RoomMemberships, RoomState,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::NativeRoomJoinRuleOwner;

/// Rooms whose member lists are checked locally for shared rooms.
const MAX_MUTUAL_ROOM_SCAN: usize = 2_000;
/// `/messages` pages walked by one bulk redaction.
const MAX_BULK_REDACT_PAGES: usize = 50;
/// Redactions sent by one bulk redaction.
const MAX_BULK_REDACT_EVENTS: usize = 500;
const BULK_REDACT_PAGE_SIZE: u32 = 100;
const MAX_REDACT_REASON_LEN: usize = 512;
const MAX_BULK_REDACT_USERS: usize = 100;

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomLocalAliases {
    pub session_generation: u64,
    pub room_id: String,
    pub aliases: Vec<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeRoomAliasAvailability {
    Available,
    Taken,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomAliasCheck {
    pub alias: String,
    pub availability: NativeRoomAliasAvailability,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeMutualRooms {
    pub session_generation: u64,
    pub user_id: String,
    /// Joined rooms where the user is a known joined member, by room id.
    pub room_ids: Vec<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomUpgradeResult {
    pub room_id: String,
    pub replacement_room_id: String,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeBulkRedactResult {
    pub room_id: String,
    /// Events read from history while searching.
    pub scanned: u32,
    pub redacted: u32,
    pub failed: u32,
    /// True when the page or redaction cap stopped the walk early.
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeRoomIdRequest {
    pub room_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeRoomAliasRequest {
    pub alias: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeRoomAliasCreateRequest {
    pub alias: String,
    pub room_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeMutualRoomsRequest {
    pub user_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeRoomUpgradeRequest {
    pub room_id: String,
    pub new_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeBulkRedactRequest {
    pub room_id: String,
    pub user_ids: Vec<String>,
    /// Redact events sent at or after this time (ms since the epoch).
    pub since_ts: u64,
    /// Limit to these event types; empty means every message-like type.
    #[serde(default)]
    pub event_types: Vec<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

fn parse_room(room_id: &str, invalid: &'static str) -> Result<OwnedRoomId, &'static str> {
    room_id.trim().parse().map_err(|_| invalid)
}

fn parse_alias(alias: &str) -> Result<OwnedRoomAliasId, &'static str> {
    alias.trim().parse().map_err(|_| "v-rooms-alias-invalid")
}

/// One history event reduced to what bulk redaction filters on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RedactCandidate {
    pub event_id: OwnedEventId,
    pub sender: String,
    pub event_type: String,
    pub origin_server_ts: u64,
    pub is_state: bool,
    pub already_redacted: bool,
}

pub(crate) fn redact_candidate(value: &JsonValue) -> Option<RedactCandidate> {
    Some(RedactCandidate {
        event_id: value.get("event_id")?.as_str()?.parse().ok()?,
        sender: value.get("sender")?.as_str()?.to_owned(),
        event_type: value.get("type")?.as_str()?.to_owned(),
        origin_server_ts: value.get("origin_server_ts")?.as_u64()?,
        is_state: value.get("state_key").is_some(),
        already_redacted: value
            .get("unsigned")
            .and_then(|unsigned| unsigned.get("redacted_because"))
            .is_some(),
    })
}

/// Whether bulk redaction should remove this event. State events, redactions
/// and already-redacted events are never targets.
pub(crate) fn should_bulk_redact(
    candidate: &RedactCandidate,
    users: &HashSet<String>,
    since_ts: u64,
    event_types: &HashSet<String>,
) -> bool {
    if candidate.is_state
        || candidate.already_redacted
        || candidate.event_type == "m.room.redaction"
        || candidate.origin_server_ts < since_ts
        || !users.contains(&candidate.sender)
    {
        return false;
    }
    event_types.is_empty() || event_types.contains(&candidate.event_type)
}

impl NativeRoomJoinRuleOwner {
    fn ensure_live(&self) -> Result<(), &'static str> {
        if self.retired.load(std::sync::atomic::Ordering::Acquire) {
            return Err("v-send.r-room-profile-join-rule-requires-session");
        }
        Ok(())
    }

    pub async fn local_aliases(
        &self,
        room_id: &str,
    ) -> Result<NativeRoomLocalAliases, &'static str> {
        self.ensure_live()?;
        let room_id = parse_room(room_id, "v-rooms-alias-invalid-room")?;
        let response = self
            .client
            .send(aliases::v3::Request::new(room_id.clone()))
            .await
            .map_err(|_| "v-rooms-alias-list-failed")?;
        let mut aliases: Vec<String> = response.aliases.iter().map(ToString::to_string).collect();
        aliases.sort();
        Ok(NativeRoomLocalAliases {
            session_generation: self.session_generation,
            room_id: room_id.to_string(),
            aliases,
        })
    }

    pub async fn alias_create(&self, alias: &str, room_id: &str) -> Result<(), &'static str> {
        self.ensure_live()?;
        let alias = parse_alias(alias)?;
        let room_id = parse_room(room_id, "v-rooms-alias-invalid-room")?;
        self.client
            .send(create_alias::v3::Request::new(alias, room_id))
            .await
            .map(|_| ())
            .map_err(|error| match error.client_api_error_kind() {
                Some(ErrorKind::Forbidden { .. }) => "v-rooms-alias-forbidden",
                _ if error.as_client_api_error().is_some_and(|api| {
                    api.status_code == matrix_sdk::reqwest::StatusCode::CONFLICT
                }) =>
                {
                    "v-rooms-alias-taken"
                }
                _ => "v-rooms-alias-create-failed",
            })
    }

    pub async fn alias_delete(&self, alias: &str) -> Result<(), &'static str> {
        self.ensure_live()?;
        let alias = parse_alias(alias)?;
        self.client
            .send(delete_alias::v3::Request::new(alias))
            .await
            .map(|_| ())
            .map_err(|error| match error.client_api_error_kind() {
                Some(ErrorKind::Forbidden { .. }) => "v-rooms-alias-forbidden",
                Some(ErrorKind::NotFound) => "v-rooms-alias-not-found",
                _ => "v-rooms-alias-delete-failed",
            })
    }

    /// Available only when the homeserver reports `M_NOT_FOUND`; any other
    /// failure is an error, never "available".
    pub async fn alias_check(&self, alias: &str) -> Result<NativeRoomAliasCheck, &'static str> {
        self.ensure_live()?;
        let parsed = parse_alias(alias)?;
        let availability = match self.client.resolve_room_alias(&parsed).await {
            Ok(_) => NativeRoomAliasAvailability::Taken,
            Err(error) if error.client_api_error_kind() == Some(&ErrorKind::NotFound) => {
                NativeRoomAliasAvailability::Available
            }
            Err(_) => return Err("v-rooms-alias-check-failed"),
        };
        Ok(NativeRoomAliasCheck {
            alias: parsed.to_string(),
            availability,
        })
    }

    /// Joined rooms where `user_id` is a joined member in the local store.
    /// Rooms with lazy-loaded members that have not been loaded may be missed.
    pub async fn mutual_rooms(&self, user_id: &str) -> Result<NativeMutualRooms, &'static str> {
        self.ensure_live()?;
        let user_id: OwnedUserId = user_id
            .trim()
            .parse()
            .map_err(|_| "v-rooms-mutual-invalid-user")?;
        let mut room_ids = Vec::new();
        for room in self
            .client
            .joined_rooms()
            .into_iter()
            .take(MAX_MUTUAL_ROOM_SCAN)
        {
            let member = room
                .get_member_no_sync(&user_id)
                .await
                .map_err(|_| "v-rooms-mutual-failed")?;
            if member.is_some_and(|member| RoomMemberships::JOIN.matches(member.membership())) {
                room_ids.push(room.room_id().to_string());
            }
        }
        room_ids.sort();
        Ok(NativeMutualRooms {
            session_generation: self.session_generation,
            user_id: user_id.to_string(),
            room_ids,
        })
    }

    /// Upgrade a joined room. Requires the power to send `m.room.tombstone`.
    pub async fn upgrade_room(
        &self,
        room_id: &str,
        new_version: &str,
    ) -> Result<NativeRoomUpgradeResult, &'static str> {
        self.ensure_live()?;
        let room_id = parse_room(room_id, "v-rooms-upgrade-invalid-room")?;
        let version = new_version.trim();
        if version.is_empty()
            || version.len() > 32
            || !version.bytes().all(|b| b.is_ascii_graphic())
        {
            return Err("v-rooms-upgrade-invalid-version");
        }
        let version =
            RoomVersionId::try_from(version).map_err(|_| "v-rooms-upgrade-invalid-version")?;
        let room = self
            .client
            .get_room(&room_id)
            .ok_or("v-rooms-upgrade-room-not-found")?;
        if room.state() != RoomState::Joined {
            return Err("v-rooms-upgrade-not-joined");
        }
        let user_id = self.client.user_id().ok_or("v-rooms-upgrade-failed")?;
        let levels = room
            .power_levels()
            .await
            .map_err(|_| "v-rooms-upgrade-failed")?;
        if !crate::app::members::room_permission_capabilities(&levels, user_id).can_upgrade_room {
            return Err("v-rooms-upgrade-forbidden");
        }
        let response = self
            .client
            .send(upgrade_room::v3::Request::new(room_id.clone(), version))
            .await
            .map_err(|error| match error.client_api_error_kind() {
                Some(ErrorKind::Forbidden { .. }) => "v-rooms-upgrade-forbidden",
                Some(ErrorKind::UnsupportedRoomVersion) => "v-rooms-upgrade-unsupported-version",
                _ => "v-rooms-upgrade-failed",
            })?;
        Ok(NativeRoomUpgradeResult {
            room_id: room_id.to_string(),
            replacement_room_id: response.replacement_room.to_string(),
        })
    }

    /// Walk history backwards and redact matching events from `user_ids`
    /// newer than `since_ts`. Bounded by page and redaction caps.
    pub async fn bulk_redact(
        &self,
        request: NativeBulkRedactRequest,
    ) -> Result<NativeBulkRedactResult, &'static str> {
        self.ensure_live()?;
        let room_id = parse_room(&request.room_id, "v-rooms-bulk-redact-invalid-room")?;
        if request.user_ids.is_empty() || request.user_ids.len() > MAX_BULK_REDACT_USERS {
            return Err("v-rooms-bulk-redact-invalid-users");
        }
        let mut users = HashSet::new();
        for user in &request.user_ids {
            let user: OwnedUserId = user
                .trim()
                .parse()
                .map_err(|_| "v-rooms-bulk-redact-invalid-users")?;
            users.insert(user.to_string());
        }
        let event_types: HashSet<String> = request.event_types.iter().cloned().collect();
        let reason = request
            .reason
            .as_deref()
            .map(str::trim)
            .filter(|reason| !reason.is_empty());
        if reason.is_some_and(|reason| reason.len() > MAX_REDACT_REASON_LEN) {
            return Err("v-rooms-bulk-redact-invalid-reason");
        }
        let now = MilliSecondsSinceUnixEpoch::now().get().into();
        if request.since_ts > now {
            return Err("v-rooms-bulk-redact-invalid-since");
        }
        let room = self
            .client
            .get_room(&room_id)
            .ok_or("v-rooms-bulk-redact-room-not-found")?;
        if room.state() != RoomState::Joined {
            return Err("v-rooms-bulk-redact-not-joined");
        }

        let mut result = NativeBulkRedactResult {
            room_id: room_id.to_string(),
            scanned: 0,
            redacted: 0,
            failed: 0,
            truncated: false,
        };
        let mut from: Option<String> = None;
        'pages: for page in 0..MAX_BULK_REDACT_PAGES {
            let mut options = MessagesOptions::backward();
            options.limit = UInt::from(BULK_REDACT_PAGE_SIZE);
            options.from = from.clone();
            let messages = room
                .messages(options)
                .await
                .map_err(|_| "v-rooms-bulk-redact-history-failed")?;
            let mut reached_start = false;
            for event in &messages.chunk {
                result.scanned = result.scanned.saturating_add(1);
                let Ok(value) = event.raw().deserialize_as_unchecked::<JsonValue>() else {
                    continue;
                };
                let Some(candidate) = redact_candidate(&value) else {
                    continue;
                };
                if candidate.origin_server_ts < request.since_ts {
                    reached_start = true;
                    continue;
                }
                if !should_bulk_redact(&candidate, &users, request.since_ts, &event_types) {
                    continue;
                }
                if result.redacted + result.failed >= MAX_BULK_REDACT_EVENTS as u32 {
                    result.truncated = true;
                    break 'pages;
                }
                match room.redact(&candidate.event_id, reason, None).await {
                    Ok(_) => result.redacted += 1,
                    Err(_) => result.failed += 1,
                }
            }
            match messages.end {
                Some(end) if !reached_start && !messages.chunk.is_empty() => from = Some(end),
                _ => break,
            }
            if page + 1 == MAX_BULK_REDACT_PAGES {
                result.truncated = true;
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(sender: &str, kind: &str, ts: u64) -> JsonValue {
        json!({
            "event_id": "$e:example.org",
            "sender": sender,
            "type": kind,
            "origin_server_ts": ts,
            "content": {}
        })
    }

    #[test]
    fn bulk_redact_targets_only_recent_messages_from_listed_users() {
        let users: HashSet<String> = ["@spam:example.org".to_owned()].into_iter().collect();
        let none = HashSet::new();
        let keep = |value: JsonValue| {
            should_bulk_redact(&redact_candidate(&value).unwrap(), &users, 1_000, &none)
        };
        assert!(keep(event("@spam:example.org", "m.room.message", 2_000)));
        assert!(keep(event("@spam:example.org", "m.reaction", 2_000)));
        assert!(!keep(event("@ok:example.org", "m.room.message", 2_000)));
        assert!(!keep(event("@spam:example.org", "m.room.message", 999)));
        assert!(!keep(event("@spam:example.org", "m.room.redaction", 2_000)));

        let mut state = event("@spam:example.org", "m.room.topic", 2_000);
        state["state_key"] = json!("");
        assert!(!keep(state));

        let mut redacted = event("@spam:example.org", "m.room.message", 2_000);
        redacted["unsigned"] = json!({ "redacted_because": {} });
        assert!(!keep(redacted));
    }

    #[test]
    fn bulk_redact_type_filter_limits_targets() {
        let users: HashSet<String> = ["@spam:example.org".to_owned()].into_iter().collect();
        let types: HashSet<String> = ["m.room.message".to_owned()].into_iter().collect();
        let message = redact_candidate(&event("@spam:example.org", "m.room.message", 5)).unwrap();
        let reaction = redact_candidate(&event("@spam:example.org", "m.reaction", 5)).unwrap();
        assert!(should_bulk_redact(&message, &users, 0, &types));
        assert!(!should_bulk_redact(&reaction, &users, 0, &types));
    }

    #[test]
    fn malformed_history_events_are_skipped() {
        assert!(redact_candidate(&json!({ "type": "m.room.message" })).is_none());
        assert!(redact_candidate(&json!({
            "event_id": "not-an-id", "sender": "@a:b", "type": "t", "origin_server_ts": 1
        }))
        .is_none());
    }
}
