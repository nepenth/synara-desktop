import XCTest
import SynaraCore
@testable import Synara

private final class LoginFlowProbeSpy: LoginFlowProbing {
    var result: Result<[LoginFlowDto], Error>
    private(set) var requestedURLs: [URL] = []

    init(result: Result<[LoginFlowDto], Error>) {
        self.result = result
    }

    func loginFlows(homeserverURL: URL) async throws -> [LoginFlowDto] {
        requestedURLs.append(homeserverURL)
        return try result.get()
    }
}

private enum LoginFlowProbeFailure: Error {
    case unavailable
}

final class HomeserverDiscoveryTests: XCTestCase {
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

    func testCoreDiscoveryReturnsNormalizedURLForPasswordFlow() async throws {
        let probe = LoginFlowProbeSpy(
            result: .success([
                LoginFlowDto(kind: "password", matrixType: "m.login.password", getLoginToken: nil)
            ])
        )
        let service = CoreHomeserverDiscoveryService(loginFlowProbe: probe)

        let result = try await service.discover(rawAddress: " Matrix.ORG/matrix/ ")

        XCTAssertEqual(probe.requestedURLs.map(\.absoluteString), ["https://matrix.org/matrix"])
        XCTAssertEqual(result.requestedURL.absoluteString, "https://matrix.org/matrix")
        XCTAssertEqual(result.homeserverBaseURL.absoluteString, "https://matrix.org/matrix")
        XCTAssertTrue(result.supportsPasswordLogin)
    }

    func testCoreDiscoveryRejectsServerWithoutPasswordFlow() async {
        let probe = LoginFlowProbeSpy(
            result: .success([
                LoginFlowDto(kind: "token", matrixType: "m.login.token", getLoginToken: true)
            ])
        )
        let service = CoreHomeserverDiscoveryService(loginFlowProbe: probe)

        do {
            _ = try await service.discover(rawAddress: "matrix.org")
            XCTFail("Expected a homeserver without password login to be rejected.")
        } catch let error as HomeserverDiscoveryError {
            XCTAssertEqual(error, .unsupportedServer)
        } catch {
            XCTFail("Expected a homeserver discovery error.")
        }
    }

    func testCoreDiscoveryMapsProbeFailureToDiscoveryFailed() async {
        let probe = LoginFlowProbeSpy(result: .failure(LoginFlowProbeFailure.unavailable))
        let service = CoreHomeserverDiscoveryService(loginFlowProbe: probe)

        do {
            _ = try await service.discover(rawAddress: "matrix.org")
            XCTFail("Expected a probe failure to be mapped.")
        } catch let error as HomeserverDiscoveryError {
            XCTAssertEqual(error, .discoveryFailed)
        } catch {
            XCTFail("Expected a homeserver discovery error.")
        }
    }

    func testCoreDiscoveryDoesNotProbeInvalidOrInsecureAddress() async {
        let probe = LoginFlowProbeSpy(result: .success([]))
        let service = CoreHomeserverDiscoveryService(loginFlowProbe: probe)

        for (rawAddress, expectedError) in [
            ("https://exa mple.org", HomeserverDiscoveryError.invalidURL),
            ("http://matrix.org", HomeserverDiscoveryError.unsupportedScheme),
        ] {
            do {
                _ = try await service.discover(rawAddress: rawAddress)
                XCTFail("Expected address normalization to fail.")
            } catch let error as HomeserverDiscoveryError {
                XCTAssertEqual(error, expectedError)
            } catch {
                XCTFail("Expected a homeserver discovery error.")
            }
        }

        XCTAssertTrue(probe.requestedURLs.isEmpty)
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
}
