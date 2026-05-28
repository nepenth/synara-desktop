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
    case failed(String)

    var description: String {
        switch self {
        case .stopped:
            return "Not connected"
        case .starting:
            return "Starting sync"
        case .syncing:
            return "Syncing"
        case .failed(let message):
            return message
        }
    }
}

protocol MatrixClientServicing: AnyObject {
    var syncStatusDescription: String { get }
    var syncStatus: MatrixSyncStatus { get }

    func start(session: AuthenticatedSession) async
    func stop() async
    func resetLocalState()
}

protocol PushServicing {
    var isRegistrationAvailable: Bool { get }
    var isRegistered: Bool { get }
    var tokenSnippet: String? { get }
    var registrationStateDescription: String { get }
    var pushGatewayURL: String? { get }

    func beginRegistration()
    func handleDeviceToken(_ tokenData: Data)
    func clearRegistrationState()
    func configure(with session: AuthenticatedSession)
    func route(from notificationPayload: [AnyHashable: Any]) -> AppRoute?
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
    func retryDecryption(roomID: String) async -> CryptoActionResult
    func requestDeviceVerification() async -> CryptoActionResult
    func recover(recoveryKey: String) async -> CryptoActionResult
}

enum SynaraRoomVisibility: String, CaseIterable, Identifiable, Equatable {
    case `private` = "Private"
    case `public` = "Public"

    var id: String { rawValue }
}

enum SynaraRoomNotificationMode: String, CaseIterable, Identifiable, Equatable {
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
}

struct RoomOperationResult: Equatable {
    let roomID: String
    let name: String?
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
    let notificationMode: SynaraRoomNotificationMode
}

enum RoomManagementError: LocalizedError, Equatable {
    case signedOut
    case missingRoomName
    case missingUserID
    case missingRoomReference
    case invalidMatrixID
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
    func inviteUser(roomID: String, userID: String) async throws
    func roomDetails(roomID: String) async -> RoomDetails?
    func updateRoomProfile(_ request: RoomProfileUpdateRequest) async throws
    func setNotificationMode(_ mode: SynaraRoomNotificationMode, roomID: String) async throws
}

protocol SettingsStoring {
    func bool(for key: String) -> Bool
    func set(_ value: Bool, for key: String)
}

final class AppSessionStore: ObservableObject {
    @Published private(set) var currentState: SessionState
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
        currentState = .signedIn(session)
    }

    func signOut() throws {
        try secureStore.delete()
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

    func stop() async {
        syncStatus = .stopped
    }

    func resetLocalState() {
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

    func clearRegistrationState() {}

    func configure(with session: AuthenticatedSession) {}

    func route(from notificationPayload: [AnyHashable: Any]) -> AppRoute? {
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

    func retryDecryption(roomID: String) async -> CryptoActionResult {
        .completed("Decryption retry started.")
    }

    func requestDeviceVerification() async -> CryptoActionResult {
        .completed("Device verification request sent.")
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
            notificationMode: .allMessages
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

    func inviteUser(roomID: String, userID: String) async throws {
        let trimmedUserID = userID.trimmingCharacters(in: .whitespacesAndNewlines)
        guard Self.isValidMatrixID(trimmedUserID) else {
            throw RoomManagementError.invalidMatrixID
        }
        invitedUsers.append((roomID: roomID, userID: trimmedUserID))
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
            notificationMode: .allMessages
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
        guard trimmedName != nil || trimmedTopic != nil else {
            throw RoomManagementError.noProfileChanges
        }
        if let trimmedName, trimmedName.isEmpty {
            throw RoomManagementError.missingRoomName
        }

        details = RoomDetails(
            roomID: details.roomID,
            name: trimmedName ?? details.name,
            topic: trimmedTopic ?? details.topic,
            aliases: details.aliases,
            isEncrypted: details.isEncrypted,
            isPublic: details.isPublic,
            memberCount: details.memberCount,
            canInvite: details.canInvite,
            canEditName: details.canEditName,
            canEditTopic: details.canEditTopic,
            notificationMode: details.notificationMode
        )
        detailsByRoomID[request.roomID] = details
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
            notificationMode: mode
        )
        detailsByRoomID[roomID] = details
    }

    private static func isValidMatrixID(_ value: String) -> Bool {
        value.hasPrefix("@") && value.contains(":") && value.count > 3
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

final class InMemorySettingsStore: SettingsStoring {
    private var values: [String: Bool] = [:]

    func bool(for key: String) -> Bool {
        values[key] ?? false
    }

    func set(_ value: Bool, for key: String) {
        values[key] = value
    }
}

final class MockMatrixClientService: MatrixClientServicing {
    private(set) var syncStatus: MatrixSyncStatus
    private(set) var startedSessions: [AuthenticatedSession] = []
    private(set) var stopCallCount = 0
    private(set) var resetCallCount = 0

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

    func stop() async {
        stopCallCount += 1
        syncStatus = .stopped
    }

    func resetLocalState() {
        resetCallCount += 1
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

    func clearRegistrationState() {
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
    private(set) var requestCallCount = 0

    init(status: NotificationPermissionStatus = .notDetermined) {
        self.status = status
    }

    func currentStatus() async -> NotificationPermissionStatus {
        status
    }

    func requestAuthorization() async -> NotificationPermissionStatus {
        requestCallCount += 1
        return status
    }
}
