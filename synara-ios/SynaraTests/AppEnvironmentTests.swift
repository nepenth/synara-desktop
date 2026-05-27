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
        XCTAssertTrue(environment.later is MatrixAccountDataLaterService)
        XCTAssertTrue(environment.messageSender is MatrixRustSDKMessageSendService)
        XCTAssertTrue(environment.eventActions is MatrixEventActionService)
        XCTAssertTrue(environment.agentApprovals is MatrixAgentApprovalService)
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
