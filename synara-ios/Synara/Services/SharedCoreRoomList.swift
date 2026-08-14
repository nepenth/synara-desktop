import Foundation
import SynaraCore

/// P4-S4 typed room-list snapshot. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps `matrix_room_list_snapshot` only. It is not a generic
/// `Core.command` FFI, not invites, and not a product room-list swap.
enum SharedCoreRoomList {
    static func roomListSnapshot(core: SharedCore) async throws -> RoomListSnapshotDto {
        try await core.roomListSnapshot()
    }
}
