//! P6.5 — Room profile / alias / directory / join-history / upgrade foundation (harness).
//!
//! Pure room presentation plus live join-rule ownership. Shells supply the
//! emit sink (desktop Tauri event / later iOS UniFFI). No SDK state PUT.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p6.5-room-profile.md`

mod error;
mod index;
mod live;
mod native;
mod retention;

pub use error::RoomProfileError;
pub use index::{
    DirectoryVisibility, HistoryVisibility, JoinRule, RoomProfile, RoomProfileIndex,
    MAX_ALIAS_CHARS, MAX_ALT_ALIASES, MAX_AVATAR_URL_CHARS, MAX_CACHED_ROOMS, MAX_NAME_CHARS,
    MAX_TOPIC_CHARS,
};
pub use live::{project_join_rule, JoinRuleUpdateEmit, NativeRoomJoinRuleOwner};
pub use native::{
    MatrixRoomDirectoryVisibilityResult, MatrixRoomDirectoryVisibilityWriteResult,
    MatrixRoomJoinRuleSnapshot, MatrixRoomRetentionSnapshot, NativeRoomJoinRuleUpdate,
    ROOM_JOIN_RULE_UPDATED_EVENT,
};
pub use retention::{
    format_media_cache_summary, format_retention_copy, format_retention_duration,
    retention_snapshot, RetentionCopy, MEDIA_CACHE_DEFAULT_SUMMARY,
    RETENTION_HISTORY_VISIBILITY_DISTINCTION, RETENTION_NO_LOCAL_COPY, RETENTION_UNKNOWN_SUMMARY,
};

#[cfg(test)]
mod tests;
