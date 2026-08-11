//! R0.3 residual: bind snapshot/delta bodies to topic-typed DTO containers.
//!
//! REV-005 requires every supported stream topic to carry bounded, typed bodies
//! and to reject secret-like fields and media byte arrays at the envelope
//! boundary. Bodies remain JSON objects on the wire; validation deserializes
//! them into topic-specific owned shapes using domain DTOs (P1.4).

use serde::Deserialize;
use serde_json::Value;

use crate::dto::{
    NotificationCandidate, Receipt, RoomMember, RoomSummary, SecurityStatus, TimelineItem,
    TypingSnapshot, FORBIDDEN_WIRE_FIELD_NAMES,
};

use super::stream::StreamTopic;

/// Room list snapshot/delta body.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoomListStreamBody {
    #[serde(default)]
    pub rooms: Vec<RoomSummary>,
}

/// Timeline snapshot/delta body.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimelineStreamBody {
    #[serde(default)]
    pub items: Vec<TimelineItem>,
}

/// Members snapshot/delta body.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MembersStreamBody {
    #[serde(default)]
    pub members: Vec<RoomMember>,
}

/// Typing snapshot/delta body (multi-room projection).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TypingStreamBody {
    #[serde(default)]
    pub rooms: Vec<TypingSnapshot>,
}

/// Receipts snapshot/delta body.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiptsStreamBody {
    #[serde(default)]
    pub receipts: Vec<Receipt>,
}

/// Account-data stream body (typed shell; content objects still scanned).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountDataStreamBody {
    #[serde(default)]
    pub events: Vec<AccountDataEventWire>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountDataEventWire {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub content: serde_json::Map<String, Value>,
}

/// Presence stream body.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresenceStreamBody {
    #[serde(default)]
    pub entries: Vec<PresenceEntryWire>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresenceEntryWire {
    pub user_id: String,
    pub presence: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_msg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_active_ts: Option<u64>,
}

/// Notification candidates snapshot/delta body.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NotificationCandidatesStreamBody {
    #[serde(default)]
    pub candidates: Vec<NotificationCandidate>,
}

/// Crypto / security status stream body.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CryptoStatusStreamBody {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<SecurityStatus>,
}

/// Send-queue / local-echo status stream body.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SendQueueStreamBody {
    #[serde(default)]
    pub items: Vec<SendQueueItemWire>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SendQueueItemWire {
    pub local_id: String,
    pub room_id: String,
    /// Product local-echo state wire name (`sending`, `sent`, `failed`, …).
    pub state: String,
}

/// Validate a snapshot/delta `body` for the given stream topic.
///
/// Rules (REV-005 residual):
/// 1. Body must be a JSON object.
/// 2. Forbidden secret / media field names are rejected anywhere in the tree.
/// 3. Pure numeric arrays (media-like byte payloads) are rejected.
/// 4. Body deserializes into the topic-owned container (`deny_unknown_fields`).
pub fn validate_stream_topic_body(
    topic: StreamTopic,
    body: &Value,
) -> Result<(), serde_json::Error> {
    if !body.is_object() {
        return Err(serde::de::Error::custom(
            "stream body must be a JSON object",
        ));
    }
    reject_forbidden_and_media_bytes(body)?;
    match topic {
        StreamTopic::RoomList => {
            let _: RoomListStreamBody = serde_json::from_value(body.clone())?;
        }
        StreamTopic::Timeline => {
            let _: TimelineStreamBody = serde_json::from_value(body.clone())?;
        }
        StreamTopic::Members => {
            let _: MembersStreamBody = serde_json::from_value(body.clone())?;
        }
        StreamTopic::Typing => {
            let _: TypingStreamBody = serde_json::from_value(body.clone())?;
        }
        StreamTopic::Receipts => {
            let _: ReceiptsStreamBody = serde_json::from_value(body.clone())?;
        }
        StreamTopic::AccountData => {
            let _: AccountDataStreamBody = serde_json::from_value(body.clone())?;
        }
        StreamTopic::Presence => {
            let _: PresenceStreamBody = serde_json::from_value(body.clone())?;
        }
        StreamTopic::NotificationCandidates => {
            let _: NotificationCandidatesStreamBody = serde_json::from_value(body.clone())?;
        }
        StreamTopic::CryptoStatus => {
            let _: CryptoStatusStreamBody = serde_json::from_value(body.clone())?;
        }
        StreamTopic::SendQueue => {
            let _: SendQueueStreamBody = serde_json::from_value(body.clone())?;
        }
    }
    Ok(())
}

