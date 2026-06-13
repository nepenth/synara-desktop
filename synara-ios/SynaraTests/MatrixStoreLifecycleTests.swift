import Security
import MatrixRustSDK
import XCTest
@testable import Synara

final class MatrixStoreLifecycleTests: XCTestCase {
    func testPersistedStoreSchemaVersionIsPositive() {
        XCTAssertGreaterThan(MatrixRustSDKClientStore.persistedStoreSchemaVersion, 0)
    }

    func testPruneLegacyPersistedStoresRemovesUnversionedDirectories() throws {
        let root = try makeTemporaryStoreRoot()
        defer { try? FileManager.default.removeItem(at: root) }

        let legacyStore = root.appendingPathComponent("matrix.org-alice", isDirectory: true)
        let versionedRoot = root.appendingPathComponent("v\(MatrixRustSDKClientStore.persistedStoreSchemaVersion)", isDirectory: true)
        let versionedStore = versionedRoot.appendingPathComponent("matrix.org-alice", isDirectory: true)

        try FileManager.default.createDirectory(at: legacyStore.appendingPathComponent("data", isDirectory: true), withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: versionedStore.appendingPathComponent("data", isDirectory: true), withIntermediateDirectories: true)

        try MatrixRustSDKClientStore.pruneLegacyPersistedStores(in: root)

        XCTAssertFalse(FileManager.default.fileExists(atPath: legacyStore.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: versionedStore.path))
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
        XCTAssertEqual(SecItemAdd(addQuery as CFDictionary, nil), errSecSuccess)

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

    private func makeSession(userID: String = "@alice:matrix.org") throws -> AuthenticatedSession {
        AuthenticatedSession(
            userID: userID,
            deviceID: "DEVICE",
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.org")),
            accessToken: "secret-token"
        )
    }
}
