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

    var errorDescription: String? {
        switch self {
        case .missingUsername:
            return "Enter your username."
        case .missingPassword:
            return "Enter your password."
        case .invalidCredentials:
            return "The username or password is incorrect."
        case .networkFailure:
            return "Could not reach the homeserver. Try again."
        case .cancelled:
            return "Login was cancelled."
        case .unsupported:
            return "Password login is not supported for this homeserver yet."
        case .slidingSyncUnsupported:
            return ServerCompatibilityCopy.slidingSyncUnsupported
        case .sessionPersistenceFailed:
            return "Could not save the session securely."
        case .localDataUnreadable:
            return "Synara can't open this account's saved data on this device, for example after restoring from a backup. Reset local data to sign in again."
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
