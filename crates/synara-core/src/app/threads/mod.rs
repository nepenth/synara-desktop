//! P5.8 — Thread list / summary index, plus the 0.19 ThreadListService owner.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p5.8-threads.md`

mod error;
mod index;
mod list;

pub use error::ThreadError;
pub use index::{ThreadIndex, MAX_THREADS_PER_ROOM};
pub use list::{
    rebuild_thread_index, thread_summary_from_list_item, NativeThreadListRequest,
    NativeThreadListSnapshot, ThreadListItemProjection, NATIVE_THREAD_LIST_SCHEMA_VERSION,
};

#[cfg(test)]
mod tests;
