import XCTest
@testable import Synara

final class PushServiceTests: XCTestCase {
    func testRouteFromPayloadUsesRoomIdAndEventId() {
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: StubPusherService()
        )

        let route = service.route(from: [
            "room_id": "!room:matrix.org",
            "event_id": "$event1:matrix.org"
        ])

        assertRoute(route, matchesRoom: "!room:matrix.org", eventID: "$event1:matrix.org")
    }

    func testRouteFromPayloadRouteStringParsesCustomScheme() {
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: StubPusherService()
        )

        let route = service.route(from: [
            "route": "synara://route/%2Froom%2F!room%3Amatrix.org%2F%24event2%3Amatrix.org"
        ])

        assertRoute(route, matchesRoom: "!room:matrix.org", eventID: "$event2:matrix.org")
    }

    func testRouteFromPayloadParsesUniversalLinkStyleRoute() {
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: StubPusherService()
        )

        let route = service.route(from: [
            "route": "https://synara.app/r/%2Froom%2F%21room%3Amatrix.org%2F%24event3%3Amatrix.org"
        ])

        assertRoute(route, matchesRoom: "!room:matrix.org", eventID: "$event3:matrix.org")
    }

    func testRouteFromPayloadFallsBackToSettingsForUnknownRoute() {
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: StubPusherService()
        )

        let route = service.route(from: ["route": "https://synara.app/r/invalid"])

        XCTAssertNil(route)
    }

    func testRouteFromPayloadSupportsNotificationAndLaterRoutes() {
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: StubPusherService()
        )

        let notifications = service.route(from: ["route": "synara://notifications"])
        XCTAssertEqual(notifications, .notifications)

        let later = service.route(from: ["route": "https://synara.app/r/%2Finbox%2Flater"])
        XCTAssertEqual(later, .later)
    }

    func testBadgeCountParsesApsBadgeAndSummaryFormats() {
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: StubPusherService()
        )

        XCTAssertEqual(service.parseBadgeCount(from: ["aps": ["badge": 3]]), 3)

        let withNumericSummary = service.parseBadgeCount(from: [
            "notification_summary": [
                "appBadgeCount": 7
            ]
        ])
        XCTAssertEqual(withNumericSummary, 7)

        let withStringSummary = service.parseBadgeCount(from: [
            "synara": [
                "notification_summary": [
                    "appBadgeCount": "9"
                ]
            ]
        ])
        XCTAssertEqual(withStringSummary, 9)
    }

    func testPushServiceRegistersAfterSessionAndToken() async {
        let expectation = expectation(description: "register pusher")
        let pusher = StubPusherService()
        pusher.onRegister = {
            expectation.fulfill()
        }

        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        service.configure(with: makeSession())
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))

        await waitForExpectations(timeout: 1)
        XCTAssertTrue(service.isRegistered)
        XCTAssertEqual(pusher.registerCount, 1)
        XCTAssertEqual(pusher.lastPushKey, "7ab13c")
    }

    func testPushServiceClearsRegistrationAndUnregistersOnLogout() async {
        let expectation = expectation(description: "unregister pusher")
        let pusher = StubPusherService()
        pusher.onUnregister = {
            expectation.fulfill()
        }

        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        service.configure(with: makeSession())
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))
        await waitForExpectations(timeout: 1)

        service.clearRegistrationState()

        XCTAssertEqual(service.tokenSnippet, nil)
        XCTAssertFalse(service.isRegistered)
        XCTAssertEqual(pusher.unregisterCount, 1)
    }

    func testPushServiceReplacesRegistrationOnTokenRotation() async {
        let pusher = StubPusherService()
        let registerExpectation = expectation(description: "register pusher twice")
        registerExpectation.expectedFulfillmentCount = 2
        pusher.onRegister = {
            registerExpectation.fulfill()
        }

        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        service.configure(with: makeSession())
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))
        await waitForExpectations(timeout: 1)
        service.handleDeviceToken(Data([0xAA, 0x55, 0x00]))
        await waitForExpectations(timeout: 1)

        XCTAssertEqual(pusher.registerCount, 2)
        XCTAssertEqual(pusher.unregisterCount, 1)
        XCTAssertEqual(pusher.lastUnregisterPushKey, "7ab13c")
        XCTAssertEqual(service.tokenSnippet, "aa5500")
    }

    func testPushServiceDoesNotRegisterWithoutGateway() async {
        let pusher = StubPusherService(isGatewayConfigured: false)

        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )

        service.configure(with: makeSession())
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))

        await Task.yield()

        XCTAssertEqual(pusher.registerCount, 0)
        XCTAssertFalse(service.isRegistered)
        XCTAssertEqual(service.registrationStateDescription, "Push gateway not configured")
    }

    func testPusherServiceSendsExpectedPushGatewayPayload() async throws {
        RecordingURLProtocol.capturedRequest = nil
        let session = URLSession(configuration: recordingSessionConfiguration())
        let gateway = URL(string: "https://push.example.internal")!
        let service = MatrixPusherService(gatewayURL: gateway, session: session)

        let authSession = makeSession()
        try await service.registerPusher(session: authSession, pushKey: "7ab13c")

        let captured = try XCTUnwrap(RecordingURLProtocol.capturedRequest)
        XCTAssertEqual(captured.httpMethod, "POST")
        XCTAssertTrue(captured.url?.path.contains("/pushers/set") == true)
        XCTAssertEqual(captured.url?.query?.contains("access_token=token"), true)

        let body = try XCTUnwrap(captured.httpBody)
        let payload = try JSONSerialization.jsonObject(with: body) as? [String: Any]
        XCTAssertEqual(payload?["app_id"] as? String, "app.synara.ios")
        XCTAssertEqual(payload?["pushkey"] as? String, "7ab13c")

        let data = try XCTUnwrap(payload?["data"] as? [String: Any])
        XCTAssertEqual(data["url"] as? String, "https://push.example.internal")
        XCTAssertEqual(data["format"] as? String, "event_id_only")
    }

    func testPusherServiceUnregisterUsesDeleteEndpoint() async throws {
        RecordingURLProtocol.capturedRequest = nil
        let session = URLSession(configuration: recordingSessionConfiguration())
        let gateway = URL(string: "https://push.example.internal")!
        let service = MatrixPusherService(gatewayURL: gateway, session: session)

        let authSession = makeSession()
        try await service.unregisterPusher(session: authSession, pushKey: "7ab13c")

        let captured = try XCTUnwrap(RecordingURLProtocol.capturedRequest)
        XCTAssertEqual(captured.httpMethod, "POST")
        XCTAssertTrue(captured.url?.path.contains("/pushers/delete") == true)
    }

    private func makeSession() -> AuthenticatedSession {
        AuthenticatedSession(
            userID: "@alice:matrix.org",
            deviceID: "DEVICE",
            homeserverURL: URL(string: "https://matrix.org")!,
            accessToken: "token"
        )
    }

    private func assertRoute(
        _ route: AppRoute?,
        matchesRoom roomID: String,
        eventID: String
    ) {
        guard case .room(let id, let parsedEventID, _) = route else {
            XCTFail("Expected room route")
            return
        }

        XCTAssertEqual(id, roomID)
        XCTAssertEqual(parsedEventID, eventID)
    }
}

