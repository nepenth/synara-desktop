import Foundation
import SynaraCore

/// P4-S9-25 typed edit message.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the registered edit-message command only.
/// No media bytes. Failed errors stay static and must not echo body,
/// event id, or room id.
/// Send poll stays on SharedCoreSendPoll.
/// Poll respond stays off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreEditMessage {
    static func editMessage(
        core: SharedCore,
        roomId: String,
        eventId: String,
        body: String,
        msgType: String?,
        formattedBody: String?,
        mentionUserIds: [String]?,
        mentionRoom: Bool?,
        txnId: String?
    ) async throws -> EditMessageDto {
        try await core.editMessage(
            roomId: roomId,
            eventId: eventId,
            body: body,
            msgType: msgType,
            formattedBody: formattedBody,
            mentionUserIds: mentionUserIds,
            mentionRoom: mentionRoom,
            txnId: txnId
        )
    }
}
