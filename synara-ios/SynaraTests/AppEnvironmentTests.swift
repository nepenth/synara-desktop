import XCTest
import UserNotifications
import SynaraCore
@testable import Synara

final class AppEnvironmentTests: XCTestCase {
    func testMockEnvironmentInstallsExpectedServices() {
        let router = AppRouter()
        let environment = AppEnvironment.mock(router: router)

        XCTAssertTrue(environment.session.currentState == .signedOut)
        XCTAssertEqual(environment.matrix.syncStatusDescription, "Not connected")
        XCTAssertFalse(environment.push.isRegistrationAvailable)
        XCTAssertTrue(environment.router === router)
        XCTAssertTrue(environment.auth is MockAuthService)
        XCTAssertTrue(environment.notificationPermission is MockNotificationPermissionService)
    }

    func testMockEnvironmentInstallsLaterService() {
        let environment = AppEnvironment.mock()

        XCTAssertTrue(environment.later is MockLaterService)
    }

    func testMatrixClientCoreSessionIdentityDefaultsToNilForMock() async {
        let matrix: any MatrixClientServicing = MockMatrixClientService()

        let identity = await matrix.coreSessionIdentity()

        XCTAssertNil(identity)
    }

    func testSettingsCoreIdentitySelectionRequiresExactSwiftSessionMatch() throws {
        let session = AuthenticatedSession(
            userID: "@alice:matrix.org",
            deviceID: "SYNARA-IOS-DEVICE",
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.org")),
            accessToken: "token"
        )
        let matching = CoreSessionIdentity(
            userID: session.userID,
            deviceID: session.deviceID,
            homeserverURL: session.homeserverURL.absoluteString
        )
        let differentDevice = CoreSessionIdentity(
            userID: session.userID,
            deviceID: "OTHER-DEVICE",
            homeserverURL: session.homeserverURL.absoluteString
        )

        XCTAssertEqual(
            SettingsAccountIdentitySelection.matchingCoreIdentity(matching, for: session),
            matching
        )
        XCTAssertNil(SettingsAccountIdentitySelection.matchingCoreIdentity(differentDevice, for: session))
        XCTAssertNil(SettingsAccountIdentitySelection.matchingCoreIdentity(nil, for: session))
        XCTAssertEqual(
            SettingsAccountIdentitySelection.homeserverDisplayValue(for: matching, fallback: session.homeserverURL),
            "matrix.org"
        )
        XCTAssertEqual(
            SettingsAccountIdentitySelection.homeserverDisplayValue(for: nil, fallback: session.homeserverURL),
            "matrix.org"
        )
    }

    func testLiveHomeserverDiscoveryConstructorUsesCoreService() {
        XCTAssertTrue(AppEnvironment.makeLiveHomeserverDiscovery() is CoreHomeserverDiscoveryService)
    }

