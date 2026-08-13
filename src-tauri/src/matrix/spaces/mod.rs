//! P4.5 space hierarchy foundation + V-ROOMS.2 live ownership.

#![allow(dead_code)]
#![allow(unused_imports)]

pub use synara_core::app::spaces::*;

pub mod live;
pub use live::{
    remove_space_child, reparent_restricted_join_allow, set_space_child, snapshot_space_children,
    snapshot_space_hierarchy, snapshot_space_parents, NativeRestrictedJoinReparentResult,
    NativeSpaceChildEdge, NativeSpaceChildMutationResult, NativeSpaceChildrenSnapshot,
    NativeSpaceHierarchyRoom, NativeSpaceHierarchySnapshot, NativeSpaceParentEntry,
    NativeSpaceParentsSnapshot,
};
