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
        let outgoingSends = OutgoingSendCoordinator(
            messageSender: MockMessageSendService(),
            connectionStatus: ConnectionStatusStore(reconnectingHold: 0)
        )
        outgoingSends.enqueue(
            localID: "$pending-wipe",
            roomID: "!room:matrix.org",
            body: "queued",
            formattedBody: nil,
            replyToEventID: nil,
            senderID: "@alice:matrix.org",
            timestamp: Date()
        )
        let wipe = AppLocalWipeService(
            session: session,
            matrix: matrix,
            roomList: roomList,
            timeline: timeline,
            drafts: drafts,
            push: push,
            router: router,
            outgoingSends: outgoingSends
        )

        try await wipe.logoutAndWipe()

        XCTAssertEqual(session.currentState, .signedOut)
        XCTAssertEqual(matrix.stopCallCount, 1)
        XCTAssertEqual(matrix.revokedSessions.map(\.userID), ["@alice:matrix.org"])
        XCTAssertEqual(matrix.resetCallCount, 1)
        XCTAssertEqual(matrix.resetSessions.count, 1)
        XCTAssertEqual(matrix.resetSessions.first??.userID, "@alice:matrix.org")
        XCTAssertEqual(roomList.clearCallCount, 1)
        XCTAssertEqual(timeline.clearSessionCachesCallCount, 1)
        XCTAssertEqual(push.clearCallCount, 1)
        XCTAssertEqual(secureStore.deleteCallCount, 1)
        XCTAssertEqual(drafts.draft(roomID: "!room:matrix.org"), "")
        XCTAssertTrue(outgoingSends.queue.items.isEmpty)
        XCTAssertEqual(router.selectedTab, .rooms)
        XCTAssertTrue(router.settingsPath.isEmpty)
        XCTAssertNil(router.sheetDestination)
    }

    func testLogoutPreservesSDKStoreWhenSecureSessionDeleteFails() async throws {
        let persistedSession = try makeSession()
        let secureStore = DeleteFailingSecureSessionStore(session: persistedSession)
        let session = AppSessionStore(secureStore: secureStore, restorePersistedSession: true)
        let matrix = MockMatrixClientService(syncStatus: .syncing)
        let roomList = MockRoomListService()
        let timeline = MockTimelineService()
        let drafts = DraftStore()
        drafts.setDraft("preserve", roomID: "!room:matrix.org")
        let push = MockPushService()
        let router = AppRouter()
        router.route(to: .settings)
        let wipe = AppLocalWipeService(
            session: session,
            matrix: matrix,
            roomList: roomList,
            timeline: timeline,
            drafts: drafts,
            push: push,
            router: router
        )

        do {
            try await wipe.logoutAndWipe()
            XCTFail("Expected secure session deletion to fail")
        } catch {
            XCTAssertEqual(error as? LocalWipeError, .sessionDeleteFailed)
        }

        XCTAssertEqual(session.currentState, .signedIn(persistedSession))
        XCTAssertEqual(matrix.stopCallCount, 0)
        XCTAssertTrue(matrix.revokedSessions.isEmpty)
        XCTAssertEqual(matrix.resetCallCount, 0)
        XCTAssertEqual(push.clearCallCount, 0)
        XCTAssertEqual(roomList.clearCallCount, 0)
        XCTAssertEqual(timeline.clearSessionCachesCallCount, 0)
        XCTAssertEqual(drafts.draft(roomID: "!room:matrix.org"), "preserve")
        XCTAssertEqual(router.selectedTab, .settings)
    }

    func testDurableSessionDeletionPrecedesRemoteAndLocalTeardown() async throws {
        let persistedSession = try makeSession()
        let recorder = WipeOperationRecorder()
        let secureStore = RecordingSecureSessionStore(session: persistedSession) {
            recorder.record("session-delete")
        }
        let session = AppSessionStore(secureStore: secureStore, restorePersistedSession: true)
        let matrix = MockMatrixClientService(syncStatus: .syncing)
        matrix.onOperation = recorder.record
        let push = MockPushService()
        push.onOperation = recorder.record
        let wipe = AppLocalWipeService(
            session: session,
            matrix: matrix,
            roomList: MockRoomListService(),
            timeline: MockTimelineService(),
            drafts: DraftStore(),
            push: push,
            router: AppRouter()
        )

        try await wipe.logoutAndWipe()

        XCTAssertEqual(
            recorder.events,
            ["session-delete", "push-clear", "server-revoke", "matrix-stop", "matrix-reset"]
        )
    }

    func testServerRevocationFailureStillCompletesLocalWipe() async throws {
        let secureStore = InMemorySecureSessionStore(session: try makeSession())
        let session = AppSessionStore(secureStore: secureStore, restorePersistedSession: true)
        let matrix = MockMatrixClientService(syncStatus: .syncing)
        matrix.serverRevocationResult = false
        let roomList = MockRoomListService()
        let timeline = MockTimelineService()
        let drafts = DraftStore()
        drafts.setDraft("remove", roomID: "!room:matrix.org")
        let wipe = AppLocalWipeService(
            session: session,
            matrix: matrix,
            roomList: roomList,
            timeline: timeline,
            drafts: drafts,
            push: MockPushService(),
            router: AppRouter()
        )

        try await wipe.logoutAndWipe()

        XCTAssertEqual(session.currentState, .signedOut)
        XCTAssertEqual(matrix.revokedSessions.count, 1)
        XCTAssertEqual(matrix.stopCallCount, 1)
        XCTAssertEqual(matrix.resetCallCount, 1)
        XCTAssertEqual(roomList.clearCallCount, 1)
        XCTAssertEqual(timeline.clearSessionCachesCallCount, 1)
        XCTAssertEqual(drafts.draft(roomID: "!room:matrix.org"), "")
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

private final class WipeOperationRecorder: @unchecked Sendable {
    private let lock = NSLock()
    private var recordedEvents: [String] = []

    var events: [String] {
        lock.lock()
        defer { lock.unlock() }
        return recordedEvents
    }

    func record(_ event: String) {
        lock.lock()
        recordedEvents.append(event)
        lock.unlock()
    }
}

private final class RecordingSecureSessionStore: SecureSessionStoring {
    private var session: AuthenticatedSession?
    private let onDelete: () -> Void

    init(session: AuthenticatedSession, onDelete: @escaping () -> Void) {
        self.session = session
        self.onDelete = onDelete
    }

    func save(_ session: AuthenticatedSession) throws {
        self.session = session
    }

    func load() throws -> AuthenticatedSession? { session }

    func delete() throws {
        onDelete()
        session = nil
    }

    func migrateIfNeeded() throws -> SessionMigrationResult { .notNeeded }
}

private final class DeleteFailingSecureSessionStore: SecureSessionStoring {
    private let session: AuthenticatedSession

    init(session: AuthenticatedSession) {
        self.session = session
    }

    func save(_: AuthenticatedSession) throws {}
    func load() throws -> AuthenticatedSession? { session }
    func delete() throws { throw SecureSessionStoreError.keychainFailure(status: -1) }
    func migrateIfNeeded() throws -> SessionMigrationResult { .notNeeded }
}
