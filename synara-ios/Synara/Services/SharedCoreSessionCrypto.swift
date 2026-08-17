import Foundation

/// P4-S27/S28 map of privacy-safe SharedCore leftover/session status reads
/// to product session and room crypto status.
///
/// Uses existing backup / secret-storage / crypto status plus invite
/// encryption when the room is an invite. Joined-room encryption is not
/// on the list DTO. Recovery keys, missing-secret lists, and UTD counts
/// never appear on the product status. This is not iOS-on-engine and not
/// P4 acceptance.
enum SharedCoreSessionCrypto {
    static func status(
        crossSigningState: String?,
        backupEnabled: Bool?,
        backupAvailability: String?,
        backupDeviceState: String?,
        recoveryState: String?,
        secretStorageState: String?
    ) -> SessionCryptoStatus {
        SessionCryptoStatus(
            verification: verification(crossSigningState),
            recovery: recovery(recoveryState: recoveryState, secretStorageState: secretStorageState),
            backup: backup(
                enabled: backupEnabled,
                availability: backupAvailability,
                deviceState: backupDeviceState
            ),
            hasDevicesToVerifyAgainst: nil,
            isLastDevice: nil,
            unableToDecryptCount: 0
        )
    }

    static func verification(_ crossSigningState: String?) -> SynaraCryptoVerificationStatus {
        switch crossSigningState {
        case "ready":
            return .verified
        case "unavailable", "not_set_up", "missing":
            return .unverified
        default:
            return .unknown
        }
    }

    static func recovery(
        recoveryState: String?,
        secretStorageState: String?
    ) -> SynaraCryptoRecoveryStatus {
        switch recoveryState {
        case "ready":
            return .enabled
        case "incomplete":
            return .incomplete
        case "not_set_up":
            return .disabled
        default:
            break
        }
        switch secretStorageState {
        case "ready":
            return .enabled
        case "locked":
            return .incomplete
        case "not_set_up", "unavailable":
            return .disabled
        default:
            return .unknown
        }
    }

    static func backup(
        enabled: Bool?,
        availability: String?,
        deviceState: String?
    ) -> SynaraCryptoBackupStatus {
        if enabled == true {
            return .enabled
        }
        switch deviceState {
        case "connecting", "downloading", "uploading":
            return .syncing
        default:
            break
        }
        if availability == "missing" {
            return .unavailable
        }
        return .unknown
    }

    static func roomStatus(
        isEncrypted: Bool?,
        session: SessionCryptoStatus
    ) -> RoomCryptoStatus {
        RoomCryptoStatus(
            encryption: encryption(isEncrypted),
            verification: session.verification,
            recovery: session.recovery,
            backup: session.backup,
            unableToDecryptCount: 0
        )
    }

    static func encryption(_ isEncrypted: Bool?) -> SynaraRoomEncryptionStatus {
        switch isEncrypted {
        case true:
            return .encrypted
        case false:
            return .notEncrypted
        default:
            return .unknown
        }
    }
}
