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

    func testReadinessAllowsOnlyOneStartupOwnerAndReleasesWaiters() async throws {
        let readiness = SignedInSessionReadiness()
        let session = try makeSession()

        let firstClaim = await readiness.claimPreparation(for: session)
        let duplicateClaim = await readiness.claimPreparation(for: session)
        XCTAssertTrue(firstClaim)
        XCTAssertFalse(duplicateClaim)

        let waiter = Task {
            await readiness.waitUntilPrepared(for: session)
        }
        await Task.yield()
        let markedPrepared = await readiness.markPrepared(for: session)

        let waiterCompleted = await waiter.value
        let preparedClaim = await readiness.claimPreparation(for: session)
        XCTAssertTrue(waiterCompleted)
        XCTAssertTrue(markedPrepared)
        XCTAssertFalse(preparedClaim)
    }

    func testSupersededReadinessWaiterFailsClosed() async throws {
        let readiness = SignedInSessionReadiness()
        let first = try makeSession()
        let second = AuthenticatedSession(
            userID: "@bob:matrix.org",
            deviceID: "OTHER",
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.org")),
            accessToken: "other-token"
        )

        let claimedFirst = await readiness.claimPreparation(for: first)
        XCTAssertTrue(claimedFirst)
        let waiter = Task { await readiness.waitUntilPrepared(for: first) }
        await Task.yield()
        let claimedSecond = await readiness.claimPreparation(for: second)
        XCTAssertTrue(claimedSecond)

        let firstWaitResult = await waiter.value
        let markedFirst = await readiness.markPrepared(for: first)
        let markedSecond = await readiness.markPrepared(for: second)
        XCTAssertFalse(firstWaitResult)
        XCTAssertFalse(markedFirst)
        XCTAssertTrue(markedSecond)
    }

    func testReadinessWaiterMayArriveBeforeStartupOwnerClaims() async throws {
        let readiness = SignedInSessionReadiness()
        let session = try makeSession()
        let waiter = Task { await readiness.waitUntilPrepared(for: session) }
        await Task.yield()

        let claimed = await readiness.claimPreparation(for: session)
        let marked = await readiness.markPrepared(for: session)
        let waitResult = await waiter.value
        XCTAssertTrue(claimed)
        XCTAssertTrue(marked)
        XCTAssertTrue(waitResult)
    }

    func testCancelledPreparationReleasesWaitersAsFailure() async throws {
        let readiness = SignedInSessionReadiness()
        let session = try makeSession()
        let claimed = await readiness.claimPreparation(for: session)
        let waiter = Task { await readiness.waitUntilPrepared(for: session) }
        await Task.yield()

        await readiness.cancelPreparation(for: session)

        let waitResult = await waiter.value
        let reclaimed = await readiness.claimPreparation(for: session)
        XCTAssertTrue(claimed)
        XCTAssertFalse(waitResult)
        XCTAssertTrue(reclaimed)
    }

    func testPreparingMatrixOwnerDoesNotConfigurePushOrPromptForPermission() async throws {
        let matrix = MockMatrixClientService()
        let push = MockPushService(isRegistrationAvailable: true)
        let permission = MockNotificationPermissionService(status: .notDetermined)
        let environment = AppEnvironment.mock(
            matrix: matrix,
            push: push,
            notificationPermission: permission
        )
        let session = try makeSession()

        let prepared = await SessionCoordinator.prepareMatrixOwner(environment: environment, session: session)
        XCTAssertTrue(prepared)

        XCTAssertEqual(matrix.startedSessions, [session])
        XCTAssertEqual(push.configureCallCount, 0)
        XCTAssertEqual(push.beginRegistrationCallCount, 0)
        XCTAssertEqual(permission.requestCallCount, 0)
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
