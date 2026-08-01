//! V-SEND.R-PACK-READ pack projection: parse Ponies* content into a DTO.
//!
//! Pure parse helpers are unit-tested against representative JSON. The async
//! snapshot functions read the live `matrix_sdk::Client` account-data / state
//! events and are fail-closed (return `Err` on any read/deserialize failure).

use std::collections::BTreeMap;

use matrix_sdk::{
    ruma::{
        events::{GlobalAccountDataEventType, StateEventType},
        OwnedRoomId,
    },
    Client, Room, RoomState,
};
use serde::Serialize;

/// `im.ponies.user_emotes` — personal pack account-data event type.
pub const PONIES_USER_EMOTES: &str = "im.ponies.user_emotes";
/// `im.ponies.emote_rooms` — enabled global pack rooms account-data event type.
pub const PONIES_EMOTE_ROOMS: &str = "im.ponies.emote_rooms";
/// `im.ponies.room_emotes` — per-room pack state event type.
pub const PONIES_ROOM_EMOTES: &str = "im.ponies.room_emotes";

const USAGE_EMOTICON: &str = "emoticon";
const USAGE_STICKER: &str = "sticker";

/// A single image pack projection (mirrors the frontend `ImagePack` shape).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeImagePack {
    pub id: String,
    pub deleted: bool,
    pub address: Option<NativeImagePackAddress>,
    pub meta: NativeImagePackMeta,
    pub images: BTreeMap<String, NativeImagePackImage>,
}

/// Pack origin (room + state key). `None` for the personal user pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeImagePackAddress {
    pub room_id: String,
    pub state_key: String,
}

/// Pack metadata (`pack` field of the content).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeImagePackMeta {
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub attribution: Option<String>,
    pub usage: Vec<String>,
}

impl Default for NativeImagePackMeta {
    fn default() -> Self {
        Self {
            name: None,
            avatar: None,
            attribution: None,
            usage: vec![USAGE_EMOTICON.to_owned(), USAGE_STICKER.to_owned()],
        }
    }
}

/// One pack image (`images` map value).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeImagePackImage {
    pub url: String,
    pub body: Option<String>,
    pub usage: Option<Vec<String>>,
    pub info: Option<serde_json::Value>,
}

/// Session-generation-stamped pack snapshot returned over IPC.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeImagePackSnapshot {
    pub session_generation: u64,
    pub packs: Vec<NativeImagePack>,
}

fn ponies_user_emotes_type() -> GlobalAccountDataEventType {
    GlobalAccountDataEventType::from(PONIES_USER_EMOTES)
}

fn ponies_emote_rooms_type() -> GlobalAccountDataEventType {
    GlobalAccountDataEventType::from(PONIES_EMOTE_ROOMS)
}

fn ponies_room_emotes_type() -> StateEventType {
    StateEventType::from(PONIES_ROOM_EMOTES)
}

