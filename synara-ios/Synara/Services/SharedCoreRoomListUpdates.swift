import Foundation
import SynaraCore

/// P4-S19 poll of queued room-list wake-ups.
/// Uses an already-constructed SharedCore.
///
/// Drains session-generation pings only. iOS re-fetches via the existing
/// snapshot command. Room ids and names are never included. This is not
/// `Platform.emit`. NSE still cannot poll.
enum SharedCoreRoomListUpdates {
    static func poll(core: SharedCore) async throws -> [RoomListUpdateDto] {
        try await core.pollRoomListUpdates()
    }
}
