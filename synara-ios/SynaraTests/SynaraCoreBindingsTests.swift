import XCTest
import SynaraCore

final class SynaraCoreBindingsTests: XCTestCase {
    func testBindingScaffoldVersionExecutesGeneratedRustFFI() {
        let version = bindingScaffoldVersion()

        XCTAssertFalse(version.isEmpty)
    }

    func testSessionProjectionFacadeExecutesOpenSnapshotAndCloseOverGeneratedRustFFI() async throws {
        let core = SessionProjectionCore()
        let expected = SessionProjection(
            generation: 7,
            userId: "@alice:matrix.org",
            deviceId: "SYNARA-IOS-DEVICE",
            homeserverUrl: "https://matrix.org",
            lifecycle: .ready,
            cryptoReady: true
        )

        try await core.open(projection: expected)
        XCTAssertEqual(try await core.sessionSnapshot(), Optional(expected))

        try await core.close()
        XCTAssertNil(try await core.sessionSnapshot())
    }

    func testSessionProjectionFacadeRejectsHostileValuesWithStaticError() async {
        let core = SessionProjectionCore()
        let hostileURL = "https://user:access-token@private.example/?password=secret"
        let invalid = SessionProjection(
            generation: 1,
            userId: "@alice:matrix.org",
            deviceId: "SYNARA-IOS-DEVICE",
            homeserverUrl: hostileURL,
            lifecycle: .ready,
            cryptoReady: true
        )

        do {
            try await core.open(projection: invalid)
            XCTFail("Hostile projection must fail before Core is opened")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4.3-session-projection-rejected"))
            XCTAssertTrue(publicError.contains("The session projection is invalid."))
            for forbidden in [hostileURL, "access-token", "password", "secret", "private.example"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }
}
