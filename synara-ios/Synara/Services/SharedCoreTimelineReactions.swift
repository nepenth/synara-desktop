import Foundation
import SynaraCore

/// P4-S9-20 typed reaction ensure / redact / toggle.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the three registered timeline reaction commands only.
/// Failed errors stay static and must not echo room id, event id,
/// reaction event id, or key.
/// Read-state stays on SharedCoreTimelineReadState. Composer reply
/// draft stays off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreTimelineReactions {
    static func reactionEnsure(
        core: SharedCore,
        roomId: String,
        eventId: String,
        key: String
    ) async throws -> TimelineReactionMutationDto {
        try await core.reactionEnsure(roomId: roomId, eventId: eventId, key: key)
    }

    static func reactionRedact(
        core: SharedCore,
        roomId: String,
        targetEventId: String,
        reactionEventId: String,
        key: String
    ) async throws -> TimelineReactionMutationDto {
        try await core.reactionRedact(
            roomId: roomId,
            targetEventId: targetEventId,
            reactionEventId: reactionEventId,
            key: key
        )
    }

    static func timelineReactionToggle(
        core: SharedCore,
        roomId: String,
        eventId: String,
        key: String
    ) async throws -> TimelineReactionMutationDto {
        try await core.timelineReactionToggle(roomId: roomId, eventId: eventId, key: key)
    }
}