fn reject_forbidden_and_media_bytes(value: &Value) -> Result<(), serde_json::Error> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if is_forbidden_field_name(key) {
                    return Err(serde::de::Error::custom(format!(
                        "forbidden wire field in stream body: {key}"
                    )));
                }
                reject_forbidden_and_media_bytes(child)?;
            }
            Ok(())
        }
        Value::Array(items) => {
            if !items.is_empty() && items.iter().all(|v| v.is_number()) {
                return Err(serde::de::Error::custom(
                    "media-like numeric byte array forbidden in stream body JSON IPC",
                ));
            }
            for child in items {
                reject_forbidden_and_media_bytes(child)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn is_forbidden_field_name(name: &str) -> bool {
    FORBIDDEN_WIRE_FIELD_NAMES.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_object_valid_for_all_topics() {
        let empty = json!({});
        for topic in StreamTopic::ALL {
            validate_stream_topic_body(*topic, &empty).unwrap_or_else(|e| {
                panic!("empty body should be valid for {:?}: {e}", topic);
            });
        }
    }

    #[test]
    fn room_list_accepts_summary_fixture_shape() {
        let body = json!({
            "rooms": [{
                "roomId": "!room:example.org",
                "name": "General",
                "membership": "join",
                "isDirect": false,
                "isEncrypted": true,
                "unreadCount": 0,
                "highlightCount": 0,
                "markedUnread": false
            }]
        });
        validate_stream_topic_body(StreamTopic::RoomList, &body).unwrap();
    }

    #[test]
    fn room_list_rejects_unknown_keys() {
        let body = json!({ "rooms": [], "extra": true });
        assert!(validate_stream_topic_body(StreamTopic::RoomList, &body).is_err());
    }

    #[test]
    fn rejects_secret_field_nested() {
        let body = json!({
            "rooms": [{
                "roomId": "!r:e.org",
                "membership": "join",
                "isDirect": false,
                "isEncrypted": false,
                "unreadCount": 0,
                "highlightCount": 0,
                "markedUnread": false,
                "accessToken": "s3cret"
            }]
        });
        let err = validate_stream_topic_body(StreamTopic::RoomList, &body).unwrap_err();
        assert!(
            err.to_string().contains("forbidden") || err.to_string().contains("accessToken"),
            "err={err}"
        );
    }

    #[test]
    fn rejects_media_byte_array() {
        let body = json!({ "items": [1, 2, 3, 4] });
        // timeline expects objects in items; numeric array fails either media rule or item parse
        assert!(validate_stream_topic_body(StreamTopic::Timeline, &body).is_err());
        let nested = json!({ "rooms": [], "blob": [0, 255, 128] });
        // unknown key rooms body — still fail media if structure were open; room_list deny_unknown
        assert!(validate_stream_topic_body(StreamTopic::RoomList, &nested).is_err());
        let media_in_account = json!({
            "events": [{
                "type": "m.media",
                "content": { "mediaBytes": [1, 2, 3] }
            }]
        });
        assert!(validate_stream_topic_body(StreamTopic::AccountData, &media_in_account).is_err());
    }

    #[test]
    fn timeline_accepts_empty_items() {
        validate_stream_topic_body(StreamTopic::Timeline, &json!({ "items": [] })).unwrap();
    }

    #[test]
    fn wrong_topic_shape_rejected() {
        // timeline-shaped body on room_list topic
        let body = json!({ "items": [] });
        assert!(validate_stream_topic_body(StreamTopic::RoomList, &body).is_err());
    }
}
