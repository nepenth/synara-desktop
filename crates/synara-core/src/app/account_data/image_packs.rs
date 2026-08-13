//! Credential-free im.ponies image-pack DTO, type filters, and write guards.
//!
//! Live Client snapshot/set/owner live in [`super::image_packs_live`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

pub const USER_EMOTES_EVENT_TYPE: &str = "im.ponies.user_emotes";
pub const EMOTE_ROOMS_EVENT_TYPE: &str = "im.ponies.emote_rooms";
pub const ROOM_EMOTES_EVENT_TYPE: &str = "im.ponies.room_emotes";

/// Tauri event: packs may have changed; UI re-snapshots via matrix_get_* commands.
/// Signal only — never carries pack content (no second data owner).
pub const IMAGE_PACKS_UPDATED_EVENT: &str = "matrix-image-packs-updated";

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
pub struct EmoteRoomsContent {
    #[serde(default)]
    pub rooms: BTreeMap<String, BTreeMap<String, JsonValue>>,
}

pub fn pack_from_account_data(id: String, content: JsonValue) -> NativeImagePack {
    NativeImagePack {
        id,
        room_id: None,
        state_key: None,
        content,
    }
}

pub fn is_image_pack_account_data_type(event_type: &str) -> bool {
    event_type == USER_EMOTES_EVENT_TYPE || event_type == EMOTE_ROOMS_EVENT_TYPE
}

pub fn is_image_pack_room_state_type(event_type: &str) -> bool {
    event_type == ROOM_EMOTES_EVENT_TYPE
}

/// Pure guard extracted from `set_user_image_pack` so the fail-closed content
/// check is unit-testable without a live client.
pub fn set_user_image_pack_content_guard(content: &JsonValue) -> Result<(), &'static str> {
    if !content.is_object() {
        return Err("v-send.r-pack-write-invalid-content");
    }
    Ok(())
}

/// Pure guard extracted from `set_global_image_packs` so the fail-closed content
/// check is unit-testable without a live client.
pub fn set_global_image_packs_content_guard(content: &JsonValue) -> Result<(), &'static str> {
    if !content.is_object() {
        return Err("v-send.r-pack-write-invalid-content");
    }
    Ok(())
}

/// Pure guard extracted from `set_room_image_pack` so the fail-closed content
/// check is unit-testable without a live client.
pub fn set_room_image_pack_content_guard(content: &JsonValue) -> Result<(), &'static str> {
    if !content.is_object() {
        return Err("v-send.r-pack-write-invalid-content");
    }
    Ok(())
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
    fn set_user_image_pack_rejects_non_object_content() {
        // Fail-closed: a non-object body is rejected before any SDK call.
        let err = set_user_image_pack_content_guard(&JsonValue::Array(vec![])).unwrap_err();
        assert_eq!(err, "v-send.r-pack-write-invalid-content");
        assert!(set_user_image_pack_content_guard(&json!({ "pack": {} })).is_ok());
    }

    #[test]
    fn set_global_image_packs_rejects_non_object_content() {
        // Fail-closed: a non-object body is rejected before any SDK call.
        let err = set_global_image_packs_content_guard(&JsonValue::String("x".into())).unwrap_err();
        assert_eq!(err, "v-send.r-pack-write-invalid-content");
        assert!(set_global_image_packs_content_guard(&json!({ "rooms": {} })).is_ok());
    }

    #[test]
    fn set_room_image_pack_rejects_non_object_content() {
        // Fail-closed: a non-object body is rejected before any SDK call.
        // Empty object is valid (delete path).
        let err = set_room_image_pack_content_guard(&JsonValue::Array(vec![])).unwrap_err();
        assert_eq!(err, "v-send.r-pack-write-invalid-content");
        assert!(set_room_image_pack_content_guard(&json!({})).is_ok());
        assert!(set_room_image_pack_content_guard(
            &json!({ "pack": { "display_name": "Room" }, "images": {} })
        )
        .is_ok());
    }

    #[test]
    fn pack_event_type_filters_match_ponies_types() {
        assert!(is_image_pack_account_data_type(USER_EMOTES_EVENT_TYPE));
        assert!(is_image_pack_account_data_type(EMOTE_ROOMS_EVENT_TYPE));
        assert!(!is_image_pack_account_data_type("m.direct"));
        assert!(is_image_pack_room_state_type(ROOM_EMOTES_EVENT_TYPE));
        assert!(!is_image_pack_room_state_type("m.room.name"));
    }
}
