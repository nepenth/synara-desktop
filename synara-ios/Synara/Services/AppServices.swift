import Combine
import Foundation
import UserNotifications

enum SessionState: Equatable {
    case signedOut
    case signedIn(AuthenticatedSession)
}

struct AuthenticatedSession: Codable, Equatable {
    let userID: String
    let deviceID: String
    let homeserverURL: URL
    let accessToken: String
    let refreshToken: String?
    let slidingSyncVersion: String
    let sdkStoreID: String?

    init(
        userID: String,
        deviceID: String,
        homeserverURL: URL,
        accessToken: String,
        refreshToken: String? = nil,
        slidingSyncVersion: String = "native",
        sdkStoreID: String? = nil
    ) {
        self.userID = userID
        self.deviceID = deviceID
        self.homeserverURL = homeserverURL
        self.accessToken = accessToken
        self.refreshToken = refreshToken
        self.slidingSyncVersion = slidingSyncVersion
        self.sdkStoreID = sdkStoreID
    }
}

enum MatrixSyncStatus: Equatable {
    case stopped
    case starting
    case syncing
    case connected
    case reconnecting
    case disconnected
    case restoreFailed
    case failed(String)

    var description: String {
        ConnectionStatusCopy.banner(self)
    }
}

protocol MatrixClientServicing: AnyObject {
    var syncStatusDescription: String { get }
    var syncStatus: MatrixSyncStatus { get }

    func start(session: AuthenticatedSession) async
    func warmSync(session: AuthenticatedSession) async
    /// Best-effort authenticated homeserver logout using an already-loaded
    /// client. Must not create or repair a crypto store during local wipe.
    func revokeServerSession(_ session: AuthenticatedSession) async -> Bool
    func stop() async
    func pauseForBackground() async
    func resumeFromForeground(session: AuthenticatedSession) async
    func syncForBackgroundNotification(session: AuthenticatedSession) async -> Bool
    func resetLocalState(for session: AuthenticatedSession?) async
    /// Optional, display-only readback. It must not affect SDK lifecycle or
    /// session/credential ownership.
    func coreSessionIdentity() async -> CoreSessionIdentity?
    func presence(userID: String) async -> SharedCorePresence?
    func setOwnPresence(_ state: String) async -> Bool
    func ownProfile() async -> SharedCoreOwnProfileInfo?
    func setOwnDisplayName(_ displayName: String) async -> Bool
    func uploadOwnAvatar(payload: Data, mimeType: String) async -> Bool
    func setOutgoingTyping(roomID: String, typing: Bool) async
    func ignoredUserIDs() async -> [String]
    func ignoreUser(_ userID: String) async -> Bool
    func unignoreUser(_ userID: String) async -> Bool
    func pushRulesSnapshot() async -> SynaraPushRulesSnapshot?
    func setPushRuleDefault(encrypted: Bool, oneToOne: Bool, mode: String) async -> Bool
    func setPushRuleMention(ruleID: String, enabled: Bool) async -> Bool
    func addPushKeyword(_ keyword: String) async -> Bool
    func removePushKeyword(_ keyword: String) async -> Bool
    func threepidEmails() async -> [String]
    func deleteThreepidEmail(_ address: String) async -> Bool
    func requestThreepidEmailToken(_ email: String) async -> Bool
    func addThreepidEmail() async -> String?
    func addThreepidEmailPassword(_ password: String) async -> String?
}

struct SynaraPushRuleMentions {
    var userMention: Bool
    var displayName: Bool
    var userName: Bool
    var roomMention: Bool
    var atRoom: Bool
}

struct SynaraPushRulesSnapshot {
    var dm: String
    var dmEncrypted: String
    var group: String
    var groupEncrypted: String
    var mentions: SynaraPushRuleMentions
    var keywords: [String]
}

extension MatrixClientServicing {
    func coreSessionIdentity() async -> CoreSessionIdentity? {
        nil
    }

    func presence(userID: String) async -> SharedCorePresence? {
        _ = userID
        return nil
    }

    func setOwnPresence(_ state: String) async -> Bool {
        _ = state
        return false
    }

    func ownProfile() async -> SharedCoreOwnProfileInfo? {
        nil
    }

    func setOwnDisplayName(_ displayName: String) async -> Bool {
        _ = displayName
        return false
    }

    func uploadOwnAvatar(payload: Data, mimeType: String) async -> Bool {
        _ = payload
        _ = mimeType
        return false
    }

    func setOutgoingTyping(roomID: String, typing: Bool) async {
        _ = roomID
        _ = typing
    }

