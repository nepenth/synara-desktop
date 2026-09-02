//! Native composer reply-draft ownership for V-TIMELINE.
//!
//! Reply transport remains `matrix_send_text` with `reply_to`. This module owns
//! the per-room reply target shown in the composer so the still-active legacy
//! presenter can re-home that affordance without selecting the native timeline
//! presenter. Message body drafts stay local (Slate / localStorage).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Version of the bounded composer reply-draft readback contract.
pub const NATIVE_COMPOSER_REPLY_DRAFT_SCHEMA_VERSION: u32 = 2;

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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeComposerClearReplyDraftRequest {
    pub room_id: String,
    /// Core-issued opaque identity of the exact draft the actor consumed.
    /// A different current draft is returned unchanged as authoritative readback.
    pub expected_draft_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeComposerReplyDraft {
    /// Monotonic Core-issued identity. Clients pass this value back unchanged;
    /// they must not infer draft identity from the Matrix relation alone.
    pub draft_revision: u64,
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
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft: Option<NativeComposerReplyDraft>,
}

fn deserialize_reply_draft_status<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    match value.as_str() {
        "set" | "cleared" | "empty" => Ok(value),
        other => Err(serde::de::Error::unknown_variant(
            other,
            &["set", "cleared", "empty"],
        )),
    }
}

#[derive(Debug, Default)]
pub struct ComposerDraftRegistry {
    by_room: HashMap<String, NativeComposerReplyDraft>,
    next_revision: u64,
}

impl ComposerDraftRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(
        &mut self,
        room_id: String,
        mut draft: NativeComposerReplyDraft,
    ) -> NativeComposerReplyDraft {
        // A process cannot realistically exhaust this space. Avoid revision 0,
        // which is reserved for an unregistered draft assembled by the loader.
        self.next_revision = self.next_revision.checked_add(1).unwrap_or(1);
        draft.draft_revision = self.next_revision;
        self.by_room.insert(room_id, draft.clone());
        draft
    }

    pub fn get(&self, room_id: &str) -> Option<&NativeComposerReplyDraft> {
        self.by_room.get(room_id)
    }

    /// Atomically clears the room draft only when the send-time target is still
    /// current. The Core-issued revision distinguishes repeated selections and
    /// classic versus threaded replies to the same event. Returns the newer
    /// current draft when the expected draft was superseded while an operation
    /// was in flight.
    pub fn compare_and_clear(
        &mut self,
        room_id: &str,
        expected_draft_revision: u64,
    ) -> Option<NativeComposerReplyDraft> {
        if let Some(current) = self.by_room.get(room_id) {
            if current.draft_revision != expected_draft_revision {
                return Some(current.clone());
            }
        }
        self.by_room.remove(room_id);
        None
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
        status: status.to_owned(),
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
            draft_revision: 0,
            event_id: "$evt:example.org".into(),
            sender_id: "@alice:example.org".into(),
            body: "hello".into(),
            formatted_body: Some("<p>hello</p>".into()),
            thread_root_event_id: None,
        };
        let draft = registry.set("!room:example.org".into(), draft);
        assert_eq!(draft.draft_revision, 1);
        assert_eq!(registry.get("!room:example.org"), Some(&draft));
        assert!(registry.get("!other:example.org").is_none());
        assert!(registry
            .compare_and_clear("!room:example.org", draft.draft_revision)
            .is_none());
        assert!(registry.get("!room:example.org").is_none());
        assert_eq!(
            reply_draft_readback("!room:example.org".into(), "cleared", None).status,
            "cleared"
        );
    }

    #[test]
    fn compare_and_clear_preserves_a_newer_draft_selected_during_send() {
        let mut registry = ComposerDraftRegistry::new();
        let room_id = "!room:example.org";
        let sent_draft = NativeComposerReplyDraft {
            draft_revision: 0,
            event_id: "$sent:example.org".into(),
            sender_id: "@alice:example.org".into(),
            body: "sent target".into(),
            formatted_body: None,
            thread_root_event_id: None,
        };
        let newer_draft = NativeComposerReplyDraft {
            draft_revision: 0,
            event_id: "$newer:example.org".into(),
            sender_id: "@bob:example.org".into(),
            body: "new target".into(),
            formatted_body: None,
            thread_root_event_id: None,
        };

        let sent_draft = registry.set(room_id.into(), sent_draft);
        let newer_draft = registry.set(room_id.into(), newer_draft);

        assert_eq!(
            registry.compare_and_clear(room_id, sent_draft.draft_revision),
            Some(newer_draft.clone())
        );
        assert_eq!(registry.get(room_id), Some(&newer_draft));
        assert_eq!(
            registry.compare_and_clear(room_id, newer_draft.draft_revision),
            None
        );
        assert!(registry.get(room_id).is_none());
    }

    #[test]
    fn compare_and_clear_preserves_same_event_with_a_new_relation_or_revision() {
        let mut registry = ComposerDraftRegistry::new();
        let room_id = "!room:example.org";
        let classic = registry.set(
            room_id.into(),
            NativeComposerReplyDraft {
                draft_revision: 0,
                event_id: "$same:example.org".into(),
                sender_id: "@alice:example.org".into(),
                body: "same target".into(),
                formatted_body: None,
                thread_root_event_id: None,
            },
        );
        let threaded = registry.set(
            room_id.into(),
            NativeComposerReplyDraft {
                draft_revision: 0,
                event_id: "$same:example.org".into(),
                sender_id: "@alice:example.org".into(),
                body: "same target".into(),
                formatted_body: None,
                thread_root_event_id: Some("$same:example.org".into()),
            },
        );

        assert_eq!(
            registry.compare_and_clear(room_id, classic.draft_revision),
            Some(threaded.clone())
        );
        assert_eq!(registry.get(room_id), Some(&threaded));

        let repeated = registry.set(
            room_id.into(),
            NativeComposerReplyDraft {
                draft_revision: 0,
                ..threaded.clone()
            },
        );
        assert_ne!(threaded.draft_revision, repeated.draft_revision);
        assert_eq!(
            registry.compare_and_clear(room_id, threaded.draft_revision),
            Some(repeated.clone())
        );
        assert_eq!(registry.get(room_id), Some(&repeated));
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
