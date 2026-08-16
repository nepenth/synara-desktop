//! P7.3 — Media cache / retention index foundation (harness).
//!
//! Tracks local media handles with size + last-access for LRU eviction and
//! privacy purge. No file bytes, no disk I/O, no SDK media network, no
//! dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p7.3-media-cache.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;

pub use error::MediaCacheError;
pub use index::{CacheEntry, MediaCacheIndex, MAX_CACHE_ENTRIES, MAX_ID_CHARS, MAX_TOTAL_BYTES};

/// Static marker for link / schema smoke.
pub const MATRIX_MEDIA_CACHE_MARKER: &str = "matrix-media-cache-p7.3";

/// Touch media-cache paths so they remain linked in non-test builds.
pub fn matrix_media_cache_markers() -> &'static str {
    let idx = MediaCacheIndex::new(0);
    debug_assert!(idx.is_empty());
    debug_assert_eq!(MAX_CACHE_ENTRIES, 4_096);
    debug_assert_eq!(MATRIX_MEDIA_CACHE_MARKER, "matrix-media-cache-p7.3");
    MATRIX_MEDIA_CACHE_MARKER
}

#[cfg(test)]
mod tests;
