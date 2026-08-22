import Foundation
import SynaraCore

enum SharedCoreRoomNotification {
    static func snapshot(core: SharedCore, roomId: String) async throws -> RoomNotificationSnapshotDto {
        try await core.roomNotificationSnapshot(roomId: roomId)
    }

    static func set(core: SharedCore, roomId: String, mode: String) async throws {
        _ = try await core.roomNotificationSet(roomId: roomId, mode: mode)
    }

    static func snapshotAll(core: SharedCore) async throws -> RoomNotificationsSnapshotDto {
        try await core.roomNotificationsSnapshot()
    }
}
