import Foundation
import SynaraCore

/// P4-S9-18 typed invite accept/decline/spam/block. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the four registered invite-action commands only.
/// Failed errors stay static and must not echo room id or sender id.
/// S5 `invites_snapshot` stays on SharedCoreInvites. Timeline jump and
/// read-state stay off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreInviteActions {
    static func invitesAccept(
        core: SharedCore,
        roomId: String
    ) async throws -> InviteSnapshotDto {
        try await core.invitesAccept(roomId: roomId)
    }

    static func invitesDecline(
        core: SharedCore,
        roomId: String
    ) async throws -> InviteSnapshotDto {
        try await core.invitesDecline(roomId: roomId)
    }

    static func invitesReportSpam(
        core: SharedCore,
        roomId: String
    ) async throws -> InviteSnapshotDto {
        try await core.invitesReportSpam(roomId: roomId)
    }

    static func invitesBlockSender(
        core: SharedCore,
        roomId: String
    ) async throws -> InviteSnapshotDto {
        try await core.invitesBlockSender(roomId: roomId)
    }
}
