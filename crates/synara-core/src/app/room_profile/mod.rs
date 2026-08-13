//! P6.5 — Room profile / alias / directory / join-history / upgrade foundation (harness).
//!
//! Pure projection of room presentation state and upgrade pointers. No avatar
//! bytes, no SDK state PUT, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.5-room-profile.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;
mod native;

pub use error::RoomProfileError;
pub use index::{
    DirectoryVisibility, HistoryVisibility, JoinRule, RoomProfile, RoomProfileIndex,
    MAX_ALIAS_CHARS, MAX_ALT_ALIASES, MAX_AVATAR_URL_CHARS, MAX_CACHED_ROOMS, MAX_NAME_CHARS,
    MAX_TOPIC_CHARS,
};
pub use native::{NativeRoomJoinRuleUpdate, ROOM_JOIN_RULE_UPDATED_EVENT};

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
