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

    func testSignOutResetReleasesWaitersAndLetsTheSameIdentityClaimStartupAgain() async throws {
        let readiness = SignedInSessionReadiness()
        let session = try makeSession()

        let firstClaim = await readiness.claimPreparation(for: session)
        let firstMark = await readiness.markPrepared(for: session)
        let claimWhilePrepared = await readiness.claimPreparation(for: session)
        XCTAssertTrue(firstClaim)
        XCTAssertTrue(firstMark)
        XCTAssertFalse(claimWhilePrepared)

        await readiness.resetForSignOut()

        // Re-login reuses the crypto device, so the token is identical. It must
        // be claimable again, and content that starts waiting before the shell
        // claims must be allowed to wait rather than fail closed.
        let earlyWaiter = Task { await readiness.waitUntilPrepared(for: session) }
        await Task.yield()
        let reclaim = await readiness.claimPreparation(for: session)
        let remark = await readiness.markPrepared(for: session)
        let earlyWaiterResult = await earlyWaiter.value
        XCTAssertTrue(reclaim)
        XCTAssertTrue(remark)
        XCTAssertTrue(earlyWaiterResult)
    }

    func testSignOutResetFailsInFlightWaitersClosed() async throws {
        let readiness = SignedInSessionReadiness()
        let session = try makeSession()
        let claimed = await readiness.claimPreparation(for: session)
        let waiter = Task { await readiness.waitUntilPrepared(for: session) }
        // A waiter that only registers after the reset is, by design, allowed
        // to wait for the next login of the same identity, so make sure this
        // one is parked before signing out.
        var parked = await readiness.pendingWaiterCount()
        var attempts = 0
        while parked == 0, attempts < 200 {
            try await Task.sleep(nanoseconds: 5_000_000)
            parked = await readiness.pendingWaiterCount()
            attempts += 1
        }
        XCTAssertEqual(parked, 1)

        await readiness.resetForSignOut()

        let waitResult = await waiter.value
        let reclaimed = await readiness.claimPreparation(for: session)
        XCTAssertTrue(claimed)
        XCTAssertFalse(waitResult)
        XCTAssertTrue(reclaimed)
    }

    func testReLoginAfterLogoutStartsTheMatrixClientAgain() async throws {
        // Regression: after Log Out -> Log In on the same device the room list
        // stayed empty until a force-quit, because the readiness gate still
        // remembered the identity as prepared and `matrix.start` was skipped.
        let matrix = MockMatrixClientService()
        let readiness = SignedInSessionReadiness()
        let sessionStore = AppSessionStore()
        let environment = AppEnvironment.mock(
            session: sessionStore,
            matrix: matrix,
            sessionReadiness: readiness
        )
        let session = try makeSession()

        try await MainActor.run { try sessionStore.completeLogin(session) }
        await SessionCoordinator.startSignedInSession(environment: environment, session: session)
        XCTAssertEqual(matrix.startedSessions, [session])

        try await environment.wipe.logoutAndWipe()
        let stateAfterLogout = await MainActor.run { sessionStore.currentState }
        XCTAssertEqual(stateAfterLogout, .signedOut)

        try await MainActor.run { try sessionStore.completeLogin(session) }
        await SessionCoordinator.startSignedInSession(environment: environment, session: session)
        let preparedAfterReLogin = await readiness.waitUntilPrepared(for: session)
        XCTAssertEqual(matrix.startedSessions, [session, session])
        XCTAssertTrue(preparedAfterReLogin)
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