    func ignoredUserIDs() async -> [String] { [] }
    func ignoreUser(_ userID: String) async -> Bool {
        _ = userID
        return false
    }
    func unignoreUser(_ userID: String) async -> Bool {
        _ = userID
        return false
    }
    func pushRulesSnapshot() async -> SynaraPushRulesSnapshot? { nil }
    func setPushRuleDefault(encrypted: Bool, oneToOne: Bool, mode: String) async -> Bool {
        _ = encrypted
        _ = oneToOne
        _ = mode
        return false
    }
    func setPushRuleMention(ruleID: String, enabled: Bool) async -> Bool {
        _ = ruleID
        _ = enabled
        return false
    }
    func addPushKeyword(_ keyword: String) async -> Bool {
        _ = keyword
        return false
    }
    func removePushKeyword(_ keyword: String) async -> Bool {
        _ = keyword
        return false
    }
    func threepidEmails() async -> [String] { [] }
    func deleteThreepidEmail(_ address: String) async -> Bool {
        _ = address
        return false
    }
    func requestThreepidEmailToken(_ email: String) async -> Bool {
        _ = email
        return false
    }
    func addThreepidEmail() async -> String? { nil }
    func addThreepidEmailPassword(_ password: String) async -> String? {
        _ = password
        return nil
    }
}

protocol PushServicing {
    var isRegistrationAvailable: Bool { get }
    var isRegistered: Bool { get }
    var tokenSnippet: String? { get }
    var registrationStateDescription: String { get }
    var pushGatewayURL: String? { get }

    func beginRegistration()
    func handleDeviceToken(_ tokenData: Data)
    func clearRegistrationState() async
    func configure(with session: AuthenticatedSession)
    func route(from notificationPayload: [AnyHashable: Any]) -> AppRoute?
    func resolveRoute(from notificationPayload: [AnyHashable: Any]) async -> AppRoute?
    func parseBadgeCount(from notificationPayload: [AnyHashable: Any]) -> Int?
    func applyIncomingBadge(from notificationPayload: [AnyHashable: Any])
}

enum NotificationPermissionStatus: Equatable {
    case notDetermined
    case denied
    case authorized
    case provisional
    case ephemeral
    case unavailable

    var displayName: String {
        switch self {
        case .notDetermined:
            return "Not Requested"
        case .denied:
            return "Denied"
        case .authorized:
            return "Authorized"
        case .provisional:
            return "Provisional"
        case .ephemeral:
            return "Ephemeral"
        case .unavailable:
            return "Unavailable"
        }
    }

    var allowsPushRegistration: Bool {
        switch self {
        case .authorized, .provisional, .ephemeral:
            return true
        case .notDetermined, .denied, .unavailable:
            return false
        }
    }

    var detail: String {
        switch self {
        case .notDetermined:
            return "Notifications can be enabled for room and agent alerts."
        case .denied:
            return "Enable notifications in iOS Settings to receive alerts."
        case .authorized:
            return "Notifications are enabled."
        case .provisional:
            return "Notifications can be delivered quietly."
        case .ephemeral:
            return "Notifications are temporarily available for this session."
        case .unavailable:
            return "Notifications are unavailable on this device."
        }
    }

    static func map(_ authorizationStatus: UNAuthorizationStatus) -> NotificationPermissionStatus {
        switch authorizationStatus {
        case .notDetermined:
            return .notDetermined
        case .denied:
            return .denied
        case .authorized:
            return .authorized
        case .provisional:
            return .provisional
        case .ephemeral:
            return .ephemeral
        @unknown default:
            return .unavailable
        }
    }
}

protocol NotificationPermissionServicing {
    func currentStatus() async -> NotificationPermissionStatus
    func requestAuthorization() async -> NotificationPermissionStatus
}

enum SynaraRoomEncryptionStatus: Equatable {
    case unknown
    case notEncrypted
    case encrypted
    case unavailable
}

enum SynaraCryptoVerificationStatus: Equatable {
    case unknown
    case verified
    case unverified
}

enum SynaraCryptoRecoveryStatus: Equatable {
    case unknown
    case enabled
    case disabled
    case incomplete
}

enum SynaraCryptoBackupStatus: Equatable {
    case unknown
    case enabled
    case unavailable
    case syncing
}

struct RoomCryptoStatus: Equatable {
    let encryption: SynaraRoomEncryptionStatus
    let verification: SynaraCryptoVerificationStatus
    let recovery: SynaraCryptoRecoveryStatus
    let backup: SynaraCryptoBackupStatus
    let unableToDecryptCount: Int

    static let unknown = RoomCryptoStatus(
        encryption: .unknown,
        verification: .unknown,
        recovery: .unknown,
        backup: .unknown,
        unableToDecryptCount: 0
    )

    var isEncrypted: Bool {
        encryption == .encrypted
    }

    var needsRecoveryAttention: Bool {
        isEncrypted && (recovery == .disabled || recovery == .incomplete || unableToDecryptCount > 0)
    }

    var needsCryptoActionBanner: Bool {
        verification == .unverified
            || unableToDecryptCount > 0
            || recovery == .incomplete
    }

    var roomHeaderLabel: String? {
        guard isEncrypted else {
            return encryption == .unavailable ? "Encryption Unavailable" : nil
        }
        if unableToDecryptCount > 0 || recovery == .incomplete || recovery == .disabled {
            return "Recovery Needed"
        }
        if backup == .unavailable {
            return "No Key Backup"
        }
        if verification == .unverified {
            return "Unverified"
        }
        return "Encrypted"
    }

