import Combine
import Foundation

enum SessionState: Equatable {
    case signedOut
    case signedIn(AuthenticatedSession)
}

struct AuthenticatedSession: Codable, Equatable {
    let userID: String
    let deviceID: String
    let homeserverURL: URL
    let accessToken: String
}

enum MatrixSyncStatus: Equatable {
    case stopped
    case starting
    case syncing
    case failed(String)

    var description: String {
        switch self {
        case .stopped:
            return "Not connected"
        case .starting:
            return "Starting sync"
        case .syncing:
            return "Syncing"
        case .failed(let message):
            return message
        }
    }
}

protocol MatrixClientServicing: AnyObject {
    var syncStatusDescription: String { get }
    var syncStatus: MatrixSyncStatus { get }

    func start(session: AuthenticatedSession) async
    func stop() async
    func resetLocalState()
}

protocol PushServicing {
    var isRegistrationAvailable: Bool { get }
    func clearRegistrationState()
}

protocol SettingsStoring {
    func bool(for key: String) -> Bool
    func set(_ value: Bool, for key: String)
}

final class AppSessionStore: ObservableObject {
    @Published private(set) var currentState: SessionState
    let secureStore: SecureSessionStoring
    private(set) var restoreFailureLogDescription: String?

    init(
        currentState: SessionState = .signedOut,
        secureStore: SecureSessionStoring = InMemorySecureSessionStore(),
        restorePersistedSession: Bool = false
    ) {
        self.secureStore = secureStore

        if restorePersistedSession {
            do {
                if let restored = try secureStore.load() {
                    self.currentState = .signedIn(restored)
                } else {
                    self.currentState = currentState
                }
            } catch let error as SecureSessionStoreError {
                restoreFailureLogDescription = error.logDescription
                self.currentState = .signedOut
            } catch {
                restoreFailureLogDescription = "secure session restore failed"
                self.currentState = .signedOut
            }
        } else {
            self.currentState = currentState
        }
    }

    func restore() throws {
        do {
            if let restored = try secureStore.load() {
                restoreFailureLogDescription = nil
                currentState = .signedIn(restored)
            } else {
                restoreFailureLogDescription = nil
                currentState = .signedOut
            }
        } catch let error as SecureSessionStoreError {
            restoreFailureLogDescription = error.logDescription
            throw error
        } catch {
            restoreFailureLogDescription = "secure session restore failed"
            throw error
        }
    }

    func completeLogin(_ session: AuthenticatedSession) throws {
        try secureStore.save(session)
        currentState = .signedIn(session)
    }

    func signOut() throws {
        try secureStore.delete()
        currentState = .signedOut
    }
}

final class PlaceholderMatrixClientService: MatrixClientServicing {
    private(set) var syncStatus: MatrixSyncStatus = .stopped

    var syncStatusDescription: String {
        syncStatus.description
    }

    func start(session: AuthenticatedSession) async {
        syncStatus = .syncing
    }

    func stop() async {
        syncStatus = .stopped
    }

    func resetLocalState() {
        syncStatus = .stopped
    }
}

final class PlaceholderPushService: PushServicing {
    let isRegistrationAvailable = false

    func clearRegistrationState() {}
}

final class InMemorySettingsStore: SettingsStoring {
    private var values: [String: Bool] = [:]

    func bool(for key: String) -> Bool {
        values[key] ?? false
    }

    func set(_ value: Bool, for key: String) {
        values[key] = value
    }
}

final class MockMatrixClientService: MatrixClientServicing {
    private(set) var syncStatus: MatrixSyncStatus
    private(set) var startedSessions: [AuthenticatedSession] = []
    private(set) var stopCallCount = 0
    private(set) var resetCallCount = 0

    var syncStatusDescription: String {
        syncStatus.description
    }

    init(syncStatus: MatrixSyncStatus = .stopped) {
        self.syncStatus = syncStatus
    }

    func start(session: AuthenticatedSession) async {
        startedSessions.append(session)
        syncStatus = .syncing
    }

    func stop() async {
        stopCallCount += 1
        syncStatus = .stopped
    }

    func resetLocalState() {
        resetCallCount += 1
        syncStatus = .stopped
    }
}

final class MockPushService: PushServicing {
    let isRegistrationAvailable: Bool
    private(set) var clearCallCount = 0

    init(isRegistrationAvailable: Bool = false) {
        self.isRegistrationAvailable = isRegistrationAvailable
    }

    func clearRegistrationState() {
        clearCallCount += 1
    }
}
