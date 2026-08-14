import Foundation
import SynaraCore

/// P4-S9-28 typed timeline pin / unpin.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the two registered timeline pin commands only.
/// Failed errors stay static and must not echo event id or room id.
/// Timeline edit/redact/report stay on SharedCoreTimelineMutate.
/// Poll vote / call decline stay off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreTimelinePin {
    static func timelinePin(
        core: SharedCore,
        roomId: String,
        eventId: String
    ) async throws -> TimelinePinDto {
        try await core.timelinePin(
            roomId: roomId,
            eventId: eventId
        )
    }

    static func timelineUnpin(
        core: SharedCore,
        roomId: String,
        eventId: String
    ) async throws -> TimelinePinDto {
        try await core.timelineUnpin(
            roomId: roomId,
            eventId: eventId
        )
    }
}
