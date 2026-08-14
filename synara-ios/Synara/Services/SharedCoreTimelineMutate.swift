import Foundation
import SynaraCore

/// P4-S9-27 typed timeline edit / redact / report.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the three registered timeline mutate commands only.
/// Failed errors stay static and must not echo body, event id, room id,
/// or reason.
/// Poll respond stays on SharedCorePollRespond.
/// Pin/unpin stay off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreTimelineMutate {
    static func timelineEditText(
        core: SharedCore,
        roomId: String,
        eventId: String,
        body: String,
        formattedBody: String?
    ) async throws -> TimelineMutateDto {
        try await core.timelineEditText(
            roomId: roomId,
            eventId: eventId,
            body: body,
            formattedBody: formattedBody
        )
    }

    static func timelineRedact(
        core: SharedCore,
        roomId: String,
        eventId: String,
        reason: String?
    ) async throws -> TimelineMutateDto {
        try await core.timelineRedact(
            roomId: roomId,
            eventId: eventId,
            reason: reason
        )
    }

    static func timelineReport(
        core: SharedCore,
        roomId: String,
        eventId: String,
        reason: String?
    ) async throws -> TimelineMutateDto {
        try await core.timelineReport(
            roomId: roomId,
            eventId: eventId,
            reason: reason
        )
    }
}
