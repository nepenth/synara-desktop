import Foundation
import SynaraCore

/// P4-S27/S28 map of privacy-safe SharedCore leftover/session status reads
/// to product session and room crypto status.
///
/// Uses existing backup / secret-storage / crypto status and the Core room
/// list's authoritative joined-room encryption tri-state. Invite encryption
/// never substitutes for a missing joined-room state. Recovery keys,
/// and missing-secret lists never appear on the product status. Session metadata
/// cannot determine UTD counts; rooms use the SDK-projected timeline rows.
/// This is not iOS-on-engine and not P4 acceptance.
enum SharedCoreSessionCrypto {
    static func status(
        crossSigningState: CrossSigningStateDto?,
        backupEnabled: Bool?,
        backupAvailability: BackupAvailabilityDto?,
        backupDeviceState: BackupDeviceStateDto?,
        recoveryState: BackupRecoveryStateDto?,
        secretStorageState: SecretStorageStateDto?
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
            unableToDecryptCount: nil
        )
    }

    static func verification(_ crossSigningState: CrossSigningStateDto?) -> SynaraCryptoVerificationStatus {
        switch crossSigningState {
        case .ready:
            return .verified
        case .unavailable, .notSetUp:
            return .unverified
        case .partial, nil:
            return .unknown
        }
    }

    static func recovery(
        recoveryState: BackupRecoveryStateDto?,
        secretStorageState: SecretStorageStateDto?
    ) -> SynaraCryptoRecoveryStatus {
        switch recoveryState {
        case .ready:
            return .enabled
        case .incomplete:
            return .incomplete
        case .notSetUp:
            return .disabled
        case .unknown, nil:
            break
        }
        switch secretStorageState {
        case .ready:
            return .enabled
        case .locked:
            return .incomplete
        case .notSetUp, .unavailable:
            return .disabled
        case nil:
            return .unknown
        }
    }

    static func backup(
        enabled: Bool?,
        availability: BackupAvailabilityDto?,
        deviceState: BackupDeviceStateDto?
    ) -> SynaraCryptoBackupStatus {
        if enabled == true {
            return .enabled
        }
        switch deviceState {
        case .connecting, .downloading, .uploading:
            return .syncing
        case .unavailable, .disconnected, .ready, nil:
            break
        }
        if availability == .missing {
            return .unavailable
        }
        return .unknown
    }

    static func roomStatus(
        isEncrypted: Bool?,
        session: SessionCryptoStatus
    ) -> RoomCryptoStatus {
        roomStatus(encryption: encryption(isEncrypted), session: session)
    }

    static func roomStatus(
        encryption: SynaraRoomEncryptionStatus,
        session: SessionCryptoStatus
    ) -> RoomCryptoStatus {
        RoomCryptoStatus(
            encryption: encryption,
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
