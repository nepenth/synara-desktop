import XCTest
@testable import Synara

final class RoomListServiceTests: XCTestCase {
    func testMatrixRoomListReturnsEmptyWhenSignedOut() async {
        let client = MockRoomListHTTPClient()
        let service = MatrixRoomListService(
            sessionStore: AppSessionStore(),
            httpClient: client
        )

        let state = await service.loadRooms()

        XCTAssertEqual(state, .empty)
        XCTAssertTrue(client.requests.isEmpty)
    }

    func testMatrixRoomListMapsJoinedAndInvitedRooms() async throws {
        let homeserverURL = try XCTUnwrap(URL(string: "https://matrix.org"))
        let session = AuthenticatedSession(
            userID: "@alice:matrix.org",
            deviceID: "DEVICE",
            homeserverURL: homeserverURL,
            accessToken: "token"
        )
        let client = MockRoomListHTTPClient(responses: [
            .success(
                statusCode: 200,
                body: """
                {
                  "rooms": {
                    "join": {
                      "!joined:matrix.org": {
                        "state": {
                          "events": [
                            {
                              "type": "m.room.name",
                              "origin_server_ts": 1000,
                              "content": { "name": "Joined Room" }
                            }
                          ]
                        },
                        "timeline": {
                          "events": [
                            {
                              "type": "m.room.message",
                              "origin_server_ts": 2000,
                              "content": { "body": "Hello there" }
                            }
                          ]
                        },
                        "unread_notifications": {
                          "notification_count": 2,
                          "highlight_count": 0
                        }
                      }
                    },
                    "invite": {
                      "!invited:matrix.org": {
                        "invite_state": {
                          "events": [
                            {
                              "type": "m.room.name",
                              "origin_server_ts": 3000,
                              "content": { "name": "Invited Room" }
                            }
                          ]
                        }
                      }
                    }
                  }
                }
                """
            )
        ])
        let service = MatrixRoomListService(
            sessionStore: AppSessionStore(currentState: .signedIn(session)),
            httpClient: client
        )

        let state = await service.loadRooms()

        guard case .loaded(let rooms) = state else {
            XCTFail("Expected loaded rooms")
            return
        }

        XCTAssertEqual(rooms.map(\.id), ["!invited:matrix.org", "!joined:matrix.org"])
        XCTAssertEqual(rooms[0].name, "Invited Room")
        XCTAssertEqual(rooms[0].lastMessagePreview, "Invited to room")
        XCTAssertEqual(rooms[0].membership, .invited)
        XCTAssertTrue(rooms[0].hasHighlight)
        XCTAssertEqual(rooms[1].name, "Joined Room")
        XCTAssertEqual(rooms[1].lastMessagePreview, "Hello there")
        XCTAssertEqual(rooms[1].membership, .joined)
        XCTAssertEqual(rooms[1].unreadCount, 2)
        XCTAssertEqual(client.requests.first?.httpMethod, "GET")
        XCTAssertEqual(
            client.requests.first?.url?.absoluteString,
            "https://matrix.org/_matrix/client/v3/sync?timeout=0"
        )
        XCTAssertEqual(client.requests.first?.value(forHTTPHeaderField: "Authorization"), "Bearer token")
    }

