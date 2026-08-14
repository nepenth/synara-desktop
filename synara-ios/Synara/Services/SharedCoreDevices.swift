import Foundation
import SynaraCore

/// P4-S9-2 typed NativeDeviceOwner consume. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the seven already-registered owner commands only.
/// It is not a generic `Core.command` FFI, not leftover password/export, and not a product swap.
enum SharedCoreDevices {
    static func deviceSnapshot(core: SharedCore) async throws -> DeviceSnapshotDto {
        try await core.deviceSnapshot()
    }

    static func deviceRename(
        core: SharedCore,
        deviceId: String,
        displayName: String
    ) async throws -> DeviceSnapshotDto {
        try await core.deviceRename(deviceId: deviceId, displayName: displayName)
    }

    static func deviceDeleteStart(core: SharedCore, deviceIds: [String]) async throws -> DeviceDeleteDto {
        try await core.deviceDeleteStart(deviceIds: deviceIds)
    }

    static func deviceDeleteCancel(
        core: SharedCore,
        operationId: UInt64,
        sessionGeneration: UInt64
    ) async throws {
        try await core.deviceDeleteCancel(operationId: operationId, sessionGeneration: sessionGeneration)
    }

    static func backupStatus(core: SharedCore) async throws -> BackupStatusDto {
        try await core.backupStatus()
    }

    static func roomKeyTransferStatus(core: SharedCore) async throws -> RoomKeyTransferStatusDto {
        try await core.roomKeyTransferStatus()
    }

    static func crossSigningSetup(core: SharedCore) async throws -> CrossSigningSetupDto {
        try await core.crossSigningSetup()
    }
}
