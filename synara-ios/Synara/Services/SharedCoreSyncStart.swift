import Foundation
import SynaraCore

/// P4-S12 start of the already-attached SyncService.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This is not `Core.command`, not leftover registration, and not a
/// product NSE swap. NSE still cannot start sync. XCTest construction
/// of `SharedCore` is not iOS-on-engine.
enum SharedCoreSyncStart {
    static func startSync(core: SharedCore) async throws -> SyncStartDto {
        try await core.startSync()
    }
}
