import Security
import MatrixRustSDK
import XCTest
@testable import Synara

final class MatrixStoreLifecycleTests: XCTestCase {
    func testPersistedStoreSchemaVersionIsPositive() {
        XCTAssertGreaterThan(MatrixRustSDKClientStore.persistedStoreSchemaVersion, 0)
    }

    func testPruneLegacyPersistedStoresRequiresValidatedRestoreEvidence() throws {
        let root = try makeTemporaryStoreRoot()
        defer { try? FileManager.default.removeItem(at: root) }

        let legacyStore = root.appendingPathComponent("matrix.org-alice", isDirectory: true)
        let versionedRoot = root.appendingPathComponent("v\(MatrixRustSDKClientStore.persistedStoreSchemaVersion)", isDirectory: true)
        let versionedStore = versionedRoot.appendingPathComponent("matrix.org-alice", isDirectory: true)

        try FileManager.default.createDirectory(at: legacyStore.appendingPathComponent("data", isDirectory: true), withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: versionedStore.appendingPathComponent("data", isDirectory: true), withIntermediateDirectories: true)

        try MatrixRustSDKClientStore.pruneLegacyPersistedStores(
            in: root,
            validatedStoreIDs: []
        )

        XCTAssertTrue(FileManager.default.fileExists(atPath: legacyStore.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: versionedStore.path))

        try MatrixRustSDKClientStore.pruneLegacyPersistedStores(
            in: root,
            validatedStoreIDs: [legacyStore.lastPathComponent]
        )

        XCTAssertFalse(FileManager.default.fileExists(atPath: legacyStore.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: versionedStore.path))
    }

    func testPersistedStoreMustContainDurableDataBeforeDeviceRestore() throws {
        let root = try makeTemporaryStoreRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let session = try makeSession(sdkStoreID: "known-device-store")
        let dataDirectory = root
            .appendingPathComponent("v\(MatrixRustSDKClientStore.persistedStoreSchemaVersion)", isDirectory: true)
            .appendingPathComponent("known-device-store", isDirectory: true)
            .appendingPathComponent("data", isDirectory: true)
        try FileManager.default.createDirectory(at: dataDirectory, withIntermediateDirectories: true)

        XCTAssertFalse(MatrixRustSDKClientStore.persistedStoreContainsDurableData(for: session, in: root))

        let database = dataDirectory.appendingPathComponent("matrix-sdk-state.sqlite3")
        XCTAssertTrue(FileManager.default.createFile(atPath: database.path, contents: Data([0x53, 0x51, 0x4c])))

        XCTAssertTrue(MatrixRustSDKClientStore.persistedStoreContainsDurableData(for: session, in: root))
        XCTAssertTrue(MatrixRustSDKClientStore.persistedStoreIsEligibleForRestore(for: session, in: root))

        try MatrixRustSDKClientStore.recordPersistedStoreIdentity(for: session, in: root)
        XCTAssertTrue(MatrixRustSDKClientStore.persistedStoreIsEligibleForRestore(for: session, in: root))

        let mismatchedDevice = AuthenticatedSession(
            userID: session.userID,
            deviceID: "DIFFERENT-DEVICE",
            homeserverURL: session.homeserverURL,
            accessToken: session.accessToken,
            sdkStoreID: session.sdkStoreID
        )
        XCTAssertFalse(MatrixRustSDKClientStore.persistedStoreIsEligibleForRestore(for: mismatchedDevice, in: root))
    }

    func testRestoreFailuresAlwaysPreservePersistedCryptoStore() {
        XCTAssertFalse(MatrixSessionRestoreError.persistedStoreUnavailable.shouldDeletePersistedStore)
        XCTAssertFalse(MatrixSessionRestoreError.restorationFailed.shouldDeletePersistedStore)
        XCTAssertFalse(MatrixSessionRestoreError.serverDeviceKeysUnavailable.shouldDeletePersistedStore)
        XCTAssertFalse(MatrixSessionRestoreError.deviceIdentityMismatch.shouldDeletePersistedStore)
    }

    func testDeviceKeyContinuityRequiresBothServerKeysToMatch() throws {
        let response = try deviceKeysResponse(
            deviceID: "DEVICE",
            curve25519Key: "curve-key",
            ed25519Key: "ed-key"
        )

        XCTAssertEqual(
            MatrixDeviceKeyContinuityValidator.validate(
                responseData: response,
                userID: "@alice:matrix.org",
                deviceID: "DEVICE",
                localCurve25519Key: "curve-key",
                localEd25519Key: "ed-key"
            ),
            .matches
        )
        XCTAssertEqual(
            MatrixDeviceKeyContinuityValidator.validate(
                responseData: response,
                userID: "@alice:matrix.org",
                deviceID: "DEVICE",
                localCurve25519Key: "different",
                localEd25519Key: "ed-key"
            ),
            .mismatch
        )
    }

