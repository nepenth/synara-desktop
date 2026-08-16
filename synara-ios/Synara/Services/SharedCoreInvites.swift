import Foundation
import SynaraCore

/// P4-S5 typed invite snapshot. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps `matrix_invites_snapshot` only. It is not a generic
/// `Core.command` FFI, not accept/decline, and not a product invite swap.
enum SharedCoreInvites {
    static func invitesSnapshot(core: SharedCore) async throws -> InviteSnapshotDto {
        try await core.invitesSnapshot()
    }
}
