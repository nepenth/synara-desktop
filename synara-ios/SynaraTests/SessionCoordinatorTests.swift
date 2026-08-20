import XCTest
@testable import Synara

final class SessionCoordinatorTests: XCTestCase {
    func testStartSignedInSessionStartsMatrixAndConfiguresPush() async throws {
        let matrix = MockMatrixClientService()
        let push = MockPushService()
        let environment = AppEnvironment.mock(matrix: matrix, push: push)
        let session = try makeSession()

        await SessionCoordinator.startSignedInSession(environment: environment, session: session)

        XCTAssertEqual(matrix.startedSessions, [session])
        XCTAssertEqual(matrix.syncStatus, .syncing)
        XCTAssertEqual(environment.connectionStatus.status, .syncing)
        XCTAssertEqual(push.configureCallCount, 1)
    }

    func testStartSignedInSessionPromptsForNotificationsOnFirstSignIn() async throws {
        let settings = InMemorySettingsStore()
        let permission = MockNotificationPermissionService(
            status: .notDetermined,
            statusAfterRequest: .authorized
        )
        let push = MockPushService(isRegistrationAvailable: true)
        let environment = AppEnvironment.mock(
            push: push,
            notificationPermission: permission,
            settings: settings
        )
        let session = try makeSession()

        await SessionCoordinator.startSignedInSession(environment: environment, session: session)

        XCTAssertEqual(permission.requestCallCount, 1)
        XCTAssertTrue(settings.bool(for: NotificationPermissionSettingsKey.hasPromptedOnSignIn))
        XCTAssertEqual(push.beginRegistrationCallCount, 1)
    }

    private func makeSession() throws -> AuthenticatedSession {
        AuthenticatedSession(
            userID: "@alice:matrix.org",
            deviceID: "DEVICE",
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.org")),
            accessToken: "token"
        )
    }
}