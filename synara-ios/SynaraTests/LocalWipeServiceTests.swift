import XCTest
@testable import Synara

final class LocalWipeServiceTests: XCTestCase {
    func testLogoutWipeCallsAllRegisteredStores() async throws {
        let secureStore = InMemorySecureSessionStore(session: try makeSession())
        let session = AppSessionStore(secureStore: secureStore, restorePersistedSession: true)
        let matrix = MockMatrixClientService(syncStatus: .syncing)
        let roomList = MockRoomListService()
        let push = MockPushService()
        let wipe = AppLocalWipeService(
            session: session,
            matrix: matrix,
            roomList: roomList,
            push: push
        )

        try await wipe.logoutAndWipe()

        XCTAssertEqual(session.currentState, .signedOut)
        XCTAssertEqual(matrix.stopCallCount, 1)
        XCTAssertEqual(matrix.resetCallCount, 1)
        XCTAssertEqual(roomList.clearCallCount, 1)
        XCTAssertEqual(push.clearCallCount, 1)
        XCTAssertEqual(secureStore.deleteCallCount, 1)
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
