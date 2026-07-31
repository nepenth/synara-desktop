//! P4.5 space hierarchy foundation + V-ROOMS.2 live ownership.
//!
//! Authoritative design note: `docs/matrix-rust-sdk/p4.5-spaces.md`
//! Product verticals: V-ROOMS.2a parents, V-ROOMS.2b hierarchy, V-ROOMS.2c local graph/writers.

#![allow(dead_code)]
#![allow(unused_imports)]

mod error;
mod hierarchy;
pub mod live;

pub use error::SpaceError;
pub use hierarchy::{space_child, SpaceHierarchy};
pub use live::{
    reparent_restricted_join_allow, remove_space_child, set_space_child, snapshot_space_children,
    snapshot_space_hierarchy, snapshot_space_parents, NativeRestrictedJoinReparentResult,
    NativeSpaceChildEdge, NativeSpaceChildMutationResult, NativeSpaceChildrenSnapshot,
    NativeSpaceHierarchyRoom, NativeSpaceHierarchySnapshot, NativeSpaceParentEntry,
    NativeSpaceParentsSnapshot,
};

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
