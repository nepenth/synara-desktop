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
}
