import Foundation
import SynaraCore

/// P4-S9-22 typed send text.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the registered send-text command only.
/// Failed errors stay static and must not echo body or room id.
/// Composer reply draft stays on SharedCoreComposerReplyDraft.
/// Sticker, poll, edit, and respond stay off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreSendText {
    static func sendText(
        core: SharedCore,
        roomId: String,
        body: String,
        msgType: String?,
        formattedBody: String?,
        mentionUserIds: [String]?,
        mentionRoom: Bool?,
        replyTo: String?,
        threadRoot: String?,
        txnId: String?
    ) async throws -> SendTextDto {
        try await core.sendText(
            roomId: roomId,
            body: body,
            msgType: msgType,
            formattedBody: formattedBody,
            mentionUserIds: mentionUserIds,
            mentionRoom: mentionRoom,
            replyTo: replyTo,
            threadRoot: threadRoot,
            txnId: txnId
        )
    }

    /// Retry one unsent SDK local echo under its transaction id.
    static func retryLocalEcho(core: SharedCore, roomId: String, transactionId: String) async throws {
        try await core.localEchoRetry(roomId: roomId, transactionId: transactionId)
    }

    /// Discard one unsent SDK local echo; false when it was already sent.
    static func discardLocalEcho(
        core: SharedCore,
        roomId: String,
        transactionId: String
    ) async throws -> Bool {
        try await core.localEchoDiscard(roomId: roomId, transactionId: transactionId)
    }

    /// Core could not take the message yet (no session, or the room is not
    /// known while the session restores). Anything else is a rejection.
    static func isNotReady(_ error: Error) -> Bool {
        guard case let SendTextError.Failed(code, _) = error else {
            return false
        }
        return code == "p2-send-text-no-session" || code == "d0.4-send-room-not-found"
    }
}
