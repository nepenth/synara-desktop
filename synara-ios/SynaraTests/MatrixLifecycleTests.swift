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
        await matrix.resetLocalState()

        XCTAssertEqual(matrix.syncStatus, .stopped)
        XCTAssertEqual(matrix.stopCallCount, 1)
        XCTAssertEqual(matrix.resetCallCount, 1)
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
