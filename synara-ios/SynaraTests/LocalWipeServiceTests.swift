import XCTest
@testable import Synara

final class LocalWipeServiceTests: XCTestCase {
    func testLogoutWipeCallsAllRegisteredStores() async throws {
        let secureStore = InMemorySecureSessionStore(session: try makeSession())
        let session = AppSessionStore(secureStore: secureStore, restorePersistedSession: true)
        let matrix = MockMatrixClientService(syncStatus: .syncing)
        let roomList = MockRoomListService()
        let timeline = MockTimelineService()
        let drafts = DraftStore()
        drafts.setDraft("draft text", roomID: "!room:matrix.org")
        let push = MockPushService()
        let router = AppRouter()
        router.route(to: .settings)
        router.present(.accountSwitcher)
        let wipe = AppLocalWipeService(
            session: session,
            matrix: matrix,
            roomList: roomList,
            timeline: timeline,
            drafts: drafts,
            push: push,
            router: router
        )

        try await wipe.logoutAndWipe()

        XCTAssertEqual(session.currentState, .signedOut)
        XCTAssertEqual(matrix.stopCallCount, 1)
        XCTAssertEqual(matrix.resetCallCount, 1)
        XCTAssertEqual(matrix.resetSessions.count, 1)
        XCTAssertEqual(matrix.resetSessions.first??.userID, "@alice:matrix.org")
        XCTAssertEqual(roomList.clearCallCount, 1)
        XCTAssertEqual(timeline.clearSessionCachesCallCount, 1)
        XCTAssertEqual(push.clearCallCount, 1)
        XCTAssertEqual(secureStore.deleteCallCount, 1)
        XCTAssertEqual(drafts.draft(roomID: "!room:matrix.org"), "")
        XCTAssertEqual(router.selectedTab, .rooms)
        XCTAssertTrue(router.settingsPath.isEmpty)
        XCTAssertNil(router.sheetDestination)
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