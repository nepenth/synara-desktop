import Foundation
import SynaraCore

/// Live encryption backup restore. Uses an already-constructed SharedCore.
///
/// Recovery secrets are dedicated arguments only. Failed errors stay static
/// and must not echo the recovery key. Leftover `recover` stays unused by
/// product iOS.
enum SharedCoreBackupRestore {
    static func restoreBackup(core: SharedCore, recoverySecret: String) async throws -> RestoreBackupDto {
        try await core.restoreBackup(recoverySecret: recoverySecret)
    }
}
