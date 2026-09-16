//! Privacy-safe MSC4426 status / in-call DTOs.
//!
//! Write acks never echo emoji or text. Parsers are byte-capped and do not
//! share the `m.presence` online/unavailable/offline vocabulary.

use serde::{Deserialize, Serialize};

pub const USER_STATUS_MARKER: &str = "matrix-user-status-msc4426";
pub const MAX_STATUS_EMOJI_BYTES: usize = 32;
pub const MAX_STATUS_TEXT_BYTES: usize = 256;

/// MSC4426 `m.status` projection. Emoji and text are independent of presence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeUserStatus {
    pub emoji: String,
    pub text: String,
}

/// MSC4426 `m.call` projection. Timestamp is seconds since Unix epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeInCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_joined_ts: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeUserStatusSnapshot {
    pub session_generation: u64,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_status: Option<NativeUserStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_call: Option<NativeInCall>,
}

/// Status SET/CLEAR ack. Status only; never echoes emoji or text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeUserStatusWriteResult {
    pub status: String,
}

/// Parsed own-status write. Empty emoji and text become a clear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusWrite {
    Clear,
    Set { emoji: String, text: String },
}

/// Enforce MSC byte caps. This is not `parse_presence_write_state`.
pub fn parse_status_write(emoji: &str, text: &str) -> Result<StatusWrite, &'static str> {
    let emoji = emoji.trim();
    let text = text.trim();
    if emoji.len() > MAX_STATUS_EMOJI_BYTES {
        return Err("v-user-status-emoji-cap");
    }
    if text.len() > MAX_STATUS_TEXT_BYTES {
        return Err("v-user-status-text-cap");
    }
    if emoji.is_empty() && text.is_empty() {
        return Ok(StatusWrite::Clear);
    }
    Ok(StatusWrite::Set {
        emoji: emoji.to_owned(),
        text: text.to_owned(),
    })
}

pub fn project_status_field(emoji: String, text: String) -> Option<NativeUserStatus> {
    if emoji.len() > MAX_STATUS_EMOJI_BYTES || text.len() > MAX_STATUS_TEXT_BYTES {
        return None;
    }
    if emoji.is_empty() && text.is_empty() {
        return None;
    }
    Some(NativeUserStatus { emoji, text })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_parser_rejects_oversize_emoji_and_text() {
        assert_eq!(
            parse_status_write(&"😀".repeat(MAX_STATUS_EMOJI_BYTES + 1), "ok").unwrap_err(),
            "v-user-status-emoji-cap"
        );
        assert_eq!(
            parse_status_write("☕", &"x".repeat(MAX_STATUS_TEXT_BYTES + 1)).unwrap_err(),
            "v-user-status-text-cap"
        );
        assert!(parse_status_write("☕", "in a meeting").is_ok());
        assert_eq!(parse_status_write("  ", "  ").unwrap(), StatusWrite::Clear);
    }

    #[test]
    fn write_parser_does_not_use_presence_state_vocabulary() {
        // MSC4426 is emoji+text, not online/unavailable/offline. Those strings
        // are allowed as emoji/text bytes but never mapped to PresenceState.
        let write = parse_status_write("online", "away").expect("emoji+text is not presence");
        match write {
            StatusWrite::Set { emoji, text } => {
                assert_eq!(emoji, "online");
                assert_eq!(text, "away");
            }
            StatusWrite::Clear => panic!("presence states must not clear m.status"),
        }
        assert!(matches!(parse_status_write("", ""), Ok(StatusWrite::Clear)));
    }

    #[test]
    fn write_ack_never_echoes_emoji_or_text() {
        let ack = NativeUserStatusWriteResult {
            status: "ok".to_owned(),
        };
        let wire = serde_json::to_value(&ack).expect("serialize");
        assert_eq!(wire, serde_json::json!({"status": "ok"}));
        let raw = serde_json::to_string(&ack).expect("serialize string");
        assert!(!raw.contains("emoji"));
        assert!(!raw.contains("text"));
        assert!(!raw.contains("online"));
        assert!(!raw.contains("unavailable"));
        assert!(!raw.contains("offline"));
    }

    #[test]
    fn snapshot_serializes_camel_case_without_presence_states() {
        let snapshot = NativeUserStatusSnapshot {
            session_generation: 4,
            user_id: "@alice:example.org".to_owned(),
            user_status: Some(NativeUserStatus {
                emoji: "☕".to_owned(),
                text: "in a meeting".to_owned(),
            }),
            in_call: Some(NativeInCall {
                call_joined_ts: Some(1_720_000_000),
            }),
        };
        let wire = serde_json::to_value(&snapshot).expect("serialize");
        assert_eq!(wire["sessionGeneration"], 4);
        assert_eq!(wire["userId"], "@alice:example.org");
        assert_eq!(wire["userStatus"]["emoji"], "☕");
        assert_eq!(wire["userStatus"]["text"], "in a meeting");
        assert_eq!(wire["inCall"]["callJoinedTs"], 1_720_000_000);
        let raw = serde_json::to_string(&snapshot).expect("serialize string");
        assert!(!raw.contains("presence"));
        assert!(!raw.contains("statusMsg"));
        assert!(!raw.contains("set_call"));
    }

    #[test]
    fn oversize_status_field_is_dropped_not_projected() {
        assert!(
            project_status_field("x".repeat(MAX_STATUS_EMOJI_BYTES + 1), "ok".into()).is_none()
        );
        assert!(project_status_field("☕".into(), "y".repeat(MAX_STATUS_TEXT_BYTES + 1)).is_none());
        assert!(project_status_field(String::new(), String::new()).is_none());
    }
}
