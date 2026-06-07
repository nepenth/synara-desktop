import XCTest
import UserNotifications
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

    func testLiveEnvironmentUsesMatrixRustSDKServices() {
        let environment = AppEnvironment.live()

        XCTAssertTrue(environment.auth is MatrixRustSDKAuthService)
        XCTAssertTrue(environment.roomList is MatrixRustSDKRoomListService)
        XCTAssertTrue(environment.roomMembership is MatrixRustSDKRoomMembershipService)
        XCTAssertTrue(environment.timeline is MatrixRustSDKTimelineService)
        XCTAssertTrue(environment.later is MatrixRustSDKLaterService)
        XCTAssertTrue(environment.messageSender is MatrixRustSDKMessageSendService)
        XCTAssertTrue(environment.eventActions is MatrixRustSDKEventActionService)
        XCTAssertTrue(environment.agentApprovals is MatrixRustSDKAgentApprovalService)
        XCTAssertTrue(environment.crypto is MatrixRustSDKCryptoStatusService)
        XCTAssertTrue(environment.roomManagement is MatrixRustSDKRoomManagementService)
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
    }

    func testMockCryptoRecoverRejectsEmptyRecoveryKey() async {
        let result = await MockCryptoStatusService().recover(recoveryKey: "   ")

        XCTAssertEqual(result, .failed("Enter a recovery key before recovering keys."))
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

    func testLiveEnvironmentReadsPushGatewayFromEnvironment() {
        let variable = "SYNARA_PUSH_GATEWAY_URL"
        setenv(variable, "https://push.example.internal", 1)

        let environment = AppEnvironment.live()

        unsetenv(variable)

        XCTAssertEqual((environment.push as? SynaraPushService)?.pushGatewayURL, "https://push.example.internal")
    }

    func testLiveEnvironmentIgnoresInvalidPushGatewayEnvironmentValue() {
        let variable = "SYNARA_PUSH_GATEWAY_URL"
        setenv(variable, "not a url", 1)

        let environment = AppEnvironment.live()

        unsetenv(variable)

        XCTAssertNil((environment.push as? SynaraPushService)?.pushGatewayURL)
    }
}
