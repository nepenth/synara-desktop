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
        XCTAssertEqual(push.configureCallCount, 1)
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