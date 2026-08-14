import Foundation
import SynaraCore

/// P4-S7 typed typing/presence consume. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps `matrix_typing_snapshot`, `matrix_typing_set`,
/// `matrix_presence_snapshot`, `matrix_presence_subscribe`, and
/// `matrix_presence_unsubscribe` only. It is not a generic `Core.command`
/// FFI, not verification, and not a product live swap.
enum SharedCoreTypingPresence {
    static func typingSnapshot(core: SharedCore) async throws -> TypingSnapshotDto {
        try await core.typingSnapshot()
    }

    static func typingSet(core: SharedCore, roomId: String, typing: Bool) async throws {
        try await core.typingSet(roomId: roomId, typing: typing)
    }

    static func presenceSnapshot(core: SharedCore, userId: String) async throws -> PresenceSnapshotDto {
        try await core.presenceSnapshot(userId: userId)
    }

    static func presenceSubscribe(
        core: SharedCore,
        userId: String
    ) async throws -> PresenceSubscriptionDto {
        try await core.presenceSubscribe(userId: userId)
    }

    static func presenceUnsubscribe(core: SharedCore, subscriptionId: String) async throws {
        try await core.presenceUnsubscribe(subscriptionId: subscriptionId)
    }
}
