//! P4.5 space hierarchy foundation + V-ROOMS.2a live parent-map ownership.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p4.5-spaces.md`
//! Product vertical: `docs/matrix-rust-sdk/v-rooms-2a-space-parents.md`

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod hierarchy;
pub mod live;

pub use error::SpaceError;
pub use hierarchy::{space_child, SpaceHierarchy};
pub use live::{snapshot_space_parents, NativeSpaceParentEntry, NativeSpaceParentsSnapshot};

/// Static marker for link / schema smoke.
pub const MATRIX_SPACES_MARKER: &str = "matrix-spaces-hierarchy-p4.5";

/// Touch space foundation paths so they remain linked in non-test builds.
pub fn matrix_spaces_markers() -> &'static str {
    let h = SpaceHierarchy::new();
    debug_assert!(h.is_empty());
    debug_assert_eq!(MATRIX_SPACES_MARKER, "matrix-spaces-hierarchy-p4.5");
    MATRIX_SPACES_MARKER
}

#[cfg(test)]
mod tests;
