import Foundation
import SynaraCore

/// P4-S9-29 typed timeline poll vote / call decline.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the two registered timeline vote/decline commands only.
/// Failed errors stay static and must not echo event id, room id, or answer.
/// Timeline pin/unpin stay on SharedCoreTimelinePin.
/// Timeline forward stays off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreTimelineVoteDecline {
    static func timelinePollVote(
        core: SharedCore,
        roomId: String,
        eventId: String,
        answerIds: [String]
    ) async throws -> TimelineVoteDeclineDto {
        try await core.timelinePollVote(
            roomId: roomId,
            eventId: eventId,
            answerIds: answerIds
        )
    }

    static func timelineCallDecline(
        core: SharedCore,
        roomId: String,
        eventId: String
    ) async throws -> TimelineVoteDeclineDto {
        try await core.timelineCallDecline(
            roomId: roomId,
            eventId: eventId
        )
    }
}