/// Normalize a usage list to known values (`emoticon` / `sticker`).
fn normalize_usage(value: Option<&serde_json::Value>) -> Option<Vec<String>> {
    let arr = value?.as_array()?;
    let mut out = Vec::new();
    for item in arr {
        match item.as_str() {
            Some(USAGE_EMOTICON) | Some(USAGE_STICKER) => {
                if !out.contains(&item.as_str().unwrap().to_owned()) {
                    out.push(item.as_str().unwrap().to_owned());
                }
            }
            _ => {}
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Parse one pack image value into a DTO. Returns `None` if the URL is missing.
fn parse_image(value: &serde_json::Value) -> Option<NativeImagePackImage> {
    let url = value.get("url")?.as_str()?.to_owned();
    if url.is_empty() {
        return None;
    }
    let body = value
        .get("body")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .filter(|b| !b.is_empty());
    let usage = normalize_usage(value.get("usage"));
    let info = value.get("info").cloned().filter(|v| !v.is_null());
    Some(NativeImagePackImage {
        url,
        body,
        usage,
        info,
    })
}

/// Parse the `pack` metadata object into a DTO.
fn parse_meta(value: Option<&serde_json::Value>) -> NativeImagePackMeta {
    let Some(meta) = value else {
        return NativeImagePackMeta::default();
    };
    let name = meta
        .get("display_name")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .filter(|s| !s.is_empty());
    let avatar = meta
        .get("avatar_url")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .filter(|s| !s.is_empty());
    let attribution = meta
        .get("attribution")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .filter(|s| !s.is_empty());
    let usage = normalize_usage(meta.get("usage")).unwrap_or_else(|| {
        vec![USAGE_EMOTICON.to_owned(), USAGE_STICKER.to_owned()]
    });
    NativeImagePackMeta {
        name,
        avatar,
        attribution,
        usage,
    }
}

/// Parse a `PackContent` object (`{ pack?, images? }`) into a DTO.
///
/// `id` is the pack identity (user id for the personal pack, event id / state
/// key for room packs). `address` is `None` for the personal pack.
fn parse_pack_content(
    id: String,
    address: Option<NativeImagePackAddress>,
    content: &serde_json::Value,
) -> NativeImagePack {
    let pack = content.get("pack");
    let images = content.get("images");
    let deleted = pack.is_none() && images.is_none();

    let mut image_map = BTreeMap::new();
    if let Some(images_obj) = images.and_then(|v| v.as_object()) {
        for (shortcode, image_value) in images_obj {
            if let Some(image) = parse_image(image_value) {
                image_map.insert(shortcode.clone(), image);
            }
        }
    }

    NativeImagePack {
        id,
        deleted,
        address,
        meta: parse_meta(pack),
        images: image_map,
    }
}

/// Parse the personal `im.ponies.user_emotes` account-data content.
///
/// `user_id` is used as the pack id (mirrors the legacy JS `getUserImagePack`).
pub fn parse_user_pack(user_id: &str, content: &serde_json::Value) -> NativeImagePack {
    parse_pack_content(user_id.to_owned(), None, content)
}

/// Parse `im.ponies.emote_rooms` content into the set of enabled global pack
/// addresses: `{ rooms: { [roomId]: { [stateKey]: {} } } }`.
fn parse_emote_rooms(content: &serde_json::Value) -> Vec<NativeImagePackAddress> {
    let mut addresses = Vec::new();
    let Some(rooms) = content.get("rooms").and_then(|v| v.as_object()) else {
        return addresses;
    };
    for (room_id, state_keys) in rooms {
        let Some(state_keys_obj) = state_keys.as_object() else {
            continue;
        };
        for state_key in state_keys_obj.keys() {
            addresses.push(NativeImagePackAddress {
                room_id: room_id.clone(),
                state_key: state_key.clone(),
            });
        }
    }
    addresses
}

/// Resolve a room's `im.ponies.room_emotes` state events into pack DTOs.
///
/// `state_keys` optionally filters to a specific set of state keys (used for
/// global packs); when `None`, all room packs are returned.
fn parse_room_packs(
    room_id: &str,
    state_keys: Option<&std::collections::BTreeSet<String>>,
    events: &[serde_json::Value],
) -> Vec<NativeImagePack> {
    let mut packs = Vec::new();
    for event in events {
        let Some(state_key) = event.get("state_key").and_then(|v| v.as_str()) else {
            continue;
        };
        if let Some(filter) = state_keys {
            if !filter.contains(state_key) {
                continue;
            }
        }
        let content = event.get("content").cloned().unwrap_or(serde_json::Value::Null);
        packs.push(parse_pack_content(
            state_key.to_owned(),
            Some(NativeImagePackAddress {
                room_id: room_id.to_owned(),
                state_key: state_key.to_owned(),
            }),
            &content,
        ));
    }
    packs
}

/// Read a global account-data event's raw content as `serde_json::Value`.
async fn load_account_data_value(
    client: &Client,
    event_type: GlobalAccountDataEventType,
    diagnostic: &'static str,
) -> Result<Option<serde_json::Value>, &'static str> {
    let raw = client
        .account()
        .account_data_raw(event_type)
        .await
        .map_err(|_| diagnostic)?;
    match raw {
        Some(raw) => raw
            .deserialize_as_unchecked::<serde_json::Value>()
            .map(Some)
            .map_err(|_| diagnostic),
        None => Ok(None),
    }
}

/// Read a room's `im.ponies.room_emotes` state events as raw JSON values.
async fn load_room_state_events(
    room: &Room,
    event_type: StateEventType,
    diagnostic: &'static str,
) -> Result<Vec<serde_json::Value>, &'static str> {
    let raw_events = room
        .get_state_events(event_type)
        .await
        .map_err(|_| diagnostic)?;
    let mut events = Vec::with_capacity(raw_events.len());
    for raw in raw_events {
        let value = match raw {
            matrix_sdk::deserialized_responses::RawAnySyncOrStrippedState::Sync(raw) => raw
                .deserialize_as_unchecked::<serde_json::Value>()
                .map_err(|_| diagnostic)?,
            matrix_sdk::deserialized_responses::RawAnySyncOrStrippedState::Stripped(raw) => raw
                .deserialize_as_unchecked::<serde_json::Value>()
                .map_err(|_| diagnostic)?,
        };
        events.push(value);
    }
    Ok(events)
}

/// Snapshot the personal `im.ponies.user_emotes` pack.
pub async fn snapshot_user_image_pack(
    client: &Client,
    session_generation: u64,
) -> Result<NativeImagePackSnapshot, &'static str> {
    let Some(user_id) = client.user_id() else {
        return Err("v-send-pack-read-user-missing");
    };
    let value = load_account_data_value(client, ponies_user_emotes_type(), "v-send-pack-read-user-fetch-failed").await?;
    let packs = match value {
        Some(content) => vec![parse_user_pack(user_id.as_str(), &content)],
        None => Vec::new(),
    };
    Ok(NativeImagePackSnapshot {
        session_generation,
        packs,
    })
}