    func testMatrixRoomListMapsHTTPFailureToFailedState() async throws {
        let session = AuthenticatedSession(
            userID: "@alice:matrix.org",
            deviceID: "DEVICE",
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.org")),
            accessToken: "token"
        )
        let client = MockRoomListHTTPClient(responses: [
            .success(statusCode: 401, body: #"{"errcode":"M_UNKNOWN_TOKEN"}"#)
        ])
        let service = MatrixRoomListService(
            sessionStore: AppSessionStore(currentState: .signedIn(session)),
            httpClient: client
        )

        let state = await service.loadRooms()

        XCTAssertEqual(state, .failed("Could not load rooms. Try again."))
    }

    func testMatrixMembershipAcceptsInvite() async throws {
        let session = try makeSession()
        let client = MockRoomListHTTPClient(responses: [
            .success(statusCode: 200, body: #"{"room_id":"!room:matrix.org"}"#)
        ])
        let service = MatrixRoomMembershipService(
            sessionStore: AppSessionStore(currentState: .signedIn(session)),
            httpClient: client
        )

        try await service.acceptInvite(roomID: "!room:matrix.org")

        XCTAssertEqual(client.requests.first?.httpMethod, "POST")
        XCTAssertEqual(
            client.requests.first?.url?.absoluteString,
            "https://matrix.org/_matrix/client/v3/rooms/!room:matrix.org/join"
        )
        XCTAssertEqual(client.requests.first?.value(forHTTPHeaderField: "Authorization"), "Bearer token")
        XCTAssertEqual(client.requests.first?.httpBody, Data("{}".utf8))
    }

    func testMatrixMembershipRejectsInvite() async throws {
        let session = try makeSession()
        let client = MockRoomListHTTPClient(responses: [
            .success(statusCode: 200, body: #"{}"#)
        ])
        let service = MatrixRoomMembershipService(
            sessionStore: AppSessionStore(currentState: .signedIn(session)),
            httpClient: client
        )

        try await service.rejectInvite(roomID: "!room:matrix.org")

        XCTAssertEqual(
            client.requests.first?.url?.absoluteString,
            "https://matrix.org/_matrix/client/v3/rooms/!room:matrix.org/leave"
        )
    }

    func testMatrixMembershipFailsWhenSignedOut() async throws {
        let client = MockRoomListHTTPClient()
        let service = MatrixRoomMembershipService(
            sessionStore: AppSessionStore(),
            httpClient: client
        )

        do {
            try await service.acceptInvite(roomID: "!room:matrix.org")
            XCTFail("Expected signed-out error")
        } catch let error as RoomMembershipError {
            XCTAssertEqual(error, .signedOut)
            XCTAssertTrue(client.requests.isEmpty)
        }
    }

    func testRoomsSortByHighlightUnreadThenActivity() {
        let rooms = RoomListFixtures.small().reversed()

        let sorted = RoomListFixtures.sorted(Array(rooms))

        XCTAssertEqual(sorted.first?.id, "!project:matrix.org")
    }

    func testLargeFixtureHasStableIdentifiers() {
        let rooms = RoomListFixtures.large()

        XCTAssertEqual(rooms.count, 1_000)
        XCTAssertEqual(Set(rooms.map(\.id)).count, 1_000)
    }

    func testMockRoomListReturnsSortedLoadedState() async {
        let rooms = RoomListFixtures.small().reversed()
        let service = MockRoomListService(state: .loaded(Array(rooms)))

        let state = await service.loadRooms()

        XCTAssertEqual(state, .loaded(RoomListFixtures.sorted(RoomListFixtures.small())))
        XCTAssertEqual(service.loadCallCount, 1)
    }

    func testClearCacheReturnsEmptyState() async {
        let service = MockRoomListService()

        service.clearCache()
        let state = await service.loadRooms()

        XCTAssertEqual(state, .empty)
        XCTAssertEqual(service.clearCallCount, 1)
    }

    func testMockInviteTransitionAcceptsInviteIntoJoinedRoom() async throws {
        let service = MockInviteTransitionService()

        try await service.acceptInvite(roomID: "!alerts:matrix.org")
        let state = await service.loadRooms()

        guard case .loaded(let rooms) = state else {
            XCTFail("Expected loaded rooms")
            return
        }

        XCTAssertEqual(rooms.first?.id, "!alerts:matrix.org")
        XCTAssertEqual(rooms.first?.membership, .joined)
        XCTAssertEqual(rooms.first?.lastMessagePreview, "Joined room")
    }

    func testMockInviteTransitionRejectsInviteIntoEmptyState() async throws {
        let service = MockInviteTransitionService()

        try await service.rejectInvite(roomID: "!alerts:matrix.org")
        let state = await service.loadRooms()

        XCTAssertEqual(state, .empty)
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

private final class MockRoomListHTTPClient: AuthHTTPClient {
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
