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

/// Stops the retained native SyncService without logging out or replacing its
/// owner set, allowing the same session to restart on foreground entry.
enum SharedCoreSyncStop {
    static func stopSync(core: SharedCore) async throws -> SyncStopDto {
        try await core.stopSync()
    }
}

/// Core's error code for a store read or write that waited, bounded, for the
/// stores `stopSync` closed and gave up. Retryable once the session resumes;
/// every gated export uses it, whatever its error type.
enum SharedCoreSessionSuspension {
    static let code = "p4-session-suspended"
}

/// Replaces the live sync connection after a network path change. Core makes
/// this a no-op while the stores are suspended or before sync first started.
enum SharedCoreSyncRestart {
    static func restartSync(core: SharedCore) async throws -> SyncStartDto {
        try await core.restartSync()
    }
}
