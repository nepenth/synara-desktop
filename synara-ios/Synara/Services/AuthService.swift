import Foundation

struct LoginRequest: Equatable {
    let homeserverURL: URL
    let username: String
    let password: String
}

enum LoginError: LocalizedError, Equatable {
    case missingUsername
    case missingPassword
    case invalidCredentials
    case networkFailure
    case cancelled
    case unsupported

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
        }
    }
}

protocol AuthServicing {
    func login(_ request: LoginRequest) async throws -> AuthenticatedSession
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
            homeserverURL: request.homeserverURL
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
