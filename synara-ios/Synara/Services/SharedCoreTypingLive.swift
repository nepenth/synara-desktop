import Foundation
import SynaraCore

/// P4-S21 map of privacy-safe SharedCore typing rooms to product user ids.
///
/// Uses the existing snapshot command only. Owner wake-ups never include
/// user ids. This is not iOS-on-engine and not P4 acceptance.
enum SharedCoreTypingLive {
    static func users(roomID: String, from snapshot: TypingSnapshotDto) -> [String] {
        users(
            roomID: roomID,
            rooms: snapshot.rooms.map { (roomId: $0.roomId, userIds: $0.userIds) }
        )
    }

    static func users(roomID: String, rooms: [(roomId: String, userIds: [String])]) -> [String] {
        rooms.first(where: { $0.roomId == roomID })?.userIds ?? []
    }

    static func shouldRefresh(watchingRoomID: String, updateRoomId: String?) -> Bool {
        guard let updateRoomId, updateRoomId.isEmpty == false else {
            return true
        }
        return updateRoomId == watchingRoomID
    }
}
