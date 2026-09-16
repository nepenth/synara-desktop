//! Account-level MSC4362 create/opt-in gate.
//!
//! Compiling `experimental-encrypted-state-events` always decrypts incoming
//! encrypted state and encrypts eligible sends in rooms that already have the
//! flag. This setting only blocks **new** create/opt-in.

use std::sync::atomic::{AtomicBool, Ordering};

static ENCRYPTED_STATE_EVENTS_SETTING: AtomicBool = AtomicBool::new(true);

/// Default-on account setting: whether this device may create or opt rooms
/// into encrypted state.
pub fn encrypted_state_events_setting_enabled() -> bool {
    ENCRYPTED_STATE_EVENTS_SETTING.load(Ordering::Acquire)
}

pub fn set_encrypted_state_events_setting_enabled(enabled: bool) {
    ENCRYPTED_STATE_EVENTS_SETTING.store(enabled, Ordering::Release);
}

/// Create-time effective flag. Setting-off and call rooms never opt in, even
/// if a stale IPC still sends `encryptStateEvents: true`.
pub fn should_create_with_encrypted_state(
    encryption: bool,
    encrypt_state_events: bool,
    setting_enabled: bool,
    is_call_room: bool,
) -> bool {
    encryption && encrypt_state_events && setting_enabled && !is_call_room
}

pub fn encryption_content_requests_encrypted_state(content: &serde_json::Value) -> bool {
    let flag = |key: &str| content.get(key).and_then(serde_json::Value::as_bool) == Some(true);
    flag("encrypt_state_events") || flag("io.element.msc4362.encrypt_state_events")
}

pub fn room_encryption_content(encrypt_state_events: bool) -> serde_json::Value {
    let mut content = serde_json::json!({ "algorithm": "m.megolm.v1.aes-sha2" });
    if encrypt_state_events {
        content["encrypt_state_events"] = serde_json::json!(true);
        content["io.element.msc4362.encrypt_state_events"] = serde_json::json!(true);
    }
    content
}

pub fn is_call_room_type(room_type: Option<&str>) -> bool {
    matches!(
        room_type.map(str::trim).filter(|value| !value.is_empty()),
        Some("org.matrix.msc3417.call") | Some("m.call")
    )
}

/// Where power-level-tags readback must look after a write.
/// Encrypted tags live under a packed `m.room.encrypted` key on the
/// homeserver; HTTP `get_state_event_for_key(in.synara.room.power_level_tags)`
/// misses them. Power levels stay HTTP (SDK exclusion).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerLevelTagsReadbackSource {
    Store,
    Http,
}

pub fn power_level_tags_readback_source(is_state_encrypted: bool) -> PowerLevelTagsReadbackSource {
    if is_state_encrypted {
        PowerLevelTagsReadbackSource::Store
    } else {
        PowerLevelTagsReadbackSource::Http
    }
}

/// MSC4362 packs encrypted state as `{type}:{state_key}` on `m.room.encrypted`.
pub fn packed_encrypted_state_type(state_key: &str) -> Option<&str> {
    let (event_type, _) = state_key.split_once(':')?;
    (!event_type.is_empty()).then_some(event_type)
}

pub fn event_is_undecrypted_encrypted_state(value: &serde_json::Value) -> bool {
    value.get("type").and_then(serde_json::Value::as_str) == Some("m.room.encrypted")
        && packed_encrypted_state_type(
            value
                .get("state_key")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(""),
        )
        .is_some()
}

/// Product write for `m.room.encryption`. The SDK helper
/// `enable_encryption_with_state_event_encryption` no-ops when the room is
/// already `is_encrypted()`, so upgrades must send the event directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomEncryptionEnableWrite {
    NoOp,
    Send(serde_json::Value),
}

