import Foundation
import SynaraCore

/// P4-S9-16 typed room members snapshots. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the four registered members-snapshot read commands only.
/// Failed errors stay static and must not echo member user ids.
/// Spaces stay off. Room create stays on SharedCoreRoomCreate.
/// Power-level writers stay on SharedCoreRoomPowerLevels.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreRoomMembersSnapshots {
    static func roomMembersSnapshot(
        core: SharedCore,
        roomId: String
    ) async throws -> RoomMembersSnapshotDto {
        try await core.roomMembersSnapshot(roomId: roomId)
    }

    static func roomPowerLevelsSnapshot(
        core: SharedCore,
        roomId: String
    ) async throws -> RoomPowerLevelsSnapshotDto {
        try await core.roomPowerLevelsSnapshot(roomId: roomId)
    }

    static func roomCreatorsSnapshot(
        core: SharedCore,
        roomId: String
    ) async throws -> RoomCreatorsSnapshotDto {
        try await core.roomCreatorsSnapshot(roomId: roomId)
    }

    static func roomPowerLevelTagsSnapshot(
        core: SharedCore,
        roomId: String
    ) async throws -> RoomPowerLevelTagsSnapshotDto {
        try await core.roomPowerLevelTagsSnapshot(roomId: roomId)
    }
}