/// Snapshot the enabled global packs from `im.ponies.emote_rooms`.
pub async fn snapshot_global_image_packs(
    client: &Client,
    session_generation: u64,
) -> Result<NativeImagePackSnapshot, &'static str> {
    let value = load_account_data_value(client, ponies_emote_rooms_type(), "v-send-pack-read-global-fetch-failed").await?;
    let Some(content) = value else {
        return Ok(NativeImagePackSnapshot {
            session_generation,
            packs: Vec::new(),
        });
    };

    let addresses = parse_emote_rooms(&content);
    // Group addresses by room so we fetch each room's state once.
    let mut by_room: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
    for address in addresses {
        by_room
            .entry(address.room_id)
            .or_default()
            .insert(address.state_key);
    }

    let mut packs = Vec::new();
    for (room_id, state_keys) in by_room {
        let Ok(room_id) = OwnedRoomId::try_from(room_id.as_str()) else {
            continue;
        };
        let Some(room) = client.get_room(&room_id) else {
            continue;
        };
        if room.state() != RoomState::Joined {
            continue;
        }
        let events = load_room_state_events(
            &room,
            ponies_room_emotes_type(),
            "v-send-pack-read-global-room-fetch-failed",
        )
        .await?;
        packs.extend(parse_room_packs(room_id.as_str(), Some(&state_keys), &events));
    }

    Ok(NativeImagePackSnapshot {
        session_generation,
        packs,
    })
}

