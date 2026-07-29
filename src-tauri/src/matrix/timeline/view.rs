//! SDK-neutral V-TIMELINE view contract.
//!
//! These DTOs are the target presenter boundary, not a serialization of SDK
//! events. In particular, they exclude Matrix client/room/event objects, raw
//! content, ciphertext, MXC URIs, and media bytes. Projection and the delta
//! subscription are deliberately separate follow-up work; this module fixes
//! the stable shape that those owners must produce.

use matrix_sdk_ui::timeline::{EventTimelineItem, MsgLikeKind, TimelineItemContent};
use serde::{Deserialize, Serialize};

use crate::matrix::dto::{EventId, RoomId, TimelineItemId, UserId};

pub const TIMELINE_VIEW_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineViewPosition {
    LiveBottom,
    Unread { anchor_event_id: EventId },
    Focused { target_event_id: EventId },
    Restored { anchor_event_id: Option<EventId> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelinePageState {
    Available,
    Exhausted,
    Loading,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelinePaginationState {
    pub backward: TimelinePageState,
    pub forward: TimelinePageState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineReadState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own_read_event_id: Option<EventId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unread_anchor_event_id: Option<EventId>,
}

/// Opaque reference resolved only by a bounded native media protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineMediaHandle {
    pub handle_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineReaction {
    pub key: String,
    pub count: u32,
    pub own: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineReplyPreview {
    pub event_id: EventId,
    pub sender_name: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineThreadSummary {
    pub root_event_id: EventId,
    pub reply_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_event_id: Option<EventId>,
}

/// Per-row affordances. `false` means the presenter must not render the
/// action; a missing native command is never represented as a fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineRowCapabilities {
    pub react: bool,
    pub reply: bool,
    pub edit: bool,
    pub redact: bool,
    pub report: bool,
    pub pin: bool,
    pub forward: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineViewCapabilities {
    pub mark_read: bool,
    pub mark_unread: bool,
    pub paginate_backward: bool,
    pub paginate_forward: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEventRowBase {
    pub item_id: TimelineItemId,
    /// Absent only for a local echo which has not received a server event ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<EventId>,
    pub sender_id: UserId,
    pub sender_name: String,
    pub origin_server_ts: u64,
    pub capabilities: TimelineRowCapabilities,
}

/// Project the SDK-owned metadata common to every event row.
///
/// The sender ID is the safe fallback display label until the native profile
/// projection supplies a resolved display name. Every action remains disabled
/// until its specific typed native command and readback exist.
pub fn project_event_row_base(item_id: &str, event: &EventTimelineItem) -> TimelineEventRowBase {
    let sender_id = event.sender().to_string();
    TimelineEventRowBase {
        item_id: item_id.to_owned(),
        event_id: event.event_id().map(ToString::to_string),
        sender_name: sender_id.clone(),
        sender_id,
        origin_server_ts: event.timestamp().get().into(),
        capabilities: TimelineRowCapabilities {
            react: false,
            reply: false,
            edit: false,
            redact: false,
            report: false,
            pin: false,
            forward: false,
        },
    }
}

pub fn project_event_row(item_id: &str, event: &EventTimelineItem) -> TimelineViewRow {
    let base = project_event_row_base(item_id, event);
    match event.content() {
        TimelineItemContent::MsgLike(content) => match &content.kind {
            MsgLikeKind::Message(message) => TimelineViewRow::Message(TimelineMessageRow {
                event: base,
                body: message.body().to_owned(),
                formatted_body: None,
                message_type: None,
                edited: message.is_edited(),
                reply: None,
                thread: None,
                reactions: Vec::new(),
                media: None,
            }),
            MsgLikeKind::Redacted => match base.event_id.clone() {
                Some(event_id) => TimelineViewRow::Redacted(TimelineRedactedRow {
                    item_id: base.item_id,
                    event_id,
                    summary: "Message removed".to_owned(),
                }),
                None => other_row(item_id, None, "Redacted local event"),
            },
            MsgLikeKind::UnableToDecrypt(_) => match base.event_id.clone() {
                Some(event_id) => {
                    TimelineViewRow::EncryptedUnavailable(TimelineEncryptedUnavailableRow {
                        item_id: base.item_id,
                        event_id,
                        reason_code: "unable_to_decrypt".to_owned(),
                    })
                }
                None => other_row(item_id, None, "Encrypted local event"),
            },
            _ => other_row(item_id, base.event_id, "Unsupported timeline event"),
        },
        _ => other_row(item_id, base.event_id, "Unsupported timeline event"),
    }
}

fn other_row(item_id: &str, event_id: Option<EventId>, summary: &str) -> TimelineViewRow {
    TimelineViewRow::Other(TimelineOtherRow {
        item_id: item_id.to_owned(),
        event_id,
        event_type: None,
        summary: summary.to_owned(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineMessageRow {
    #[serde(flatten)]
    pub event: TimelineEventRowBase,
    pub body: String,
    /// Already-sanitized rendering markup; never raw event content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_type: Option<String>,
    pub edited: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply: Option<TimelineReplyPreview>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread: Option<TimelineThreadSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactions: Vec<TimelineReaction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<TimelineMediaHandle>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelinePollRow {
    #[serde(flatten)]
    pub event: TimelineEventRowBase,
    pub question: String,
    pub closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineMembershipRow {
    #[serde(flatten)]
    pub event: TimelineEventRowBase,
    pub target_user_id: UserId,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineStateRow {
    #[serde(flatten)]
    pub event: TimelineEventRowBase,
    pub state_type: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineCallRow {
    #[serde(flatten)]
    pub event: TimelineEventRowBase,
    pub call_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineRedactedRow {
    pub item_id: TimelineItemId,
    pub event_id: EventId,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEncryptedUnavailableRow {
    pub item_id: TimelineItemId,
    pub event_id: EventId,
    pub reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineOtherRow {
    pub item_id: TimelineItemId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<EventId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineViewRow {
    Message(TimelineMessageRow),
    Sticker {
        event: TimelineEventRowBase,
        media: TimelineMediaHandle,
    },
    Poll(TimelinePollRow),
    Membership(TimelineMembershipRow),
    State(TimelineStateRow),
    Call(TimelineCallRow),
    Redacted(TimelineRedactedRow),
    EncryptedUnavailable(TimelineEncryptedUnavailableRow),
    Other(TimelineOtherRow),
    DateSeparator {
        item_id: TimelineItemId,
        day_key: String,
    },
    ReadMarker {
        item_id: TimelineItemId,
    },
    UnreadMarker {
        item_id: TimelineItemId,
    },
    TimelineStart {
        item_id: TimelineItemId,
    },
    Pagination {
        item_id: TimelineItemId,
        direction: String,
        state: TimelinePageState,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineViewSnapshot {
    pub schema_version: u32,
    pub session_generation: u64,
    pub room_id: RoomId,
    pub revision: u64,
    pub position: TimelineViewPosition,
    pub pagination: TimelinePaginationState,
    pub read_state: TimelineReadState,
    pub rows: Vec<TimelineViewRow>,
    pub capabilities: TimelineViewCapabilities,
}
