//! Timeline item DTOs — product virtualization rows (not MatrixEvent graphs).
//!
//! Wire: internally tagged on `kind` (snake_case). Fields camelCase.
//! Body fields are plain-text previews only; no ciphertext dumps required.

use serde::{Deserialize, Serialize};

use super::ids::{EventId, RoomId, TimelineItemId, UserId};
use super::relation::RelationRef;

/// Local-echo / send-queue state for outbound messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalEchoState {
    Sending,
    Sent,
    Failed,
    Cancelled,
}

/// Product timeline row (exhaustive tagged union).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineItem {
    Message(TimelineMessageItem),
    State(TimelineStateItem),
    Membership(TimelineMembershipItem),
    ReactionSummary(TimelineReactionSummaryItem),
    Redacted(TimelineRedactedItem),
    EncryptedUnavailable(TimelineEncryptedUnavailableItem),
    DateSeparator(TimelineDateSeparatorItem),
    ReadMarker(TimelineReadMarkerItem),
    Other(TimelineOtherItem),
}

impl TimelineItem {
    pub fn item_id(&self) -> &str {
        match self {
            Self::Message(i) => &i.item_id,
            Self::State(i) => &i.item_id,
            Self::Membership(i) => &i.item_id,
            Self::ReactionSummary(i) => &i.item_id,
            Self::Redacted(i) => &i.item_id,
            Self::EncryptedUnavailable(i) => &i.item_id,
            Self::DateSeparator(i) => &i.item_id,
            Self::ReadMarker(i) => &i.item_id,
            Self::Other(i) => &i.item_id,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Message(_) => "message",
            Self::State(_) => "state",
            Self::Membership(_) => "membership",
            Self::ReactionSummary(_) => "reaction_summary",
            Self::Redacted(_) => "redacted",
            Self::EncryptedUnavailable(_) => "encrypted_unavailable",
            Self::DateSeparator(_) => "date_separator",
            Self::ReadMarker(_) => "read_marker",
            Self::Other(_) => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineMessageItem {
    pub item_id: TimelineItemId,
    pub event_id: EventId,
    pub room_id: RoomId,
    pub sender: UserId,
    pub origin_server_ts: u64,
    /// Plain-text preview body only (already redacted/privacy-filtered if needed).
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msgtype: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relates_to: Option<RelationRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_echo_state: Option<LocalEchoState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_edited: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_redacted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_root_id: Option<EventId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineStateItem {
    pub item_id: TimelineItemId,
    pub event_id: EventId,
    pub room_id: RoomId,
    pub sender: UserId,
    pub origin_server_ts: u64,
    pub state_key: String,
    /// Stable type string (e.g. `m.room.name`); not an SDK type object.
    pub state_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineMembershipItem {
    pub item_id: TimelineItemId,
    pub event_id: EventId,
    pub room_id: RoomId,
    pub sender: UserId,
    pub origin_server_ts: u64,
    pub target_user_id: UserId,
    /// Short membership-change summary for UI (e.g. "joined", "invited").
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineReactionSummaryItem {
    pub item_id: TimelineItemId,
    /// Target event the reactions annotate.
    pub event_id: EventId,
    pub room_id: RoomId,
    pub key: String,
    pub count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub me: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineRedactedItem {
    pub item_id: TimelineItemId,
    pub event_id: EventId,
    pub room_id: RoomId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redacted_by: Option<EventId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEncryptedUnavailableItem {
    pub item_id: TimelineItemId,
    pub event_id: EventId,
    pub room_id: RoomId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineDateSeparatorItem {
    pub item_id: TimelineItemId,
    /// Stable day key for virtualization (e.g. `2026-07-24`).
    pub day_key: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineReadMarkerItem {
    pub item_id: TimelineItemId,
}

/// Safe fallback for unknown/unsupported event shapes — no raw full content dump.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineOtherItem {
    pub item_id: TimelineItemId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<EventId>,
    /// Matrix event type string when known (e.g. `m.sticker`).
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}
