import XCTest
@testable import Synara

final class ComposerServiceTests: XCTestCase {
    func testMatrixSendCreatesRequestAndTimelineItem() async throws {
        let client = MockComposerHTTPClient(responses: [
            .success(statusCode: 200, body: #"{"event_id":"$sent"}"#)
        ])
        let service = MatrixMessageSendService(
            sessionStore: AppSessionStore(currentState: .signedIn(try makeSession())),
            httpClient: client
        )
        let request = MessageSendRequest(
            roomID: "!room:matrix.org",
            body: " hello ",
            replyToEventID: "$parent",
            editEventID: nil
        )

        let item = try await service.send(request)

        XCTAssertEqual(item.eventID, "$sent")
        XCTAssertEqual(item.senderID, "@alice:matrix.org")
        XCTAssertEqual(item.kind, .text("hello"))
        XCTAssertEqual(item.replyToEventID, "$parent")
        XCTAssertEqual(client.requests.first?.httpMethod, "PUT")
        XCTAssertEqual(
            client.requests.first?.url?.absoluteString.hasPrefix(
                "https://matrix.org/_matrix/client/v3/rooms/!room:matrix.org/send/m.room.message/"
            ),
            true
        )
        XCTAssertEqual(client.requests.first?.value(forHTTPHeaderField: "Authorization"), "Bearer token")

        let body = try XCTUnwrap(client.requests.first?.httpBody)
        let payload = try XCTUnwrap(JSONSerialization.jsonObject(with: body) as? [String: Any])
        XCTAssertEqual(payload["msgtype"] as? String, "m.text")
        XCTAssertEqual(payload["body"] as? String, "hello")

        let relatesTo = try XCTUnwrap(payload["m.relates_to"] as? [String: Any])
        let inReplyTo = try XCTUnwrap(relatesTo["m.in_reply_to"] as? [String: Any])
        XCTAssertEqual(inReplyTo["event_id"] as? String, "$parent")
    }

    func testMatrixSendRejectsEmptyBeforeNetwork() async throws {
        let client = MockComposerHTTPClient()
        let service = MatrixMessageSendService(
            sessionStore: AppSessionStore(currentState: .signedIn(try makeSession())),
            httpClient: client
        )
        let request = MessageSendRequest(
            roomID: "!room:matrix.org",
            body: " ",
            replyToEventID: nil,
            editEventID: nil
        )

        do {
            _ = try await service.send(request)
            XCTFail("Expected empty message error")
        } catch let error as MessageSendError {
            XCTAssertEqual(error, .emptyMessage)
            XCTAssertTrue(client.requests.isEmpty)
        }
    }

    func testMatrixSendCreatesEditRelation() async throws {
        let client = MockComposerHTTPClient(responses: [
            .success(statusCode: 200, body: #"{"event_id":"$edit"}"#)
        ])
        let service = MatrixMessageSendService(
            sessionStore: AppSessionStore(currentState: .signedIn(try makeSession())),
            httpClient: client
        )
        let request = MessageSendRequest(
            roomID: "!room:matrix.org",
            body: " updated ",
            replyToEventID: "$ignored-parent",
            editEventID: "$original"
        )

        let item = try await service.send(request)

        XCTAssertEqual(item.eventID, "$edit")
        XCTAssertEqual(item.kind, .text("updated"))
        XCTAssertTrue(item.isEdited)

        let body = try XCTUnwrap(client.requests.first?.httpBody)
        let payload = try XCTUnwrap(JSONSerialization.jsonObject(with: body) as? [String: Any])
        XCTAssertEqual(payload["msgtype"] as? String, "m.text")
        XCTAssertEqual(payload["body"] as? String, "* updated")

        let newContent = try XCTUnwrap(payload["m.new_content"] as? [String: Any])
        XCTAssertEqual(newContent["msgtype"] as? String, "m.text")
        XCTAssertEqual(newContent["body"] as? String, "updated")

        let relatesTo = try XCTUnwrap(payload["m.relates_to"] as? [String: Any])
        XCTAssertEqual(relatesTo["rel_type"] as? String, "m.replace")
        XCTAssertEqual(relatesTo["event_id"] as? String, "$original")
        XCTAssertNil(relatesTo["m.in_reply_to"])
    }

    func testDraftStorePreservesDraftByRoom() {
        let store = DraftStore()

        store.setDraft("hello", roomID: "!room:matrix.org")

        XCTAssertEqual(store.draft(roomID: "!room:matrix.org"), "hello")
        XCTAssertEqual(store.draft(roomID: "!other:matrix.org"), "")
    }

    func testSendRejectsWhitespaceOnlyMessage() async throws {
        let service = MockMessageSendService()
        let request = MessageSendRequest(
            roomID: "!room:matrix.org",
            body: "   ",
            replyToEventID: nil,
            editEventID: nil
        )

        do {
            _ = try await service.send(request)
            XCTFail("Expected empty message error")
        } catch let error as MessageSendError {
            XCTAssertEqual(error, .emptyMessage)
        }
    }

    func testSendCreatesLocalEchoWithReplyMetadata() async throws {
        let service = MockMessageSendService()
        let request = MessageSendRequest(
            roomID: "!room:matrix.org",
            body: " reply body ",
            replyToEventID: "$parent:matrix.org",
            editEventID: nil
        )

        let item = try await service.send(request)

        XCTAssertEqual(item.kind, .text("reply body"))
        XCTAssertEqual(item.replyToEventID, "$parent:matrix.org")
        XCTAssertFalse(item.isEdited)
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

private final class MockComposerHTTPClient: AuthHTTPClient {
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
