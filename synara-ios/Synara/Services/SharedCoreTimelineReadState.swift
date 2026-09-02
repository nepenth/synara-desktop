import Foundation
import SynaraCore

/// P4-S9-19 typed timeline event-readback / set-read-state / jump-latest.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the three registered timeline read-state commands only.
/// Jump returns the existing open readback. Failed errors stay static
/// and must not echo event id, room id, or stream id.
/// S6 open/close/paginate stay on SharedCoreTimeline. Reactions stay off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreTimelineReadState {
    static func timelineEventReadback(
        core: SharedCore,
        roomId: String,
        eventId: String
    ) async throws -> TimelineEventReadbackDto {
        try await core.timelineEventReadback(roomId: roomId, eventId: eventId)
    }

    static func timelineSetReadState(
        core: SharedCore,
        streamId: String,
        action: String,
        intent: String,
        observedLiveTailEventId: String? = nil
    ) async throws -> TimelineReadStateDto {
        try await core.timelineSetReadState(
            streamId: streamId,
            action: action,
            intent: intent,
            observedLiveTailEventId: observedLiveTailEventId
        )
    }

    static func timelineJumpLatest(
        core: SharedCore,
        streamId: String
    ) async throws -> TimelineOpenDto {
        try await core.timelineJumpLatest(streamId: streamId)
    }
}
