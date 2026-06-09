import XCTest
@testable import Synara

final class MatrixLifecycleTests: XCTestCase {
    func testStartSyncAfterLogin() async throws {
        let matrix = MockMatrixClientService()
        let session = try makeSession()

        await matrix.start(session: session)

        XCTAssertEqual(matrix.syncStatus, .syncing)
        XCTAssertEqual(matrix.syncStatusDescription, "Syncing")
        XCTAssertEqual(matrix.startedSessions, [session])
    }

    func testStopAndResetSync() async {
        let matrix = MockMatrixClientService(syncStatus: .syncing)

        await matrix.stop()
        await matrix.resetLocalState(for: nil)

        XCTAssertEqual(matrix.syncStatus, .stopped)
        XCTAssertEqual(matrix.stopCallCount, 1)
        XCTAssertEqual(matrix.resetCallCount, 1)
    }

    func testPauseAndResumeBackgroundLifecycle() async throws {
        let matrix = MockMatrixClientService(syncStatus: .syncing)
        let session = try makeSession()

        await matrix.pauseForBackground()
        XCTAssertEqual(matrix.pauseCallCount, 1)
        XCTAssertEqual(matrix.syncStatus, .stopped)

        await matrix.resumeFromForeground(session: session)
        XCTAssertEqual(matrix.resumeCallCount, 1)
        XCTAssertEqual(matrix.resumedSessions, [session])
        XCTAssertEqual(matrix.syncStatus, .syncing)
    }

    func testBackgroundNotificationSyncReportsResult() async throws {
        let matrix = MockMatrixClientService()
        matrix.backgroundSyncResult = true
        let session = try makeSession()

        let synced = await matrix.syncForBackgroundNotification(session: session)

        XCTAssertTrue(synced)
        XCTAssertEqual(matrix.backgroundSyncCallCount, 1)
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
