import Foundation
import SynaraCore

/// P4-S9-13 typed room invite/kick/ban/unban. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the four registered room-moderation commands only.
/// Write ack is status only. Failed errors stay static.
/// Power levels and room create stay off. Leave/join stays on SharedCoreRoomLeaveJoin.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreRoomModeration {
    static func roomInvite(
        core: SharedCore,
        roomId: String,
        userId: String,
        reason: String?
    ) async throws -> RoomModerationWriteDto {
        try await core.roomInvite(roomId: roomId, userId: userId, reason: reason)
    }

    static func roomKick(
        core: SharedCore,
        roomId: String,
        userId: String,
        reason: String?
    ) async throws -> RoomModerationWriteDto {
        try await core.roomKick(roomId: roomId, userId: userId, reason: reason)
    }

    static func roomBan(
        core: SharedCore,
        roomId: String,
        userId: String,
        reason: String?
    ) async throws -> RoomModerationWriteDto {
        try await core.roomBan(roomId: roomId, userId: userId, reason: reason)
    }

    static func roomUnban(
        core: SharedCore,
        roomId: String,
        userId: String
    ) async throws -> RoomModerationWriteDto {
        try await core.roomUnban(roomId: roomId, userId: userId)
    }
}
