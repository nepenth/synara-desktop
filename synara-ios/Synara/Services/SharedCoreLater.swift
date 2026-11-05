import Foundation
import SynaraCore

/// P4-S9-5 typed later account-data. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the six registered later commands only.
/// Item ids/timestamps may cross. m.direct, room notes, and profile writes stay off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreLater {
    static func laterSnapshot(core: SharedCore) async throws -> LaterSnapshotDto {
        try await core.laterSnapshot()
    }

    static func laterUpsert(core: SharedCore, item: LaterItemDto) async throws -> LaterSnapshotDto {
        try await core.laterUpsert(item: item)
    }

    /// A "saved" item for one message, field for field what desktop's
    /// `createLaterItemFromIds(roomId, eventId, 'saved')` writes: the shared
    /// `roomId\neventId` id, `createdAt` in epoch milliseconds, and no due,
    /// reminded or completed time (saving again reopens a completed save).
    static func savedItem(roomID: String, eventID: String, createdAt: Date = Date()) -> LaterItemDto {
        LaterItemDto(
            id: LaterItemIdentity.itemID(roomID: roomID, eventID: eventID),
            kind: .saved,
            roomId: roomID,
            eventId: eventID,
            createdAt: (createdAt.timeIntervalSince1970 * 1000).rounded(.down),
            dueTs: nil,
            remindedAt: nil,
            completedAt: nil
        )
    }

    static func laterComplete(
        core: SharedCore,
        itemId: String,
        completedAt: Double?
    ) async throws -> LaterSnapshotDto {
        try await core.laterComplete(itemId: itemId, completedAt: completedAt)
    }

    static func laterSnooze(
        core: SharedCore,
        itemId: String,
        dueTs: Double
    ) async throws -> LaterSnapshotDto {
        try await core.laterSnooze(itemId: itemId, dueTs: dueTs)
    }

    static func laterClearCompleted(core: SharedCore) async throws -> LaterSnapshotDto {
        try await core.laterClearCompleted()
    }

    static func laterMarkReminded(
        core: SharedCore,
        itemId: String,
        remindedAt: Double?
    ) async throws -> LaterSnapshotDto {
        try await core.laterMarkReminded(itemId: itemId, remindedAt: remindedAt)
    }
}
