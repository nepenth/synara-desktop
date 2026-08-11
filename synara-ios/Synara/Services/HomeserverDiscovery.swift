import Foundation
import SynaraCore

struct HomeserverSuggestion: Identifiable, Equatable {
    let id: String
    let name: String
    let address: String

    init(name: String, address: String) {
        self.id = address
        self.name = name
        self.address = address
    }
}

struct HomeserverDiscoveryResult: Equatable {
    let requestedURL: URL
    let homeserverBaseURL: URL
    let supportsPasswordLogin: Bool
}

enum HomeserverDiscoveryError: LocalizedError, Equatable {
    case empty
    case invalidURL
    case unsupportedScheme
    case missingHost
    case discoveryFailed
    case unsupportedServer

    var errorDescription: String? {
        switch self {
        case .empty:
            return "Enter a homeserver address."
        case .invalidURL:
            return "Enter a valid homeserver address."
        case .unsupportedScheme:
            return "Homeserver addresses must use HTTPS."
        case .missingHost:
            return "Enter a homeserver host, such as matrix.org."
        case .discoveryFailed:
            return "Could not discover this homeserver. Check the address and try again."
        case .unsupportedServer:
            return "This homeserver is not supported yet."
        }
    }
}

enum HomeserverAddressNormalizer {
    static func normalize(_ rawValue: String) throws -> URL {
        let trimmed = rawValue.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false else {
            throw HomeserverDiscoveryError.empty
        }

        let candidate = trimmed.contains("://") ? trimmed : "https://\(trimmed)"
        guard var components = URLComponents(string: candidate) else {
            throw HomeserverDiscoveryError.invalidURL
        }

        guard components.scheme?.lowercased() == "https" else {
            throw HomeserverDiscoveryError.unsupportedScheme
        }

        guard let host = components.host?.trimmingCharacters(in: .whitespacesAndNewlines), host.isEmpty == false else {
            throw HomeserverDiscoveryError.missingHost
        }

        components.scheme = "https"
        components.host = host.lowercased()
        var path = components.path
        while path.hasSuffix("/") {
            path.removeLast()
        }
        components.path = path
        components.query = nil
        components.fragment = nil

        guard let normalized = components.url else {
            throw HomeserverDiscoveryError.invalidURL
        }

        return normalized
    }
}

protocol HomeserverDiscovering {
    var suggestions: [HomeserverSuggestion] { get }

    func discover(rawAddress: String) async throws -> HomeserverDiscoveryResult
}

protocol LoginFlowProbing {
    func loginFlows(homeserverURL: URL) async throws -> [LoginFlowDto]
}

struct CoreLoginFlowProbe: LoginFlowProbing {
    func loginFlows(homeserverURL: URL) async throws -> [LoginFlowDto] {
        try await SynaraCore.loginFlows(homeserverUrl: homeserverURL.absoluteString)
    }
}

struct CoreHomeserverDiscoveryService: HomeserverDiscovering {
    let suggestions: [HomeserverSuggestion]
    private let loginFlowProbe: any LoginFlowProbing

    init(
        suggestions: [HomeserverSuggestion] = [
            HomeserverSuggestion(name: "Matrix.org", address: "matrix.org")
        ],
        loginFlowProbe: any LoginFlowProbing = CoreLoginFlowProbe()
    ) {
        self.suggestions = suggestions
        self.loginFlowProbe = loginFlowProbe
    }

    func discover(rawAddress: String) async throws -> HomeserverDiscoveryResult {
        let normalizedURL = try HomeserverAddressNormalizer.normalize(rawAddress)
        let flows: [LoginFlowDto]

        do {
            flows = try await loginFlowProbe.loginFlows(homeserverURL: normalizedURL)
        } catch {
            throw HomeserverDiscoveryError.discoveryFailed
        }

        guard flows.contains(where: { $0.kind == "password" }) else {
            throw HomeserverDiscoveryError.unsupportedServer
        }

        return HomeserverDiscoveryResult(
            requestedURL: normalizedURL,
            homeserverBaseURL: normalizedURL,
            supportsPasswordLogin: true
        )
    }
}

final class MockHomeserverDiscoveryService: HomeserverDiscovering {
    var suggestions: [HomeserverSuggestion]
    var result: Result<HomeserverDiscoveryResult, HomeserverDiscoveryError>?
    private(set) var requestedAddresses: [String] = []

    init(
        suggestions: [HomeserverSuggestion] = [HomeserverSuggestion(name: "Matrix.org", address: "matrix.org")],
        result: Result<HomeserverDiscoveryResult, HomeserverDiscoveryError>? = nil
    ) {
        self.suggestions = suggestions
        self.result = result
    }

    func discover(rawAddress: String) async throws -> HomeserverDiscoveryResult {
        requestedAddresses.append(rawAddress)

        if let result {
            switch result {
            case .success(let discoveryResult):
                return discoveryResult
            case .failure(let error):
                throw error
            }
        }

        let url = try HomeserverAddressNormalizer.normalize(rawAddress)
        return HomeserverDiscoveryResult(
            requestedURL: url,
            homeserverBaseURL: url,
            supportsPasswordLogin: true
        )
    }
}
