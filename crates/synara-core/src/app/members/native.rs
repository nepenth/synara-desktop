//! Credential-free V-ROOMS.MEMBERS presentation DTOs.
//!
//! Live Client member/power-level I/O stays in the desktop shell.

use serde::{Deserialize, Serialize};

use crate::dto::RoomMember;

pub const ROOM_POWER_LEVELS_EVENT_TYPE: &str = "m.room.power_levels";
pub const ROOM_CREATE_EVENT_TYPE: &str = "m.room.create";
pub const ROOM_POWER_LEVEL_TAGS_EVENT_TYPE: &str = "in.synara.room.power_level_tags";

/// V-ROOMS.R-MEMBERS-READ — live native room-member projection.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomMembersSnapshot {
    pub session_generation: u64,
    pub room_id: String,
    pub members: Vec<RoomMember>,
}

/// V-ROOMS.MEMBERS-READ — live native room power-level projection.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomPowerLevelsSnapshot {
    #[cfg_attr(feature = "ts-export", ts(type = "\"ok\""))]
    pub status: &'static str,
    pub session_generation: u64,
    pub room_id: String,
    #[cfg_attr(feature = "ts-export", ts(type = "\"m.room.power_levels\""))]
    pub event_type: &'static str,
    #[cfg_attr(feature = "ts-export", ts(type = "\"\""))]
    pub state_key: &'static str,
    pub content: serde_json::Value,
    /// The signed-in user's permissions under these levels, evaluated with
    /// the room version's rules. `None` when the SDK could not load them.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-export", ts(optional))]
    pub capabilities: Option<super::RoomPermissionCapabilities>,
}

/// V-ROOMS.MEMBERS-READ — live native room creator projection.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomCreatorsSnapshot {
    #[cfg_attr(feature = "ts-export", ts(type = "\"ok\""))]
    pub status: &'static str,
    pub session_generation: u64,
    pub room_id: String,
    #[cfg_attr(feature = "ts-export", ts(type = "\"m.room.create\""))]
    pub event_type: &'static str,
    #[cfg_attr(feature = "ts-export", ts(type = "\"\""))]
    pub state_key: &'static str,
    pub creators: Vec<String>,
}

/// V-ROOMS.MEMBERS-READ — live native custom power-level tag projection.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomPowerLevelTagsSnapshot {
    #[cfg_attr(feature = "ts-export", ts(type = "\"ok\""))]
    pub status: &'static str,
    pub session_generation: u64,
    pub room_id: String,
    #[cfg_attr(
        feature = "ts-export",
        ts(type = "\"in.synara.room.power_level_tags\"")
    )]
    pub event_type: &'static str,
    #[cfg_attr(feature = "ts-export", ts(type = "\"\""))]
    pub state_key: &'static str,
    pub content: serde_json::Value,
}

/// V-ROOMS.R-POWERS-BULK — acknowledged complete state replacement.
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativePowerLevelWriteResult {
    #[cfg_attr(feature = "ts-export", ts(type = "\"ok\""))]
    pub status: &'static str,
    pub room_id: String,
    #[cfg_attr(
        feature = "ts-export",
        ts(type = "\"m.room.power_levels\" | \"in.synara.room.power_level_tags\"")
    )]
    pub event_type: &'static str,
    #[cfg_attr(feature = "ts-export", ts(type = "\"\""))]
    pub state_key: &'static str,
    pub session_generation: u64,
    pub content: serde_json::Value,
}