    func testSharedCoreLaunchResetCompletesBeforeCoreConstruction() throws {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("synara-launch-reset-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        let staleFile = root.appendingPathComponent("stale-store-marker")
        try Data("stale".utf8).write(to: staleFile)
        defer { try? FileManager.default.removeItem(at: root) }

        XCTAssertTrue(try SharedCoreLaunchReset.resetStoreRootIfRequested(
            root,
            environment: ["SYNARA_RESET_SESSION_ON_LAUNCH": "1"]
        ))
        XCTAssertTrue(FileManager.default.fileExists(atPath: root.path))
        XCTAssertFalse(FileManager.default.fileExists(atPath: staleFile.path))
        XCTAssertFalse(try SharedCoreLaunchReset.resetStoreRootIfRequested(root, environment: [:]))
    }

    func testSharedCoreLoginErrorMappingDoesNotMislabelVaultFailureAsBadCredentials() {
        XCTAssertEqual(
            SharedCoreLoginErrorMapping.loginError(for: SessionLoginError.Failed(
                code: "p4-s3c-secret-vault-unavailable",
                description: "The secret store is unavailable."
            )),
            .sessionPersistenceFailed
        )
        XCTAssertEqual(
            SharedCoreLoginErrorMapping.loginError(for: SessionLoginError.Failed(
                code: "p4-s3c-login-failed",
                description: "The session could not be authenticated."
            )),
            .invalidCredentials
        )
    }

    @MainActor
    func testLiveEnvironmentUsesSharedCoreServices() {
        let environment = AppEnvironment.live()

        XCTAssertTrue(environment.auth is SharedCoreAuthService)
        XCTAssertTrue(environment.roomList is SharedCoreRoomListService)
        XCTAssertTrue(environment.roomMembership is SharedCoreRoomMembershipService)
        XCTAssertTrue(environment.timeline is SharedCoreTimelineService)
        XCTAssertTrue(environment.later is SharedCoreLaterService)
        XCTAssertTrue(environment.messageSender is SharedCoreMessageSendService)
        XCTAssertTrue(environment.eventActions is SharedCoreEventActionService)
        XCTAssertTrue(environment.agentApprovals is SharedCoreAgentApprovalService)
        XCTAssertTrue(environment.crypto is SharedCoreCryptoStatusService)
        XCTAssertTrue(environment.roomManagement is SharedCoreRoomManagementService)
    }

    func testRoomCryptoStatusFlagsRecoveryAttentionForEncryptedProblems() {
        let status = RoomCryptoStatus(
            encryption: .encrypted,
            verification: .unverified,
            recovery: .incomplete,
            backup: .unavailable,
            unableToDecryptCount: 1
        )

        XCTAssertTrue(status.isEncrypted)
        XCTAssertTrue(status.needsRecoveryAttention)
        XCTAssertTrue(status.needsCryptoActionBanner)
        XCTAssertEqual(status.roomHeaderLabel, "Recovery Needed")
        XCTAssertEqual(status.roomHeaderSystemImage, "lock.trianglebadge.exclamationmark")
    }

    func testRoomCryptoStatusOmitsCryptoActionBannerWhenHealthy() {
        let status = RoomCryptoStatus(
            encryption: .encrypted,
            verification: .verified,
            recovery: .enabled,
            backup: .enabled,
            unableToDecryptCount: 0
        )

        XCTAssertFalse(status.needsCryptoActionBanner)
        XCTAssertEqual(status.roomHeaderLabel, "Encrypted")
        XCTAssertEqual(status.roomHeaderSystemImage, "lock.fill")
    }

    func testMockCryptoRecoverRejectsEmptyRecoveryKey() async {
        let result = await MockCryptoStatusService().recover(recoveryKey: "   ")

        XCTAssertEqual(result, .failed("Enter a recovery key before recovering keys."))
    }

    func testCryptoVerificationLogLabelsOmitVerificationPayloadValues() {
        let request = CryptoVerificationRequest(
            userID: "@alice:example.org",
            displayName: "Alice",
            deviceID: "DEVICE123",
            deviceDisplayName: "Alice Phone",
            flowID: "flow-123"
        )
        let emojiState = CryptoVerificationState.emojis([
            CryptoVerificationEmoji(symbol: "key-symbol", description: "Key")
        ])
        let decimalState = CryptoVerificationState.decimals([1234, 5678, 9012])

        XCTAssertEqual(CryptoVerificationState.requestReceived(request).logLabel, "request_received")
        XCTAssertEqual(emojiState.logLabel, "emojis:1")
        XCTAssertEqual(decimalState.logLabel, "decimals:3")
        XCTAssertFalse(emojiState.logLabel.contains("key-symbol"))
        XCTAssertFalse(decimalState.logLabel.contains("1234"))
        XCTAssertFalse(CryptoVerificationState.requestReceived(request).logLabel.contains("@alice"))
    }

    func testMockRoomManagementCreatesEncryptedPrivateRoom() async throws {
        let service = MockRoomManagementService()

        let result = try await service.createRoom(
            RoomCreateRequest(
                name: "Incident Room",
                topic: "Operational response",
                visibility: .private,
                isEncrypted: true
            )
        )
        let details = await service.roomDetails(roomID: result.roomID)

        XCTAssertEqual(result.name, "Incident Room")
        XCTAssertEqual(details?.name, "Incident Room")
        XCTAssertEqual(details?.topic, "Operational response")
        XCTAssertEqual(details?.isEncrypted, true)
        XCTAssertEqual(details?.isPublic, false)
        XCTAssertEqual(details?.powerLevels?.ownUserLevel, 100)
        XCTAssertEqual(details?.powerLevels?.canEditTopic, true)
    }

    func testMockRoomManagementValidatesMatrixIDs() async throws {
        let service = MockRoomManagementService()

        do {
            _ = try await service.createDirectMessage(DirectMessageCreateRequest(userID: "alice", isEncrypted: true))
            XCTFail("Expected invalid Matrix ID to fail.")
        } catch let error as RoomManagementError {
            XCTAssertEqual(error, .invalidMatrixID)
        }
    }

    func testMockRoomManagementUpdatesRoomProfile() async throws {
        let service = MockRoomManagementService()
        let result = try await service.createRoom(
            RoomCreateRequest(
                name: "Incident Room",
                topic: "Operational response",
                visibility: .private,
                isEncrypted: true
            )
        )

        try await service.updateRoomProfile(
            RoomProfileUpdateRequest(
                roomID: result.roomID,
                name: "Incident Review",
                topic: "Post-incident follow-up"
            )
        )
        let details = await service.roomDetails(roomID: result.roomID)

        XCTAssertEqual(details?.name, "Incident Review")
        XCTAssertEqual(details?.topic, "Post-incident follow-up")
    }

    func testMockRoomManagementUpdatesAliasesAndAvatar() async throws {
        let service = MockRoomManagementService()
        let result = try await service.createRoom(
            RoomCreateRequest(
                name: "Incident Room",
                topic: "Operational response",
                visibility: .private,
                isEncrypted: true
            )
        )

        try await service.updateRoomProfile(
            RoomProfileUpdateRequest(
                roomID: result.roomID,
                name: nil,
                topic: nil,
                canonicalAlias: "#incident:matrix.org",
                alternativeAliases: ["#incident-review:matrix.org"],
                avatar: .upload(data: Data("avatar".utf8), mimeType: "image/jpeg")
            )
        )
        let details = await service.roomDetails(roomID: result.roomID)

        XCTAssertEqual(details?.aliases, ["#incident:matrix.org", "#incident-review:matrix.org"])
        XCTAssertEqual(details?.avatarURL, "mxc://mock/room-avatar")
    }

    func testMockRoomManagementSearchesPublicRooms() async throws {
        let service = MockRoomManagementService()

        let results = try await service.searchPublicRooms(query: "alerts")

        XCTAssertEqual(results.first?.name, "alerts Public")
        XCTAssertEqual(results.first?.joinReference, "#alerts:matrix.org")
    }

    func testMockRoomManagementRejectsEmptyProfileUpdate() async throws {
        let service = MockRoomManagementService()

        do {
            try await service.updateRoomProfile(RoomProfileUpdateRequest(roomID: "!room:matrix.org", name: nil, topic: nil))
            XCTFail("Expected empty profile update to fail.")
        } catch let error as RoomManagementError {
            XCTAssertEqual(error, .noProfileChanges)
        }
    }

    func testUserDefaultsSettingsStorePersistsAcrossInstances() {
        let suiteName = "synara.settings.test.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer {
            defaults.removePersistentDomain(forName: suiteName)
        }

        let first = UserDefaultsSettingsStore(defaults: defaults)
        first.set(true, for: "largeText")

        let second = UserDefaultsSettingsStore(defaults: defaults)
        XCTAssertTrue(second.bool(for: "largeText"))
    }

