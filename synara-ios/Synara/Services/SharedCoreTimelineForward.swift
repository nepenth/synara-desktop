import Foundation
import SynaraCore

/// P4-S9-30 typed timeline forward text / media.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the two registered timeline forward commands only.
/// Failed errors stay static and must not echo event id or room id.
/// Timeline poll vote / call decline stay on SharedCoreTimelineVoteDecline.
/// Session/status reads stay off. No media bytes cross the envelope.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreTimelineForward {
    static func timelineForwardText(
        core: SharedCore,
        sourceRoomId: String,
        eventId: String,
        targetRoomId: String,
        asQuote: Bool,
        confirmedEncryptionDowngrade: Bool
    ) async throws -> TimelineForwardDto {
        try await core.timelineForwardText(
            sourceRoomId: sourceRoomId,
            eventId: eventId,
            targetRoomId: targetRoomId,
            asQuote: asQuote,
            confirmedEncryptionDowngrade: confirmedEncryptionDowngrade
        )
    }

    static func timelineForwardMedia(
        core: SharedCore,
        sourceRoomId: String,
        eventId: String,
        targetRoomId: String,
        confirmedEncryptionDowngrade: Bool
    ) async throws -> TimelineForwardDto {
        try await core.timelineForwardMedia(
            sourceRoomId: sourceRoomId,
            eventId: eventId,
            targetRoomId: targetRoomId,
            confirmedEncryptionDowngrade: confirmedEncryptionDowngrade
        )
    }
}
