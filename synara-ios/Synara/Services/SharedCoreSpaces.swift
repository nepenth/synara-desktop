import Foundation
import SynaraCore

/// P4-S9-17 typed spaces. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the six registered space snapshot/mutation commands only.
/// Child set/remove carry room ids, via, order, and suggested. No bytes.
/// Failed errors stay static and must not echo room ids.
/// Invite accept/decline stay off. Members snapshots stay on
/// SharedCoreRoomMembersSnapshots.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreSpaces {
    static func spaceParentsSnapshot(
        core: SharedCore
    ) async throws -> SpaceParentsSnapshotDto {
        try await core.spaceParentsSnapshot()
    }

    static func spaceHierarchySnapshot(
        core: SharedCore,
        roomId: String
    ) async throws -> SpaceHierarchySnapshotDto {
        try await core.spaceHierarchySnapshot(roomId: roomId)
    }

    static func spaceChildrenSnapshot(
        core: SharedCore
    ) async throws -> SpaceChildrenSnapshotDto {
        try await core.spaceChildrenSnapshot()
    }

    static func spaceChildSet(
        core: SharedCore,
        parentId: String,
        childId: String,
        via: [String],
        order: String?,
        suggested: Bool?
    ) async throws -> SpaceChildMutationDto {
        try await core.spaceChildSet(
            parentId: parentId,
            childId: childId,
            via: via,
            order: order,
            suggested: suggested
        )
    }

    static func spaceChildRemove(
        core: SharedCore,
        parentId: String,
        childId: String
    ) async throws -> SpaceChildMutationDto {
        try await core.spaceChildRemove(parentId: parentId, childId: childId)
    }

    static func restrictedJoinReparent(
        core: SharedCore,
        roomId: String,
        removeParentId: String?,
        addParentId: String
    ) async throws -> RestrictedJoinReparentDto {
        try await core.restrictedJoinReparent(
            roomId: roomId,
            removeParentId: removeParentId,
            addParentId: addParentId
        )
    }
}
