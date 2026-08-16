import Foundation
import SynaraCore

/// P4-S6 typed timeline open/close/paginate. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps `matrix_timeline_open`, `matrix_timeline_close`, and
/// `matrix_timeline_paginate` only. It is not a generic `Core.command` FFI,
/// not jump/read-state/send, and not a product timeline swap.
enum SharedCoreTimeline {
    static func timelineOpen(
        core: SharedCore,
        roomId: String,
        position: TimelineOpenPositionDto
    ) async throws -> TimelineOpenDto {
        try await core.timelineOpen(roomId: roomId, position: position)
    }

    static func timelineClose(core: SharedCore, streamId: String) async throws -> Bool {
        try await core.timelineClose(streamId: streamId)
    }

    static func timelinePaginate(
        core: SharedCore,
        streamId: String,
        direction: String
    ) async throws -> TimelineSnapshotDto {
        try await core.timelinePaginate(streamId: streamId, direction: direction)
    }
}
