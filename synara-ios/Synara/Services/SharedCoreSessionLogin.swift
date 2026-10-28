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
    /// `user` is sent as typed (a full Matrix ID or a bare localpart);
    /// `serverName` only locates the account's store from an earlier sign-in.
    /// The session belongs to the `user_id` the server returns.
    static func loginWithPassword(
        user: String,
        homeserverURL: String,
        storeRoot: URL,
        password: String,
        serverName: String? = nil,
        core: SharedCore
    ) async throws -> SessionLoginDto {
        try await core.loginWithPassword(
            user: user,
            homeserverUrl: homeserverURL,
            storeRoot: storeRoot.path,
            password: password,
            serverName: serverName
        )
    }
}