    var roomHeaderSystemImage: String {
        needsCryptoActionBanner || needsRecoveryAttention
            ? "lock.trianglebadge.exclamationmark"
            : "lock.fill"
    }
}

struct SessionCryptoStatus: Equatable {
    let verification: SynaraCryptoVerificationStatus
    let recovery: SynaraCryptoRecoveryStatus
    let backup: SynaraCryptoBackupStatus
    let hasDevicesToVerifyAgainst: Bool?
    let isLastDevice: Bool?
    let unableToDecryptCount: Int

    static let unknown = SessionCryptoStatus(
        verification: .unknown,
        recovery: .unknown,
        backup: .unknown,
        hasDevicesToVerifyAgainst: nil,
        isLastDevice: nil,
        unableToDecryptCount: 0
    )
}

struct CryptoVerificationRequest: Equatable {
    let userID: String
    let displayName: String?
    let deviceID: String
    let deviceDisplayName: String?
    let flowID: String
}

struct CryptoVerificationEmoji: Equatable, Identifiable {
    let symbol: String
    let description: String

    var id: String {
        "\(symbol)-\(description)"
    }
}

enum CryptoVerificationState: Equatable, Identifiable {
    case requestReceived(CryptoVerificationRequest)
    case requestSent
    case accepted
    case sasStarted
    case emojis([CryptoVerificationEmoji])
    case decimals([UInt16])
    case confirmed
    case mismatched
    case finished
    case cancelled
    case failed

    var id: String {
        "session-verification"
    }

    var isTerminal: Bool {
        switch self {
        case .finished, .cancelled, .failed, .mismatched:
            return true
        case .requestReceived, .requestSent, .accepted, .sasStarted, .emojis, .decimals, .confirmed:
            return false
        }
    }

    var logLabel: String {
        switch self {
        case .requestReceived:
            return "request_received"
        case .requestSent:
            return "request_sent"
        case .accepted:
            return "accepted"
        case .sasStarted:
            return "sas_started"
        case .emojis(let emojis):
            return "emojis:\(emojis.count)"
        case .decimals(let values):
            return "decimals:\(values.count)"
        case .confirmed:
            return "confirmed"
        case .mismatched:
            return "mismatched"
        case .finished:
            return "finished"
        case .cancelled:
            return "cancelled"
        case .failed:
            return "failed"
        }
    }
}

enum CryptoActionResult: Equatable {
    case completed(String)
    case unavailable(String)
    case failed(String)

    var message: String {
        switch self {
        case .completed(let message), .unavailable(let message), .failed(let message):
            return message
        }
    }
}

protocol CryptoStatusServicing {
    func roomStatus(roomID: String) async -> RoomCryptoStatus
    func sessionStatus() async -> SessionCryptoStatus
    func verificationUpdates() -> AsyncStream<CryptoVerificationState>
    func currentVerificationState() async -> CryptoVerificationState?
    func retryDecryption(roomID: String) async -> CryptoActionResult
    func requestDeviceVerification() async -> CryptoActionResult
    func acceptVerificationRequest() async -> CryptoActionResult
    func startSasVerification() async -> CryptoActionResult
    func approveVerification() async -> CryptoActionResult
    func declineVerification() async -> CryptoActionResult
    func cancelVerification() async -> CryptoActionResult
    func recover(recoveryKey: String) async -> CryptoActionResult
    func sessionDevices() async -> [SharedCoreSessionDevice]
    func signOutSession(deviceId: String, password: String) async -> CryptoActionResult
    func dismissVerification() async -> CryptoActionResult
}

extension CryptoStatusServicing {
    func sessionDevices() async -> [SharedCoreSessionDevice] {
        []
    }

    func signOutSession(deviceId: String, password: String) async -> CryptoActionResult {
        _ = deviceId
        _ = password
        return .unavailable("Session sign-out is unavailable.")
    }

    func dismissVerification() async -> CryptoActionResult {
        .completed("Verification closed.")
    }

    func currentVerificationState() async -> CryptoVerificationState? {
        nil
    }
}

enum SynaraRoomVisibility: String, CaseIterable, Identifiable, Equatable {
    case `private` = "Private"
    case `public` = "Public"

    var id: String { rawValue }
}

enum SynaraRoomNotificationMode: String, CaseIterable, Identifiable, Equatable {
    case `default` = "Default"
    case allMessages = "All"
    case mentionsOnly = "Mentions"
    case mute = "Mute"

    var id: String { rawValue }
}

struct RoomCreateRequest: Equatable {
    let name: String
    let topic: String
    let visibility: SynaraRoomVisibility
    let isEncrypted: Bool
}

struct DirectMessageCreateRequest: Equatable {
    let userID: String
    let isEncrypted: Bool
}

