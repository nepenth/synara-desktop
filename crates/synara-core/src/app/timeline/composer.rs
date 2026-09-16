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
    /// Live drafts omit this; thread-view drafts key the same room separately.
    #[serde(default)]
    pub thread_root_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeComposerClearReplyDraftRequest {
    pub room_id: String,
    /// Core-issued opaque identity of the exact draft the actor consumed.
    /// A different current draft is returned unchanged as authoritative readback.
    pub expected_draft_revision: u64,
    /// Live drafts omit this; thread-view drafts key the same room separately.
    #[serde(default)]
    pub thread_root_event_id: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ComposerDraftKey {
    room_id: String,
    thread_root: Option<String>,
}

fn composer_draft_key(room_id: &str, thread_root: Option<&str>) -> ComposerDraftKey {
    ComposerDraftKey {
        room_id: room_id.to_owned(),
        thread_root: thread_root
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
    }
}

#[derive(Debug, Default)]
pub struct ComposerDraftRegistry {
    by_slot: HashMap<ComposerDraftKey, NativeComposerReplyDraft>,
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
        let key = composer_draft_key(&room_id, draft.thread_root_event_id.as_deref());
        self.by_slot.insert(key, draft.clone());
        draft
    }

    pub fn get(
        &self,
        room_id: &str,
        thread_root: Option<&str>,
    ) -> Option<&NativeComposerReplyDraft> {
        self.by_slot.get(&composer_draft_key(room_id, thread_root))
    }

    /// Atomically clears the room (or thread) draft only when the send-time
    /// target is still current. The Core-issued revision distinguishes repeated
    /// selections in the same slot. Live and thread slots do not share a key, so
    /// a classic room reply cannot clobber or clear a thread reply. Returns the
    /// newer current draft when the expected draft was superseded while an
    /// operation was in flight.
    pub fn compare_and_clear(
        &mut self,
        room_id: &str,
        thread_root: Option<&str>,
        expected_draft_revision: u64,
    ) -> Option<NativeComposerReplyDraft> {
        let key = composer_draft_key(room_id, thread_root);
        if let Some(current) = self.by_slot.get(&key) {
            if current.draft_revision != expected_draft_revision {
                return Some(current.clone());
            }
        }
        self.by_slot.remove(&key);
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
        assert_eq!(registry.get("!room:example.org", None), Some(&draft));
        assert!(registry.get("!other:example.org", None).is_none());
        assert!(registry
            .compare_and_clear("!room:example.org", None, draft.draft_revision)
            .is_none());
        assert!(registry.get("!room:example.org", None).is_none());
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
            registry.compare_and_clear(room_id, None, sent_draft.draft_revision),
            Some(newer_draft.clone())
        );
        assert_eq!(registry.get(room_id, None), Some(&newer_draft));
        assert_eq!(
            registry.compare_and_clear(room_id, None, newer_draft.draft_revision),
            None
        );
        assert!(registry.get(room_id, None).is_none());
    }

    #[test]
    fn compare_and_clear_preserves_a_newer_revision_in_the_same_slot() {
        let mut registry = ComposerDraftRegistry::new();
        let room_id = "!room:example.org";
        let first = registry.set(
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
        let repeated = registry.set(
            room_id.into(),
            NativeComposerReplyDraft {
                draft_revision: 0,
                ..first.clone()
            },
        );
        assert_ne!(first.draft_revision, repeated.draft_revision);
        assert_eq!(
            registry.compare_and_clear(room_id, Some("$same:example.org"), first.draft_revision),
            Some(repeated.clone())
        );
        assert_eq!(
            registry.get(room_id, Some("$same:example.org")),
            Some(&repeated)
        );
    }

    #[test]
    fn live_and_thread_drafts_do_not_clobber() {
        let mut registry = ComposerDraftRegistry::new();
        let room_id = "!room:example.org";
        let live = registry.set(
            room_id.into(),
            NativeComposerReplyDraft {
                draft_revision: 0,
                event_id: "$live:example.org".into(),
                sender_id: "@alice:example.org".into(),
                body: "live target".into(),
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
                body: "thread target".into(),
                formatted_body: None,
                thread_root_event_id: Some("$same:example.org".into()),
            },
        );

        assert_eq!(registry.get(room_id, None), Some(&live));
        assert_eq!(
            registry.get(room_id, Some("$same:example.org")),
            Some(&threaded)
        );
        assert!(registry
            .compare_and_clear(room_id, None, live.draft_revision)
            .is_none());
        assert!(registry.get(room_id, None).is_none());
        assert_eq!(
            registry.get(room_id, Some("$same:example.org")),
            Some(&threaded)
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

    #[test]
    fn get_and_clear_requests_accept_optional_thread_root() {
        let get: NativeComposerReplyDraftRoomRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "threadRootEventId": "$root:example.org"
        }))
        .unwrap();
        assert_eq!(
            get.thread_root_event_id.as_deref(),
            Some("$root:example.org")
        );
        let live_get: NativeComposerReplyDraftRoomRequest =
            serde_json::from_value(serde_json::json!({ "roomId": "!room:example.org" })).unwrap();
        assert!(live_get.thread_root_event_id.is_none());
        let clear: NativeComposerClearReplyDraftRequest =
            serde_json::from_value(serde_json::json!({
                "roomId": "!room:example.org",
                "expectedDraftRevision": 3u64,
                "threadRootEventId": "$root:example.org"
            }))
            .unwrap();
        assert_eq!(clear.expected_draft_revision, 3);
        assert_eq!(
            clear.thread_root_event_id.as_deref(),
            Some("$root:example.org")
        );
    }
}
