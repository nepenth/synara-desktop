//! Raw global and room account data for product features and Developer Tools.
//!
//! Synara-owned and Element-compatible layout data (`in.synara.spaces`,
//! `io.element.recent_emoji`) and the Developer Tools editors read and write
//! account data as JSON objects. Matrix has no account-data listing endpoint,
//! so the owner records the types it has seen in sync, bounded by the index
//! caps. Secret-bearing types are never read or written through this path.

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::{MAX_GLOBAL_TYPES, MAX_ROOMS_WITH_ACCOUNT_DATA, MAX_ROOM_TYPES};

/// Largest content object accepted for a write, in serialized bytes.
pub const MAX_RAW_ACCOUNT_DATA_BYTES: usize = 64 * 1024;
const MAX_EVENT_TYPE_LEN: usize = 255;

/// Prefixes of account data that carries key material (encrypted or not).
/// Secret storage, cross-signing and backup owners manage these.
const SECRET_BEARING_PREFIXES: &[&str] = &[
    "m.secret_storage.",
    "m.cross_signing.",
    "m.megolm_backup.",
    "org.matrix.msc3814.",
    "m.org.matrix.custom.backup_disabled",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawAccountDataError {
    InvalidType,
    SecretBearingType,
    InvalidRoom,
    RoomNotFound,
    InvalidContent,
    ContentTooLarge,
    FetchFailed,
    SetFailed,
}

impl RawAccountDataError {
    pub const fn diagnostic_id(self) -> &'static str {
        match self {
            Self::InvalidType => "v-account-data.invalid-type",
            Self::SecretBearingType => "v-account-data.secret-type",
            Self::InvalidRoom => "v-account-data.invalid-room",
            Self::RoomNotFound => "v-account-data.room-not-found",
            Self::InvalidContent => "v-account-data.invalid-content",
            Self::ContentTooLarge => "v-account-data.content-too-large",
            Self::FetchFailed => "v-account-data.fetch-failed",
            Self::SetFailed => "v-account-data.set-failed",
        }
    }
}

/// A Matrix event type: 1-255 printable non-space ASCII characters.
pub fn validate_account_data_type(event_type: &str) -> Result<&str, RawAccountDataError> {
    let valid = !event_type.is_empty()
        && event_type.len() <= MAX_EVENT_TYPE_LEN
        && event_type.bytes().all(|byte| byte.is_ascii_graphic());
    if !valid {
        return Err(RawAccountDataError::InvalidType);
    }
    if is_secret_bearing_type(event_type) {
        return Err(RawAccountDataError::SecretBearingType);
    }
    Ok(event_type)
}

pub fn is_secret_bearing_type(event_type: &str) -> bool {
    SECRET_BEARING_PREFIXES
        .iter()
        .any(|prefix| event_type.starts_with(prefix))
}

/// Content must be a JSON object within the size cap.
pub fn validate_account_data_content(content: &JsonValue) -> Result<(), RawAccountDataError> {
    if !content.is_object() {
        return Err(RawAccountDataError::InvalidContent);
    }
    let size = serde_json::to_vec(content)
        .map_err(|_| RawAccountDataError::InvalidContent)?
        .len();
    if size > MAX_RAW_ACCOUNT_DATA_BYTES {
        return Err(RawAccountDataError::ContentTooLarge);
    }
    Ok(())
}

/// Account-data types observed in sync or written in this session.
#[derive(Debug, Default)]
pub struct AccountDataTypeRegistry {
    global: BTreeSet<String>,
    rooms: HashMap<String, BTreeSet<String>>,
}

impl AccountDataTypeRegistry {
    pub fn record_global(&mut self, event_type: &str) {
        if is_secret_bearing_type(event_type) || self.global.contains(event_type) {
            return;
        }
        if self.global.len() < MAX_GLOBAL_TYPES {
            self.global.insert(event_type.to_owned());
        }
    }

    pub fn record_room(&mut self, room_id: &str, event_type: &str) {
        if is_secret_bearing_type(event_type) {
            return;
        }
        if !self.rooms.contains_key(room_id) && self.rooms.len() >= MAX_ROOMS_WITH_ACCOUNT_DATA {
            return;
        }
        let types = self.rooms.entry(room_id.to_owned()).or_default();
        if types.len() < MAX_ROOM_TYPES {
            types.insert(event_type.to_owned());
        }
    }

    pub fn global_types(&self) -> Vec<String> {
        self.global.iter().cloned().collect()
    }

    pub fn room_types(&self, room_id: &str) -> Vec<String> {
        self.rooms
            .get(room_id)
            .map(|types| types.iter().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAccountDataTypes {
    pub session_generation: u64,
    /// `None` for global account data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    pub types: Vec<String>,
}

#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeAccountDataContent {
    pub session_generation: u64,
    pub event_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    /// `None` when no event of this type is stored.
    #[cfg_attr(feature = "ts-export", ts(type = "Record<string, unknown> | null"))]
    pub content: Option<JsonValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeAccountDataTypesRequest {
    #[serde(default)]
    pub room_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeAccountDataGetRequest {
    pub event_type: String,
    #[serde(default)]
    pub room_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeAccountDataSetRequest {
    pub event_type: String,
    #[serde(default)]
    pub room_id: Option<String>,
    pub content: JsonValue,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn event_types_are_bounded_printable_and_never_secret_bearing() {
        assert!(validate_account_data_type("in.synara.spaces").is_ok());
        assert!(validate_account_data_type("io.element.recent_emoji").is_ok());
        assert_eq!(
            validate_account_data_type(""),
            Err(RawAccountDataError::InvalidType)
        );
        assert_eq!(
            validate_account_data_type("has space"),
            Err(RawAccountDataError::InvalidType)
        );
        assert_eq!(
            validate_account_data_type(&"a".repeat(256)),
            Err(RawAccountDataError::InvalidType)
        );
        for secret in [
            "m.secret_storage.default_key",
            "m.secret_storage.key.abc",
            "m.cross_signing.master",
            "m.megolm_backup.v1",
        ] {
            assert_eq!(
                validate_account_data_type(secret),
                Err(RawAccountDataError::SecretBearingType)
            );
        }
    }

    #[test]
    fn content_must_be_a_bounded_object() {
        assert!(validate_account_data_content(&json!({ "a": 1 })).is_ok());
        assert_eq!(
            validate_account_data_content(&json!([1])),
            Err(RawAccountDataError::InvalidContent)
        );
        let big = json!({ "x": "y".repeat(MAX_RAW_ACCOUNT_DATA_BYTES) });
        assert_eq!(
            validate_account_data_content(&big),
            Err(RawAccountDataError::ContentTooLarge)
        );
    }

    #[test]
    fn registry_skips_secret_types_and_respects_caps() {
        let mut registry = AccountDataTypeRegistry::default();
        registry.record_global("m.direct");
        registry.record_global("m.secret_storage.default_key");
        registry.record_global("m.direct");
        registry.record_room("!r:x", "m.fully_read");
        registry.record_room("!r:x", "m.cross_signing.self_signing");
        assert_eq!(registry.global_types(), vec!["m.direct".to_owned()]);
        assert_eq!(registry.room_types("!r:x"), vec!["m.fully_read".to_owned()]);
        assert!(registry.room_types("!other:x").is_empty());
        for index in 0..(MAX_GLOBAL_TYPES + 10) {
            registry.record_global(&format!("t.{index}"));
        }
        assert_eq!(registry.global_types().len(), MAX_GLOBAL_TYPES);
    }
}