pub fn room_encryption_enable_write(
    current_is_encrypted: bool,
    current_is_state_encrypted: bool,
    encrypt_state_events: bool,
    setting_enabled: bool,
    is_call_room: bool,
) -> RoomEncryptionEnableWrite {
    let encrypt_state = encrypt_state_events && setting_enabled && !is_call_room;
    if current_is_state_encrypted || (current_is_encrypted && !encrypt_state) {
        RoomEncryptionEnableWrite::NoOp
    } else {
        RoomEncryptionEnableWrite::Send(room_encryption_content(encrypt_state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_opt_in_requires_encryption_setting_and_non_call() {
        assert!(should_create_with_encrypted_state(true, true, true, false));
        assert!(!should_create_with_encrypted_state(
            true, true, false, false
        ));
        assert!(!should_create_with_encrypted_state(
            true, false, true, false
        ));
        assert!(!should_create_with_encrypted_state(
            false, true, true, false
        ));
        assert!(!should_create_with_encrypted_state(true, true, true, true));
    }

    #[test]
    fn encryption_content_emits_stable_and_unstable_flags() {
        let flagged = room_encryption_content(true);
        assert_eq!(flagged["algorithm"], "m.megolm.v1.aes-sha2");
        assert_eq!(flagged["encrypt_state_events"], true);
        assert_eq!(flagged["io.element.msc4362.encrypt_state_events"], true);
        assert!(encryption_content_requests_encrypted_state(&flagged));
        let messages_only = room_encryption_content(false);
        assert!(messages_only.get("encrypt_state_events").is_none());
        assert!(!encryption_content_requests_encrypted_state(&messages_only));
    }

    #[test]
    fn tags_readback_uses_store_only_when_state_encrypted() {
        assert_eq!(
            power_level_tags_readback_source(true),
            PowerLevelTagsReadbackSource::Store
        );
        assert_eq!(
            power_level_tags_readback_source(false),
            PowerLevelTagsReadbackSource::Http
        );
    }

    #[test]
    fn packed_state_key_is_detected_without_exposing_the_key() {
        assert_eq!(
            packed_encrypted_state_type("m.room.name:"),
            Some("m.room.name")
        );
        assert_eq!(
            packed_encrypted_state_type("im.ponies.room_emotes:default"),
            Some("im.ponies.room_emotes")
        );
        assert!(packed_encrypted_state_type("").is_none());
        assert!(event_is_undecrypted_encrypted_state(&serde_json::json!({
            "type": "m.room.encrypted",
            "state_key": "m.room.name:",
            "content": { "algorithm": "m.megolm.v1.aes-sha2" }
        })));
        assert!(!event_is_undecrypted_encrypted_state(&serde_json::json!({
            "type": "m.room.encrypted",
            "content": { "algorithm": "m.megolm.v1.aes-sha2" }
        })));
    }

    #[test]
    fn mixed_rooms_prefer_packed_encrypted_over_plaintext_siblings() {
        let packed = serde_json::json!({
            "type": "m.room.encrypted",
            "state_key": "m.room.name:",
            "content": { "algorithm": "m.megolm.v1.aes-sha2" }
        });
        let plaintext = serde_json::json!({
            "type": "m.room.name",
            "state_key": "",
            "content": { "name": "Stale plaintext" }
        });
        assert!(event_is_undecrypted_encrypted_state(&packed));
        assert!(!event_is_undecrypted_encrypted_state(&plaintext));
        assert_eq!(
            packed_encrypted_state_type("m.room.name:"),
            Some("m.room.name")
        );
    }

    #[test]
    fn enable_helper_is_noop_when_already_encrypted_and_product_opt_in_sends() {
        assert_eq!(
            room_encryption_enable_write(true, false, true, true, false),
            RoomEncryptionEnableWrite::Send(room_encryption_content(true))
        );
        assert_eq!(
            room_encryption_enable_write(true, false, true, true, true),
            RoomEncryptionEnableWrite::NoOp
        );
        assert_eq!(
            room_encryption_enable_write(true, false, false, true, false),
            RoomEncryptionEnableWrite::NoOp
        );
        assert_eq!(
            room_encryption_enable_write(true, true, true, true, false),
            RoomEncryptionEnableWrite::NoOp
        );
        assert_eq!(
            room_encryption_enable_write(false, false, false, true, false),
            RoomEncryptionEnableWrite::Send(room_encryption_content(false))
        );
        assert_eq!(
            room_encryption_enable_write(true, false, true, false, false),
            RoomEncryptionEnableWrite::NoOp
        );
    }
}
