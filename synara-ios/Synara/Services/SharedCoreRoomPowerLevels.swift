import Foundation
import SynaraCore

/// P4-S9-14 typed room power-level writers. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the three registered power-level write commands only.
/// Write ack is status only. Failed errors stay static.
/// Room create, members snapshots, and spaces stay off.
/// Invite/kick/ban stays on SharedCoreRoomModeration.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreRoomPowerLevels {
    static func roomSetPowerLevel(
        core: SharedCore,
        roomId: String,
        userId: String,
        powerLevel: Int64
    ) async throws -> RoomPowerLevelWriteDto {
        try await core.roomSetPowerLevel(roomId: roomId, userId: userId, powerLevel: powerLevel)
    }

    static func roomSetPowerLevels(
        core: SharedCore,
        roomId: String,
        contentJson: String
    ) async throws -> RoomPowerLevelWriteDto {
        try await core.roomSetPowerLevels(roomId: roomId, contentJson: contentJson)
    }

    static func roomSetPowerLevelTags(
        core: SharedCore,
        roomId: String,
        contentJson: String
    ) async throws -> RoomPowerLevelWriteDto {
        try await core.roomSetPowerLevelTags(roomId: roomId, contentJson: contentJson)
    }
}