struct RoomJoinRequest: Equatable {
    let reference: String
}

struct RoomProfileUpdateRequest: Equatable {
    let roomID: String
    let name: String?
    let topic: String?
    let canonicalAlias: String?
    let alternativeAliases: [String]?
    let avatar: RoomAvatarUpdate?

    init(
        roomID: String,
        name: String?,
        topic: String?,
        canonicalAlias: String? = nil,
        alternativeAliases: [String]? = nil,
        avatar: RoomAvatarUpdate? = nil
    ) {
        self.roomID = roomID
        self.name = name
        self.topic = topic
        self.canonicalAlias = canonicalAlias
        self.alternativeAliases = alternativeAliases
        self.avatar = avatar
    }
}

enum RoomAvatarUpdate: Equatable {
    case upload(data: Data, mimeType: String)
    case remove
}

struct RoomOperationResult: Equatable {
    let roomID: String
    let name: String?
}

struct PublicRoomSummary: Identifiable, Equatable {
    let id: String
    let name: String
    let topic: String?
    let alias: String?
    let memberCount: Int
    let isWorldReadable: Bool

    var joinReference: String {
        alias ?? id
    }
}

struct RoomPowerLevelSummary: Equatable {
    let ownUserLevel: Int64
    let usersDefault: Int64
    let eventsDefault: Int64
    let stateDefault: Int64
    let invite: Int64
    let kick: Int64
    let ban: Int64
    let redact: Int64
    let roomName: Int64
    let roomTopic: Int64
    let roomAvatar: Int64
    let canInvite: Bool
    let canKick: Bool
    let canBan: Bool
    let canRedactOther: Bool
    let canEditName: Bool
    let canEditTopic: Bool
    let canEditAvatar: Bool

    static let fullPower = RoomPowerLevelSummary(
        ownUserLevel: 100,
        usersDefault: 0,
        eventsDefault: 0,
        stateDefault: 50,
        invite: 50,
        kick: 50,
        ban: 50,
        redact: 50,
        roomName: 50,
        roomTopic: 50,
        roomAvatar: 50,
        canInvite: true,
        canKick: true,
        canBan: true,
        canRedactOther: true,
        canEditName: true,
        canEditTopic: true,
        canEditAvatar: true
    )
}

struct RoomMemberSummary: Equatable, Identifiable {
    let userID: String
    let membership: String
    let powerLevel: Int

    var id: String { userID }
}

struct RoomDetails: Equatable {
    let roomID: String
    let name: String
    let topic: String?
    let aliases: [String]
    let isEncrypted: Bool
    let isPublic: Bool?
    let memberCount: Int
    let canInvite: Bool
    let canEditName: Bool
    let canEditTopic: Bool
    let canEditAvatar: Bool
    let canEditAliases: Bool
    let powerLevels: RoomPowerLevelSummary?
    let notificationMode: SynaraRoomNotificationMode
    let avatarURL: String?
    let members: [RoomMemberSummary]
}

enum RoomManagementError: LocalizedError, Equatable {
    case signedOut
    case missingRoomName
    case missingUserID
    case missingRoomReference
    case invalidMatrixID
    case invalidRoomAlias
    case noProfileChanges
    case failed

    var errorDescription: String? {
        switch self {
        case .signedOut:
            return "Sign in before managing rooms."
        case .missingRoomName:
            return "Enter a room name."
        case .missingUserID:
            return "Enter a valid Matrix ID."
        case .missingRoomReference:
            return "Enter a room ID or alias."
        case .invalidMatrixID:
            return "Matrix IDs must look like @name:server."
        case .invalidRoomAlias:
            return "Room aliases must look like #room:server."
        case .noProfileChanges:
            return "Change the room name or topic before saving."
        case .failed:
            return "Room action failed. Try again."
        }
    }
}

protocol RoomManagementServicing {
    func createRoom(_ request: RoomCreateRequest) async throws -> RoomOperationResult
    func createDirectMessage(_ request: DirectMessageCreateRequest) async throws -> RoomOperationResult
    func joinRoom(_ request: RoomJoinRequest) async throws -> RoomOperationResult
    func leaveRoom(roomID: String) async throws
    func setRoomFavorite(_ favorite: Bool, roomID: String) async throws
    func inviteUser(roomID: String, userID: String) async throws
    func searchPublicRooms(query: String) async throws -> [PublicRoomSummary]
    func roomDetails(roomID: String) async -> RoomDetails?
    func updateRoomProfile(_ request: RoomProfileUpdateRequest) async throws
    func setNotificationMode(_ mode: SynaraRoomNotificationMode, roomID: String) async throws
    func stickers(roomID: String) async -> [SharedCoreSticker]
}

extension RoomManagementServicing {
    func stickers(roomID: String) async -> [SharedCoreSticker] {
        _ = roomID
        return []
    }
}

protocol SettingsStoring {
    func bool(for key: String) -> Bool
    func set(_ value: Bool, for key: String)
    func string(for key: String) -> String?
    func setString(_ value: String?, for key: String)
}

