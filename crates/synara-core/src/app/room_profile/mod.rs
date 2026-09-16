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
mod retention;

pub use error::RoomProfileError;
pub use index::{
    DirectoryVisibility, HistoryVisibility, JoinRule, MAX_ALIAS_CHARS, MAX_ALT_ALIASES,
    MAX_AVATAR_URL_CHARS, MAX_CACHED_ROOMS, MAX_NAME_CHARS, MAX_TOPIC_CHARS, RoomProfile,
    RoomProfileIndex,
};
pub use live::{JoinRuleUpdateEmit, NativeRoomJoinRuleOwner, project_join_rule};
pub use native::{
    MatrixRoomDirectoryVisibilityResult, MatrixRoomDirectoryVisibilityWriteResult,
    MatrixRoomJoinRuleSnapshot, MatrixRoomRetentionSnapshot, NativeRoomJoinRuleUpdate,
    ROOM_JOIN_RULE_UPDATED_EVENT,
};
pub use retention::{
    MEDIA_CACHE_DEFAULT_SUMMARY, RETENTION_HISTORY_VISIBILITY_DISTINCTION, RETENTION_NO_LOCAL_COPY,
    RETENTION_UNKNOWN_SUMMARY, RetentionCopy, format_media_cache_summary, format_retention_copy,
    format_retention_duration, retention_snapshot,
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
