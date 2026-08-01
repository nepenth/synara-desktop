//! V-SEND.R-PACK-READ — native read projection for im.ponies image packs.
//!
//! Snapshot-only (no push subscribe in this slice). Live pack mutation remains
//! V-SEND.R-PACK-WRITE. Pack media bytes remain media/timeline vertical.

use std::collections::BTreeMap;

use matrix_sdk::{
    deserialized_responses::RawAnySyncOrStrippedState,
    ruma::{
        events::{GlobalAccountDataEventType, StateEventType},
        OwnedRoomId, RoomId,
    },
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

pub const USER_EMOTES_EVENT_TYPE: &str = "im.ponies.user_emotes";
pub const EMOTE_ROOMS_EVENT_TYPE: &str = "im.ponies.emote_rooms";
pub const ROOM_EMOTES_EVENT_TYPE: &str = "im.ponies.room_emotes";

/// Privacy-safe pack DTO (no secrets). Content is the ponies pack JSON body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeImagePack {
    /// Stable id: user id for personal pack, event id for room state packs.
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_key: Option<String>,
    /// Raw pack content (`pack` + `images` keys per MSC2545-style ponies packs).
    pub content: JsonValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeUserImagePackSnapshot {
    pub session_generation: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pack: Option<NativeImagePack>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomImagePacksSnapshot {
    pub session_generation: u64,
    pub room_id: String,
    pub packs: Vec<NativeImagePack>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeGlobalImagePacksSnapshot {
    pub session_generation: u64,
    pub packs: Vec<NativeImagePack>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct EmoteRoomsContent {
    #[serde(default)]
    rooms: BTreeMap<String, BTreeMap<String, JsonValue>>,
}

fn user_emotes_type() -> GlobalAccountDataEventType {
    GlobalAccountDataEventType::from(USER_EMOTES_EVENT_TYPE)
}

fn emote_rooms_type() -> GlobalAccountDataEventType {
    GlobalAccountDataEventType::from(EMOTE_ROOMS_EVENT_TYPE)
}

fn room_emotes_type() -> StateEventType {
    StateEventType::from(ROOM_EMOTES_EVENT_TYPE)
}

fn parse_room_id(room_id: &str) -> Result<OwnedRoomId, &'static str> {
    RoomId::parse(room_id).map_err(|_| "v-send.r-pack-read-invalid-room")
}

async fn load_account_data_value(
    client: &Client,
    event_type: GlobalAccountDataEventType,
) -> Result<Option<JsonValue>, &'static str> {
    let raw = client
        .account()
        .account_data_raw(event_type)
        .await
        .map_err(|_| "v-send.r-pack-read-fetch-failed")?;
    match raw {
        Some(raw) => raw
            .deserialize_as_unchecked::<JsonValue>()
            .map(Some)
            .map_err(|_| "v-send.r-pack-read-deserialize-failed"),
        None => Ok(None),
    }
}

fn pack_from_account_data(id: String, content: JsonValue) -> NativeImagePack {
    NativeImagePack {
        id,
        room_id: None,
        state_key: None,
        content,
    }
}

fn extract_sync_state_pack(
    room_id: &str,
    raw: &RawAnySyncOrStrippedState,
) -> Option<NativeImagePack> {
    let RawAnySyncOrStrippedState::Sync(raw_ev) = raw else {
        return None;
    };
    let value: JsonValue = raw_ev.deserialize_as_unchecked().ok()?;
    let state_key = value.get("state_key")?.as_str()?.to_owned();
    let event_id = value
        .get("event_id")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{room_id}:{state_key}"));
    let content = value.get("content")?.clone();
    if !content.is_object() {
        return None;
    }
    Some(NativeImagePack {
        id: event_id,
        room_id: Some(room_id.to_owned()),
        state_key: Some(state_key),
        content,
    })
}

async fn load_room_packs(
    client: &Client,
    room_id: &RoomId,
) -> Result<Vec<NativeImagePack>, &'static str> {
    let room = client
        .get_room(room_id)
        .ok_or("v-send.r-pack-read-room-missing")?;
    let raw_events = room
        .get_state_events(room_emotes_type())
        .await
        .map_err(|_| "v-send.r-pack-read-state-fetch-failed")?;
    let mut packs = Vec::new();
    for raw in raw_events {
        if let Some(pack) = extract_sync_state_pack(room_id.as_str(), &raw) {
            packs.push(pack);
        }
    }
    packs.sort_by(|a, b| a.state_key.cmp(&b.state_key));
    Ok(packs)
}

pub async fn snapshot_user_image_pack(
    client: &Client,
    session_generation: u64,
) -> Result<NativeUserImagePackSnapshot, &'static str> {
    let user_id = client
        .user_id()
        .ok_or("v-send.r-pack-read-no-user")?
        .to_string();
    let content = load_account_data_value(client, user_emotes_type()).await?;
    let pack = content.map(|c| pack_from_account_data(user_id, c));
    Ok(NativeUserImagePackSnapshot {
        session_generation,
        pack,
    })
}