    func testDeviceKeyContinuityFailsClosedForMissingServerOrLocalKeys() throws {
        let response = try deviceKeysResponse(
            deviceID: "DEVICE",
            curve25519Key: "curve-key",
            ed25519Key: "ed-key"
        )
        let missingServerDevice = try JSONSerialization.data(withJSONObject: ["device_keys": [:]])

        XCTAssertEqual(
            MatrixDeviceKeyContinuityValidator.validate(
                responseData: missingServerDevice,
                userID: "@alice:matrix.org",
                deviceID: "DEVICE",
                localCurve25519Key: "curve-key",
                localEd25519Key: "ed-key"
            ),
            .unavailable
        )
        XCTAssertEqual(
            MatrixDeviceKeyContinuityValidator.validate(
                responseData: response,
                userID: "@alice:matrix.org",
                deviceID: "DEVICE",
                localCurve25519Key: nil,
                localEd25519Key: "ed-key"
            ),
            .unavailable
        )
    }

    func testDeletePersistedStoreRemovesOnlyTargetSession() throws {
        let alice = try makeSession(userID: "@alice:matrix.org")
        let bob = try makeSession(userID: "@bob:matrix.org")
        defer {
            try? MatrixRustSDKClientStore.deletePersistedStore(for: alice)
            try? MatrixRustSDKClientStore.deletePersistedStore(for: bob)
        }

        try MatrixRustSDKClientStore.materializePersistedStore(for: alice)
        try MatrixRustSDKClientStore.materializePersistedStore(for: bob)

        XCTAssertTrue(MatrixRustSDKClientStore.persistedStoreExists(for: alice))
        XCTAssertTrue(MatrixRustSDKClientStore.persistedStoreExists(for: bob))

        try MatrixRustSDKClientStore.deletePersistedStore(for: alice)

        XCTAssertFalse(MatrixRustSDKClientStore.persistedStoreExists(for: alice))
        XCTAssertTrue(MatrixRustSDKClientStore.persistedStoreExists(for: bob))
    }

    func testKeychainSecureStoreMigratesLegacyEnvelope() throws {
        let store = KeychainSecureSessionStore()
        try? store.delete()
        defer { try? store.delete() }

        let session = try makeSession()
        let legacyData = try JSONEncoder().encode(session)

        let addQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: "com.whylandcreative.synara.session",
            kSecAttrAccount as String: "current",
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
            kSecValueData as String: legacyData
        ]
        let addStatus = SecItemAdd(addQuery as CFDictionary, nil)
        if addStatus == errSecMissingEntitlement {
            throw XCTSkip("Unsigned simulator test target cannot access Keychain entitlements.")
        }
        XCTAssertEqual(addStatus, errSecSuccess)

        XCTAssertEqual(try store.migrateIfNeeded(), .migrated)
        XCTAssertEqual(try store.load(), session)
    }

    func testSlidingSyncCompatibilityStoresNoneWhenNativeIsUnavailable() {
        let rawValue = MatrixSlidingSyncCompatibility.storedRawValue(
            reported: .native,
            available: [.none]
        )

        XCTAssertEqual(rawValue, "none")
    }

    func testSlidingSyncCompatibilityPreservesNativeWhenAvailable() {
        let rawValue = MatrixSlidingSyncCompatibility.storedRawValue(
            reported: .native,
            available: [.none, .native]
        )

        XCTAssertEqual(rawValue, "native")
    }

    func testSlidingSyncCompatibilityDowngradesOldNativeSessionWhenServerDoesNotAdvertiseNative() {
        let version = MatrixSlidingSyncCompatibility.sdkVersion(
            storedRawValue: "native",
            available: [.none]
        )

        XCTAssertEqual(version, .none)
    }

    private func makeTemporaryStoreRoot() throws -> URL {
        let base = FileManager.default.temporaryDirectory
            .appendingPathComponent("synara-store-test-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
        return base
    }

    private func deviceKeysResponse(
        deviceID: String,
        curve25519Key: String,
        ed25519Key: String
    ) throws -> Data {
        try JSONSerialization.data(withJSONObject: [
            "device_keys": [
                "@alice:matrix.org": [
                    deviceID: [
                        "keys": [
                            "curve25519:\(deviceID)": curve25519Key,
                            "ed25519:\(deviceID)": ed25519Key,
                        ],
                    ],
                ],
            ],
        ])
    }

    private func makeSession(
        userID: String = "@alice:matrix.org",
        sdkStoreID: String? = nil
    ) throws -> AuthenticatedSession {
        AuthenticatedSession(
            userID: userID,
            deviceID: "DEVICE",
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.org")),
            accessToken: "secret-token",
            sdkStoreID: sdkStoreID
        )
    }
}
