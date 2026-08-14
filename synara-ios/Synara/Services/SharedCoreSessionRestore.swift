import Foundation
import SynaraCore

/// P4-S3b restore entry. Uses an already-constructed S3a `SharedCore`.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This is not `matrix_restore_session`, not password login, and not owner
/// attach. XCTest construction of `SharedCore` is not iOS-on-engine.
enum SharedCoreSessionRestore {
    static func restorePersistedSession(
        userID: String,
        homeserverURL: String,
        storeRoot: URL,
        core: SharedCore
    ) async throws -> SessionRestoreDto {
        try await core.restorePersistedSession(
            userId: userID,
            homeserverUrl: homeserverURL,
            storeRoot: storeRoot.path
        )
    }
}