final class AppSessionStore: ObservableObject {
    @Published private(set) var currentState: SessionState
    @Published private(set) var sessionEpoch: Int = 0
    let secureStore: SecureSessionStoring
    private(set) var restoreFailureLogDescription: String?

    init(
        currentState: SessionState = .signedOut,
        secureStore: SecureSessionStoring = InMemorySecureSessionStore(),
        restorePersistedSession: Bool = false
    ) {
        self.secureStore = secureStore

        if restorePersistedSession {
            do {
                _ = try secureStore.migrateIfNeeded()
                if let restored = try secureStore.load() {
                    self.currentState = .signedIn(restored)
                } else {
                    self.currentState = currentState
                }
            } catch let error as SecureSessionStoreError {
                restoreFailureLogDescription = error.logDescription
                self.currentState = .signedOut
            } catch {
                restoreFailureLogDescription = "secure session restore failed"
                self.currentState = .signedOut
            }
        } else {
            self.currentState = currentState
        }
    }

    func restore() throws {
        do {
            if let restored = try secureStore.load() {
                restoreFailureLogDescription = nil
                currentState = .signedIn(restored)
            } else {
                restoreFailureLogDescription = nil
                currentState = .signedOut
            }
        } catch let error as SecureSessionStoreError {
            restoreFailureLogDescription = error.logDescription
            throw error
        } catch {
            restoreFailureLogDescription = "secure session restore failed"
            throw error
        }
    }

    func completeLogin(_ session: AuthenticatedSession) throws {
        try secureStore.save(session)
        sessionEpoch += 1
        currentState = .signedIn(session)
    }

    func signOut() throws {
        try secureStore.delete()
        sessionEpoch += 1
        currentState = .signedOut
    }
}

final class PlaceholderMatrixClientService: MatrixClientServicing {
    private(set) var syncStatus: MatrixSyncStatus = .stopped

    var syncStatusDescription: String {
        syncStatus.description
    }

    func start(session: AuthenticatedSession) async {
        syncStatus = .syncing
    }

    func warmSync(session: AuthenticatedSession) async {
        syncStatus = .syncing
    }

    func revokeServerSession(_ session: AuthenticatedSession) async -> Bool {
        _ = session
        return false
    }

    func stop() async {
        syncStatus = .stopped
    }

    func pauseForBackground() async {
        syncStatus = .stopped
    }

    func resumeFromForeground(session: AuthenticatedSession) async {
        _ = session
        syncStatus = .syncing
    }

    func syncForBackgroundNotification(session: AuthenticatedSession) async -> Bool {
        _ = session
        return false
    }

    func resetLocalState(for session: AuthenticatedSession?) async {
        _ = session
        syncStatus = .stopped
    }
}

final class PlaceholderPushService: PushServicing {
    let isRegistrationAvailable = false
    let isRegistered = false
    let tokenSnippet: String? = nil
    let registrationStateDescription = "Push unavailable"
    let pushGatewayURL: String? = nil

    func beginRegistration() {}

    func handleDeviceToken(_ tokenData: Data) {}

    func clearRegistrationState() async {}

    func configure(with session: AuthenticatedSession) {}

    func route(from notificationPayload: [AnyHashable: Any]) -> AppRoute? {
        nil
    }

    func resolveRoute(from notificationPayload: [AnyHashable: Any]) async -> AppRoute? {
        nil
    }

    func parseBadgeCount(from notificationPayload: [AnyHashable: Any]) -> Int? {
        nil
    }

    func applyIncomingBadge(from notificationPayload: [AnyHashable: Any]) {}
}

struct MockCryptoStatusService: CryptoStatusServicing {
    var roomCryptoStatus: RoomCryptoStatus
    var sessionCryptoStatus: SessionCryptoStatus

    init(
        roomCryptoStatus: RoomCryptoStatus = .unknown,
        sessionCryptoStatus: SessionCryptoStatus = .unknown
    ) {
        self.roomCryptoStatus = roomCryptoStatus
        self.sessionCryptoStatus = sessionCryptoStatus
    }

    func roomStatus(roomID: String) async -> RoomCryptoStatus {
        roomCryptoStatus
    }

    func sessionStatus() async -> SessionCryptoStatus {
        sessionCryptoStatus
    }

    func verificationUpdates() -> AsyncStream<CryptoVerificationState> {
        AsyncStream { continuation in
            continuation.finish()
        }
    }

    func retryDecryption(roomID: String) async -> CryptoActionResult {
        .completed("Decryption retry started.")
    }

    func requestDeviceVerification() async -> CryptoActionResult {
        .completed("Device verification request sent.")
    }

    func acceptVerificationRequest() async -> CryptoActionResult {
        .completed("Verification request accepted.")
    }

    func startSasVerification() async -> CryptoActionResult {
        .completed("Verification comparison started.")
    }

    func approveVerification() async -> CryptoActionResult {
        .completed("Device verified.")
    }

