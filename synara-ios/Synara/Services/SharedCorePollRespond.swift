import Foundation
import SynaraCore

/// P4-S9-26 typed poll respond.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the registered poll-respond command only.
/// No media bytes. Failed errors stay static and must not echo answers,
/// event id, or room id.
/// Edit message stays on SharedCoreEditMessage.
/// Timeline edit/redact/report stay off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCorePollRespond {
    static func pollRespond(
        core: SharedCore,
        roomId: String,
        pollEventId: String,
        answerIds: [String]
    ) async throws -> PollRespondDto {
        try await core.pollRespond(
            roomId: roomId,
            pollEventId: pollEventId,
            answerIds: answerIds
        )
    }
}
