import Foundation
import SynaraCore

/// P4-S9-7 typed room-notes account-data. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the five registered room-notes commands only.
/// Note body text may cross in DTOs. Failed errors stay static.
/// Own display-name/avatar stay off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreRoomNotes {
    static func roomNotesSnapshot(core: SharedCore) async throws -> RoomNotesSnapshotDto {
        try await core.roomNotesSnapshot()
    }

    static func roomNotesUpsert(
        core: SharedCore,
        item: RoomNoteItemDto
    ) async throws -> RoomNotesSnapshotDto {
        try await core.roomNotesUpsert(item: item)
    }

    static func roomNotesDelete(
        core: SharedCore,
        roomId: String,
        itemId: String
    ) async throws -> RoomNotesSnapshotDto {
        try await core.roomNotesDelete(roomId: roomId, itemId: itemId)
    }

    static func roomNotesCompleteTodo(
        core: SharedCore,
        roomId: String,
        itemId: String,
        completed: Bool
    ) async throws -> RoomNotesSnapshotDto {
        try await core.roomNotesCompleteTodo(roomId: roomId, itemId: itemId, completed: completed)
    }

    static func roomNotesMoveTodo(
        core: SharedCore,
        roomId: String,
        itemId: String,
        direction: String
    ) async throws -> RoomNotesSnapshotDto {
        try await core.roomNotesMoveTodo(roomId: roomId, itemId: itemId, direction: direction)
    }
}
