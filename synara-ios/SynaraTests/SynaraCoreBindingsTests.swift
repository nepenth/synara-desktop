import XCTest
@testable import Synara
import SynaraCore

final class SynaraCoreBindingsTests: XCTestCase {
    func testBindingScaffoldVersionExecutesGeneratedRustFFI() {
        let version = bindingScaffoldVersion()

        XCTAssertFalse(version.isEmpty)
    }

    func testSharedCoreConstructsOverGeneratedRustFFI() {
        let core = SharedCore()

        XCTAssertNotNil(core)
    }

    func testSharedCoreAcceptsInMemorySecretStore() {
        // UniFFI 0.28 Swift emits the named UDL constructor as a static
        // factory, not a second init(store:).
        let core = SharedCore.newWithSecretStore(store: InMemoryIosSecretVault())

        XCTAssertNotNil(core)
    }

    func testSharedCoreRestoreWithoutVaultFailsClosed() async {
        let core = SharedCore()
        let storeRoot = FileManager.default.temporaryDirectory
            .appendingPathComponent("synara-s3b-no-vault", isDirectory: true)

        do {
            _ = try await core.restorePersistedSession(
                userId: "@alice:example.org",
                homeserverUrl: "https://matrix.example.org",
                storeRoot: storeRoot.path
            )
            XCTFail("Fail-closed SharedCore must not restore")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s3b-secret-vault-unavailable"))
            XCTAssertFalse(publicError.contains("p4-s3b-session-material-missing"))
            for forbidden in ["@alice:example.org", "matrix.example.org", "password", storeRoot.path] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreRestoreRejectsHostileIdentityWithoutEcho() async {
        let core = SharedCore.newWithSecretStore(store: InMemoryIosSecretVault())
        let storeRoot = FileManager.default.temporaryDirectory
            .appendingPathComponent("synara-s3b-hostile", isDirectory: true)
        let hostileURL = "https://user:secret@evil.example/?password=hunter2"

        do {
            _ = try await SharedCoreSessionRestore.restorePersistedSession(
                userID: "not-a-user",
                homeserverURL: hostileURL,
                storeRoot: storeRoot,
                core: core
            )
            XCTFail("Hostile identity must fail closed")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s3b-identity-invalid"))
            for forbidden in [hostileURL, "secret", "hunter2", "evil.example", "password"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreRestoreHoldsInstanceAcrossCalls() async {
        let vault = InMemoryIosSecretVault()
        let core = SharedCore.newWithSecretStore(store: vault)
        let storeRoot = FileManager.default.temporaryDirectory
            .appendingPathComponent("synara-s3b-hold-core", isDirectory: true)

        do {
            _ = try await SharedCoreSessionRestore.restorePersistedSession(
                userID: "@alice:example.org",
                homeserverURL: "https://matrix.example.org",
                storeRoot: storeRoot,
                core: core
            )
            XCTFail("Empty vault must not restore")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s3b-session-material-missing"))
        }

        do {
            _ = try await SharedCoreSessionRestore.restorePersistedSession(
                userID: "@alice:example.org",
                homeserverURL: "https://matrix.example.org",
                storeRoot: storeRoot,
                core: core
            )
            XCTFail("Second call on the same instance must still run")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s3b-session-material-missing"))
            XCTAssertFalse(publicError.contains("p4-s3b-secret-vault-unavailable"))
        }
    }

    func testSharedCoreLoginWithoutVaultFailsClosed() async {
        let core = SharedCore()
        let storeRoot = FileManager.default.temporaryDirectory
            .appendingPathComponent("synara-s3c-no-vault", isDirectory: true)
        let password = "hunter2-s3c-secret"

        do {
            _ = try await SharedCoreSessionLogin.loginWithPassword(
                userID: "@alice:example.org",
                homeserverURL: "https://matrix.example.org",
                storeRoot: storeRoot,
                password: password,
                core: core
            )
            XCTFail("Fail-closed SharedCore must not login")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s3c-secret-vault-unavailable"))
            for forbidden in [password, "hunter2", "@alice:example.org", "password"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreLoginRejectsHostileIdentityWithoutEchoingPassword() async {
        let core = SharedCore.newWithSecretStore(store: InMemoryIosSecretVault())
        let storeRoot = FileManager.default.temporaryDirectory
            .appendingPathComponent("synara-s3c-hostile", isDirectory: true)
        let hostileURL = "https://user:secret@evil.example/?password=hunter2"
        let password = "s3c-password-must-not-leak"

        do {
            _ = try await SharedCoreSessionLogin.loginWithPassword(
                userID: "not-a-user",
                homeserverURL: hostileURL,
                storeRoot: storeRoot,
                password: password,
                core: core
            )
            XCTFail("Hostile identity must fail closed")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s3c-identity-invalid"))
            for forbidden in [password, hostileURL, "secret", "hunter2", "evil.example"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
    }

    func testSharedCoreAttachWithoutSessionFailsClosed() async {
        let core = SharedCore()

        do {
            _ = try await SharedCoreSessionAttach.attachSessionOwners(core: core)
            XCTFail("Fail-closed SharedCore must not attach without a session")
        } catch {
            let publicError = String(reflecting: error)
            XCTAssertTrue(publicError.contains("p4-s3d-session-missing"))
            for forbidden in ["password", "syt_", "@alice:example.org", "token"] {
                XCTAssertFalse(publicError.contains(forbidden))
            }
        }
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
