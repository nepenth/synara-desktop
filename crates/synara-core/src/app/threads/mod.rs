//! P5.8 — Thread list / summary index, plus the 0.19 ThreadListService owner.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p5.8-threads.md`

#![allow(unused_imports)]

mod error;
mod index;
mod list;

pub use error::ThreadError;
pub use index::{ThreadIndex, MAX_THREADS_PER_ROOM};
pub use list::{
    rebuild_thread_index, thread_summary_from_list_item, NativeThreadListRequest,
    NativeThreadListSnapshot, ThreadListItemProjection, NATIVE_THREAD_LIST_SCHEMA_VERSION,
};

/// Static marker for link / schema smoke.
pub const MATRIX_THREADS_MARKER: &str = "matrix-threads-p5.8";

/// Touch thread paths so they remain linked in non-test builds.
pub fn matrix_threads_markers() -> &'static str {
    let idx = ThreadIndex::new(0);
    debug_assert!(idx.is_empty());
    debug_assert_eq!(idx.thread_count(), 0);
    debug_assert_eq!(MATRIX_THREADS_MARKER, "matrix-threads-p5.8");
    debug_assert_eq!(NATIVE_THREAD_LIST_SCHEMA_VERSION, 1);
    MATRIX_THREADS_MARKER
}

#[cfg(test)]
mod tests;
