import XCTest
@testable import Synara

final class NotificationPermissionCoordinatorTests: XCTestCase {
    func testPromptOnFirstSignInRequestsAuthorizationAndRegistersPush() async throws {
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

        await NotificationPermissionCoordinator.promptOnFirstSignInIfNeeded(environment: environment)

        XCTAssertEqual(permission.requestCallCount, 1)
        XCTAssertTrue(settings.bool(for: NotificationPermissionSettingsKey.hasPromptedOnSignIn))
        XCTAssertEqual(push.beginRegistrationCallCount, 1)
    }

    func testPromptOnFirstSignInSkipsRequestWhenAlreadyAuthorized() async throws {
        let settings = InMemorySettingsStore()
        let permission = MockNotificationPermissionService(status: .authorized)
        let push = MockPushService(isRegistrationAvailable: true)
        let environment = AppEnvironment.mock(
            push: push,
            notificationPermission: permission,
            settings: settings
        )

        await NotificationPermissionCoordinator.promptOnFirstSignInIfNeeded(environment: environment)

        XCTAssertEqual(permission.requestCallCount, 0)
        XCTAssertTrue(settings.bool(for: NotificationPermissionSettingsKey.hasPromptedOnSignIn))
        XCTAssertEqual(push.beginRegistrationCallCount, 1)
    }

    func testPromptOnFirstSignInDoesNotRegisterWhenDenied() async throws {
        let settings = InMemorySettingsStore()
        let permission = MockNotificationPermissionService(status: .denied)
        let push = MockPushService(isRegistrationAvailable: true)
        let environment = AppEnvironment.mock(
            push: push,
            notificationPermission: permission,
            settings: settings
        )

        await NotificationPermissionCoordinator.promptOnFirstSignInIfNeeded(environment: environment)

        XCTAssertEqual(permission.requestCallCount, 0)
        XCTAssertTrue(settings.bool(for: NotificationPermissionSettingsKey.hasPromptedOnSignIn))
        XCTAssertEqual(push.beginRegistrationCallCount, 0)
    }

    func testPromptOnFirstSignInOnlyRunsOnce() async throws {
        let settings = InMemorySettingsStore()
        settings.set(true, for: NotificationPermissionSettingsKey.hasPromptedOnSignIn)
        let permission = MockNotificationPermissionService(status: .authorized)
        let push = MockPushService(isRegistrationAvailable: true)
        let environment = AppEnvironment.mock(
            push: push,
            notificationPermission: permission,
            settings: settings
        )

        await NotificationPermissionCoordinator.promptOnFirstSignInIfNeeded(environment: environment)

        XCTAssertEqual(permission.requestCallCount, 0)
        XCTAssertEqual(push.beginRegistrationCallCount, 1)
    }
}