//! Privacy-conscious poll and state projection models.
//!
//! Rows intentionally do not implement `Debug` or `Display`: question and
//! summary text may contain event plaintext and must not be formatted into logs.

use std::collections::BTreeMap;

use crate::dto::{EventId, RoomId};

/// Current projection of one poll start plus its response/end state.
#[derive(Clone, PartialEq, Eq)]
pub struct PollProjection {
    pub poll_event_id: EventId,
    pub room_id: RoomId,
    pub question: String,
    pub closed: bool,
    /// Stable answer-id ordering for deterministic consumers.
    pub response_counts: BTreeMap<String, u64>,
}

/// Coarse state/membership kind safe for projection consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StateProjectionKind {
    MemberJoin,
    MemberLeave,
    MemberBan,
    Name,
    Topic,
    Other,
}

/// One simple room state or membership summary.
///
/// This type intentionally has no `Debug`/`Display` implementation because
/// `summary` may contain event plaintext.
#[derive(Clone, PartialEq, Eq)]
pub struct StateProjectionRow {
    pub room_id: RoomId,
    pub event_id: EventId,
    pub kind: StateProjectionKind,
    /// Matrix user localpart only, never a full MXID.
    pub target_user_localpart: Option<String>,
    pub summary: String,
}
