//! Credential-free native timeline presentation DTOs.
//!
//! Live Client registry, Tauri view streams, and SDK event-id parse stay in
//! the desktop shell.

use serde::{Deserialize, Serialize};

use super::view::{
    TimelinePageState, TimelinePaginationState, TimelineReadState, TimelineViewCapabilities,
    TimelineViewPosition, TimelineViewSnapshot, TIMELINE_VIEW_SCHEMA_VERSION,
};

/// Version of the bounded native timeline-open contract.
pub const NATIVE_TIMELINE_OPEN_SCHEMA_VERSION: u32 = 1;
pub const NATIVE_TIMELINE_VIEWPORT_RESTORE_TTL_MS: u64 = 10 * 60 * 1000;

/// Requested initial position for one native timeline view.
///
/// `Normal` is the native owner route for an ordinary room open. It resolves
/// shared unread state before considering the optional, UI-held restore hint;
/// the hint is neither sync state nor a server-side viewport command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct NativeTimelineViewportHint {
    #[serde(default)]
    pub at_bottom: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restored_anchor_event_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live_tail_event_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NativeTimelineOpenPosition {
    Normal {
        #[serde(flatten, default)]
        viewport: NativeTimelineViewportHint,
    },
    LiveBottom,
    Unread,
    Focused {
        event_id: String,
    },
}

/// Typed input for the native timeline-open owner.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineOpenRequest {
    pub room_id: String,
    pub position: NativeTimelineOpenPosition,
}

/// Bounded authoritative result of opening the requested native timeline
/// position. This is the versioned, SDK-neutral view boundary; it has no
/// active React consumer until the complete presenter cutover is ready.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineOpenReadback {
    pub schema_version: u32,
    /// Opaque identifier carried by every delta from this exact opened view.
    pub stream_id: String,
    /// Native-selected placement after resolving the request. In particular,
    /// a normal open can select unread, restored, or live-bottom placement.
    pub position: TimelineViewPosition,
    pub snapshot: TimelineViewSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeTimelineDirection {
    Backwards,
    Forwards,
}

/// A pagination request addresses the exact opened view, rather than assuming
/// a room has only one timeline. `stream_id` comes from `matrix_timeline_open`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineViewPaginationRequest {
    pub stream_id: String,
    pub direction: NativeTimelineDirection,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineCloseRequest {
    pub stream_id: String,
}

/// Rebind one opened view to the live bottom without a JS timeline fetch.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineJumpLatestRequest {
    pub stream_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeTimelineReadAction {
    MarkRead,
    MarkUnread,
}

