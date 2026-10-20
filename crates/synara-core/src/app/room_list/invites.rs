//! Native V-ROOMS.1 invite triage primitives.
//!
//! The spam classifier deliberately carries the same word corpus and boundary
//! semantics as the desktop's former `badwords-list` JavaScript owner. A
//! substitute profanity list would silently change an invite-safety decision.

use std::{collections::BTreeSet, sync::OnceLock};

use matrix_sdk::{
    deserialized_responses::RawSyncOrStrippedState,
    ruma::{
        events::{
            ignored_user_list::IgnoredUserListEventContent,
            room::{member::MembershipState, topic::RoomTopicEventContent},
        },
        OwnedUserId, UserId,
    },
    Client, Room, RoomState,
};
use serde::{Deserialize, Serialize};

use super::{dm_avatar_source, live::room_encryption_state, InviteAvatarHandles};

const BAD_WORDS_JSON: &str = include_str!("invite_bad_words.json");
const SYNARA_BAD_WORD_ADDITIONS: &[&str] = &["torture", "t0rture"];

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeInviteTriage {
    Known,
    Public,
    Spam,
}

impl NativeInviteTriage {
    pub const ALL: &'static [Self] = &[Self::Known, Self::Public, Self::Spam];
}

/// Bounded, SDK-neutral invite data. The later live projection must populate
/// each field without giving the webview an SDK `Room` or raw member event.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeInvite {
    pub room_id: String,
    pub room_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub avatar_handle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub room_topic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub room_alias: Option<String>,
    pub sender_id: String,
    pub sender_name: String,
    pub sender_ignored: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub invite_ts: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub reason: Option<String>,
    pub is_space: bool,
    pub is_direct: bool,
    pub is_encrypted: bool,
    pub triage: NativeInviteTriage,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeInviteSnapshot {
    pub session_generation: u64,
    pub invites: Vec<NativeInvite>,
}

/// Projects the locally-known invited-room state without a membership-sync
/// network fan-out. This intentionally matches the current JS inbox, which
/// classifies using the already loaded Matrix room cache.
pub async fn snapshot_invites(
    client: &Client,
    session_generation: u64,
    avatar_handles: &mut InviteAvatarHandles,
) -> Result<NativeInviteSnapshot, &'static str> {
    let current_user = client
        .user_id()
        .ok_or("v-rooms.1-invites-requires-session")?;
    let rooms = client.rooms();
    let joined_rooms: Vec<Room> = rooms
        .iter()
        .filter(|room| room.state() == RoomState::Joined)
        .cloned()
        .collect();
    // Account data is retrieved from the SDK state store. Do this once per
    // snapshot so ignore status remains a local projection, not an invite-card
    // network lookup.
    let ignored_senders: BTreeSet<OwnedUserId> = client
        .account()
        .account_data::<IgnoredUserListEventContent>()
        .await
        .ok()
        .flatten()
        .and_then(|raw| raw.deserialize().ok())
        .map(|content| content.ignored_users.into_keys().collect())
        .unwrap_or_default();
    let mut invites = Vec::new();
    for room in rooms
        .into_iter()
        .filter(|room| room.state() == RoomState::Invited)
    {
        // One unreadable invite must not hide the others: the shells turn a
        // failed snapshot into "no invites", so a room whose stripped state
        // lacks our own member event still gets a minimal, actionable row.
        let invite = match project_invite(
            &room,
            current_user,
            &joined_rooms,
            &ignored_senders,
            avatar_handles,
        )
        .await
        {
            Ok(invite) => invite,
            Err(_) => degraded_invite(&room).await,
        };
        invites.push(invite);
    }
    invites.sort_by(|left, right| left.room_id.cmp(&right.room_id));
    Ok(NativeInviteSnapshot {
        session_generation,
        invites,
    })
}

