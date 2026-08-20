import Foundation
import SynaraCore

/// P4-S9-12 typed room leave/join. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the registered room-membership and favorite-tag commands.
/// Write ack is status only. Failed errors stay static.
/// Invite/kick/ban stay off. Directory search stays on SharedCoreDirectorySearch.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreRoomLeaveJoin {
    static func roomLeave(
        core: SharedCore,
        roomId: String
    ) async throws -> RoomMembershipWriteDto {
        try await core.roomLeave(roomId: roomId)
    }

    static func roomJoin(
        core: SharedCore,
        roomIdOrAlias: String,
        viaServers: [String]?
    ) async throws -> RoomMembershipWriteDto {
        try await core.roomJoin(roomIdOrAlias: roomIdOrAlias, viaServers: viaServers)
    }

    static func roomSetFavorite(
        core: SharedCore,
        roomId: String,
        favorite: Bool
    ) async throws -> RoomMembershipWriteDto {
        try await core.roomSetFavorite(roomId: roomId, favorite: favorite)
    }
}
