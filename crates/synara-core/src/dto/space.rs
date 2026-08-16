//! Space hierarchy summary DTO.

use serde::{Deserialize, Serialize};

use super::ids::RoomId;

/// Child room/space edge in a space hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceChild {
    pub room_id: RoomId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested: Option<bool>,
}

/// Space summary for lobby / hierarchy UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceSummary {
    pub room_id: RoomId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub children: Vec<SpaceChild>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_room_ids: Option<Vec<RoomId>>,
}
