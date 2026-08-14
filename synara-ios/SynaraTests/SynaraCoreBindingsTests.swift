import XCTest
@testable import Synara
import SynaraCore

final class SynaraCoreBindingsTests: XCTestCase {
    func testBindingScaffoldVersionExecutesGeneratedRustFFI() {
        let version = bindingScaffoldVersion()

        XCTAssertFalse(version.isEmpty)
    }

    func testRegisterFlowsRejectsHostileURLWithStaticPrivacySafeError() async {
        let hostileURL = "https://user:secret@example.invalid"

        do {
            _ = try await SynaraCore.registerFlows(homeserverUrl: hostileURL)
            XCTFail("Hostile registration-flow URL must fail before a request")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p3.1-invalid-homeserver-url"))
            XCTAssertTrue(publicError.contains("The homeserver URL is invalid."))
            for forbidden in [hostileURL, "user:secret", "secret", "example.invalid"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
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
        let openedSnapshot = try await core.sessionSnapshot()
        XCTAssertEqual(openedSnapshot, Optional(expected))

        try await core.close()
        let closedSnapshot = try await core.sessionSnapshot()
        XCTAssertNil(closedSnapshot)
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

    func testProductionMirrorReadsReadyCoreIdentityThenClearsOnClose() async {
        let mirror = MatrixSessionProjectionMirror()
        let expected = CoreSessionIdentity(
            userID: "@alice:matrix.org",
            deviceID: "SYNARA-IOS-DEVICE",
            homeserverURL: "https://matrix.org"
        )

        let beforeOpen = await mirror.coreSessionIdentity()
        XCTAssertNil(beforeOpen)

        await mirror.openAfterInstalledClient(
            userID: expected.userID,
            deviceID: expected.deviceID,
            homeserverURL: expected.homeserverURL,
            cryptoReady: true
        )

        let afterOpen = await mirror.coreSessionIdentity()
        XCTAssertEqual(afterOpen, expected)

        await mirror.closeBeforeSDKWipe()

        let afterClose = await mirror.coreSessionIdentity()
        XCTAssertNil(afterClose)
    }

    func testMirrorFailsClosedForMismatchedNonReadyAndMissingCoreSnapshots() async throws {
        let core = SessionProjectionCore()
        let mirror = MatrixSessionProjectionMirror(core: core)

        await mirror.openAfterInstalledClient(
            userID: "@alice:matrix.org",
            deviceID: "SYNARA-IOS-DEVICE",
            homeserverURL: "https://matrix.org",
            cryptoReady: true
        )

        try await core.open(
            projection: SessionProjection(
                generation: 2,
                userId: "@mallory:matrix.org",
                deviceId: "OTHER-DEVICE",
                homeserverUrl: "https://matrix.org",
                lifecycle: .ready,
                cryptoReady: true
            )
        )
        let mismatchedIdentity = await mirror.coreSessionIdentity()
        XCTAssertNil(mismatchedIdentity)

        try await core.open(
            projection: SessionProjection(
                generation: 3,
                userId: "@alice:matrix.org",
                deviceId: "SYNARA-IOS-DEVICE",
                homeserverUrl: "https://matrix.org",
                lifecycle: .syncing,
                cryptoReady: true
            )
        )
        let nonReadyIdentity = await mirror.coreSessionIdentity()
        XCTAssertNil(nonReadyIdentity)

        try await core.close()
        let missingIdentity = await mirror.coreSessionIdentity()
        XCTAssertNil(missingIdentity)
    }

    func testMirrorDoesNotPublishAnIdentityWhenCoreOpenFails() async {
        let mirror = MatrixSessionProjectionMirror()

        await mirror.openAfterInstalledClient(
            userID: "@alice:matrix.org",
            deviceID: "SYNARA-IOS-DEVICE",
            homeserverURL: "https://user:access-token@private.example/?password=secret",
            cryptoReady: true
        )

        let identity = await mirror.coreSessionIdentity()
        XCTAssertNil(identity)
    }
}
