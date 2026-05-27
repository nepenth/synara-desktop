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
    case sessionPersistenceFailed

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
        case .sessionPersistenceFailed:
            return "Could not save the session securely."
        }
    }
}

protocol AuthServicing {
    func login(_ request: LoginRequest) async throws -> AuthenticatedSession
}

protocol AuthHTTPClient {
    func data(for request: URLRequest) async throws -> (Data, URLResponse)
}

extension URLSession: AuthHTTPClient {}

struct MatrixPasswordAuthService: AuthServicing {
    private let httpClient: AuthHTTPClient
    private let jsonDecoder: JSONDecoder
    private let jsonEncoder: JSONEncoder

    init(
        httpClient: AuthHTTPClient = URLSession.shared,
        jsonDecoder: JSONDecoder = JSONDecoder(),
        jsonEncoder: JSONEncoder = JSONEncoder()
    ) {
        self.httpClient = httpClient
        self.jsonDecoder = jsonDecoder
        self.jsonEncoder = jsonEncoder
    }

    func login(_ request: LoginRequest) async throws -> AuthenticatedSession {
        let username = request.username.trimmingCharacters(in: .whitespacesAndNewlines)

        guard username.isEmpty == false else {
            throw LoginError.missingUsername
        }

        guard request.password.isEmpty == false else {
            throw LoginError.missingPassword
        }

        do {
            try await validatePasswordLoginSupport(homeserverURL: request.homeserverURL)

            var loginRequest = URLRequest(url: matrixLoginURL(for: request.homeserverURL))
            loginRequest.httpMethod = "POST"
            loginRequest.setValue("application/json", forHTTPHeaderField: "Content-Type")
            loginRequest.httpBody = try jsonEncoder.encode(
                MatrixPasswordLoginRequest(
                    identifier: MatrixLoginIdentifier(user: username),
                    password: request.password
                )
            )

            let (data, response) = try await httpClient.data(for: loginRequest)
            guard let httpResponse = response as? HTTPURLResponse else {
                throw LoginError.networkFailure
            }

            switch httpResponse.statusCode {
            case 200:
                let loginResponse = try jsonDecoder.decode(MatrixPasswordLoginResponse.self, from: data)
                return AuthenticatedSession(
                    userID: loginResponse.userID,
                    deviceID: loginResponse.deviceID,
                    homeserverURL: request.homeserverURL,
                    accessToken: loginResponse.accessToken
                )
            case 401, 403:
                throw LoginError.invalidCredentials
            case 400:
                throw try mapMatrixError(data: data)
            default:
                throw LoginError.networkFailure
            }
        } catch let error as LoginError {
            throw error
        } catch {
            throw LoginError.networkFailure
        }
    }

    private func validatePasswordLoginSupport(homeserverURL: URL) async throws {
        var request = URLRequest(url: matrixLoginURL(for: homeserverURL))
        request.httpMethod = "GET"

        let (data, response) = try await httpClient.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse else {
            throw LoginError.networkFailure
        }

        switch httpResponse.statusCode {
        case 200:
            let response = try jsonDecoder.decode(MatrixLoginFlowsResponse.self, from: data)
            guard response.flows.contains(where: { $0.type == "m.login.password" }) else {
                throw LoginError.unsupported
            }
        case 404:
            throw LoginError.unsupported
        default:
            throw LoginError.networkFailure
        }
    }

    private func matrixLoginURL(for homeserverURL: URL) -> URL {
        var url = homeserverURL
        url.appendPathComponent("_matrix")
        url.appendPathComponent("client")
        url.appendPathComponent("v3")
        url.appendPathComponent("login")
        return url
    }

    private func mapMatrixError(data: Data) throws -> LoginError {
        guard let error = try? jsonDecoder.decode(MatrixErrorResponse.self, from: data) else {
            return .networkFailure
        }

        switch error.errcode {
        case "M_FORBIDDEN":
            return .invalidCredentials
        case "M_UNRECOGNIZED", "M_UNKNOWN":
            return .unsupported
        default:
            return .networkFailure
        }
    }
}

private struct MatrixLoginFlowsResponse: Decodable {
    let flows: [MatrixLoginFlow]
}

private struct MatrixLoginFlow: Decodable {
    let type: String
}

private struct MatrixPasswordLoginRequest: Encodable {
    let type = "m.login.password"
    let identifier: MatrixLoginIdentifier
    let password: String
    let initialDeviceDisplayName = "Synara iOS"

    enum CodingKeys: String, CodingKey {
        case type
        case identifier
        case password
        case initialDeviceDisplayName = "initial_device_display_name"
    }
}

private struct MatrixLoginIdentifier: Encodable {
    let type = "m.id.user"
    let user: String
}

private struct MatrixPasswordLoginResponse: Decodable {
    let userID: String
    let deviceID: String
    let accessToken: String

    enum CodingKeys: String, CodingKey {
        case userID = "user_id"
        case deviceID = "device_id"
        case accessToken = "access_token"
    }
}

private struct MatrixErrorResponse: Decodable {
    let errcode: String
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