/// A read-state transition always targets one opened native timeline view.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineReadStateRequest {
    pub stream_id: String,
    pub action: NativeTimelineReadAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineReadStateReadback {
    pub action: NativeTimelineReadAction,
    /// `None` for an account-data unread-flag change; otherwise whether the
    /// SDK actually sent a private read receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_sent: Option<bool>,
    pub snapshot: TimelineViewSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineItem {
    pub item_id: String,
    pub event_id: String,
    pub sender: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub body: String,
    pub origin_server_ts: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decryption_state: Option<NativeDecryptionState>,
    /// Aggregated reactions are projected by the native timeline owner. The
    /// webview never derives reaction ownership from a Matrix JS timeline.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactions: Vec<NativeTimelineReaction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineReaction {
    pub key: String,
    pub count: u32,
    pub me: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub senders: Vec<NativeTimelineReactionSender>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineReactionSender {
    pub user_id: String,
    /// Remote reaction annotations can be redacted by their event id. Local
    /// echoes intentionally have no fabricated event id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reaction_event_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeReactionMutation {
    Added,
    Removed,
    AlreadyPresent,
    Redacted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeReactionMutationResult {
    pub room_id: String,
    pub target_event_id: String,
    pub key: String,
    pub mutation: NativeReactionMutation,
    /// State reprojected from the same Rust timeline owner after the SDK call.
    pub readback: Option<NativeTimelineReaction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeDecryptionState {
    Pending,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeUtdPhase {
    Idle,
    Recovering,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeUtdStatus {
    pub phase: NativeUtdPhase,
    pub pending_count: u32,
    pub unavailable_count: u32,
    pub recovered_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineSnapshot {
    pub session_generation: u64,
    pub room_id: String,
    pub is_encrypted: bool,
    pub items: Vec<NativeTimelineItem>,
    pub hit_start: bool,
    pub utd: NativeUtdStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTimelineEventReadback {
    pub session_generation: u64,
    pub room_id: String,
    pub event_id: String,
    pub item: NativeTimelineItem,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_snapshot_schema_has_no_secret_or_ciphertext_fields() {
        let snapshot = NativeTimelineSnapshot {
            session_generation: 7,
            room_id: "!room:example.org".into(),
            is_encrypted: true,
            items: vec![NativeTimelineItem {
                item_id: "item-1".into(),
                event_id: "$event".into(),
                sender: "@alice:example.org".into(),
                event_type: "m.room.message".into(),
                body: "hello".into(),
                origin_server_ts: 42,
                decryption_state: None,
                reactions: vec![NativeTimelineReaction {
                    key: "✅".into(),
                    count: 1,
                    me: true,
                    senders: vec![NativeTimelineReactionSender {
                        user_id: "@alice:example.org".into(),
                        reaction_event_id: Some("$reaction".into()),
                    }],
                }],
            }],
            hit_start: false,
            utd: NativeUtdStatus {
                phase: NativeUtdPhase::Idle,
                pending_count: 0,
                unavailable_count: 0,
                recovered_count: 0,
            },
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        for forbidden in [
            "accessToken",
            "access_token",
            "refreshToken",
            "refresh_token",
            "sessionKey",
            "ciphertext",
        ] {
            assert!(!json.contains(forbidden));
        }
        assert!(json.contains("\"type\":\"m.room.message\""));
        assert!(json.contains("\"body\":\"hello\""));
        assert!(json.contains("\"isEncrypted\":true"));
        assert!(json.contains("\"reactionEventId\":\"$reaction\""));
    }

    #[test]
    fn reaction_mutation_readback_schema_has_no_secret_fields() {
        let result = NativeReactionMutationResult {
            room_id: "!room:example.org".into(),
            target_event_id: "$event:example.org".into(),
            key: "✅".into(),
            mutation: NativeReactionMutation::AlreadyPresent,
            readback: Some(NativeTimelineReaction {
                key: "✅".into(),
                count: 2,
                me: true,
                senders: vec![NativeTimelineReactionSender {
                    user_id: "@alice:example.org".into(),
                    reaction_event_id: Some("$reaction:example.org".into()),
                }],
            }),
        };
        let json = serde_json::to_string(&result).unwrap();
        for forbidden in [
            "accessToken",
            "access_token",
            "refreshToken",
            "refresh_token",
            "sessionKey",
            "ciphertext",
            "private_key",
        ] {
            assert!(!json.contains(forbidden));
        }
        assert!(json.contains("\"mutation\":\"already_present\""));
        assert!(json.contains("\"reactionEventId\":\"$reaction:example.org\""));
        assert!(json.contains("\"me\":true"));
    }

    #[test]
    fn focused_open_request_keeps_the_event_link_at_the_native_boundary() {
        let request: NativeTimelineOpenRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "position": { "kind": "focused", "event_id": "$event:example.org" }
        }))
        .unwrap();
        assert_eq!(request.room_id, "!room:example.org");
        assert_eq!(
            request.position,
            NativeTimelineOpenPosition::Focused {
                event_id: "$event:example.org".into()
            }
        );
    }

    #[test]
    fn unread_open_request_stays_distinct_from_live_bottom() {
        let request: NativeTimelineOpenRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "position": { "kind": "unread" }
        }))
        .unwrap();
        assert_eq!(request.position, NativeTimelineOpenPosition::Unread);
    }

    #[test]
    fn normal_open_request_keeps_the_ui_restore_hint_typed_at_the_native_boundary() {
        let request: NativeTimelineOpenRequest = serde_json::from_value(serde_json::json!({
            "roomId": "!room:example.org",
            "position": {
                "kind": "normal",
                "restored_anchor_event_id": "$restore:example.org",
                "at_bottom": false,
                "live_tail_event_id": "$tail:example.org",
                "updated_at_ms": 1_700_000_000_000_u64
            }
        }))
        .unwrap();
        assert_eq!(
            request.position,
            NativeTimelineOpenPosition::Normal {
                viewport: NativeTimelineViewportHint {
                    at_bottom: false,
                    restored_anchor_event_id: Some("$restore:example.org".into()),
                    live_tail_event_id: Some("$tail:example.org".into()),
                    updated_at_ms: Some(1_700_000_000_000),
                },
            }
        );
    }

    #[test]
    fn jump_latest_request_addresses_the_exact_opened_stream() {
        let request: NativeTimelineJumpLatestRequest = serde_json::from_value(serde_json::json!({
            "streamId": "live:!room:example.org:1"
        }))
        .unwrap();
        assert_eq!(request.stream_id, "live:!room:example.org:1");
    }

    #[test]
    fn typed_open_readback_uses_the_versioned_view_boundary() {
        let readback = NativeTimelineOpenReadback {
            schema_version: NATIVE_TIMELINE_OPEN_SCHEMA_VERSION,
            stream_id: "live:!room:example.org".into(),
            position: TimelineViewPosition::LiveBottom,
            snapshot: TimelineViewSnapshot {
                schema_version: TIMELINE_VIEW_SCHEMA_VERSION,
                session_generation: 7,
                room_id: "!room:example.org".into(),
                revision: 0,
                position: TimelineViewPosition::LiveBottom,
                pagination: TimelinePaginationState {
                    backward: TimelinePageState::Available,
                    forward: TimelinePageState::Available,
                },
                read_state: TimelineReadState {
                    own_read_event_id: None,
                    unread_anchor_event_id: None,
                    is_marked_unread: false,
                },
                pinned_event_ids: vec!["$pinned:example.org".into()],
                rows: Vec::new(),
                capabilities: TimelineViewCapabilities {
                    mark_read: false,
                    mark_unread: false,
                    paginate_backward: true,
                    paginate_forward: true,
                },
            },
        };
        let json = serde_json::to_value(readback).unwrap();
        let snapshot = &json["snapshot"];
        assert_eq!(snapshot["schemaVersion"], TIMELINE_VIEW_SCHEMA_VERSION);
        assert_eq!(snapshot["roomId"], "!room:example.org");
        assert!(snapshot.get("isEncrypted").is_none());
        assert!(snapshot.get("items").is_none());
        assert_eq!(snapshot["readState"]["isMarkedUnread"], false);
        assert_eq!(snapshot["pinnedEventIds"][0], "$pinned:example.org");
        assert_eq!(json["position"]["kind"], "live_bottom");
    }
}
