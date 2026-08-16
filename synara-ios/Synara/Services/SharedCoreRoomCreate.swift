import Foundation
import SynaraCore

/// P4-S9-15 typed room create. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the registered `matrix_room_create` command only.
/// Request is name/topic/alias/visibility/preset plus Core scalar extras.
/// creation_content, power_level_content_override, paths, passphrases, and
/// media bytes stay off. Success returns the created room id.
/// Failed errors stay static. Members snapshots and spaces stay off.
/// Power levels stay on SharedCoreRoomPowerLevels.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreRoomCreate {
    static func roomCreate(
        core: SharedCore,
        request: RoomCreateRequestDto
    ) async throws -> RoomCreateDto {
        try await core.roomCreate(request: request)
    }
}
