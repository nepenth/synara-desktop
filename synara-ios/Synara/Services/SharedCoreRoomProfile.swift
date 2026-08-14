import Foundation
import SynaraCore

/// P4-S9-9 typed room name / topic / avatar. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the three registered room-profile commands only.
/// Room avatar is an `mxc://` (or empty clear) reference. Image bytes stay off.
/// Failed errors stay static. Directory visibility stays off.
/// Join-rule snapshot stays on SharedCoreJoinRules.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreRoomProfile {
    static func setRoomName(
        core: SharedCore,
        roomId: String,
        name: String
    ) async throws -> RoomProfileWriteDto {
        try await core.setRoomName(roomId: roomId, name: name)
    }

    static func setRoomTopic(
        core: SharedCore,
        roomId: String,
        topic: String
    ) async throws -> RoomProfileWriteDto {
        try await core.setRoomTopic(roomId: roomId, topic: topic)
    }

    static func setRoomAvatar(
        core: SharedCore,
        roomId: String,
        mxc: String
    ) async throws -> RoomProfileWriteDto {
        try await core.setRoomAvatar(roomId: roomId, mxc: mxc)
    }
}
