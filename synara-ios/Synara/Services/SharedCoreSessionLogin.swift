import Foundation
import SynaraCore

/// P4-S3c dedicated password-login entry. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// Password is forwarded as the dedicated FFI argument only; this helper
/// does not store it. This is not `matrix_login_password`, not owner attach,
/// and not `Core.command`. XCTest construction of `SharedCore` is not
/// iOS-on-engine.
enum SharedCoreSessionLogin {
    static func loginWithPassword(
        userID: String,
        homeserverURL: String,
        storeRoot: URL,
        password: String,
        core: SharedCore
    ) async throws -> SessionLoginDto {
        try await core.loginWithPassword(
            userId: userID,
            homeserverUrl: homeserverURL,
            storeRoot: storeRoot.path,
            password: password
        )
    }
}