    func declineVerification() async -> CryptoActionResult {
        .completed("Verification declined.")
    }

    func cancelVerification() async -> CryptoActionResult {
        .completed("Verification cancelled.")
    }

    func recover(recoveryKey: String) async -> CryptoActionResult {
        let trimmed = recoveryKey.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false else {
            return .failed("Enter a recovery key before recovering keys.")
        }
        return .completed("Recovery key accepted.")
    }
}

final class MockRoomManagementService: RoomManagementServicing {
    private var detailsByRoomID: [String: RoomDetails]
    private var nextRoomIndex = 0
    private(set) var createdRooms: [RoomCreateRequest] = []
    private(set) var createdDMs: [DirectMessageCreateRequest] = []
    private(set) var joinedRooms: [RoomJoinRequest] = []
    private(set) var leftRoomIDs: [String] = []
    private(set) var invitedUsers: [(roomID: String, userID: String)] = []

    init(detailsByRoomID: [String: RoomDetails] = [:]) {
        self.detailsByRoomID = detailsByRoomID
    }

    func createRoom(_ request: RoomCreateRequest) async throws -> RoomOperationResult {
        let trimmedName = request.name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmedName.isEmpty == false else {
            throw RoomManagementError.missingRoomName
        }
        createdRooms.append(request)
        nextRoomIndex += 1
        let roomID = "!created-\(nextRoomIndex):matrix.org"
        detailsByRoomID[roomID] = RoomDetails(
            roomID: roomID,
            name: trimmedName,
            topic: request.topic.isEmpty ? nil : request.topic,
            aliases: [],
            isEncrypted: request.isEncrypted,
            isPublic: request.visibility == .public,
            memberCount: 1,
            canInvite: true,
            canEditName: true,
            canEditTopic: true,
            canEditAvatar: true,
            canEditAliases: true,
            powerLevels: .fullPower,
            notificationMode: .default,
            avatarURL: nil,
            members: []
        )
        return RoomOperationResult(roomID: roomID, name: trimmedName)
    }

    func createDirectMessage(_ request: DirectMessageCreateRequest) async throws -> RoomOperationResult {
        let trimmedUserID = request.userID.trimmingCharacters(in: .whitespacesAndNewlines)
        guard Self.isValidMatrixID(trimmedUserID) else {
            throw RoomManagementError.invalidMatrixID
        }
        createdDMs.append(request)
        nextRoomIndex += 1
        return RoomOperationResult(roomID: "!dm-\(nextRoomIndex):matrix.org", name: trimmedUserID)
    }

    func joinRoom(_ request: RoomJoinRequest) async throws -> RoomOperationResult {
        let trimmedReference = request.reference.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmedReference.isEmpty == false else {
            throw RoomManagementError.missingRoomReference
        }
        joinedRooms.append(request)
        return RoomOperationResult(roomID: trimmedReference.hasPrefix("!") ? trimmedReference : "!joined:matrix.org", name: trimmedReference)
    }

    func leaveRoom(roomID: String) async throws {
        leftRoomIDs.append(roomID)
    }

    func setRoomFavorite(_ favorite: Bool, roomID: String) async throws {
        _ = favorite
        _ = roomID
    }

    func inviteUser(roomID: String, userID: String) async throws {
        let trimmedUserID = userID.trimmingCharacters(in: .whitespacesAndNewlines)
        guard Self.isValidMatrixID(trimmedUserID) else {
            throw RoomManagementError.invalidMatrixID
        }
        invitedUsers.append((roomID: roomID, userID: trimmedUserID))
    }

    func searchPublicRooms(query: String) async throws -> [PublicRoomSummary] {
        let trimmedQuery = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmedQuery.isEmpty == false else {
            return []
        }

        return [
            PublicRoomSummary(
                id: "!public-\(trimmedQuery.lowercased()):matrix.org",
                name: "\(trimmedQuery) Public",
                topic: "Public room matching \(trimmedQuery).",
                alias: "#\(trimmedQuery.lowercased()):matrix.org",
                memberCount: 42,
                isWorldReadable: true
            )
        ]
    }

    func roomDetails(roomID: String) async -> RoomDetails? {
        detailsByRoomID[roomID] ?? RoomDetails(
            roomID: roomID,
            name: roomID,
            topic: "Room details from the current Matrix session.",
            aliases: [],
            isEncrypted: roomID.localizedCaseInsensitiveContains("encrypted"),
            isPublic: nil,
            memberCount: 3,
            canInvite: true,
            canEditName: true,
            canEditTopic: true,
            canEditAvatar: true,
            canEditAliases: true,
            powerLevels: .fullPower,
            notificationMode: .default,
            avatarURL: nil,
            members: []
        )
    }

