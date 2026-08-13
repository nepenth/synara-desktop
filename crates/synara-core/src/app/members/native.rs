//! Credential-free V-ROOMS.MEMBERS presentation DTOs.
//!
//! Live Client member/power-level I/O stays in the desktop shell.

use serde::Serialize;

use crate::dto::RoomMember;

pub const ROOM_POWER_LEVELS_EVENT_TYPE: &str = "m.room.power_levels";
pub const ROOM_CREATE_EVENT_TYPE: &str = "m.room.create";
pub const ROOM_POWER_LEVEL_TAGS_EVENT_TYPE: &str = "in.synara.room.power_level_tags";

/// V-ROOMS.R-MEMBERS-READ — live native room-member projection.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomMembersSnapshot {
    pub session_generation: u64,
    pub room_id: String,
    pub members: Vec<RoomMember>,
}

/// V-ROOMS.MEMBERS-READ — live native room power-level projection.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomPowerLevelsSnapshot {
    pub status: &'static str,
    pub session_generation: u64,
    pub room_id: String,
    pub event_type: &'static str,
    pub state_key: &'static str,
    pub content: serde_json::Value,
}

/// V-ROOMS.MEMBERS-READ — live native room creator projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomCreatorsSnapshot {
    pub status: &'static str,
    pub session_generation: u64,
    pub room_id: String,
    pub event_type: &'static str,
    pub state_key: &'static str,
    pub creators: Vec<String>,
}

/// V-ROOMS.MEMBERS-READ — live native custom power-level tag projection.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeRoomPowerLevelTagsSnapshot {
    pub status: &'static str,
    pub session_generation: u64,
    pub room_id: String,
    pub event_type: &'static str,
    pub state_key: &'static str,
    pub content: serde_json::Value,
}

/// V-ROOMS.R-POWERS-BULK — acknowledged complete state replacement.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativePowerLevelWriteResult {
    pub status: &'static str,
    pub room_id: String,
    pub event_type: &'static str,
    pub state_key: &'static str,
    pub session_generation: u64,
    pub content: serde_json::Value,
}
