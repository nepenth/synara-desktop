//! P5.8 — Thread list / summary index foundation (harness).
//!
//! Pure projection of Synara [`ThreadSummary`] DTOs. No SDK thread APIs,
//! no production Tauri commands, no dual-backend.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p5.8-threads.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod index;

pub use error::ThreadError;
pub use index::{ThreadIndex, MAX_THREADS_PER_ROOM};

/// Static marker for link / schema smoke.
pub const MATRIX_THREADS_MARKER: &str = "matrix-threads-p5.8";

/// Touch thread paths so they remain linked in non-test builds.
pub fn matrix_threads_markers() -> &'static str {
    let idx = ThreadIndex::new(0);
    debug_assert!(idx.is_empty());
    debug_assert_eq!(idx.thread_count(), 0);
    debug_assert_eq!(MATRIX_THREADS_MARKER, "matrix-threads-p5.8");
    MATRIX_THREADS_MARKER
}

#[cfg(test)]
mod tests;
