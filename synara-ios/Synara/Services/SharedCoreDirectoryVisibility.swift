import Foundation
import SynaraCore

/// P4-S9-10 typed room directory visibility. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the two registered directory-visibility commands only.
/// Visibility is `public` / `private`. Directory search/protocols/cancel stay off.
/// Failed errors stay static. Room name/topic/avatar stays on SharedCoreRoomProfile.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreDirectoryVisibility {
    static func getRoomDirectoryVisibility(
        core: SharedCore,
        roomId: String,
        sessionGeneration: UInt64
    ) async throws -> RoomDirectoryVisibilityDto {
        try await core.getRoomDirectoryVisibility(
            roomId: roomId,
            sessionGeneration: sessionGeneration
        )
    }

    static func setRoomDirectoryVisibility(
        core: SharedCore,
        roomId: String,
        sessionGeneration: UInt64,
        visibility: String
    ) async throws -> RoomDirectoryVisibilityWriteDto {
        try await core.setRoomDirectoryVisibility(
            roomId: roomId,
            sessionGeneration: sessionGeneration,
            visibility: visibility
        )
    }
}
