import Foundation
import SynaraCore

/// P4-S9-23 typed send sticker.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the registered send-sticker command only.
/// Metadata / mxc only. No image bytes or file path.
/// Failed errors stay static and must not echo mxc or room id.
/// Send text stays on SharedCoreSendText.
/// Poll, edit, and respond stay off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreSendSticker {
    static func sendSticker(
        core: SharedCore,
        roomId: String,
        body: String,
        mxc: String,
        width: UInt64?,
        height: UInt64?,
        mimetype: String?,
        size: UInt64?,
        replyTo: String?,
        threadRoot: String?
    ) async throws -> SendStickerDto {
        try await core.sendSticker(
            roomId: roomId,
            body: body,
            mxc: mxc,
            width: width,
            height: height,
            mimetype: mimetype,
            size: size,
            replyTo: replyTo,
            threadRoot: threadRoot
        )
    }
}
