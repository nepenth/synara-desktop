import Foundation
import SynaraCore

/// P4-S9-31 typed session/status reads.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the four registered session/status read commands only.
/// Failed errors stay static and must not echo user id, homeserver, or device id.
/// Timeline forward stays on SharedCoreTimelineForward.
/// Backup/crypto/cross-signing/room-key status stay off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreSessionStatus {
    static func sessionSnapshot(core: SharedCore) async throws -> SessionSnapshotDto {
        try await core.sessionSnapshot()
    }

    static func syncStatus(core: SharedCore) async throws -> SyncStatusDto {
        try await core.syncStatus()
    }

    static func mediaConfig(core: SharedCore) async throws -> MediaConfigDto {
        try await core.mediaConfig()
    }

    static func secretStorageStatus(core: SharedCore) async throws -> SecretStorageStatusDto {
        try await core.secretStorageStatus()
    }
}
