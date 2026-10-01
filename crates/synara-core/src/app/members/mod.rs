//! P4.6 — Room member / power-level index foundation (harness).
//!
//! Pure projection of Synara [`RoomMember`] DTOs. No SDK member APIs,
//! no production Tauri commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p4.6-members.md`

mod error;
mod index;
mod native;
mod power_levels;
mod snapshots;

pub use error::MemberError;
pub use index::{MemberIndex, MAX_MEMBERS_PER_ROOM};
pub use native::{
    NativePowerLevelWriteResult, NativeRoomCreatorsSnapshot, NativeRoomMembersSnapshot,
    NativeRoomPowerLevelTagsSnapshot, NativeRoomPowerLevelsSnapshot, ROOM_CREATE_EVENT_TYPE,
    ROOM_POWER_LEVELS_EVENT_TYPE, ROOM_POWER_LEVEL_TAGS_EVENT_TYPE,
};
pub use power_levels::{
    validate_power_level_tags_content, validate_room_power_levels_content,
    MAX_POWER_LEVEL_CONTENT_JSON_BYTES,
};
pub use snapshots::{
    parse_room_members_room_id, project_room_creators, project_room_member,
    validate_power_level_tags_snapshot_content, validate_power_levels_snapshot_content,
};

#[cfg(test)]
mod tests;