async fn project_invite(
    room: &Room,
    current_user: &UserId,
    joined_rooms: &[Room],
    ignored_senders: &BTreeSet<OwnedUserId>,
    avatar_handles: &mut InviteAvatarHandles,
) -> Result<NativeInvite, &'static str> {
    let own_member = room
        .get_member_no_sync(current_user)
        .await
        .map_err(|_| "v-rooms.1-invite-member-read-failed")?
        .ok_or("v-rooms.1-invite-member-missing")?;
    let member_event = own_member.event();
    let sender_id = member_event.sender().to_string();
    let sender_ignored = ignored_senders.contains(member_event.sender());
    let sender_name = room
        .get_member_no_sync(member_event.sender())
        .await
        .ok()
        .flatten()
        .map(|member| member.name().to_owned())
        .unwrap_or_else(|| member_event.sender().localpart().to_owned());
    let room_name = invite_room_name(room);
    let room_topic = room_topic(room).await;
    let is_spam = contains_bad_word(&room_name)
        || room_topic.as_deref().is_some_and(contains_bad_word)
        || contains_bad_word(&sender_id)
        || contains_bad_word(&sender_name)
        || member_event.reason().is_some_and(contains_bad_word)
        || sender_is_banned_in_joined_room(joined_rooms, member_event.sender()).await;
    let triage = if is_spam {
        NativeInviteTriage::Spam
    } else if sender_shares_joined_room(joined_rooms, member_event.sender()).await {
        NativeInviteTriage::Known
    } else {
        NativeInviteTriage::Public
    };

    let is_direct = room.is_direct().await.unwrap_or(false);
    let avatar_handle_id = dm_avatar_source(room, current_user, is_direct)
        .await
        .and_then(|mxc_uri| avatar_handles.issue(room.room_id().as_str(), mxc_uri).ok());

    Ok(NativeInvite {
        room_id: room.room_id().to_string(),
        room_name,
        avatar_handle_id,
        room_topic,
        room_alias: room.canonical_alias().map(|alias| alias.to_string()),
        sender_id,
        sender_name,
        sender_ignored,
        invite_ts: member_event.timestamp().map(Into::into),
        reason: member_event.reason().map(ToOwned::to_owned),
        is_space: room.is_space(),
        is_direct,
        is_encrypted: invite_is_encrypted(room).await,
        triage,
    })
}

/// Minimal row for an invite whose own member event cannot be read. The
/// sender is unknown, so `sender_id` and `sender_name` are empty: accept and
/// decline still work, blocking the sender fails closed, and triage never
/// claims a known sender.
async fn degraded_invite(room: &Room) -> NativeInvite {
    let room_name = invite_room_name(room);
    let room_topic = room_topic(room).await;
    let triage =
        if contains_bad_word(&room_name) || room_topic.as_deref().is_some_and(contains_bad_word) {
            NativeInviteTriage::Spam
        } else {
            NativeInviteTriage::Public
        };
    NativeInvite {
        room_id: room.room_id().to_string(),
        room_name,
        avatar_handle_id: None,
        room_topic,
        room_alias: room.canonical_alias().map(|alias| alias.to_string()),
        sender_id: String::new(),
        sender_name: String::new(),
        sender_ignored: false,
        invite_ts: None,
        reason: None,
        is_space: room.is_space(),
        is_direct: room.is_direct().await.unwrap_or(false),
        is_encrypted: invite_is_encrypted(room).await,
        triage,
    }
}

fn invite_room_name(room: &Room) -> String {
    room.cached_display_name()
        .map(|name| name.to_string())
        .or_else(|| room.canonical_alias().map(|alias| alias.to_string()))
        .unwrap_or_else(|| room.room_id().to_string())
}

/// Uses the room list's throttled probe: an invite whose stripped state has
/// no `m.room.encryption` event would otherwise cost one
/// `GET /state/m.room.encryption` per invite on every room-list refresh.
async fn invite_is_encrypted(room: &Room) -> bool {
    room_encryption_state(room)
        .await
        .map(|state| state.is_encrypted())
        .unwrap_or(false)
}

async fn room_topic(room: &Room) -> Option<String> {
    let state = room
        .get_state_event_static::<RoomTopicEventContent>()
        .await
        .ok()??;
    match state {
        RawSyncOrStrippedState::Sync(raw) => raw
            .deserialize()
            .ok()?
            .as_original()
            .map(|event| event.content.topic.clone()),
        RawSyncOrStrippedState::Stripped(raw) => {
            raw.deserialize().ok().and_then(|event| event.content.topic)
        }
    }
}

async fn sender_shares_joined_room(joined_rooms: &[Room], sender: &UserId) -> bool {
    for room in joined_rooms {
        if let Ok(Some(member)) = room.get_member_no_sync(sender).await {
            if *member.membership() == MembershipState::Join {
                return true;
            }
        }
    }
    false
}

async fn sender_is_banned_in_joined_room(joined_rooms: &[Room], sender: &UserId) -> bool {
    for room in joined_rooms {
        if let Ok(Some(member)) = room.get_member_no_sync(sender).await {
            if *member.membership() == MembershipState::Ban {
                return true;
            }
        }
    }
    false
}

fn bad_words() -> &'static Vec<String> {
    static WORDS: OnceLock<Vec<String>> = OnceLock::new();
    WORDS.get_or_init(|| {
        let mut words: Vec<String> = serde_json::from_str::<Vec<String>>(BAD_WORDS_JSON)
            .expect("the vendored badwords-list JSON must stay valid")
            .into_iter()
            .map(|word: String| word.to_lowercase())
            .collect();
        for word in SYNARA_BAD_WORD_ADDITIONS {
            if !words.iter().any(|existing| existing == word) {
                words.push((*word).to_owned());
            }
        }
        words
    })
}

