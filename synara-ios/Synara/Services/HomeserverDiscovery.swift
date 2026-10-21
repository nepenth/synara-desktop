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
    /// What the user typed, normalized. Shown on the sign-in screen.
    let requestedURL: URL
    /// Client-API URL to sign in against. Differs from `requestedURL` when the
    /// server name delegates through `.well-known/matrix/client`.
    let homeserverBaseURL: URL
    let supportsPasswordLogin: Bool
}

/// Server-compatibility copy shared by discovery, sign-in and the
/// connection banner.
enum ServerCompatibilityCopy {
    static let slidingSyncUnsupported =
        "This server doesn't support the sync protocol Synara needs (simplified sliding sync, MSC4186). Contact the server's administrator."
    static let singleSignOnOnly =
        "This server signs in with single sign-on, which Synara for iPhone doesn't support yet."
}

enum HomeserverDiscoveryError: LocalizedError, Equatable {
    case empty
    case invalidURL
    case unsupportedScheme
    case missingHost
    case discoveryFailed
    case slidingSyncUnsupported
    case singleSignOnOnly
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
        case .slidingSyncUnsupported:
            return ServerCompatibilityCopy.slidingSyncUnsupported
        case .singleSignOnOnly:
            return ServerCompatibilityCopy.singleSignOnOnly
        case .unsupportedServer:
            return "This server doesn't offer password sign-in, which Synara for iPhone needs."
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

protocol HomeserverResolving {
    func resolve(serverNameOrURL: String) async throws -> ResolvedHomeserverDto
}

/// Core resolves with the SDK: `.well-known/matrix/client` delegation, else
/// the address itself as a homeserver URL, validated with `/versions`. The
/// requests carry `Synara-iOS/<version>`, like the signed-in client's.
struct CoreHomeserverResolver: HomeserverResolving {
    func resolve(serverNameOrURL: String) async throws -> ResolvedHomeserverDto {
        try await SynaraCore.resolveHomeserver(
            serverNameOrUrl: serverNameOrURL,
            appVersion: Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? ""
        )
    }
}

struct CoreHomeserverDiscoveryService: HomeserverDiscovering {
    let suggestions: [HomeserverSuggestion]
    private let resolver: any HomeserverResolving

    init(
        suggestions: [HomeserverSuggestion] = [
            HomeserverSuggestion(name: "Matrix.org", address: "matrix.org")
        ],
        resolver: any HomeserverResolving = CoreHomeserverResolver()
    ) {
        self.suggestions = suggestions
        self.resolver = resolver
    }

    func discover(rawAddress: String) async throws -> HomeserverDiscoveryResult {
        let requestedURL = try HomeserverAddressNormalizer.normalize(rawAddress)
        let resolved: ResolvedHomeserverDto

        do {
            resolved = try await resolver.resolve(serverNameOrURL: requestedURL.absoluteString)
        } catch {
            throw HomeserverDiscoveryError.discoveryFailed
        }

        guard let homeserverURL = URL(string: resolved.homeserverUrl), homeserverURL.host != nil else {
            throw HomeserverDiscoveryError.discoveryFailed
        }
        // A well-known document may delegate to any URL; keep the HTTPS rule.
        guard homeserverURL.scheme?.lowercased() == "https" else {
            throw HomeserverDiscoveryError.unsupportedScheme
        }
        guard resolved.nativeSlidingSync else {
            throw HomeserverDiscoveryError.slidingSyncUnsupported
        }
        guard resolved.loginFlows.contains(where: { $0.kind == "password" }) else {
            if resolved.loginFlows.contains(where: { $0.matrixType == "m.login.sso" }) {
                throw HomeserverDiscoveryError.singleSignOnOnly
            }
            throw HomeserverDiscoveryError.unsupportedServer
        }

        return HomeserverDiscoveryResult(
            requestedURL: requestedURL,
            homeserverBaseURL: homeserverURL,
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
