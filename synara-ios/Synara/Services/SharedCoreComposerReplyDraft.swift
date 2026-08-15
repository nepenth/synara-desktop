import Foundation
import SynaraCore

/// P4-S9-21 typed composer set / get / clear reply draft.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the three registered composer reply-draft commands only.
/// Failed errors stay static and must not echo room id or event id.
/// Reactions stay on SharedCoreTimelineReactions. Send text stays off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreComposerReplyDraft {
    static func composerSetReplyDraft(
        core: SharedCore,
        roomId: String,
        eventId: String,
        startThread: Bool
    ) async throws -> ComposerReplyDraftDto {
        try await core.composerSetReplyDraft(
            roomId: roomId,
            eventId: eventId,
            startThread: startThread
        )
    }

    static func composerGetReplyDraft(
        core: SharedCore,
        roomId: String
    ) async throws -> ComposerReplyDraftDto {
        try await core.composerGetReplyDraft(roomId: roomId)
    }

    static func composerClearReplyDraft(
        core: SharedCore,
        roomId: String
    ) async throws -> ComposerReplyDraftDto {
        try await core.composerClearReplyDraft(roomId: roomId)
    }
}
