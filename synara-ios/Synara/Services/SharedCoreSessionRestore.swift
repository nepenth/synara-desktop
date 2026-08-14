import Foundation
import SynaraCore

/// P4-S3b restore entry. Uses the S3a vault and Core persist/restore.
///
/// This is not `matrix_restore_session`, not password login, and not owner
/// attach. XCTest construction of `SharedCore` is not iOS-on-engine.
enum SharedCoreSessionRestore {
    static func restorePersistedSession(
        userID: String,
        homeserverURL: String,
        storeRoot: URL,
        vault: IosSecretVault
    ) async throws -> SessionRestoreDto {
        let core = SharedCore(store: vault)
        return try await core.restorePersistedSession(
            userId: userID,
            homeserverUrl: homeserverURL,
            storeRoot: storeRoot.path
        )
    }
}