    func updateRoomProfile(_ request: RoomProfileUpdateRequest) async throws {
        var existingDetails = detailsByRoomID[request.roomID]
        if existingDetails == nil {
            existingDetails = await roomDetails(roomID: request.roomID)
        }
        guard var details = existingDetails else {
            throw RoomManagementError.failed
        }

        let trimmedName = request.name?.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedTopic = request.topic?.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedAlias = request.canonicalAlias?.trimmingCharacters(in: .whitespacesAndNewlines)
        let alternativeAliases = request.alternativeAliases?
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { $0.isEmpty == false }
        guard trimmedName != nil || trimmedTopic != nil || trimmedAlias != nil || alternativeAliases != nil || request.avatar != nil else {
            throw RoomManagementError.noProfileChanges
        }
        if let trimmedName, trimmedName.isEmpty {
            throw RoomManagementError.missingRoomName
        }
        if let trimmedAlias, trimmedAlias.isEmpty == false, Self.isValidRoomAlias(trimmedAlias) == false {
            throw RoomManagementError.invalidRoomAlias
        }

        var aliases = details.aliases
        if let trimmedAlias {
            aliases = trimmedAlias.isEmpty ? (alternativeAliases ?? []) : [trimmedAlias] + (alternativeAliases ?? [])
        } else if let alternativeAliases {
            aliases = alternativeAliases
        }

        details = RoomDetails(
            roomID: details.roomID,
            name: trimmedName ?? details.name,
            topic: trimmedTopic ?? details.topic,
            aliases: aliases,
            isEncrypted: details.isEncrypted,
            isPublic: details.isPublic,
            memberCount: details.memberCount,
            canInvite: details.canInvite,
            canEditName: details.canEditName,
            canEditTopic: details.canEditTopic,
            canEditAvatar: details.canEditAvatar,
            canEditAliases: details.canEditAliases,
            powerLevels: details.powerLevels,
            notificationMode: details.notificationMode,
            avatarURL: avatarURL(after: request.avatar, current: details.avatarURL),
            members: details.members
        )
        detailsByRoomID[request.roomID] = details
    }

    private func avatarURL(after update: RoomAvatarUpdate?, current: String?) -> String? {
        switch update {
        case .upload:
            return "mxc://mock/room-avatar"
        case .remove:
            return nil
        case nil:
            return current
        }
    }

    func setNotificationMode(_ mode: SynaraRoomNotificationMode, roomID: String) async throws {
        guard var details = detailsByRoomID[roomID] else {
            return
        }
        details = RoomDetails(
            roomID: details.roomID,
            name: details.name,
            topic: details.topic,
            aliases: details.aliases,
            isEncrypted: details.isEncrypted,
            isPublic: details.isPublic,
            memberCount: details.memberCount,
            canInvite: details.canInvite,
            canEditName: details.canEditName,
            canEditTopic: details.canEditTopic,
            canEditAvatar: details.canEditAvatar,
            canEditAliases: details.canEditAliases,
            powerLevels: details.powerLevels,
            notificationMode: mode,
            avatarURL: details.avatarURL,
            members: details.members
        )
        detailsByRoomID[roomID] = details
    }

    private static func isValidMatrixID(_ value: String) -> Bool {
        value.hasPrefix("@") && value.contains(":") && value.count > 3
    }

    private static func isValidRoomAlias(_ value: String) -> Bool {
        value.hasPrefix("#") && value.contains(":") && value.count > 3
    }
}

struct UserNotificationPermissionService: NotificationPermissionServicing {
    func currentStatus() async -> NotificationPermissionStatus {
        let settings = await UNUserNotificationCenter.current().notificationSettings()
        return NotificationPermissionStatus.map(settings.authorizationStatus)
    }

    func requestAuthorization() async -> NotificationPermissionStatus {
        do {
            _ = try await UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .badge, .sound])
            return await currentStatus()
        } catch {
            return .unavailable
        }
    }
}

final class UserDefaultsSettingsStore: SettingsStoring {
    private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
        defaults.register(defaults: SynaraSharedConstants.registeredUserDefaults)
    }

    func bool(for key: String) -> Bool {
        defaults.bool(forKey: key)
    }

    func set(_ value: Bool, for key: String) {
        defaults.set(value, forKey: key)
    }

    func string(for key: String) -> String? {
        defaults.string(forKey: key)
    }

    func setString(_ value: String?, for key: String) {
        if let value {
            defaults.set(value, forKey: key)
        } else {
            defaults.removeObject(forKey: key)
        }
    }
}

final class InMemorySettingsStore: SettingsStoring {
    private var values: [String: Bool] = [:]
    private var strings: [String: String] = [:]

    func bool(for key: String) -> Bool {
        values[key] ?? false
    }

    func set(_ value: Bool, for key: String) {
        values[key] = value
    }

    func string(for key: String) -> String? {
        strings[key]
    }

    func setString(_ value: String?, for key: String) {
        if let value {
            strings[key] = value
        } else {
            strings.removeValue(forKey: key)
        }
    }
}

