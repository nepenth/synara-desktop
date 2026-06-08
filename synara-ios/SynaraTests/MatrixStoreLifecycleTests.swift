import Security
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

    func testKeychainSecureStoreMigratesLegacyEnvelope() throws {
        let store = KeychainSecureSessionStore()
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

    private func makeTemporaryStoreRoot() throws -> URL {
        let base = FileManager.default.temporaryDirectory
            .appendingPathComponent("synara-store-test-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
        return base
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