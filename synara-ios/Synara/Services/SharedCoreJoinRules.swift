import Foundation
import SynaraCore

/// P4-S9-3 typed join-rule snapshot. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps `matrix_room_join_rule_snapshot` only. There is no join-rule writer.
/// It is not a generic `Core.command` FFI, not image packs, and not a product swap.
enum SharedCoreJoinRules {
    static func roomJoinRuleSnapshot(
        core: SharedCore,
        roomId: String,
        sessionGeneration: UInt64
    ) async throws -> RoomJoinRuleSnapshotDto {
        try await core.roomJoinRuleSnapshot(roomId: roomId, sessionGeneration: sessionGeneration)
    }
}
