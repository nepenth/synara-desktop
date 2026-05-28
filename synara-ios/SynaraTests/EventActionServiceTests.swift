import XCTest
@testable import Synara

final class EventActionServiceTests: XCTestCase {
    func testAvailabilityAllowsAuthorToEditAndRedact() {
        let service = MockEventActionService()
        let item = makeItem(senderID: "@alice:matrix.org")

        let availability = service.availability(for: item, currentUserID: "@alice:matrix.org")

        XCTAssertTrue(availability.canReply)
        XCTAssertTrue(availability.canEdit)
        XCTAssertTrue(availability.canRedact)
        XCTAssertTrue(availability.canReact)
    }

    func testRedactedEventsHaveNoActions() {
        let service = MockEventActionService()
        let item = TimelineItem(
            id: "$redacted:matrix.org",
            eventID: "$redacted:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .redacted,
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )

        let availability = service.availability(for: item, currentUserID: "@alice:matrix.org")

        XCTAssertFalse(availability.canReply)
        XCTAssertFalse(availability.canEdit)
        XCTAssertFalse(availability.canRedact)
        XCTAssertFalse(availability.canReact)
    }

    func testEncryptedMediaHasNoActions() throws {
        let service = MockEventActionService()
        let item = TimelineItem(
            id: "$encrypted-media:matrix.org",
            eventID: "$encrypted-media:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .mediaPlaceholder(
                MediaResource(
                    id: "$encrypted-media:matrix.org",
                    filename: "secret.png",
                    authenticatedURL: try XCTUnwrap(URL(string: "mxc://matrix.org/secret")),
                    requiresAuthentication: true,
                    isEncrypted: true
                )
            ),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:],
            isEncrypted: true
        )

        let availability = service.availability(for: item, currentUserID: "@alice:matrix.org")

        XCTAssertFalse(availability.canReply)
        XCTAssertFalse(availability.canEdit)
        XCTAssertFalse(availability.canRedact)
        XCTAssertFalse(availability.canReact)
    }

    func testReactAggregatesWithoutDuplicateLocalEcho() async {
        let service = MockEventActionService()
        let item = makeItem(senderID: "@alice:matrix.org")

        let updated = await service.apply(.react("👍"), to: item, currentUserID: "@bob:matrix.org", roomID: "!room:matrix.org")

        XCTAssertEqual(updated.id, item.id)
        XCTAssertEqual(updated.reactions["👍"], 1)
    }

    func testRedactKeepsStableIdentity() async {
        let service = MockEventActionService()
        let item = makeItem(senderID: "@alice:matrix.org")

        let updated = await service.apply(.redact, to: item, currentUserID: "@alice:matrix.org", roomID: "!room:matrix.org")

        XCTAssertEqual(updated.id, item.id)
        XCTAssertEqual(updated.kind, .redacted)
    }

    func testMatrixRedactCreatesRequestAndLocalUpdate() async throws {
        let client = MockEventActionHTTPClient(responses: [.success(statusCode: 200, body: #"{}"#)])
        let service = MatrixEventActionService(
            sessionStore: AppSessionStore(currentState: .signedIn(try makeSession())),
            httpClient: client
        )
        let item = makeItem(senderID: "@alice:matrix.org")

        let updated = await service.apply(.redact, to: item, currentUserID: "@alice:matrix.org", roomID: "!room:matrix.org")

        XCTAssertEqual(updated.kind, .redacted)
        XCTAssertEqual(client.requests.first?.httpMethod, "PUT")
        XCTAssertEqual(
            client.requests.first?.url?.absoluteString.hasPrefix(
                "https://matrix.org/_matrix/client/v3/rooms/!room:matrix.org/redact/$event:matrix.org/"
            ),
            true
        )
        XCTAssertEqual(client.requests.first?.value(forHTTPHeaderField: "Authorization"), "Bearer token")
        XCTAssertEqual(client.requests.first?.httpBody, Data("{}".utf8))
    }

    func testMatrixReactionCreatesAnnotationRequestAndLocalUpdate() async throws {
        let client = MockEventActionHTTPClient(responses: [.success(statusCode: 200, body: #"{"event_id":"$reaction"}"#)])
        let service = MatrixEventActionService(
            sessionStore: AppSessionStore(currentState: .signedIn(try makeSession())),
            httpClient: client
        )
        let item = makeItem(senderID: "@alice:matrix.org")

        let updated = await service.apply(.react("👍"), to: item, currentUserID: "@bob:matrix.org", roomID: "!room:matrix.org")

        XCTAssertEqual(updated.reactions["👍"], 1)
        XCTAssertEqual(client.requests.first?.httpMethod, "PUT")
        XCTAssertEqual(
            client.requests.first?.url?.absoluteString.hasPrefix(
                "https://matrix.org/_matrix/client/v3/rooms/!room:matrix.org/send/m.reaction/"
            ),
            true
        )

        let body = try XCTUnwrap(client.requests.first?.httpBody)
        let payload = try XCTUnwrap(JSONSerialization.jsonObject(with: body) as? [String: Any])
        let relatesTo = try XCTUnwrap(payload["m.relates_to"] as? [String: Any])
        XCTAssertEqual(relatesTo["rel_type"] as? String, "m.annotation")
        XCTAssertEqual(relatesTo["event_id"] as? String, "$event:matrix.org")
        XCTAssertEqual(relatesTo["key"] as? String, "👍")
    }

    func testMatrixActionFailureKeepsOriginalItem() async throws {
        let client = MockEventActionHTTPClient(responses: [.success(statusCode: 500, body: #"{}"#)])
        let service = MatrixEventActionService(
            sessionStore: AppSessionStore(currentState: .signedIn(try makeSession())),
            httpClient: client
        )
        let item = makeItem(senderID: "@alice:matrix.org")

        let updated = await service.apply(.redact, to: item, currentUserID: "@alice:matrix.org", roomID: "!room:matrix.org")

        XCTAssertEqual(updated, item)
    }

    private func makeItem(senderID: String) -> TimelineItem {
        TimelineItem(
            id: "$event:matrix.org",
            eventID: "$event:matrix.org",
            senderID: senderID,
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Hello"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
    }

    private func makeSession() throws -> AuthenticatedSession {
        AuthenticatedSession(
            userID: "@alice:matrix.org",
            deviceID: "DEVICE",
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.org")),
            accessToken: "token"
        )
    }
}

private final class MockEventActionHTTPClient: AuthHTTPClient {
    enum Response {
        case success(statusCode: Int, body: String)
        case failure(Error)
    }

    private var responses: [Response]
    private(set) var requests: [URLRequest] = []

    init(responses: [Response]) {
        self.responses = responses
    }

    func data(for request: URLRequest) async throws -> (Data, URLResponse) {
        requests.append(request)

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