final class MockMatrixClientService: MatrixClientServicing {
    private(set) var syncStatus: MatrixSyncStatus
    private(set) var startedSessions: [AuthenticatedSession] = []
    private(set) var stopCallCount = 0
    private(set) var resetCallCount = 0
    private(set) var revokedSessions: [AuthenticatedSession] = []
    var serverRevocationResult = true
    var onOperation: ((String) -> Void)?

    var syncStatusDescription: String {
        syncStatus.description
    }

    init(syncStatus: MatrixSyncStatus = .stopped) {
        self.syncStatus = syncStatus
    }

    func start(session: AuthenticatedSession) async {
        startedSessions.append(session)
        syncStatus = .syncing
    }

    func warmSync(session: AuthenticatedSession) async {
        startedSessions.append(session)
        syncStatus = .syncing
    }

    func stop() async {
        onOperation?("matrix-stop")
        stopCallCount += 1
        syncStatus = .stopped
    }

    func revokeServerSession(_ session: AuthenticatedSession) async -> Bool {
        onOperation?("server-revoke")
        revokedSessions.append(session)
        return serverRevocationResult
    }

    private(set) var pauseCallCount = 0
    private(set) var resumeCallCount = 0
    private(set) var resumedSessions: [AuthenticatedSession] = []
    private(set) var backgroundSyncCallCount = 0
    var backgroundSyncResult = false

    func pauseForBackground() async {
        pauseCallCount += 1
        syncStatus = .stopped
    }

    func resumeFromForeground(session: AuthenticatedSession) async {
        resumeCallCount += 1
        resumedSessions.append(session)
        syncStatus = .syncing
    }

    func syncForBackgroundNotification(session: AuthenticatedSession) async -> Bool {
        backgroundSyncCallCount += 1
        _ = session
        return backgroundSyncResult
    }

    private(set) var resetSessions: [AuthenticatedSession?] = []

    func resetLocalState(for session: AuthenticatedSession?) async {
        onOperation?("matrix-reset")
        resetCallCount += 1
        resetSessions.append(session)
        syncStatus = .stopped
    }
}

final class MockPushService: PushServicing {
    let isRegistrationAvailable: Bool
    let pushGatewayURL: String?
    private(set) var clearCallCount = 0
    private(set) var beginRegistrationCallCount = 0
    private(set) var configureCallCount = 0
    private(set) var routeCallCount = 0
    private(set) var badgeParseCallCount = 0
    private(set) var badgeApplyCallCount = 0
    private(set) var tokenCallCount = 0
    var isRegistered = false
    var tokenSnippet: String?
    var onOperation: ((String) -> Void)?
    var registrationStateDescription: String {
        isRegistered ? "Mock registered" : "Mock unregistered"
    }

    private let routeOverride: AppRoute?

    init(
        isRegistrationAvailable: Bool = false,
        pushGatewayURL: String? = nil,
        routeOverride: AppRoute? = nil
    ) {
        self.isRegistrationAvailable = isRegistrationAvailable
        self.pushGatewayURL = pushGatewayURL
        self.routeOverride = routeOverride
    }

    func beginRegistration() {
        beginRegistrationCallCount += 1
    }

    func handleDeviceToken(_ tokenData: Data) {
        tokenCallCount += 1
        tokenSnippet = tokenData.map { String(format: "%02x", $0) }.joined()
        isRegistered = true
    }

    func clearRegistrationState() async {
        onOperation?("push-clear")
        clearCallCount += 1
        isRegistered = false
        tokenSnippet = nil
    }

    func configure(with session: AuthenticatedSession) {
        configureCallCount += 1
    }

    func route(from notificationPayload: [AnyHashable: Any]) -> AppRoute? {
        routeCallCount += 1
        return routeOverride
    }

    func resolveRoute(from notificationPayload: [AnyHashable: Any]) async -> AppRoute? {
        routeCallCount += 1
        return routeOverride
    }

    func parseBadgeCount(from notificationPayload: [AnyHashable: Any]) -> Int? {
        badgeParseCallCount += 1

        if let value = notificationPayload["badge"] as? Int {
            return value
        }

        if let value = notificationPayload["badge_count"] as? Int {
            return value
        }

        if let value = notificationPayload["synara.badge"] as? Int {
            return value
        }

        return nil
    }

    func applyIncomingBadge(from notificationPayload: [AnyHashable: Any]) {
        badgeApplyCallCount += 1
    }
}

final class MockNotificationPermissionService: NotificationPermissionServicing {
    var status: NotificationPermissionStatus
    var statusAfterRequest: NotificationPermissionStatus?
    private(set) var requestCallCount = 0

    init(
        status: NotificationPermissionStatus = .notDetermined,
        statusAfterRequest: NotificationPermissionStatus? = nil
    ) {
        self.status = status
        self.statusAfterRequest = statusAfterRequest
    }

    func currentStatus() async -> NotificationPermissionStatus {
        status
    }

    func requestAuthorization() async -> NotificationPermissionStatus {
        requestCallCount += 1
        if let statusAfterRequest {
            status = statusAfterRequest
        }
        return status
    }
}