private final class StubPusherService: MatrixPusherServicing {
        var isGatewayConfigured: Bool
        private(set) var registerCount = 0
        private(set) var unregisterCount = 0
        private(set) var lastPushKey: String?
        private(set) var lastUnregisterPushKey: String?
        var onRegister: () -> Void = {}
        var onUnregister: () -> Void = {}

        init(isGatewayConfigured: Bool = true) {
            self.isGatewayConfigured = isGatewayConfigured
        }

    func registerPusher(session: AuthenticatedSession, pushKey: String) async throws {
        registerCount += 1
        lastPushKey = pushKey
        onRegister()
    }

    func unregisterPusher(session: AuthenticatedSession, pushKey: String) async throws {
        unregisterCount += 1
        lastUnregisterPushKey = pushKey
        onUnregister()
    }
}

private func recordingSessionConfiguration() -> URLSessionConfiguration {
    let config = URLSessionConfiguration.ephemeral
    config.protocolClasses = [RecordingURLProtocol.self]
    return config
}

private final class RecordingURLProtocol: URLProtocol {
    static var capturedRequest: URLRequest?

    override class func canInit(with request: URLRequest) -> Bool {
        true
    }

    override class func canonicalRequest(for request: URLRequest) -> URLRequest {
        request
    }

    override func startLoading() {
        Self.capturedRequest = request
        guard let url = request.url else {
            client?.urlProtocol(self, didFailWithError: URLError(.badURL))
            return
        }

        guard let response = HTTPURLResponse(
            url: url,
            statusCode: 200,
            httpVersion: "HTTP/1.1",
            headerFields: nil
        ) else {
            client?.urlProtocol(self, didFailWithError: URLError(.badServerResponse))
            return
        }

        client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
        client?.urlProtocol(self, didLoad: Data())
        client?.urlProtocolDidFinishLoading(self)
    }

    override func stopLoading() {}
}
