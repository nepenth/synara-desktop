import XCTest
@testable import Synara

final class AuthServiceTests: XCTestCase {
    func testMatrixPasswordAuthRejectsMissingUsernameBeforeNetwork() async throws {
        let client = MockAuthHTTPClient()
        let service = MatrixPasswordAuthService(httpClient: client)
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
            XCTAssertTrue(client.requests.isEmpty)
        }
    }

    func testMatrixPasswordAuthRejectsMissingPasswordBeforeNetwork() async throws {
        let client = MockAuthHTTPClient()
        let service = MatrixPasswordAuthService(httpClient: client)
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
            XCTAssertTrue(client.requests.isEmpty)
        }
    }

    func testMatrixPasswordAuthChecksFlowsAndCreatesSession() async throws {
        let homeserverURL = try XCTUnwrap(URL(string: "https://matrix.org"))
        let client = MockAuthHTTPClient(responses: [
            .success(
                statusCode: 200,
                body: #"{"flows":[{"type":"m.login.password"}]}"#
            ),
            .success(
                statusCode: 200,
                body: #"{"user_id":"@alice:matrix.org","device_id":"DEVICEID","access_token":"token"}"#
            )
        ])
        let service = MatrixPasswordAuthService(httpClient: client)
        let request = LoginRequest(
            homeserverURL: homeserverURL,
            username: "alice",
            password: "secret"
        )

        let session = try await service.login(request)

        XCTAssertEqual(session.userID, "@alice:matrix.org")
        XCTAssertEqual(session.deviceID, "DEVICEID")
        XCTAssertEqual(session.homeserverURL, homeserverURL)
        XCTAssertEqual(session.accessToken, "token")
        XCTAssertEqual(client.requests.map(\.httpMethod), ["GET", "POST"])
        XCTAssertEqual(client.requests.map(\.url?.absoluteString), [
            "https://matrix.org/_matrix/client/v3/login",
            "https://matrix.org/_matrix/client/v3/login"
        ])

        let body = try XCTUnwrap(client.requests.last?.httpBody)
        let payload = try XCTUnwrap(
            JSONSerialization.jsonObject(with: body) as? [String: Any]
        )
        XCTAssertEqual(payload["type"] as? String, "m.login.password")
        XCTAssertEqual(payload["password"] as? String, "secret")
        XCTAssertEqual(payload["initial_device_display_name"] as? String, "Synara iOS")

        let identifier = try XCTUnwrap(payload["identifier"] as? [String: Any])
        XCTAssertEqual(identifier["type"] as? String, "m.id.user")
        XCTAssertEqual(identifier["user"] as? String, "alice")
    }

    func testMatrixPasswordAuthRejectsUnsupportedLoginFlow() async throws {
        let client = MockAuthHTTPClient(responses: [
            .success(
                statusCode: 200,
                body: #"{"flows":[{"type":"m.login.sso"}]}"#
            )
        ])
        let service = MatrixPasswordAuthService(httpClient: client)
        let request = LoginRequest(
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.org")),
            username: "alice",
            password: "secret"
        )

        do {
            _ = try await service.login(request)
            XCTFail("Expected unsupported login error")
        } catch let error as LoginError {
            XCTAssertEqual(error, .unsupported)
            XCTAssertEqual(client.requests.count, 1)
            XCTAssertEqual(client.requests.first?.httpMethod, "GET")
        }
    }

    func testMatrixPasswordAuthMapsForbiddenToInvalidCredentials() async throws {
        let client = MockAuthHTTPClient(responses: [
            .success(
                statusCode: 200,
                body: #"{"flows":[{"type":"m.login.password"}]}"#
            ),
            .success(
                statusCode: 403,
                body: #"{"errcode":"M_FORBIDDEN","error":"Invalid password"}"#
            )
        ])
        let service = MatrixPasswordAuthService(httpClient: client)
        let request = LoginRequest(
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.org")),
            username: "alice",
            password: "wrong"
        )

        do {
            _ = try await service.login(request)
            XCTFail("Expected invalid credentials error")
        } catch let error as LoginError {
            XCTAssertEqual(error, .invalidCredentials)
        }
    }

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

private final class MockAuthHTTPClient: AuthHTTPClient {
    enum Response {
        case success(statusCode: Int, body: String)
        case failure(Error)
    }

    private var responses: [Response]
    private(set) var requests: [URLRequest] = []

    init(responses: [Response] = []) {
        self.responses = responses
    }

    func data(for request: URLRequest) async throws -> (Data, URLResponse) {
        requests.append(request)

        guard responses.isEmpty == false else {
            throw LoginError.networkFailure
        }

        let response = responses.removeFirst()
        switch response {
        case .success(let statusCode, let body):
            let url = try XCTUnwrap(request.url)
            let httpResponse = try XCTUnwrap(
                HTTPURLResponse(
                    url: url,
                    statusCode: statusCode,
                    httpVersion: nil,
                    headerFields: nil
                )
            )
            return (Data(body.utf8), httpResponse)
        case .failure(let error):
            throw error
        }
    }
}