    func testSettingsStorePersistsBooleansInMemory() {
        let settings = InMemorySettingsStore()

        XCTAssertFalse(settings.bool(for: "largeText"))

        settings.set(true, for: "largeText")

        XCTAssertTrue(settings.bool(for: "largeText"))
    }

    func testNotificationPermissionStatusMapsAuthorizationStates() {
        XCTAssertEqual(NotificationPermissionStatus.map(.notDetermined), .notDetermined)
        XCTAssertEqual(NotificationPermissionStatus.map(.denied), .denied)
        XCTAssertEqual(NotificationPermissionStatus.map(.authorized), .authorized)
        XCTAssertEqual(NotificationPermissionStatus.map(.provisional), .provisional)
        XCTAssertEqual(NotificationPermissionStatus.map(.ephemeral), .ephemeral)
    }

    @MainActor
    func testLiveEnvironmentReadsPushGatewayFromEnvironment() {
        let variable = "SYNARA_PUSH_GATEWAY_URL"
        setenv(variable, "https://push.example.internal", 1)

        let environment = AppEnvironment.live()

        unsetenv(variable)

        XCTAssertEqual((environment.push as? SynaraPushService)?.pushGatewayURL, "https://push.example.internal")
    }

    func testPushGatewayConfigurationPrefersEnvironmentValue() {
        let url = AppEnvironment.configuredPushGatewayURL(
            environmentValue: "https://push.example.internal/_matrix/push/v1/notify",
            bundleValue: "https://push.example.com/_matrix/push/v1/notify"
        )

        XCTAssertEqual(url?.absoluteString, "https://push.example.internal/_matrix/push/v1/notify")
    }

    func testPushGatewayConfigurationFallsBackToBundleValue() {
        let url = AppEnvironment.configuredPushGatewayURL(
            environmentValue: nil,
            bundleValue: "https://push.example.com/_matrix/push/v1/notify"
        )

        XCTAssertEqual(url?.absoluteString, "https://push.example.com/_matrix/push/v1/notify")
    }

    func testPushGatewayConfigurationRejectsInvalidValues() {
        let url = AppEnvironment.configuredPushGatewayURL(
            environmentValue: "not a url",
            bundleValue: "http://push.example.com/_matrix/push/v1/notify"
        )

        XCTAssertNil(url)
    }

    @MainActor
    func testLiveEnvironmentIgnoresInvalidPushGatewayEnvironmentValue() {
        let variable = "SYNARA_PUSH_GATEWAY_URL"
        setenv(variable, "not a url", 1)

        let environment = AppEnvironment.live()

        unsetenv(variable)

        XCTAssertNil((environment.push as? SynaraPushService)?.pushGatewayURL)
    }
}
