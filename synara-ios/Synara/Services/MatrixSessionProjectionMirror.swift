import Foundation
import SynaraCore

/// The only safe Core readback values used for observational account display.
/// This intentionally excludes Swift session persistence, credentials, Core
/// lifecycle/generation, and Matrix SDK ownership.
struct CoreSessionIdentity: Equatable, Sendable {
    let userID: String
    let deviceID: String
    let homeserverURL: String
}

/// A one-way, in-memory mirror of the safe state of an installed Matrix SDK
/// client. It is intentionally private to the app's SDK owner: this type has
/// no `AuthenticatedSession`, Matrix client, Keychain/store, token, callback,
/// network, room, timeline, or crypto API.
///
/// The generated Rust object only receives the six values in `SessionProjection`.
/// Failures are deliberately ignored here because this optional mirror must
/// never alter leftover MatrixRustSDK service/lifecycle selection (retired).
actor MatrixSessionProjectionMirror {
    private let core: SessionProjectionCore
    private var generation: UInt64 = 0
    private var expectedIdentity: CoreSessionIdentity?

    init(core: SessionProjectionCore = SessionProjectionCore()) {
        self.core = core
    }

    /// Call only after a leftover MatrixRustSDK client would have been installed.
    /// Individual safe values are accepted instead of `AuthenticatedSession` so
    /// credentials cannot be handed to the UniFFI facade by construction.
    func openAfterInstalledClient(
        userID: String,
        deviceID: String,
        homeserverURL: String,
        cryptoReady: Bool
    ) async {
        guard generation < UInt64.max else {
            return
        }

        let nextGeneration = generation + 1
        let identity = CoreSessionIdentity(
            userID: userID,
            deviceID: deviceID,
            homeserverURL: homeserverURL
        )
        let projection = SessionProjection(
            generation: nextGeneration,
            userId: userID,
            deviceId: deviceID,
            homeserverUrl: homeserverURL,
            lifecycle: .ready,
            cryptoReady: cryptoReady
        )
        // Do not read an old projection while a replacement is being opened.
        expectedIdentity = nil
        do {
            try await core.open(projection: projection)
            generation = nextGeneration
            expectedIdentity = identity
        } catch {
            // UniFFI exposes only static errors. Do not log even those here:
            // this mirror is observational and must not affect SDK ownership.
        }
    }

    /// Returns only a ready Core snapshot that exactly matches the safe values
    /// recorded after a successful installed-client mirror open.
    func coreSessionIdentity() async -> CoreSessionIdentity? {
        guard let expectedIdentity else {
            return nil
        }

        do {
            guard let snapshot = try await core.sessionSnapshot(), snapshot.lifecycle == .ready else {
                return nil
            }
            let identity = CoreSessionIdentity(
                userID: snapshot.userId,
                deviceID: snapshot.deviceId,
                homeserverURL: snapshot.homeserverUrl
            )
            // Recheck after the awaited FFI call so a concurrent close or open
            // cannot return an identity that is no longer expected.
            guard identity == expectedIdentity, self.expectedIdentity == expectedIdentity else {
                return nil
            }
            return identity
        } catch {
            return nil
        }
    }

    /// Clear the in-memory Core projection before the SDK client/store is
    /// released or wiped. This does not alter the Matrix SDK or any persistence.
    func closeBeforeSDKWipe() async {
        expectedIdentity = nil
        try? await core.close()
    }
}
