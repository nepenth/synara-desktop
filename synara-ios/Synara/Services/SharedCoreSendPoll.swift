import Foundation
import SynaraCore

/// P4-S9-24 typed send poll.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the registered send-poll command only.
/// No media bytes. Failed errors stay static and must not echo question,
/// options, or room id.
/// Send sticker stays on SharedCoreSendSticker.
/// Edit and respond stay off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreSendPoll {
    static func sendPoll(
        core: SharedCore,
        roomId: String,
        question: String,
        answers: [String],
        maxSelections: UInt32,
        threadRoot: String?,
        replyTo: String?
    ) async throws -> SendPollDto {
        try await core.sendPoll(
            roomId: roomId,
            question: question,
            answers: answers,
            maxSelections: maxSelections,
            threadRoot: threadRoot,
            replyTo: replyTo
        )
    }
}
