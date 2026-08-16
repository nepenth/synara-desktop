import Security
import XCTest
@testable import Synara

final class MatrixStoreLifecycleTests: XCTestCase {
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
            reported: "native",
            available: ["none"]
        )

        XCTAssertEqual(rawValue, "none")
    }

    func testSlidingSyncCompatibilityPreservesNativeWhenAvailable() {
        let rawValue = MatrixSlidingSyncCompatibility.storedRawValue(
            reported: "native",
            available: ["none", "native"]
        )

        XCTAssertEqual(rawValue, "native")
    }

    func testSlidingSyncCompatibilityDowngradesOldNativeSessionWhenServerDoesNotAdvertiseNative() {
        let version = MatrixSlidingSyncCompatibility.sdkVersion(
            storedRawValue: "native",
            available: ["none"]
        )

        XCTAssertEqual(version, "none")
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