pub async fn snapshot_room_image_packs(
    client: &Client,
    session_generation: u64,
    room_id: &str,
) -> Result<NativeRoomImagePacksSnapshot, &'static str> {
    let room_id = parse_room_id(room_id)?;
    let packs = load_room_packs(client, &room_id).await?;
    Ok(NativeRoomImagePacksSnapshot {
        session_generation,
        room_id: room_id.to_string(),
        packs,
    })
}

pub async fn snapshot_global_image_packs(
    client: &Client,
    session_generation: u64,
) -> Result<NativeGlobalImagePacksSnapshot, &'static str> {
    let raw = load_account_data_value(client, emote_rooms_type()).await?;
    let emote_rooms: EmoteRoomsContent = match raw {
        Some(value) => {
            serde_json::from_value(value).map_err(|_| "v-send.r-pack-read-deserialize-failed")?
        }
        None => EmoteRoomsContent::default(),
    };

    let mut packs = Vec::new();
    for (room_id_str, enabled_keys) in emote_rooms.rooms {
        let Ok(room_id) = RoomId::parse(&room_id_str) else {
            continue;
        };
        if client.get_room(&room_id).is_none() {
            continue;
        }
        let room_packs = match load_room_packs(client, &room_id).await {
            Ok(p) => p,
            Err(_) => continue,
        };
        for pack in room_packs {
            let Some(state_key) = pack.state_key.as_deref() else {
                continue;
            };
            if enabled_keys.contains_key(state_key) {
                packs.push(pack);
            }
        }
    }
    packs.sort_by(|a, b| {
        (
            a.room_id.as_deref().unwrap_or(""),
            a.state_key.as_deref().unwrap_or(""),
        )
            .cmp(&(
                b.room_id.as_deref().unwrap_or(""),
                b.state_key.as_deref().unwrap_or(""),
            ))
    });
    Ok(NativeGlobalImagePacksSnapshot {
        session_generation,
        packs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pack_from_account_data_preserves_content() {
        let content = json!({
            "pack": { "display_name": "Me", "usage": ["emoticon", "sticker"] },
            "images": {
                "smile": { "url": "mxc://example.org/abc", "body": ":)" }
            }
        });
        let pack = pack_from_account_data("@u:example.org".into(), content.clone());
        assert_eq!(pack.id, "@u:example.org");
        assert!(pack.room_id.is_none());
        assert_eq!(pack.content, content);
    }

    #[test]
    fn emote_rooms_content_parses_nested_keys() {
        let value = json!({
            "rooms": {
                "!r:example.org": {
                    "": {},
                    "extra": {}
                }
            }
        });
        let parsed: EmoteRoomsContent = serde_json::from_value(value).unwrap();
        assert_eq!(parsed.rooms.get("!r:example.org").unwrap().len(), 2);
        assert!(parsed.rooms.get("!r:example.org").unwrap().contains_key(""));
    }

    #[test]
    fn invalid_room_id_is_privacy_safe_diagnostic() {
        let err = parse_room_id("not-a-room").unwrap_err();
        assert_eq!(err, "v-send.r-pack-read-invalid-room");
        assert!(!err.contains('@'));
        assert!(!err.contains('!'));
    }
}
