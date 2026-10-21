import Foundation
import SynaraCore

/// Live encryption backup restore and recovery setup. Uses an
/// already-constructed SharedCore.
///
/// Recovery secrets, passphrases and the account password are dedicated
/// arguments only. Failed errors stay static and must not echo them. The
/// generated recovery key comes back once, from `secretStorageBootstrap`.
/// Leftover `recover` stays unused by product iOS.
enum SharedCoreBackupRestore {
    static func restoreBackup(core: SharedCore, recoverySecret: String) async throws -> RestoreBackupDto {
        try await core.restoreBackup(recoverySecret: recoverySecret)
    }

    static func crossSigningSetup(core: SharedCore) async throws -> CrossSigningSetupDto {
        try await core.crossSigningSetup()
    }

    static func crossSigningSetupPassword(
        core: SharedCore,
        password: String
    ) async throws -> CrossSigningSetupDto {
        try await core.crossSigningSetupPassword(password: password)
    }

    static func secretStorageBootstrap(
        core: SharedCore,
        passphrase: String
    ) async throws -> SecretStorageSetupDto {
        try await core.secretStorageBootstrap(passphrase: passphrase)
    }
}
