//! P6.5 — Room profile / alias / directory / join-history / upgrade foundation (harness).
//!
//! Pure room presentation plus live join-rule ownership. Shells supply the
//! emit sink (desktop Tauri event / later iOS UniFFI). No SDK state PUT.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.5-room-profile.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;
mod live;
mod native;

pub use error::RoomProfileError;
pub use index::{
    DirectoryVisibility, HistoryVisibility, JoinRule, RoomProfile, RoomProfileIndex,
    MAX_ALIAS_CHARS, MAX_ALT_ALIASES, MAX_AVATAR_URL_CHARS, MAX_CACHED_ROOMS, MAX_NAME_CHARS,
    MAX_TOPIC_CHARS,
};
pub use live::{project_join_rule, JoinRuleUpdateEmit, NativeRoomJoinRuleOwner};
pub use native::{
    MatrixRoomDirectoryVisibilityResult, MatrixRoomDirectoryVisibilityWriteResult,
    MatrixRoomJoinRuleSnapshot, NativeRoomJoinRuleUpdate, ROOM_JOIN_RULE_UPDATED_EVENT,
};

/// Static marker for link / schema smoke.
pub const MATRIX_ROOM_PROFILE_MARKER: &str = "matrix-room-profile-p6.5";

/// Touch room-profile paths so they remain linked in non-test builds.
pub fn matrix_room_profile_markers() -> &'static str {
    let idx = RoomProfileIndex::new(0);
    debug_assert!(idx.is_empty());
    debug_assert_eq!(MAX_CACHED_ROOMS, 4_096);
    debug_assert_eq!(JoinRule::Public.as_str(), "public");
    debug_assert_eq!(HistoryVisibility::Shared.as_str(), "shared");
    debug_assert_eq!(MATRIX_ROOM_PROFILE_MARKER, "matrix-room-profile-p6.5");
    MATRIX_ROOM_PROFILE_MARKER
}

#[cfg(test)]
mod tests;
