import Foundation
import SynaraCore

/// P4-S9-4 typed image-pack get/set. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the six registered image-pack commands only.
/// Pack metadata/IDs/URLs/JSON may cross. Image/media bytes stay off.
/// Later, m.direct, room notes, and profile writes stay off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreImagePacks {
    static func getGlobalImagePacks(core: SharedCore) async throws -> GlobalImagePacksSnapshotDto {
        try await core.getGlobalImagePacks()
    }

    static func getUserImagePack(core: SharedCore) async throws -> UserImagePackSnapshotDto {
        try await core.getUserImagePack()
    }

    static func getRoomImagePacks(core: SharedCore, roomId: String) async throws -> RoomImagePacksSnapshotDto {
        try await core.getRoomImagePacks(roomId: roomId)
    }

    static func setUserImagePack(core: SharedCore, contentJson: String) async throws -> ImagePackWriteDto {
        try await core.setUserImagePack(contentJson: contentJson)
    }

    static func setGlobalImagePacks(core: SharedCore, contentJson: String) async throws -> ImagePackWriteDto {
        try await core.setGlobalImagePacks(contentJson: contentJson)
    }

    static func setRoomImagePack(
        core: SharedCore,
        roomId: String,
        stateKey: String,
        contentJson: String
    ) async throws -> ImagePackWriteDto {
        try await core.setRoomImagePack(roomId: roomId, stateKey: stateKey, contentJson: contentJson)
    }
}
