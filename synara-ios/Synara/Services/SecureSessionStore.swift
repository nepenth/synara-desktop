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
    private let service = "app.synara.ios.session"
    private let account = "current"

    func save(_ session: AuthenticatedSession) throws {
        let data = try JSONEncoder().encode(SecureSessionEnvelope(version: 1, session: session))
        try delete()

        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly,
            kSecValueData as String: data
        ]

        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw SecureSessionStoreError.keychainFailure(status: status)
        }
    }

    func load() throws -> AuthenticatedSession? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]

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

        return try decode(data)
    }

    func delete() throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]

        let status = SecItemDelete(query as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw SecureSessionStoreError.keychainFailure(status: status)
        }
    }

    func migrateIfNeeded() throws -> SessionMigrationResult {
        _ = try load()
        return .notNeeded
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
