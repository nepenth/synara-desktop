import Foundation

struct LoginRequest: Equatable {
    let homeserverURL: URL
    let username: String
    let password: String
    /// Server name for a bare username. With `.well-known` delegation this is
    /// the typed server, not the client-API host.
    var accountServerName: String? = nil

    /// Full Matrix ID for `username`, completing a bare localpart.
    var userID: String {
        let username = username.trimmingCharacters(in: .whitespacesAndNewlines)
        if username.hasPrefix("@") {
            return username
        }
        return "@\(username):\(accountServerName ?? homeserverURL.host ?? "localhost")"
    }
}

enum LoginError: LocalizedError, Equatable {
    case missingUsername
    case missingPassword
    case invalidCredentials
    case networkFailure
    case cancelled
    case unsupported
    case slidingSyncUnsupported
    case sessionPersistenceFailed
    /// This account's encrypted data on the device exists but its key does
    /// not (for example after restoring a backup to a new iPhone).
    case localDataUnreadable
    /// The homeserver answered with an outage (5xx, endpoint missing).
    case serverUnavailable
    case rateLimited
    case accountDeactivated
    /// Another process holds this account's local store.
    case localDataInUse
    /// Unclassified: not reported as wrong credentials.
    case signInFailed

    var errorDescription: String? {
        switch self {
        case .missingUsername:
            return String(localized: "Enter your username.")
        case .missingPassword:
            return String(localized: "Enter your password.")
        case .invalidCredentials:
            return String(localized: "The username or password is incorrect.")
        case .networkFailure:
            return String(localized: "Could not reach the homeserver. Try again.")
        case .cancelled:
            return String(localized: "Login was cancelled.")
        case .unsupported:
            return String(localized: "Password login is not supported for this homeserver yet.")
        case .slidingSyncUnsupported:
            return ServerCompatibilityCopy.slidingSyncUnsupported
        case .sessionPersistenceFailed:
            return String(localized: "Could not save the session securely.")
        case .localDataUnreadable:
            return String(localized: "Synara can't open this account's saved data on this device, for example after restoring from a backup. Reset local data to sign in again.")
        case .serverUnavailable:
            return String(localized: "The homeserver is having problems. Try again later.")
        case .rateLimited:
            return String(localized: "Too many sign-in attempts. Wait a moment, then try again.")
        case .accountDeactivated:
            return String(localized: "This account has been deactivated.")
        case .localDataInUse:
            return String(localized: "This account's saved data is in use. Close other copies of Synara, then try again.")
        case .signInFailed:
            return String(localized: "Sign-in failed. Try again.")
        }
    }
}

protocol AuthServicing {
    func login(_ request: LoginRequest) async throws -> AuthenticatedSession
    /// Delete this account's local store and its key after the user
    /// confirmed a `.localDataUnreadable` reset. Other accounts stay.
    func resetLocalData(for request: LoginRequest) async throws
}

extension AuthServicing {
    func resetLocalData(for request: LoginRequest) async throws {
        _ = request
        throw LoginError.unsupported
    }
}

struct PlaceholderAuthService: AuthServicing {
    func login(_ request: LoginRequest) async throws -> AuthenticatedSession {
        let username = request.username.trimmingCharacters(in: .whitespacesAndNewlines)

        guard username.isEmpty == false else {
            throw LoginError.missingUsername
        }

        guard request.password.isEmpty == false else {
            throw LoginError.missingPassword
        }

        return AuthenticatedSession(
            userID: username.hasPrefix("@") ? username : "@\(username):\(request.homeserverURL.host ?? "localhost")",
            deviceID: "SYNARA-IOS-MOCK",
            homeserverURL: request.homeserverURL,
            accessToken: "mock-access-token"
        )
    }
}

final class MockAuthService: AuthServicing {
    var result: Result<AuthenticatedSession, LoginError>?
    private(set) var requests: [LoginRequest] = []

    init(result: Result<AuthenticatedSession, LoginError>? = nil) {
        self.result = result
    }

    func login(_ request: LoginRequest) async throws -> AuthenticatedSession {
        requests.append(request)

        if let result {
            switch result {
            case .success(let session):
                return session
            case .failure(let error):
                throw error
            }
        }

        return try await PlaceholderAuthService().login(request)
    }
}
