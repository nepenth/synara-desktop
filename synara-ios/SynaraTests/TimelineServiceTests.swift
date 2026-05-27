import XCTest
@testable import Synara

final class TimelineServiceTests: XCTestCase {
    func testMatrixTimelineMapsMessagesAndMedia() async throws {
        let client = MockTimelineHTTPClient(responses: [
            .success(
                statusCode: 200,
                body: """
                {
                  "chunk": [
                    {
                      "event_id": "$new",
                      "sender": "@bob:matrix.org",
                      "origin_server_ts": 2000,
                      "type": "m.room.message",
                      "content": {
                        "msgtype": "m.image",
                        "body": "photo.jpg",
                        "url": "mxc://matrix.org/media"
                      }
                    },
                    {
                      "event_id": "$old",
                      "sender": "@alice:matrix.org",
                      "origin_server_ts": 1000,
                      "type": "m.room.message",
                      "content": {
                        "msgtype": "m.text",
                        "body": "hello",
                        "m.relates_to": {
                          "m.in_reply_to": {
                            "event_id": "$parent"
                          }
                        }
                      }
                    }
                  ]
                }
                """
            )
        ])
        let service = MatrixTimelineService(
            sessionStore: AppSessionStore(currentState: .signedIn(try makeSession())),
            httpClient: client
        )

        let items = await service.loadInitialTimeline(roomID: "!room:matrix.org")

        XCTAssertEqual(items.map(\.eventID), ["$old", "$new"])
        XCTAssertEqual(items[0].kind, .text("hello"))
        XCTAssertEqual(items[0].replyToEventID, "$parent")
        guard case .mediaPlaceholder(let resource) = items[1].kind else {
            XCTFail("Expected media placeholder")
            return
        }
        XCTAssertEqual(resource.filename, "photo.jpg")
        XCTAssertEqual(client.requests.first?.httpMethod, "GET")
        XCTAssertEqual(
            client.requests.first?.url?.absoluteString,
            "https://matrix.org/_matrix/client/v3/rooms/!room:matrix.org/messages?dir=b&limit=50"
        )
    }

    func testMatrixTimelineReturnsEmptyWhenSignedOut() async {
        let client = MockTimelineHTTPClient()
        let service = MatrixTimelineService(
            sessionStore: AppSessionStore(),
            httpClient: client
        )

        let items = await service.loadInitialTimeline(roomID: "!room:matrix.org")

        XCTAssertTrue(items.isEmpty)
        XCTAssertTrue(client.requests.isEmpty)
    }

    func testMatrixTimelineHidesRoomStateEventsButKeepsCustomUnknowns() async throws {
        let client = MockTimelineHTTPClient(responses: [
            .success(
                statusCode: 200,
                body: """
                {
                  "chunk": [
                    {
                      "event_id": "$create",
                      "sender": "@alice:matrix.org",
                      "origin_server_ts": 1000,
                      "type": "m.room.create",
                      "content": {}
                    },
                    {
                      "event_id": "$agent",
                      "sender": "@agent:matrix.org",
                      "origin_server_ts": 2000,
                      "type": "synara.agent.card",
                      "content": {}
                    },
                    {
                      "event_id": "$text",
                      "sender": "@alice:matrix.org",
                      "origin_server_ts": 3000,
                      "type": "m.room.message",
                      "content": {
                        "msgtype": "m.text",
                        "body": "visible"
                      }
                    }
                  ]
                }
                """
            )
        ])
        let service = MatrixTimelineService(
            sessionStore: AppSessionStore(currentState: .signedIn(try makeSession())),
            httpClient: client
        )

        let items = await service.loadInitialTimeline(roomID: "!room:matrix.org")

        XCTAssertEqual(items.map(\.eventID), ["$text", "$agent"])
        XCTAssertEqual(items[0].kind, .text("visible"))
        XCTAssertEqual(items[1].kind, .unknown(type: "synara.agent.card"))
    }

    func testMapperKeepsStableIdentityAndMetadata() {
        let event = RawTimelineEvent(
            eventID: "$event:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            type: "m.room.message",
            body: "Hello",
            replyToEventID: "$parent:matrix.org",
            isEdited: true,
            mediaURL: nil
        )

        let item = TimelineMapper.map(event)

        XCTAssertEqual(item.id, "$event:matrix.org")
        XCTAssertEqual(item.eventID, "$event:matrix.org")
        XCTAssertEqual(item.senderID, "@alice:matrix.org")
        XCTAssertEqual(item.kind, .text("Hello"))
        XCTAssertEqual(item.replyToEventID, "$parent:matrix.org")
        XCTAssertTrue(item.isEdited)
    }

    func testUnknownEventsRenderAsSafePlaceholders() {
        let event = RawTimelineEvent(
            eventID: "$unknown:matrix.org",
            senderID: "@agent:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            type: "synara.agent.card",
            body: nil,
            replyToEventID: nil,
            isEdited: false,
            mediaURL: nil
        )

        XCTAssertEqual(TimelineMapper.map(event).kind, .unknown(type: "synara.agent.card"))
    }

    func testMediaEventsUseSafeResourceDescription() throws {
        let mediaURL = try XCTUnwrap(URL(string: "mxc://matrix.org/private-media-id"))
        let event = RawTimelineEvent(
            eventID: "$media:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            type: "m.room.media",
            body: "photo.jpg",
            replyToEventID: nil,
            isEdited: false,
            mediaURL: mediaURL
        )

        let item = TimelineMapper.map(event)

        guard case .mediaPlaceholder(let resource) = item.kind else {
            XCTFail("Expected media placeholder")
            return
        }
        XCTAssertEqual(resource.safeDescription, "photo.jpg")
        XCTAssertFalse(resource.safeDescription.contains("matrix.org"))
        XCTAssertTrue(resource.requiresAuthentication)
    }

    func testMockTimelineCanLoadInitialAndOlderEvents() async {
        let service = MockTimelineService()

        let initial = await service.loadInitialTimeline(roomID: "!room:matrix.org")
        let older = await service.loadOlderTimeline(roomID: "!room:matrix.org", before: initial[0].eventID)

        XCTAssertEqual(initial.count, 4)
        XCTAssertEqual(older.count, 3)
    }

    func testLargeTimelineFixtureHasStableIdentity() {
        let items = TimelineFixtures.largeTimeline()

        XCTAssertEqual(items.count, 10_000)
        XCTAssertEqual(Set(items.map(\.id)).count, 10_000)
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

private final class MockTimelineHTTPClient: AuthHTTPClient {
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
