import Foundation
import Security
import SynaraCore

/// In-memory `IosSecretVault` for tests and fail-closed previews.
///
/// This is not a session object store. Keys and values are opaque.
final class InMemoryIosSecretVault: IosSecretVault, @unchecked Sendable {
    private let lock = NSLock()
    private var storage: [String: Data] = [:]

    func get(key: String) throws -> Data? {
        lock.lock()
        defer { lock.unlock() }
        return storage[key]
    }

    func put(key: String, value: Data) throws {
        lock.lock()
        defer { lock.unlock() }
        storage[key] = value
    }

    func delete(key: String) throws {
        lock.lock()
        defer { lock.unlock() }
        storage.removeValue(forKey: key)
    }
}

/// Generic Keychain key/value adapter for Core's `SecretVault`.
///
/// This does not replace `KeychainSecureSessionStore`. It never logs keys or
/// values. Failures become the static UniFFI unavailable error.
final class KeychainIosSecretVault: IosSecretVault, @unchecked Sendable {
    private let service: String

    init(service: String = "com.whylandcreative.synara.core-vault") {
        self.service = service
    }

    func get(key: String) throws -> Data? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        if status == errSecItemNotFound {
            return nil
        }
        guard status == errSecSuccess else {
            throw Self.unavailable
        }
        return item as? Data
    }

    func put(key: String, value: Data) throws {
        try delete(key: key)
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecValueData as String: value,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
        ]
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw Self.unavailable
        }
    }

    func delete(key: String) throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
        ]
        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw Self.unavailable
        }
    }

    private static var unavailable: IosSecretVaultError {
        .unavailable(
            code: "p4-s3-secret-vault-unavailable",
            description: "The secret store is unavailable."
        )
    }
}
