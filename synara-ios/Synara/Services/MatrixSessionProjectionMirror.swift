import Foundation
import SynaraCore

/// A one-way, in-memory mirror of the safe state of an installed Matrix SDK
/// client. It is intentionally private to the app's SDK owner: this type has
/// no `AuthenticatedSession`, Matrix client, Keychain/store, token, callback,
/// network, room, timeline, or crypto API.
///
/// The generated Rust object only receives the six values in `SessionProjection`.
/// Failures are deliberately ignored here because this optional mirror must
/// never alter the existing MatrixRustSDK service/lifecycle selection.
actor MatrixSessionProjectionMirror {
    private let core = SessionProjectionCore()
    private var generation: UInt64 = 0

    /// Call only after the MatrixRustSDK client has been installed successfully.
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
        let projection = SessionProjection(
            generation: nextGeneration,
            userId: userID,
            deviceId: deviceID,
            homeserverUrl: homeserverURL,
            lifecycle: .ready,
            cryptoReady: cryptoReady
        )
        do {
            try await core.open(projection: projection)
            generation = nextGeneration
        } catch {
            // UniFFI exposes only static errors. Do not log even those here:
            // this mirror is observational and must not affect SDK ownership.
        }
    }

    /// Clear the in-memory Core projection before the SDK client/store is
    /// released or wiped. This does not alter the Matrix SDK or any persistence.
    func closeBeforeSDKWipe() async {
        try? await core.close()
    }
}
