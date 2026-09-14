import Foundation
import SynaraCore

/// P4-S17 poll of queued owner emit summaries.
/// Uses an already-constructed SharedCore.
///
/// Drains presence/devices/join_rules/image_packs/agent_approval_history
/// signals. iOS re-fetches via the existing snapshot commands. Presence user
/// ids are never included. This is not `Platform.emit`. NSE still cannot poll.
enum SharedCoreOwnerUpdates {
    static func poll(core: SharedCore) async throws -> [OwnerUpdateDto] {
        try await core.pollOwnerUpdates()
    }
}
