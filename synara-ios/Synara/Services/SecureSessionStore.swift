import Foundation
import Security

enum SecureSessionStoreError: LocalizedError, Equatable {
    case corruptEntry
    case keychainFailure(status: Int32)

    var errorDescription: String? {
        switch self {
        case .corruptEntry:
            return "Stored session data is corrupt."
        case .keychainFailure:
            return "Could not access secure session storage."
        }
    }

    var logDescription: String {
        switch self {
        case .corruptEntry:
            return "secure session entry is corrupt"
        case .keychainFailure(let status):
            return "secure session keychain failure status \(status)"
        }
    }
}

enum SessionMigrationResult: Equatable {
    case notNeeded
    case migrated
}

protocol SecureSessionStoring {
    func save(_ session: AuthenticatedSession) throws
    func load() throws -> AuthenticatedSession?
    func delete() throws
    func migrateIfNeeded() throws -> SessionMigrationResult
}

private struct SecureSessionEnvelope: Codable {
    let version: Int
    let session: AuthenticatedSession
}

final class KeychainSecureSessionStore: SecureSessionStoring {
    private let service = "com.whylandcreative.synara.session"
    private let account = "current"
    private let sharedAccessGroup: String?

    init(sharedAccessGroup: String? = KeychainSecureSessionStore.defaultSharedAccessGroup()) {
        self.sharedAccessGroup = sharedAccessGroup
    }

    func save(_ session: AuthenticatedSession) throws {
        let data = try JSONEncoder().encode(SecureSessionEnvelope(version: 1, session: session))

        if let sharedAccessGroup {
            do {
                try save(data, accessGroup: sharedAccessGroup)
                try? deleteItem(accessGroup: nil)
                // Some Keychain implementations let the unscoped cleanup query
                // match the shared item too. Verify and restore it without putting
                // the existing fallback at risk before the preferred write succeeds.
                if try loadData(accessGroup: sharedAccessGroup) == nil {
                    try save(data, accessGroup: sharedAccessGroup)
                }
                return
            } catch SecureSessionStoreError.keychainFailure(let status)
                where Self.shouldIgnoreAccessGroupFailure(status: status, accessGroup: sharedAccessGroup) {
                try save(data, accessGroup: nil)
                return
            } catch {
                // Preserve a usable fallback if the preferred store fails for a
                // reason other than an unavailable entitlement.
                try? save(data, accessGroup: nil)
                throw error
            }
        }

        try save(data, accessGroup: nil)
    }

    func load() throws -> AuthenticatedSession? {
        for accessGroup in loadAccessGroups {
            do {
                if let data = try loadData(accessGroup: accessGroup) {
                    let session = try decode(data)
                    if accessGroup == nil, sharedAccessGroup != nil {
                        try? save(session)
                    }
                    return session
                }
            } catch SecureSessionStoreError.keychainFailure(let status)
                where Self.shouldIgnoreAccessGroupFailure(status: status, accessGroup: accessGroup) {
                continue
            }
        }

        return nil
    }

    func delete() throws {
        var firstFailure: Int32?
        for accessGroup in loadAccessGroups {
            do {
                try deleteItem(accessGroup: accessGroup)
                continue
            } catch SecureSessionStoreError.keychainFailure(let status) {
                if Self.shouldIgnoreAccessGroupFailure(status: status, accessGroup: accessGroup) {
                    continue
                }
                firstFailure = firstFailure ?? status
            } catch {
                firstFailure = firstFailure ?? errSecInternalError
            }
        }

        if let firstFailure {
            throw SecureSessionStoreError.keychainFailure(status: firstFailure)
        }
    }

    func migrateIfNeeded() throws -> SessionMigrationResult {
        guard let stored = try loadStoredData() else {
            return .notNeeded
        }

        if let session = try? JSONDecoder().decode(AuthenticatedSession.self, from: stored.data) {
            try save(session)
            return .migrated
        }

        let session = try decode(stored.data)
        if stored.accessGroup == nil, sharedAccessGroup != nil {
            try save(session)
            return .migrated
        }

        return .notNeeded
    }

    private var loadAccessGroups: [String?] {
        if let sharedAccessGroup {
            return [sharedAccessGroup, nil]
        }
        return [nil]
    }