/// Snapshot all `im.ponies.room_emotes` packs for a joined room.
pub async fn snapshot_room_image_packs(
    client: &Client,
    room_id: &str,
    session_generation: u64,
) -> Result<NativeImagePackSnapshot, &'static str> {
    let room_id = OwnedRoomId::try_from(room_id.trim())
        .map_err(|_| "v-send-pack-read-invalid-room")?;
    let room = client
        .get_room(&room_id)
        .ok_or("v-send-pack-read-room-missing")?;
    if room.state() != RoomState::Joined {
        return Err("v-send-pack-read-room-not-joined");
    }
    let events = load_room_state_events(
        &room,
        ponies_room_emotes_type(),
        "v-send-pack-read-room-fetch-failed",
    )
    .await?;
    let packs = parse_room_packs(room_id.as_str(), None, &events);
    Ok(NativeImagePackSnapshot {
        session_generation,
        packs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn pack_content() -> serde_json::Value {
        json!({
            "pack": {
                "display_name": "My Pack",
                "avatar_url": "mxc://example.org/avatar",
                "attribution": "Someone",
                "usage": ["emoticon"]
            },
            "images": {
                "smile": { "url": "mxc://example.org/smile", "body": "smile" },
                "wave": { "url": "mxc://example.org/wave", "usage": ["sticker"] },
                "broken": { "body": "no url" }
            }
        })
    }

    #[test]
    fn parse_user_pack_uses_user_id_and_meta() {
        let content = pack_content();
        let pack = parse_user_pack("@alice:example.org", &content);
        assert_eq!(pack.id, "@alice:example.org");
        assert!(!pack.deleted);
        assert!(pack.address.is_none());
        assert_eq!(pack.meta.name.as_deref(), Some("My Pack"));
        assert_eq!(pack.meta.avatar.as_deref(), Some("mxc://example.org/avatar"));
        assert_eq!(pack.meta.attribution.as_deref(), Some("Someone"));
        assert_eq!(pack.meta.usage, vec!["emoticon".to_owned()]);
        // Only valid images with a URL are kept.
        assert_eq!(pack.images.len(), 2);
        assert_eq!(pack.images["smile"].url, "mxc://example.org/smile");
        assert_eq!(pack.images["smile"].body.as_deref(), Some("smile"));
        assert_eq!(pack.images["wave"].usage.as_deref(), Some(&["sticker".to_owned()][..]));
    }

    #[test]
    fn parse_user_pack_empty_content_is_deleted() {
        let content = json!({});
        let pack = parse_user_pack("@alice:example.org", &content);
        assert!(pack.deleted);
        assert!(pack.images.is_empty());
        assert_eq!(pack.meta.usage, vec!["emoticon".to_owned(), "sticker".to_owned()]);
    }

    #[test]
    fn parse_user_pack_missing_usage_defaults_to_both() {
        let content = json!({
            "pack": { "display_name": "No Usage" },
            "images": { "a": { "url": "mxc://example.org/a" } }
        });
        let pack = parse_user_pack("@alice:example.org", &content);
        assert_eq!(pack.meta.usage, vec!["emoticon".to_owned(), "sticker".to_owned()]);
        assert_eq!(pack.images["a"].usage, None);
    }

    #[test]
    fn parse_emote_rooms_extracts_addresses() {
        let content = json!({
            "rooms": {
                "!room1:example.org": { "packA": {}, "packB": {} },
                "!room2:example.org": { "packC": {} }
            }
        });
        let addresses = parse_emote_rooms(&content);
        assert_eq!(addresses.len(), 3);
        assert!(addresses.contains(&NativeImagePackAddress {
            room_id: "!room1:example.org".to_owned(),
            state_key: "packA".to_owned(),
        }));
        assert!(addresses.contains(&NativeImagePackAddress {
            room_id: "!room2:example.org".to_owned(),
            state_key: "packC".to_owned(),
        }));
    }

    #[test]
    fn parse_emote_rooms_missing_rooms_is_empty() {
        assert!(parse_emote_rooms(&json!({})).is_empty());
        assert!(parse_emote_rooms(&json!({ "rooms": "nope" })).is_empty());
    }

    #[test]
    fn parse_room_packs_filters_by_state_keys() {
        let events = vec![
            json!({
                "state_key": "packA",
                "content": { "pack": { "display_name": "A" }, "images": { "a": { "url": "mxc://x/a" } } }
            }),
            json!({
                "state_key": "packB",
                "content": { "pack": { "display_name": "B" }, "images": { "b": { "url": "mxc://x/b" } } }
            }),
        ];
        let all = parse_room_packs("!room:example.org", None, &events);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].address.as_ref().unwrap().room_id, "!room:example.org");
        assert_eq!(all[0].address.as_ref().unwrap().state_key, "packA");

        let filter: std::collections::BTreeSet<String> =
            ["packB".to_owned()].into_iter().collect();
        let filtered = parse_room_packs("!room:example.org", Some(&filter), &events);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "packB");
    }

    #[test]
    fn snapshot_serializes_camel_case() {
        let snap = NativeImagePackSnapshot {
            session_generation: 7,
            packs: vec![NativeImagePack {
                id: "packA".to_owned(),
                deleted: false,
                address: Some(NativeImagePackAddress {
                    room_id: "!room:example.org".to_owned(),
                    state_key: "packA".to_owned(),
                }),
                meta: NativeImagePackMeta {
                    name: Some("A".to_owned()),
                    avatar: None,
                    attribution: None,
                    usage: vec!["emoticon".to_owned()],
                },
                images: BTreeMap::new(),
            }],
        };
        let value = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(value["sessionGeneration"], 7);
        assert_eq!(value["packs"][0]["id"], "packA");
        assert_eq!(value["packs"][0]["address"]["roomId"], "!room:example.org");
        assert_eq!(value["packs"][0]["address"]["stateKey"], "packA");
        assert_eq!(value["packs"][0]["meta"]["name"], "A");
    }
}
