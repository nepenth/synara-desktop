import Combine
import Foundation

enum SessionState: Equatable {
    case signedOut
    case signedIn(AuthenticatedSession)
}

struct AuthenticatedSession: Equatable {
    let userID: String
    let deviceID: String
    let homeserverURL: URL
}

protocol MatrixClientServicing {
    var syncStatusDescription: String { get }
}

protocol PushServicing {
    var isRegistrationAvailable: Bool { get }
}

protocol SettingsStoring {
    func bool(for key: String) -> Bool
    func set(_ value: Bool, for key: String)
}

final class AppSessionStore: ObservableObject {
    @Published private(set) var currentState: SessionState

    init(currentState: SessionState = .signedOut) {
        self.currentState = currentState
    }

    func completeLogin(_ session: AuthenticatedSession) {
        currentState = .signedIn(session)
    }

    func signOut() {
        currentState = .signedOut
    }
}

final class PlaceholderMatrixClientService: MatrixClientServicing {
    let syncStatusDescription = "Not connected"
}

final class PlaceholderPushService: PushServicing {
    let isRegistrationAvailable = false
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
    let syncStatusDescription: String

    init(syncStatusDescription: String = "Mock sync idle") {
        self.syncStatusDescription = syncStatusDescription
    }
}

final class MockPushService: PushServicing {
    let isRegistrationAvailable: Bool

    init(isRegistrationAvailable: Bool = false) {
        self.isRegistrationAvailable = isRegistrationAvailable
    }
}
