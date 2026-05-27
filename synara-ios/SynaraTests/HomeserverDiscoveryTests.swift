import XCTest
@testable import Synara

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
