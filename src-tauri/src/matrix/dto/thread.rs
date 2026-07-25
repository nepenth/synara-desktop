//! Thread summary DTO.

use serde::{Deserialize, Serialize};

use super::ids::{EventId, RoomId};

/// Thread root summary for timeline / list UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadSummary {
    pub room_id: RoomId,
    pub root_event_id: EventId,
    pub reply_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_event_id: Option<EventId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_origin_server_ts: Option<u64>,
    pub participated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unread_count: Option<u32>,
}
