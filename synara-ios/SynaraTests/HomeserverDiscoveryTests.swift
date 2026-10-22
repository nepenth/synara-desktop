import XCTest
import SynaraCore
@testable import Synara

private final class HomeserverResolverSpy: HomeserverResolving {
    var result: Result<ResolvedHomeserverDto, Error>
    private(set) var requestedAddresses: [String] = []

    init(result: Result<ResolvedHomeserverDto, Error>) {
        self.result = result
    }

    func resolve(serverNameOrURL: String) async throws -> ResolvedHomeserverDto {
        requestedAddresses.append(serverNameOrURL)
        return try result.get()
    }
}

private enum HomeserverResolverFailure: Error {
    case unavailable
}

private let passwordFlow = LoginFlowDto(kind: "password", matrixType: "m.login.password", getLoginToken: nil)
private let ssoFlow = LoginFlowDto(kind: "unknown", matrixType: "m.login.sso", getLoginToken: nil)
private let tokenFlow = LoginFlowDto(kind: "token", matrixType: "m.login.token", getLoginToken: true)

private func resolved(
    _ homeserverURL: String,
    flows: [LoginFlowDto] = [passwordFlow],
    nativeSlidingSync: Bool = true
) -> ResolvedHomeserverDto {
    ResolvedHomeserverDto(
        homeserverUrl: homeserverURL,
        loginFlows: flows,
        nativeSlidingSync: nativeSlidingSync
    )
}

final class HomeserverDiscoveryTests: XCTestCase {
    func testLiveCoreHomeserverResolverWhenConfigured() async throws {
        let environment = ProcessInfo.processInfo.environment
        guard environment["SYNARA_LIVE_DISCOVERY_SMOKE"] == "1"
            || environment["TEST_RUNNER_SYNARA_LIVE_DISCOVERY_SMOKE"] == "1"
        else {
            throw XCTSkip("Set SYNARA_LIVE_DISCOVERY_SMOKE=1 for live shared-core discovery.")
        }
        guard let homeserver = environment["SYNARA_LIVE_HOMESERVER"]
            ?? environment["TEST_RUNNER_SYNARA_LIVE_HOMESERVER"]
        else {
            XCTFail("Live discovery requires a homeserver environment variable.")
            return
        }

        do {
            let result = try await CoreHomeserverResolver().resolve(serverNameOrURL: homeserver)
            XCTAssertTrue(result.loginFlows.contains(where: { $0.kind == "password" }))
            XCTAssertTrue(result.nativeSlidingSync)
        } catch {
            XCTFail("Shared-core discovery failed: \(String(reflecting: error))")
        }
    }

    func testNormalizerAddsHttpsAndLowercasesHost() throws {
        let url = try HomeserverAddressNormalizer.normalize(" Matrix.ORG ")

        XCTAssertEqual(url.absoluteString, "https://matrix.org")
    }

    func testNormalizerRemovesQueryFragmentAndTrailingSlash() throws {
        let url = try HomeserverAddressNormalizer.normalize("https://matrix.org/?token=secret#fragment")

        XCTAssertEqual(url.absoluteString, "https://matrix.org")
    }

    func testNormalizerKeepsBasePath() throws {
        let url = try HomeserverAddressNormalizer.normalize("https://example.org/matrix/")

        XCTAssertEqual(url.absoluteString, "https://example.org/matrix")
    }

    func testNormalizerKeepsPort() throws {
        let url = try HomeserverAddressNormalizer.normalize("127.0.0.1:8449")

        XCTAssertEqual(url.absoluteString, "https://127.0.0.1:8449")
    }

    func testNormalizerRejectsEmptyInput() {
        XCTAssertThrowsError(try HomeserverAddressNormalizer.normalize("   ")) { error in
            XCTAssertEqual(error as? HomeserverDiscoveryError, .empty)
        }
    }

    func testNormalizerRejectsInsecureScheme() {
        XCTAssertThrowsError(try HomeserverAddressNormalizer.normalize("http://example.org")) { error in
            XCTAssertEqual(error as? HomeserverDiscoveryError, .unsupportedScheme)
        }
    }

    func testCoreDiscoverySignsInAtTheDelegatedHomeserverAndKeepsTheTypedAddress() async throws {
        let resolver = HomeserverResolverSpy(result: .success(resolved("https://matrix-client.example.org")))
        let service = CoreHomeserverDiscoveryService(resolver: resolver)

        let result = try await service.discover(rawAddress: " Example.ORG ")

        XCTAssertEqual(resolver.requestedAddresses, ["https://example.org"])
        XCTAssertEqual(result.requestedURL.absoluteString, "https://example.org")
        XCTAssertEqual(result.homeserverBaseURL.absoluteString, "https://matrix-client.example.org")
        XCTAssertTrue(result.supportsPasswordLogin)
    }