/// Matches the existing JavaScript `/(\\b|_)word(\\b|_)/g` policy using JS's
/// ASCII definition of a word character. Values are literals, never regexes.
pub fn contains_bad_word(value: &str) -> bool {
    let normalized = value.to_lowercase();
    bad_words().iter().any(|word| {
        normalized
            .match_indices(word)
            .any(|(start, _)| has_js_word_boundaries(&normalized, start, word))
    })
}

fn has_js_word_boundaries(value: &str, start: usize, word: &str) -> bool {
    let before = value[..start].chars().next_back();
    let after = value[start + word.len()..].chars().next();
    let first = word.chars().next().expect("vendored words cannot be empty");
    let last = word
        .chars()
        .next_back()
        .expect("vendored words cannot be empty");
    (before == Some('_') || is_js_boundary(before, Some(first)))
        && (after == Some('_') || is_js_boundary(Some(last), after))
}

fn is_js_boundary(left: Option<char>, right: Option<char>) -> bool {
    left.is_some_and(is_js_word_char) != right.is_some_and(is_js_word_char)
}

fn is_js_word_char(value: char) -> bool {
    value.is_ascii_alphanumeric() || value == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_and_synara_additions_match_at_js_word_boundaries() {
        assert_eq!(bad_words().len(), 452);
        assert!(contains_bad_word("a_bitch_b"));
        assert!(contains_bad_word("T0RTURE in an invite reason"));
        assert!(!contains_bad_word("ambitious"));
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;
    use matrix_sdk::ruma::{events::AnyStrippedStateEvent, room_id, serde::Raw};
    use matrix_sdk::test_utils::mocks::MatrixMockServer;
    use matrix_sdk_test::InvitedRoomBuilder;

    fn stripped(event: serde_json::Value) -> Raw<AnyStrippedStateEvent> {
        serde_json::from_value(event).expect("stripped state event")
    }

    fn room_name(name: &str) -> Raw<AnyStrippedStateEvent> {
        stripped(serde_json::json!({
            "type": "m.room.name",
            "state_key": "",
            "sender": "@eve:example.org",
            "content": { "name": name }
        }))
    }

    #[tokio::test]
    async fn an_invite_without_own_member_event_degrades_instead_of_failing_the_list() {
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        let own_user = client.user_id().expect("logged in").to_owned();
        let complete = room_id!("!invites-complete:example.org");
        let missing_member = room_id!("!invites-missing-member:example.org");
        server
            .sync_room(
                &client,
                InvitedRoomBuilder::new(complete)
                    .add_state_event(room_name("Launch Party"))
                    .add_state_event(stripped(serde_json::json!({
                        "type": "m.room.member",
                        "state_key": own_user,
                        "sender": "@eve:example.org",
                        "content": { "membership": "invite", "reason": "Come along" }
                    }))),
            )
            .await;
        server
            .sync_room(
                &client,
                InvitedRoomBuilder::new(missing_member).add_state_event(room_name("Orphan")),
            )
            .await;

        let mut handles = InviteAvatarHandles::new(7);
        let snapshot = snapshot_invites(&client, 7, &mut handles)
            .await
            .expect("one unreadable invite must not fail the snapshot");
        assert_eq!(snapshot.invites.len(), 2);

        let complete = snapshot
            .invites
            .iter()
            .find(|invite| invite.room_id == complete.as_str())
            .expect("complete invite");
        assert_eq!(complete.room_name, "Launch Party");
        assert_eq!(complete.sender_id, "@eve:example.org");
        assert_eq!(complete.reason.as_deref(), Some("Come along"));

        let degraded = snapshot
            .invites
            .iter()
            .find(|invite| invite.room_id == missing_member.as_str())
            .expect("degraded invite stays visible");
        assert_eq!(degraded.room_name, "Orphan");
        assert!(degraded.sender_id.is_empty());
        assert!(degraded.sender_name.is_empty());
        assert_eq!(degraded.triage, NativeInviteTriage::Public);

        // Unknown encryption state is probed at most once per room across
        // refreshes, not once per invite per snapshot.
        snapshot_invites(&client, 7, &mut handles)
            .await
            .expect("second snapshot");
        let probes = server
            .server()
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .filter(|request| request.url.path().contains("/state/m.room.encryption"))
            .count();
        assert!(probes <= 2, "encryption probes are throttled, saw {probes}");
    }
}
