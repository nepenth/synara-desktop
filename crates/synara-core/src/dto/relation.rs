//! Relation DTOs — reactions, edits, references, threads (product shapes).

use serde::{Deserialize, Serialize};

use super::ids::{EventId, RoomId, UserId};

/// Well-known relation type strings (product aliases + Matrix rel_type values).
pub const REL_TYPE_ANNOTATION: &str = "annotation";
pub const REL_TYPE_REPLACE: &str = "m.replace";
pub const REL_TYPE_REFERENCE: &str = "m.reference";
pub const REL_TYPE_THREAD: &str = "m.thread";

/// Relation type on the wire is a stable string (`annotation`, `m.replace`, …).
/// Open string so unknown future rel types remain representable.
pub type RelationType = String;

/// Compact relation reference used inside timeline items and standalone streams.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationRef {
    pub rel_type: RelationType,
    /// Target event id.
    pub event_id: EventId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_id: Option<RoomId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender: Option<UserId>,
    /// Reaction key / emoji when `rel_type` is annotation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}