    private func save(_ data: Data, accessGroup: String?) throws {
        let matchQuery = baseQuery(accessGroup: accessGroup)
        let updateAttributes: [String: Any] = [
            kSecValueData as String: data,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        ]

        let updateStatus = SecItemUpdate(matchQuery as CFDictionary, updateAttributes as CFDictionary)
        if updateStatus == errSecSuccess {
            return
        }

        if updateStatus != errSecItemNotFound {
            throw SecureSessionStoreError.keychainFailure(status: updateStatus)
        }

        var addQuery = matchQuery
        addQuery[kSecValueData as String] = data
        addQuery[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly

        let addStatus = SecItemAdd(addQuery as CFDictionary, nil)
        guard addStatus == errSecSuccess else {
            throw SecureSessionStoreError.keychainFailure(status: addStatus)
        }
    }

    private func deleteItem(accessGroup: String?) throws {
        let status = SecItemDelete(baseQuery(accessGroup: accessGroup) as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw SecureSessionStoreError.keychainFailure(status: status)
        }
    }

    private func loadData(accessGroup: String?) throws -> Data? {
        var query = baseQuery(accessGroup: accessGroup)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne

        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)

        if status == errSecItemNotFound {
            return nil
        }

        guard status == errSecSuccess else {
            throw SecureSessionStoreError.keychainFailure(status: status)
        }

        guard let data = item as? Data else {
            throw SecureSessionStoreError.corruptEntry
        }

        return data
    }

    private func loadStoredData() throws -> (data: Data, accessGroup: String?)? {
        for accessGroup in loadAccessGroups {
            do {
                if let data = try loadData(accessGroup: accessGroup) {
                    return (data, accessGroup)
                }
            } catch SecureSessionStoreError.keychainFailure(let status)
                where Self.shouldIgnoreAccessGroupFailure(status: status, accessGroup: accessGroup) {
                continue
            }
        }
        return nil
    }

    private func baseQuery(accessGroup: String?) -> [String: Any] {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]

        if let accessGroup {
            query[kSecAttrAccessGroup as String] = accessGroup
        }

        return query
    }

    static func defaultSharedAccessGroup(bundle: Bundle = .main) -> String? {
        guard let value = bundle.object(forInfoDictionaryKey: SynaraSharedConstants.keychainAccessGroupInfoKey) as? String else {
            return nil
        }

        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false,
              trimmed.contains("$(") == false else {
            return nil
        }

        return trimmed
    }

    private static func shouldIgnoreAccessGroupFailure(status: Int32, accessGroup: String?) -> Bool {
        accessGroup != nil && status == errSecMissingEntitlement
    }

    private func decode(_ data: Data) throws -> AuthenticatedSession {
        do {
            let envelope = try JSONDecoder().decode(SecureSessionEnvelope.self, from: data)
            guard envelope.version == 1 else {
                throw SecureSessionStoreError.corruptEntry
            }
            return envelope.session
        } catch let error as SecureSessionStoreError {
            throw error
        } catch {
            throw SecureSessionStoreError.corruptEntry
        }
    }
}

final class InMemorySecureSessionStore: SecureSessionStoring {
    private var data: Data?
    private(set) var saveCallCount = 0
    private(set) var loadCallCount = 0
    private(set) var deleteCallCount = 0
    private(set) var migrationCallCount = 0

    init(session: AuthenticatedSession? = nil) {
        if let session {
            data = try? JSONEncoder().encode(SecureSessionEnvelope(version: 1, session: session))
        }
    }

    func save(_ session: AuthenticatedSession) throws {
        saveCallCount += 1
        data = try JSONEncoder().encode(SecureSessionEnvelope(version: 1, session: session))
    }

    func load() throws -> AuthenticatedSession? {
        loadCallCount += 1
        guard let data else {
            return nil
        }
        return try decode(data)
    }

    func delete() throws {
        deleteCallCount += 1
        data = nil
    }

    func migrateIfNeeded() throws -> SessionMigrationResult {
        migrationCallCount += 1
        guard let data else {
            return .notNeeded
        }

        if let session = try? JSONDecoder().decode(AuthenticatedSession.self, from: data) {
            self.data = try JSONEncoder().encode(SecureSessionEnvelope(version: 1, session: session))
            return .migrated
        }

        _ = try decode(data)
        return .notNeeded
    }

    func writeRawDataForTesting(_ data: Data) {
        self.data = data
    }

    private func decode(_ data: Data) throws -> AuthenticatedSession {
        do {
            let envelope = try JSONDecoder().decode(SecureSessionEnvelope.self, from: data)
            guard envelope.version == 1 else {
                throw SecureSessionStoreError.corruptEntry
            }
            return envelope.session
        } catch let error as SecureSessionStoreError {
            throw error
        } catch {
            throw SecureSessionStoreError.corruptEntry
        }
    }
}