    func testCoreDiscoveryFallsBackToTheTypedHomeserverURL() async throws {
        let resolver = HomeserverResolverSpy(result: .success(resolved("https://matrix.org/matrix")))
        let service = CoreHomeserverDiscoveryService(resolver: resolver)

        let result = try await service.discover(rawAddress: " Matrix.ORG/matrix/ ")

        XCTAssertEqual(resolver.requestedAddresses, ["https://matrix.org/matrix"])
        XCTAssertEqual(result.requestedURL.absoluteString, "https://matrix.org/matrix")
        XCTAssertEqual(result.homeserverBaseURL.absoluteString, "https://matrix.org/matrix")
    }

    func testCoreDiscoveryRefusesAServerWithoutSimplifiedSlidingSync() async {
        let resolver = HomeserverResolverSpy(
            result: .success(resolved("https://conduit.example.org", nativeSlidingSync: false))
        )
        let service = CoreHomeserverDiscoveryService(resolver: resolver)

        await assertDiscoveryError(service, "conduit.example.org", .slidingSyncUnsupported)
        XCTAssertTrue(
            HomeserverDiscoveryError.slidingSyncUnsupported.errorDescription?.contains("MSC4186") == true
        )
    }

    func testCoreDiscoveryExplainsSingleSignOnOnlyServers() async {
        let resolver = HomeserverResolverSpy(
            result: .success(resolved("https://sso.example.org", flows: [ssoFlow, tokenFlow]))
        )
        let service = CoreHomeserverDiscoveryService(resolver: resolver)

        await assertDiscoveryError(service, "sso.example.org", .singleSignOnOnly)
        XCTAssertEqual(
            HomeserverDiscoveryError.singleSignOnOnly.errorDescription,
            "This server signs in with single sign-on, which Synara for iPhone doesn't support yet."
        )
    }

    func testCoreDiscoveryRejectsServerWithoutPasswordFlow() async {
        let resolver = HomeserverResolverSpy(
            result: .success(resolved("https://token.example.org", flows: [tokenFlow]))
        )
        let service = CoreHomeserverDiscoveryService(resolver: resolver)

        await assertDiscoveryError(service, "token.example.org", .unsupportedServer)
        XCTAssertNotEqual(
            HomeserverDiscoveryError.unsupportedServer.errorDescription,
            "This homeserver is not supported yet."
        )
    }

    func testCoreDiscoveryRejectsAnInsecureDelegatedHomeserver() async {
        let resolver = HomeserverResolverSpy(result: .success(resolved("http://matrix.example.org")))
        let service = CoreHomeserverDiscoveryService(resolver: resolver)

        await assertDiscoveryError(service, "example.org", .unsupportedScheme)
    }

    func testCoreDiscoveryMapsResolverFailureToDiscoveryFailed() async {
        let resolver = HomeserverResolverSpy(result: .failure(HomeserverResolverFailure.unavailable))
        let service = CoreHomeserverDiscoveryService(resolver: resolver)

        await assertDiscoveryError(service, "matrix.org", .discoveryFailed)
    }

    func testCoreDiscoveryDoesNotResolveInvalidOrInsecureAddress() async {
        let resolver = HomeserverResolverSpy(result: .success(resolved("https://matrix.org")))
        let service = CoreHomeserverDiscoveryService(resolver: resolver)

        await assertDiscoveryError(service, "https://exa mple.org", .invalidURL)
        await assertDiscoveryError(service, "http://matrix.org", .unsupportedScheme)

        XCTAssertTrue(resolver.requestedAddresses.isEmpty)
    }

    func testMockDiscoveryRecordsRequests() async throws {
        let service = MockHomeserverDiscoveryService()
        let result = try await service.discover(rawAddress: "matrix.org")

        XCTAssertEqual(service.requestedAddresses, ["matrix.org"])
        XCTAssertEqual(result.homeserverBaseURL.absoluteString, "https://matrix.org")
    }

    func testRouterCanRouteToLoginPlaceholder() {
        let router = AppRouter()

        router.route(to: .login(homeserverURL: "https://matrix.org"))

        XCTAssertEqual(router.authPath, [.login(homeserverURL: "https://matrix.org")])
    }

    func testLoginRouteCarriesTheTypedAddressBesideTheResolvedURL() {
        let router = AppRouter()
        let route = AppRoute.login(
            homeserverURL: "https://matrix-client.example.org",
            requestedAddress: "https://example.org"
        )

        router.route(to: route)

        XCTAssertEqual(router.authPath, [route])
        XCTAssertNotEqual(route, .login(homeserverURL: "https://matrix-client.example.org"))
    }

    private func assertDiscoveryError(
        _ service: CoreHomeserverDiscoveryService,
        _ rawAddress: String,
        _ expected: HomeserverDiscoveryError,
        file: StaticString = #filePath,
        line: UInt = #line
    ) async {
        do {
            _ = try await service.discover(rawAddress: rawAddress)
            XCTFail("Expected \(expected).", file: file, line: line)
        } catch let error as HomeserverDiscoveryError {
            XCTAssertEqual(error, expected, file: file, line: line)
        } catch {
            XCTFail("Expected a homeserver discovery error.", file: file, line: line)
        }
    }
}
