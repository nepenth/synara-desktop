import XCTest
@testable import Synara

final class AuthServiceTests: XCTestCase {
    func testPlaceholderAuthRejectsMissingUsername() async throws {
        let service = PlaceholderAuthService()
        let request = LoginRequest(
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.org")),
            username: " ",
            password: "password"
        )

        do {
            _ = try await service.login(request)
            XCTFail("Expected missing username error")
        } catch let error as LoginError {
            XCTAssertEqual(error, .missingUsername)
        }
    }

    func testPlaceholderAuthRejectsMissingPassword() async throws {
        let service = PlaceholderAuthService()
        let request = LoginRequest(
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.org")),
            username: "alice",
            password: ""
        )

        do {
            _ = try await service.login(request)
            XCTFail("Expected missing password error")
        } catch let error as LoginError {
            XCTAssertEqual(error, .missingPassword)
        }
    }

    func testPlaceholderAuthReturnsNonPersistentSession() async throws {
        let service = PlaceholderAuthService()
        let homeserverURL = try XCTUnwrap(URL(string: "https://matrix.org"))
        let request = LoginRequest(
            homeserverURL: homeserverURL,
            username: "alice",
            password: "password"
        )

        let session = try await service.login(request)

        XCTAssertEqual(session.userID, "@alice:matrix.org")
        XCTAssertEqual(session.deviceID, "SYNARA-IOS-MOCK")
        XCTAssertEqual(session.homeserverURL, homeserverURL)
    }

    func testBareUsernameUsesTheTypedServerNotTheDelegatedHost() throws {
        let delegated = LoginRequest(
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix-client.example.org")),
            username: " alice ",
            password: "password",
            accountServerName: "example.org"
        )
        XCTAssertEqual(delegated.userID, "@alice:example.org")

        let direct = LoginRequest(
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.example.org:8448")),
            username: "alice",
            password: "password"
        )
        XCTAssertEqual(direct.userID, "@alice:matrix.example.org")

        let fullID = LoginRequest(
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix-client.example.org")),
            username: "@ada:127.0.0.1",
            password: "password",
            accountServerName: "example.org"
        )
        XCTAssertEqual(fullID.userID, "@ada:127.0.0.1")
    }

    func testLoginSendsTheUsernameAsTypedBehindAProxyHost() throws {
        // matrix-v4.example.org proxies a server named matrix.example.org: the
        // app cannot know that name, so a bare username goes out unchanged
        // and the server completes it. The guess only finds a local store.
        let proxied = LoginRequest(
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix-v4.example.org")),
            username: " Pete ",
            password: "password",
            accountServerName: "matrix-v4.example.org"
        )
        XCTAssertEqual(proxied.loginUser, "Pete")
        XCTAssertEqual(proxied.serverName, "matrix-v4.example.org")
        XCTAssertEqual(proxied.userID, "@pete:matrix-v4.example.org")

        let fullID = LoginRequest(
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix-v4.example.org")),
            username: "@pete:matrix.example.org",
            password: "password",
            accountServerName: "matrix-v4.example.org"
        )
        XCTAssertEqual(fullID.loginUser, "@pete:matrix.example.org")
    }

    func testMockAuthRecordsRequestsAndReturnsFixture() async throws {
        let homeserverURL = try XCTUnwrap(URL(string: "https://example.org"))
        let fixture = AuthenticatedSession(
            userID: "@tester:example.org",
            deviceID: "DEVICE",
            homeserverURL: homeserverURL,
            accessToken: "token"
        )
        let service = MockAuthService(result: .success(fixture))
        let request = LoginRequest(
            homeserverURL: homeserverURL,
            username: "tester",
            password: "secret"
        )

        let session = try await service.login(request)

        XCTAssertEqual(session, fixture)
        XCTAssertEqual(service.requests, [request])
    }

    @MainActor
    func testSessionStoreTransitions() throws {
        let homeserverURL = try XCTUnwrap(URL(string: "https://matrix.org"))
        let session = AuthenticatedSession(
            userID: "@alice:matrix.org",
            deviceID: "DEVICE",
            homeserverURL: homeserverURL,
            accessToken: "token"
        )
        let store = AppSessionStore()

        XCTAssertEqual(store.currentState, .signedOut)

        try store.completeLogin(session)
        XCTAssertEqual(store.currentState, .signedIn(session))

        try store.signOut()
        XCTAssertEqual(store.currentState, .signedOut)
    }
}
