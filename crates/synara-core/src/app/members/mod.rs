//! P4.6 — Room member / power-level index foundation (harness).
//!
//! Pure projection of Synara [`RoomMember`] DTOs. No SDK member APIs,
//! no production Tauri commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p4.6-members.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;
mod native;

pub use error::MemberError;
pub use index::{MemberIndex, MAX_MEMBERS_PER_ROOM};
pub use native::{
    NativePowerLevelWriteResult, NativeRoomCreatorsSnapshot, NativeRoomMembersSnapshot,
    NativeRoomPowerLevelTagsSnapshot, NativeRoomPowerLevelsSnapshot, ROOM_CREATE_EVENT_TYPE,
    ROOM_POWER_LEVELS_EVENT_TYPE, ROOM_POWER_LEVEL_TAGS_EVENT_TYPE,
};

/// Static marker for link / schema smoke.
pub const MATRIX_MEMBERS_MARKER: &str = "matrix-members-p4.6";

/// Touch member paths so they remain linked in non-test builds.
pub fn matrix_members_markers() -> &'static str {
    let idx = MemberIndex::new(0);
    debug_assert!(idx.is_empty());
    debug_assert_eq!(idx.room_count(), 0);
    debug_assert_eq!(MATRIX_MEMBERS_MARKER, "matrix-members-p4.6");
    MATRIX_MEMBERS_MARKER
}

#[cfg(test)]
mod tests;
