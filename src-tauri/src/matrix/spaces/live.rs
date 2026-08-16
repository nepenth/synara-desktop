//! Desktop re-export of Core space live Client I/O.

pub use synara_core::app::spaces::{
    remove_space_child, reparent_restricted_join_allow, set_space_child, snapshot_space_children,
    snapshot_space_hierarchy, snapshot_space_parents, NativeRestrictedJoinReparentResult,
    NativeSpaceChildEdge, NativeSpaceChildMutationResult, NativeSpaceChildrenSnapshot,
    NativeSpaceHierarchyRoom, NativeSpaceHierarchySnapshot, NativeSpaceParentEntry,
    NativeSpaceParentsSnapshot,
};
