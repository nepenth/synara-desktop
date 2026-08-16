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
        OwnedMxcUri, OwnedUserId, UserId,
    },
    Client, Room, RoomMemberships, RoomState,
};
use serde::{Deserialize, Serialize};

use super::InviteAvatarHandles;

const BAD_WORDS_JSON: &str = include_str!("invite_bad_words.json");
const SYNARA_BAD_WORD_ADDITIONS: &[&str] = &["torture", "t0rture"];

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeInvite {
    pub room_id: String,
    pub room_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_handle_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_topic: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_alias: Option<String>,
    pub sender_id: String,
    pub sender_name: String,
    pub sender_ignored: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invite_ts: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub is_space: bool,
    pub is_direct: bool,
    pub is_encrypted: bool,
    pub triage: NativeInviteTriage,
}

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
        invites.push(
            project_invite(
                &room,
                current_user,
                &joined_rooms,
                &ignored_senders,
                avatar_handles,
            )
            .await?,
        );
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
        .map_err(|_| "v-rooms.1-invite-sender-read-failed")?
        .map(|member| member.name().to_owned())
        .unwrap_or_else(|| member_event.sender().localpart().to_owned());
    let room_name = room
        .cached_display_name()
        .map(|name| name.to_string())
        .or_else(|| room.canonical_alias().map(|alias| alias.to_string()))
        .unwrap_or_else(|| room.room_id().to_string());
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
    let avatar_handle_id = invite_avatar_source(room, current_user, is_direct)
        .await
        .map(|mxc_uri| avatar_handles.issue(room.room_id().as_str(), mxc_uri))
        .transpose()?;

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
        is_encrypted: room
            .latest_encryption_state()
            .await
            .map(|state| state.is_encrypted())
            .unwrap_or(false),
        triage,
    })
}

/// Match the current direct-room avatar selection without synchronizing room
/// members: use a direct room's non-service heroes first, then its cached
/// members when it is a two-party conversation, finally its room avatar.
async fn invite_avatar_source(
    room: &Room,
    current_user: &UserId,
    is_direct: bool,
) -> Option<OwnedMxcUri> {
    let room_avatar = room.avatar_url();
    if !is_direct {
        return room_avatar;
    }

    let service_members = room.service_members().unwrap_or_default();
    let Ok(active_members) = room.members_no_sync(RoomMemberships::ACTIVE).await else {
        return room_avatar;
    };
    let active_non_service_count = active_members
        .iter()
        .filter(|member| !service_members.contains(member.user_id()))
        .count();
    if active_non_service_count > 2 {
        return room_avatar;
    }

    for hero in room.heroes() {
        if hero.avatar_url.is_some() || hero.display_name.is_some() {
            return hero.avatar_url;
        }
        if let Ok(Some(member)) = room.get_member_no_sync(&hero.user_id).await {
            return member.avatar_url().map(ToOwned::to_owned);
        }
    }

    let Ok(members) = room.members_no_sync(RoomMemberships::empty()).await else {
        return room_avatar;
    };
    let non_service_members: Vec<_> = members
        .into_iter()
        .filter(|member| !service_members.contains(member.user_id()))
        .collect();
    if non_service_members.len() <= 2 {
        if let Some(member) = non_service_members
            .into_iter()
            .find(|member| member.user_id() != current_user)
        {
            return member.avatar_url().map(ToOwned::to_owned).or(room_avatar);
        }
    }
    room_avatar
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
