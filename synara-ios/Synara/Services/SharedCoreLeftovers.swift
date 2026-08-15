import Foundation
import SynaraCore

/// P4-S10 leftover UniFFI. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps leftover status reads plus dedicated leftover wipe/logout/
/// recover/raw-send/notification/media/avatar/pusher methods.
/// Secrets and bytes are dedicated arguments only. Failed errors stay
/// static and must not echo password, recovery key, event, body, bytes,
/// URL, or token. Oversize fail-closes at 1 MiB with no truncate.
/// It is not a generic `Core.command` FFI and does not start SyncService.
enum SharedCoreLeftovers {
    static func backupStatus(core: SharedCore) async throws -> BackupStatusDto {
        try await core.backupStatus()
    }

    static func cryptoStatus(core: SharedCore) async throws -> CryptoStatusDto {
        try await core.cryptoStatus()
    }

    static func crossSigningStatus(core: SharedCore) async throws -> CrossSigningStatusDto {
        try await core.crossSigningStatus()
    }

    static func roomKeyTransferStatus(core: SharedCore) async throws -> RoomKeyTransferStatusDto {
        try await core.roomKeyTransferStatus()
    }

    static func wipePersistedStores(core: SharedCore, storeRoot: String) async throws -> LeftoverAckDto {
        try await core.wipePersistedStores(storeRoot: storeRoot)
    }

    static func logout(core: SharedCore) async throws -> LeftoverAckDto {
        try await core.logout()
    }

    static func recover(core: SharedCore, recoveryKey: String) async throws -> LeftoverAckDto {
        try await core.recover(recoveryKey: recoveryKey)
    }

    static func sendRawRoomEvent(
        core: SharedCore,
        roomId: String,
        eventType: String,
        contentJson: String
    ) async throws -> LeftoverAckDto {
        try await core.sendRawRoomEvent(
            roomId: roomId,
            eventType: eventType,
            contentJson: contentJson
        )
    }

    static func setNotificationMode(
        core: SharedCore,
        roomId: String,
        mode: String
    ) async throws -> LeftoverAckDto {
        try await core.setNotificationMode(roomId: roomId, mode: mode)
    }

    static func mediaDownload(core: SharedCore, mxc: String) async throws -> LeftoverBytesDto {
        try await core.mediaDownload(mxc: mxc)
    }

    static func mediaThumbnail(
        core: SharedCore,
        mxc: String,
        width: UInt64,
        height: UInt64
    ) async throws -> LeftoverBytesDto {
        try await core.mediaThumbnail(mxc: mxc, width: width, height: height)
    }

    static func mediaUpload(
        core: SharedCore,
        payload: Data,
        mimeType: String,
        filename: String
    ) async throws -> LeftoverAckDto {
        try await core.mediaUpload(
            payload: payload,
            mimeType: mimeType,
            filename: filename
        )
    }

    static func roomAvatarBytes(core: SharedCore, roomId: String) async throws -> LeftoverBytesDto {
        try await core.roomAvatarBytes(roomId: roomId)
    }

    static func pusherSet(
        core: SharedCore,
        pushKey: String,
        appId: String,
        gatewayUrl: String,
        appDisplayName: String,
        deviceDisplayName: String,
        lang: String
    ) async throws -> LeftoverAckDto {
        try await core.pusherSet(
            pushKey: pushKey,
            appId: appId,
            gatewayUrl: gatewayUrl,
            appDisplayName: appDisplayName,
            deviceDisplayName: deviceDisplayName,
            lang: lang
        )
    }

    static func pusherDelete(
        core: SharedCore,
        pushKey: String,
        appId: String
    ) async throws -> LeftoverAckDto {
        try await core.pusherDelete(pushKey: pushKey, appId: appId)
    }
}
