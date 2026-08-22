import Foundation
import SynaraCore

/// Live content upload and room attachment send. Uses an already-constructed SharedCore.
///
/// Bytes are dedicated arguments only. Failed errors stay static and must not
/// echo filename, mime, room id, or token. Leftover `mediaUpload` stays unused
/// by product iOS.
enum SharedCoreMediaSend {
    static func uploadContent(
        core: SharedCore,
        payload: Data,
        mimeType: String,
        filename: String?
    ) async throws -> MediaUploadDto {
        try await core.uploadContent(
            payload: payload,
            mimeType: mimeType,
            filename: filename
        )
    }

    static func sendRoomAttachment(
        core: SharedCore,
        roomId: String,
        filename: String,
        mimeType: String,
        payload: Data,
        replyTo: String?,
        threadRoot: String?
    ) async throws -> SendRoomAttachmentDto {
        try await core.sendRoomAttachment(
            roomId: roomId,
            filename: filename,
            mimeType: mimeType,
            payload: payload,
            replyTo: replyTo,
            threadRoot: threadRoot
        )
    }
}
