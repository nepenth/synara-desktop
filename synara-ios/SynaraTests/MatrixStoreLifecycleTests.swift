import Security
import XCTest
@testable import Synara

final class MatrixStoreLifecycleTests: XCTestCase {
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
