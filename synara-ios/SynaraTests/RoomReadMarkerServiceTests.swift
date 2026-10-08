@testable import Synara
import XCTest

final class RoomReadMarkerServiceTests: XCTestCase {
    func testServerEventPolicyAcceptsOnlyPersistedMatrixEventIDs() {
        XCTAssertTrue(MatrixServerEventIDPolicy.canAcknowledge("$event:matrix.example"))
        XCTAssertFalse(MatrixServerEventIDPolicy.canAcknowledge("$pending-local"))
        XCTAssertFalse(MatrixServerEventIDPolicy.canAcknowledge("$local-generated"))
        XCTAssertFalse(MatrixServerEventIDPolicy.canAcknowledge("transaction-123"))
        XCTAssertFalse(MatrixServerEventIDPolicy.canAcknowledge(""))
    }

    func testMockMarkRoomAsReadUsesLatestEventMarker() async {
        let service = MockRoomReadMarkerService()

        let acknowledgedEventID = await service.markRoomAsRead(roomID: "!room:matrix.example")

        XCTAssertEqual(acknowledgedEventID, "$latest:!room:matrix.example")
        XCTAssertEqual(service.eventID, "$latest:!room:matrix.example")
    }

    func testExplicitMarkReadIsNilWithoutAFullyReadReceipt() {
        XCTAssertNil(
            ExplicitRoomReadReceipt.acknowledgedEventID(receiptSent: false, acknowledgedEventID: "$event:example.org")
        )
        XCTAssertNil(
            ExplicitRoomReadReceipt.acknowledgedEventID(receiptSent: nil, acknowledgedEventID: "$event:example.org")
        )
        XCTAssertEqual(
            ExplicitRoomReadReceipt.acknowledgedEventID(receiptSent: true, acknowledgedEventID: "$event:example.org"),
            "$event:example.org"
        )
    }

    func testExplicitMarkReadFailureCopyIsFixedAndOnlyForMissingReadback() {
        XCTAssertEqual(
            ExplicitRoomReadReceipt.failureMessage(acknowledgedEventID: nil),
            "Couldn't mark this channel as read."
        )
        XCTAssertNil(ExplicitRoomReadReceipt.failureMessage(acknowledgedEventID: "$event:example.org"))
    }

    func testRejectedAuthenticationDiagnosticIsTheOnlyRetirementTrigger() {
        XCTAssertTrue(
            RejectedAuthenticationRetirement.shouldRetire(
                failureDiagnosticID: "p4.1-session-authentication-rejected"
            )
        )
        XCTAssertFalse(RejectedAuthenticationRetirement.shouldRetire(failureDiagnosticID: "p4.1-sync-service-error"))
        XCTAssertFalse(RejectedAuthenticationRetirement.shouldRetire(failureDiagnosticID: nil))
    }

    /// Acceptance 13: a rejected refresh ends signed out with the expiry notice.
    /// The retirer's only effects are the injected ones, so there is no store
    /// wipe and no remote logout to call.
    @MainActor
    func testRejectedAuthenticationRetirementSignsOutAndForgetsCredentials() async throws {
        let store = AppSessionStore(
            secureStore: InMemorySecureSessionStore(session: try Self.makeSession()),
            restorePersistedSession: true
        )
        XCTAssertNotEqual(store.currentState, .signedOut)
        let forgets = RetirementCallCounter()

        let outcome = await RejectedAuthenticationRetirer(
            forgetCredentials: { forgets.increment() },
            noteSessionExpired: { store.noteSessionExpired() },
            signOut: { try store.signOut() },
            retryDelayNanoseconds: 0
        ).run()

        XCTAssertEqual(outcome, .retired)
        XCTAssertEqual(forgets.count, 1)
        XCTAssertEqual(store.currentState, .signedOut)
        XCTAssertTrue(store.sessionExpiredNotice)
        // The app session record that drives launch restore is gone.
        XCTAssertNil(try store.secureStore.load())
    }

    @MainActor
    func testRejectedAuthenticationRetirementRetriesForgetAndStillSignsOut() async throws {
        let store = AppSessionStore(
            secureStore: InMemorySecureSessionStore(session: try Self.makeSession()),
            restorePersistedSession: true
        )
        let forgets = RetirementCallCounter()

        let outcome = await RejectedAuthenticationRetirer(
            forgetCredentials: {
                forgets.increment()
                throw RetirementTestFailure()
            },
            noteSessionExpired: { store.noteSessionExpired() },
            signOut: { try store.signOut() },
            maxAttempts: 3,
            retryDelayNanoseconds: 0
        ).run()

        // A failed forget is reported, not treated as success, and the shell
        // still signs out so the retired generation does not restore.
        XCTAssertEqual(outcome, .signedOutCredentialsKept)
        XCTAssertEqual(forgets.count, 3)
        XCTAssertEqual(store.currentState, .signedOut)
        XCTAssertNil(try store.secureStore.load())
    }

    @MainActor
    func testRejectedAuthenticationRetirementReportsSignOutFailure() async throws {
        let session = try Self.makeSession()
        let store = AppSessionStore(
            currentState: .signedIn(session),
            secureStore: RetirementDeleteFailingSecureSessionStore(session: session)
        )

        let outcome = await RejectedAuthenticationRetirer(
            forgetCredentials: {},
            noteSessionExpired: { store.noteSessionExpired() },
            signOut: { try store.signOut() },
            retryDelayNanoseconds: 0
        ).run()

        XCTAssertEqual(outcome, .signOutFailed)
    }

    private static func makeSession() throws -> AuthenticatedSession {
        AuthenticatedSession(
            userID: "@alice:example.org",
            deviceID: "DEVICE",
            homeserverURL: try XCTUnwrap(URL(string: "https://example.org")),
            accessToken: ""
        )
    }
}

private final class RetirementCallCounter: @unchecked Sendable {
    private let lock = NSLock()
    private var value = 0

    var count: Int {
        lock.withLock { value }
    }

    func increment() {
        lock.withLock { value += 1 }
    }
}

private struct RetirementTestFailure: Error {}

private final class RetirementDeleteFailingSecureSessionStore: SecureSessionStoring {
    private let session: AuthenticatedSession

    init(session: AuthenticatedSession) {
        self.session = session
    }

    func save(_: AuthenticatedSession) throws {}
    func load() throws -> AuthenticatedSession? { session }
    func delete() throws { throw SecureSessionStoreError.keychainFailure(status: -1) }
    func migrateIfNeeded() throws -> SessionMigrationResult { .notNeeded }
}
