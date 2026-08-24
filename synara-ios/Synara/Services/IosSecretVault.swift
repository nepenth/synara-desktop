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
    private let sharedAccessGroup: String?

    init(
        service: String = "com.whylandcreative.synara.core-vault",
        sharedAccessGroup: String? = KeychainSecureSessionStore.defaultSharedAccessGroup()
    ) {
        self.service = service
        self.sharedAccessGroup = sharedAccessGroup
    }

    func get(key: String) throws -> Data? {
        for accessGroup in loadAccessGroups {
            do {
                if let value = try load(key: key, accessGroup: accessGroup) {
                    if accessGroup == nil, sharedAccessGroup != nil {
                        try? save(key: key, value: value, accessGroup: sharedAccessGroup)
                    }
                    return value
                }
            } catch VaultKeychainError.failure(let status)
                where accessGroup != nil && status == errSecMissingEntitlement {
                continue
            }
        }
        return nil
    }

    func put(key: String, value: Data) throws {
        if let sharedAccessGroup {
            do {
                try save(key: key, value: value, accessGroup: sharedAccessGroup)
                return
            } catch VaultKeychainError.failure(let status) where status == errSecMissingEntitlement {
                // Unit-test and unsigned hosts may not have the production entitlement.
            } catch {
                throw Self.unavailable
            }
        }
        do {
            try save(key: key, value: value, accessGroup: nil)
        } catch {
            throw Self.unavailable
        }
    }

    func delete(key: String) throws {
        var failed = false
        for accessGroup in loadAccessGroups {
            let query = baseQuery(key: key, accessGroup: accessGroup)
            let status = SecItemDelete(query as CFDictionary)
            if status != errSecSuccess,
               status != errSecItemNotFound,
               !(accessGroup != nil && status == errSecMissingEntitlement) {
                failed = true
            }
        }
        if failed {
            throw Self.unavailable
        }
    }

    private var loadAccessGroups: [String?] {
        if let sharedAccessGroup {
            return [sharedAccessGroup, nil]
        }
        return [nil]
    }

    private func baseQuery(key: String, accessGroup: String?) -> [String: Any] {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
        ]
        if let accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }
        return query
    }

    private func load(key: String, accessGroup: String?) throws -> Data? {
        var query = baseQuery(key: key, accessGroup: accessGroup)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        if status == errSecItemNotFound { return nil }
        guard status == errSecSuccess else { throw VaultKeychainError.failure(status) }
        return item as? Data
    }

    private func save(key: String, value: Data, accessGroup: String?) throws {
        let query = baseQuery(key: key, accessGroup: accessGroup)
        let attributes: [String: Any] = [
            kSecValueData as String: value,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
        ]
        let updateStatus = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)
        if updateStatus == errSecSuccess { return }
        guard updateStatus == errSecItemNotFound else {
            throw VaultKeychainError.failure(updateStatus)
        }
        var addQuery = query
        for (key, value) in attributes { addQuery[key] = value }
        let addStatus = SecItemAdd(addQuery as CFDictionary, nil)
        guard addStatus == errSecSuccess else { throw VaultKeychainError.failure(addStatus) }
    }

    private static var unavailable: IosSecretVaultError {
        .Unavailable(
            code: "p4-s3-secret-vault-unavailable",
            description: "The secret store is unavailable."
        )
    }
}

private enum VaultKeychainError: Error {
    case failure(OSStatus)
}
