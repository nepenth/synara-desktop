import Foundation

protocol RoomReadMarkerServicing {
    func fullyReadEventID(roomID: String) async -> String?
    /// Conditional automatic acknowledgement for the exact tail the UI
    /// observed. Implementations must no-op if that tail is no longer current.
    func markFullyRead(roomID: String, eventID: String) async -> Bool
    /// Explicit user Mark Read. Returns the SDK-authoritative event ID that was
    /// acknowledged and remains available when automatic activity is hidden.
    func markRoomAsRead(roomID: String) async -> String?
}

enum MatrixServerEventIDPolicy {
    static func canAcknowledge(_ eventID: String) -> Bool {
        eventID.hasPrefix("$")
            && eventID.hasPrefix("$pending-") == false
            && eventID.hasPrefix("$local-") == false
    }
}

final class MockRoomReadMarkerService: RoomReadMarkerServicing {
    var eventID: String?

    init(eventID: String? = nil) {
        self.eventID = eventID
    }

    func fullyReadEventID(roomID _: String) async -> String? {
        eventID
    }

    func markFullyRead(roomID _: String, eventID: String) async -> Bool {
        self.eventID = eventID
        return true
    }

    func markRoomAsRead(roomID: String) async -> String? {
        let eventID = "$latest:\(roomID)"
        return await markFullyRead(roomID: roomID, eventID: eventID) ? eventID : nil
    }
}
