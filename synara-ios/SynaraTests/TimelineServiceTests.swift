import XCTest
@testable import Synara

final class TimelineServiceTests: XCTestCase {
    func testMatrixTimelineMapsMessagesAndMedia() async throws {
        let client = MockTimelineHTTPClient(responses: [
            .success(
                statusCode: 200,
                body: """
                {
                  "end": "next-token",
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

    func testMatrixTimelineUsesPaginationTokenForOlderMessages() async throws {
        let client = MockTimelineHTTPClient(responses: [
            .success(
                statusCode: 200,
                body: """
                {
                  "end": "page-token",
                  "chunk": [
                    {
                      "event_id": "$new",
                      "sender": "@alice:matrix.org",
                      "origin_server_ts": 2000,
                      "type": "m.room.message",
                      "content": { "msgtype": "m.text", "body": "new" }
                    }
                  ]
                }
                """
            ),
            .success(
                statusCode: 200,
                body: """
                {
                  "end": "older-token",
                  "chunk": [
                    {
                      "event_id": "$old",
                      "sender": "@alice:matrix.org",
                      "origin_server_ts": 1000,
                      "type": "m.room.message",
                      "content": { "msgtype": "m.text", "body": "old" }
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

        _ = await service.loadInitialTimeline(roomID: "!room:matrix.org")
        let older = await service.loadOlderTimeline(roomID: "!room:matrix.org", before: "$new")

        XCTAssertEqual(older.map(\.eventID), ["$old"])
        XCTAssertEqual(
            client.requests.last?.url?.absoluteString,
            "https://matrix.org/_matrix/client/v3/rooms/!room:matrix.org/messages?dir=b&limit=50&from=page-token"
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

    func testMatrixTimelineParsesAgentCardPayloadFromConfiguredContentKeys() async throws {
        let client = MockTimelineHTTPClient(responses: [
            .success(
                statusCode: 200,
                body: """
                {
                  "chunk": [
                    {
                      "event_id": "$agent-card",
                      "sender": "@agent:matrix.org",
                      "origin_server_ts": 1600001000,
                      "type": "m.room.message",
                      "content": {
                        "body": "Build result",
                        "org.hermes.agent": {
                          "title": "Build result",
                          "status": "passed",
                          "summary": "Everything is good.",
                          "actions": [
                            {
                              "id": "continue",
                              "title": "Continue",
                              "prompt": "Continue from last step."
                            }
                          ]
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

        XCTAssertEqual(items.count, 1)
        let item = try XCTUnwrap(items.first)
        guard case .agentCard(let card) = item.kind else {
            XCTFail("Expected parsed agent card kind")
            return
        }
        XCTAssertEqual(card.title, "Build result")
        XCTAssertEqual(card.summary, "Everything is good.")
        XCTAssertEqual(card.status, "passed")
        XCTAssertEqual(card.actions.count, 1)
        XCTAssertEqual(card.actions.first?.id, "continue")
    }

    func testAgentCardPayloadParserReadsHermesJSONMessageBody() throws {
        let body = #"""
        {
          "hermes": true,
          "payload": {
            "title": "Approval required",
            "status": "pending",
            "summary": "Review the proposed action.",
            "actions": [
              {
                "id": "approve",
                "title": "Approve",
                "kind": "approve",
                "prompt": "approve request"
              }
            ]
          }
        }
        """#

        let card = try XCTUnwrap(SynaraAgentCardPayloadParser.parse(body: body))

        XCTAssertEqual(card.title, "Approval required")
        XCTAssertEqual(card.status, "pending")
        XCTAssertEqual(card.actions.first?.id, "approve")
        XCTAssertEqual(card.actions.first?.kind, "approve")
    }

    func testSDKTimelineMergePrefersRawAgentCardFallbackForSameEvent() throws {
        let card = try SynaraAgentCard(
            title: "Approval required",
            status: "pending",
            summary: "Review this action.",
            actions: [
                try SynaraAgentCardAction(
                    id: "approve",
                    title: "Approve",
                    kind: "approve",
                    prompt: "approve request"
                )
            ]
        )
        let sdkItem = TimelineItem(
            id: "$agent",
            eventID: "$agent",
            senderID: "@agent:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("{\"hermes\":true}"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
        let rawAgentItem = TimelineItem(
            id: "$agent",
            eventID: "$agent",
            senderID: "@agent:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .agentCard(card),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )

        let merged = MatrixRustSDKTimelineService.mergedTimelineItems(
            sdkItems: [sdkItem],
            rawAgentItems: [rawAgentItem]
        )

        XCTAssertEqual(merged.count, 1)
        XCTAssertEqual(merged.first?.kind, .agentCard(card))
    }

    func testMapperMapsAgentCardKind() {
        let card = try! SynaraAgentCard(
            title: "Agent summary",
            status: "ok",
            summary: "Plan complete.",
            actions: [
                try! SynaraAgentCardAction(
                    id: "continue",
                    title: "Continue",
                    prompt: "continue"
                )
            ]
        )
        let event = RawTimelineEvent(
            eventID: "$agent:matrix.org",
            senderID: "@agent:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            type: "org.hermes.agent",
            body: nil,
            replyToEventID: nil,
            isEdited: false,
            mediaURL: nil,
            agentCard: card
        )

        if case .agentCard(let mapped) = TimelineMapper.map(event).kind {
            XCTAssertEqual(mapped, card)
        } else {
            XCTFail("Expected agent card mapped kind")
        }
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

    func testEncryptedEventsRenderAsSafePlaceholders() {
        let event = RawTimelineEvent(
            eventID: "$encrypted:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            type: "m.room.encrypted",
            body: nil,
            replyToEventID: nil,
            isEdited: false,
            mediaURL: nil
        )

        XCTAssertEqual(TimelineMapper.map(event).kind, .encryptedPlaceholder)
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

    func testMatrixAccountDataLaterServiceLoadsSortedActiveAndCompletedItems() async throws {
        let expectedNow = 1_800_000_000_000
        let client = MockTimelineHTTPClient(responses: [
            .success(
                statusCode: 200,
                body: #"""
                {
                  "content": {
                    "version": 1,
                    "items": {
                      "!room:example.org\n$event-active-late": {
                        "id": "!room:example.org\n$event-active-late",
                        "kind": "saved",
                        "roomId": "!room:example.org",
                        "eventId": "$event-active-late",
                        "createdAt": 1770000000000,
                        "dueTs": 1800001000000
                      },
                      "!room:example.org\n$event-active-soon": {
                        "id": "!room:example.org\n$event-active-soon",
                        "kind": "reminder",
                        "roomId": "!room:example.org",
                        "eventId": "$event-active-soon",
                        "createdAt": 1790000000000,
                        "dueTs": 1795000000000
                      },
                      "!room:example.org\n$event-completed": {
                        "id": "!room:example.org\n$event-completed",
                        "kind": "reminder",
                        "roomId": "!room:example.org",
                        "eventId": "$event-completed",
                        "createdAt": 1780000000000,
                        "dueTs": 1810000000000,
                        "completedAt": 1798000000000
                      }
                    }
                  }
                }
                """#
            )
        ])

        let service = MatrixAccountDataLaterService(
            sessionStore: AppSessionStore(currentState: .signedIn(try makeSession())),
            httpClient: client,
            now: { expectedNow }
        )

        guard case let .success((items, error)) = await service.loadItems() else {
            XCTFail("Expected success payload")
            return
        }

        XCTAssertNil(error)
        XCTAssertEqual(items.map(\.id), [
            "!room:example.org\n$event-active-soon",
            "!room:example.org\n$event-active-late",
            "!room:example.org\n$event-completed"
        ])
    }

    func testMatrixAccountDataLaterServiceReturnsMalformedPayloadError() async throws {
        let client = MockTimelineHTTPClient(responses: [
            .success(
                statusCode: 200,
                body: #"{"items":{}}"#
            )
        ])
        let service = MatrixAccountDataLaterService(
            sessionStore: AppSessionStore(currentState: .signedIn(try makeSession())),
            httpClient: client
        )

        guard case let .success((items, error)) = await service.loadItems() else {
            XCTFail("Expected success payload")
            return
        }

        XCTAssertEqual(items, [])
        XCTAssertEqual(error, .malformedPayload)
    }

    func testMatrixAccountDataLaterServiceReturnsNoSessionError() async {
        let service = MatrixAccountDataLaterService(sessionStore: AppSessionStore())

        guard case let .success((items, error)) = await service.loadItems() else {
            XCTFail("Expected success payload")
            return
        }

        XCTAssertTrue(items.isEmpty)
        XCTAssertEqual(error, .noSession)
    }

    func testSynaraLaterListSortingPrioritizesActiveItems() {
        let now = 1_760_000_000_000
        let items: SynaraLaterContent
        do {
            items = try SynaraLaterContent(
                version: 1,
                items: [
                    "a": .init(
                        id: "a",
                        kind: .saved,
                        roomId: "!room:example.org",
                        eventId: "$one",
                        createdAt: 5,
                        dueTs: now + 3600_000,
                        completedAt: 9_000
                    ),
                    "b": .init(
                        id: "b",
                        kind: .reminder,
                        roomId: "!room:example.org",
                        eventId: "$two",
                        createdAt: 6,
                        dueTs: now - 10_000,
                        completedAt: nil
                    ),
                    "c": .init(
                        id: "c",
                        kind: .saved,
                        roomId: "!room:example.org",
                        eventId: "$three",
                        createdAt: 7,
                        dueTs: now + 1000,
                        completedAt: nil
                    )
                ]
            )
        } catch {
            XCTFail("Failed fixture: \(error)")
            return
        }

        let sorted = SynaraLaterListItem.sorted(items: items, now: now)

        XCTAssertEqual(sorted.map(\.id), ["b", "c", "a"])
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
