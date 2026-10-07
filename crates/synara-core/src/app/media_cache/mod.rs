//! P7.3 — Media cache / retention index foundation (harness).
//!
//! Tracks local media handles with size + last-access for LRU eviction and
//! privacy purge. No file bytes, no disk I/O, no SDK media network, no
//! dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p7.3-media-cache.md`

mod error;
mod index;
mod live;
mod policy;

pub use error::MediaCacheError;
pub use index::{CacheEntry, MediaCacheIndex, MAX_CACHE_ENTRIES, MAX_ID_CHARS, MAX_TOTAL_BYTES};
pub use live::{
    joined_room_max_lifetimes, shortest_joined_room_max_lifetime, NativeMediaRetentionOwner,
};
pub use policy::{
    build_media_retention_policy_spec, shortest_joined_max_lifetime, MediaRetentionPolicySpec,
    DEFAULT_CLEANUP_FREQUENCY, DEFAULT_LAST_ACCESS_EXPIRY, MEDIA_STORE_MAX_CACHE_BYTES,
    MEDIA_STORE_MAX_FILE_BYTES,
};

#[cfg(test)]
mod tests;
