//! Native composer reply-draft ownership for V-TIMELINE.
//!
//! Reply transport remains `matrix_send_text` with `reply_to`. This module owns
//! the per-room reply target shown in the composer so the still-active legacy
//! presenter can re-home that affordance without selecting the native timeline
//! presenter. Message body drafts stay local (Slate / localStorage).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Version of the bounded composer reply-draft readback contract.
pub const NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeComposerSetReplyDraftRequest {
    pub room_id: String,
    pub event_id: String,
    /// When true, the reply targets a new thread rooted at `event_id`.
    #[serde(default)]
    pub start_thread: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeComposerReplyDraftRoomRequest {
    pub room_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeComposerReplyDraft {
    pub event_id: String,
    pub sender_id: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_body: Option<String>,
    /// Present when the reply should carry an `m.thread` relation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_root_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeComposerReplyDraftReadback {
    pub schema_version: u32,
    pub room_id: String,
    /// `set`, `cleared`, or `empty`.
    #[serde(deserialize_with = "deserialize_reply_draft_status")]
    pub status: &'static str,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft: Option<NativeComposerReplyDraft>,
}

fn deserialize_reply_draft_status<'de, D>(deserializer: D) -> Result<&'static str, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    match value.as_str() {
        "set" => Ok("set"),
        "cleared" => Ok("cleared"),
        "empty" => Ok("empty"),
        other => Err(serde::de::Error::unknown_variant(
            other,
            &["set", "cleared", "empty"],
        )),
    }
}

#[derive(Debug, Default)]
pub struct ComposerDraftRegistry {
    by_room: HashMap<String, NativeComposerReplyDraft>,
}

impl ComposerDraftRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, room_id: String, draft: NativeComposerReplyDraft) {
        self.by_room.insert(room_id, draft);
    }

    pub fn get(&self, room_id: &str) -> Option<&NativeComposerReplyDraft> {
        self.by_room.get(room_id)
    }

    pub fn clear(&mut self, room_id: &str) -> bool {
        self.by_room.remove(room_id).is_some()
    }
}

pub fn reply_draft_readback(
    room_id: String,
    status: &'static str,
    draft: Option<NativeComposerReplyDraft>,
) -> NativeComposerReplyDraftReadback {
    NativeComposerReplyDraftReadback {
        schema_version: NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION,
        room_id,
        status,
        draft,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_set_get_and_clear_are_room_scoped() {
        let mut registry = ComposerDraftRegistry::new();
        let draft = NativeComposerReplyDraft {
            event_id: "$evt:example.org".into(),
            sender_id: "@alice:example.org".into(),
            body: "hello".into(),
            formatted_body: Some("<p>hello</p>".into()),
            thread_root_event_id: None,
        };
        registry.set("!room:example.org".into(), draft.clone());
        assert_eq!(registry.get("!room:example.org"), Some(&draft));
        assert!(registry.get("!other:example.org").is_none());
        assert!(registry.clear("!room:example.org"));
        assert!(!registry.clear("!room:example.org"));
        assert_eq!(
            reply_draft_readback("!room:example.org".into(), "cleared", None).status,
            "cleared"
        );
    }

    #[test]
    fn set_reply_draft_request_accepts_optional_start_thread() {
        let request: NativeComposerSetReplyDraftRequest =
            serde_json::from_value(serde_json::json!({
                "roomId": "!room:example.org",
                "eventId": "$evt:example.org",
                "startThread": true
            }))
            .unwrap();
        assert!(request.start_thread);
        assert_eq!(request.event_id, "$evt:example.org");
    }
}
