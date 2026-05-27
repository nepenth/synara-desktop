import XCTest
@testable import Synara

final class SecureSessionStoreTests: XCTestCase {
    func testSaveAndLoadSession() throws {
        let store = InMemorySecureSessionStore()
        let session = try makeSession()

        try store.save(session)

        XCTAssertEqual(try store.load(), session)
        XCTAssertEqual(store.saveCallCount, 1)
        XCTAssertEqual(store.loadCallCount, 1)
    }

    func testDeleteClearsSession() throws {
        let store = InMemorySecureSessionStore(session: try makeSession())

        try store.delete()

        XCTAssertNil(try store.load())
        XCTAssertEqual(store.deleteCallCount, 1)
    }

    func testCorruptEntryThrows() throws {
        let store = InMemorySecureSessionStore()
        store.writeRawDataForTesting(Data("not-json".utf8))

        XCTAssertThrowsError(try store.load()) { error in
            XCTAssertEqual(error as? SecureSessionStoreError, .corruptEntry)
        }
    }

    func testSecureStoreErrorsHaveNonSensitiveLogDescriptions() {
        XCTAssertEqual(
            SecureSessionStoreError.keychainFailure(status: -34018).logDescription,
            "secure session keychain failure status -34018"
        )
        XCTAssertEqual(
            SecureSessionStoreError.corruptEntry.logDescription,
            "secure session entry is corrupt"
        )
    }

    func testLegacySessionMigratesToEnvelope() throws {
        let store = InMemorySecureSessionStore()
        let session = try makeSession()
        let legacyData = try JSONEncoder().encode(session)
        store.writeRawDataForTesting(legacyData)

        XCTAssertEqual(try store.migrateIfNeeded(), .migrated)
        XCTAssertEqual(try store.load(), session)
        XCTAssertEqual(store.migrationCallCount, 1)
    }

    func testAppSessionStoreRestoresPersistedSession() throws {
        let session = try makeSession()
        let secureStore = InMemorySecureSessionStore(session: session)
        let sessionStore = AppSessionStore(
            secureStore: secureStore,
            restorePersistedSession: true
        )

        XCTAssertEqual(sessionStore.currentState, .signedIn(session))
    }

    func testAppSessionStoreCapturesRestoreFailureDiagnostics() {
        let secureStore = InMemorySecureSessionStore()
        secureStore.writeRawDataForTesting(Data("not-json".utf8))
        let sessionStore = AppSessionStore(
            secureStore: secureStore,
            restorePersistedSession: true
        )

        XCTAssertEqual(sessionStore.currentState, .signedOut)
        XCTAssertEqual(sessionStore.restoreFailureLogDescription, "secure session entry is corrupt")
    }

    private func makeSession() throws -> AuthenticatedSession {
        AuthenticatedSession(
            userID: "@alice:matrix.org",
            deviceID: "DEVICE",
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.org")),
            accessToken: "secret-token"
        )
    }
}
