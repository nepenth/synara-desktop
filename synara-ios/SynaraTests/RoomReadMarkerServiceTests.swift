import Foundation
import XCTest
@testable import Synara

final class RoomReadMarkerServiceTests: XCTestCase {
    func testReadMarkerReturnsNilWhenSignedOut() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: #"{"event_id":"$event"}"#)
        let service = MatrixRoomReadMarkerService(sessionStore: AppSessionStore(), httpClient: http)

        let eventID = await service.fullyReadEventID(roomID: "!room:matrix.example")

        XCTAssertNil(eventID)
        XCTAssertNil(http.lastRequest)
    }

    func testReadMarkerReadsFullyReadAccountData() async throws {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: #"{"event_id":"$event:matrix.example"}"#)
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(sessionStore: sessionStore, httpClient: http)

        let eventID = await service.fullyReadEventID(roomID: "!room:matrix.example")

        XCTAssertEqual(eventID, "$event:matrix.example")
        let request = try XCTUnwrap(http.lastRequest)
        XCTAssertEqual(request.httpMethod, "GET")
        XCTAssertEqual(request.value(forHTTPHeaderField: "Authorization"), "Bearer token")
        XCTAssertTrue(try XCTUnwrap(request.url?.absoluteString).contains("/_matrix/client/v3/user/"))
        XCTAssertTrue(try XCTUnwrap(request.url?.absoluteString).contains("/account_data/m.fully_read"))
    }

    func testReadMarkerReturnsNilForNonSuccessStatus() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 404, body: #"{"errcode":"M_NOT_FOUND"}"#)
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(sessionStore: sessionStore, httpClient: http)

        let eventID = await service.fullyReadEventID(roomID: "!room:matrix.example")

        XCTAssertNil(eventID)
    }

    func testReadMarkerReturnsNilForMalformedPayload() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: #"{"event_id":42}"#)
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(sessionStore: sessionStore, httpClient: http)

        let eventID = await service.fullyReadEventID(roomID: "!room:matrix.example")

        XCTAssertNil(eventID)
    }

    func testMarkFullyReadReturnsFalseWhenSignedOut() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: #"{"event_id":"$event"}"#)
        let service = MatrixRoomReadMarkerService(sessionStore: AppSessionStore(), httpClient: http)

        let didMark = await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$event:matrix.example")

        XCTAssertFalse(didMark)
        XCTAssertNil(http.lastRequest)
    }

    func testMarkFullyReadWritesAccountData() async throws {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: #"{"event_id":"$event:matrix.example"}"#)
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(sessionStore: sessionStore, httpClient: http)

        let didMark = await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$event:matrix.example")

        XCTAssertTrue(didMark)
        let request = try XCTUnwrap(http.lastRequest)
        XCTAssertEqual(request.httpMethod, "PUT")
        XCTAssertEqual(request.value(forHTTPHeaderField: "Authorization"), "Bearer token")
        XCTAssertEqual(request.value(forHTTPHeaderField: "Content-Type"), "application/json")
        let body = try XCTUnwrap(request.httpBody)
        XCTAssertEqual(String(data: body, encoding: .utf8), #"{"event_id":"$event:matrix.example"}"#)
    }

    func testMarkFullyReadReturnsFalseForNonSuccessStatus() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 500, body: #"{"errcode":"M_UNKNOWN"}"#)
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(sessionStore: sessionStore, httpClient: http)

        let didMark = await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$event:matrix.example")

        XCTAssertFalse(didMark)
    }

    func testMockMarkRoomAsReadUsesLatestEventMarker() async {
        let service = MockRoomReadMarkerService()

        let didMark = await service.markRoomAsRead(roomID: "!room:matrix.example")

        XCTAssertTrue(didMark)
        XCTAssertEqual(service.eventID, "$latest:!room:matrix.example")
    }

    private func makeSession() -> AuthenticatedSession {
        AuthenticatedSession(
            userID: "@test:matrix.example",
            deviceID: "DEVICE",
            homeserverURL: URL(string: "https://matrix.example")!,
            accessToken: "token"
        )
    }
}

private final class RecordingReadMarkerHTTPClient: RoomReadMarkerHTTPClient {
    private let statusCode: Int
    private let body: String
    private(set) var lastRequest: URLRequest?

    init(statusCode: Int, body: String) {
        self.statusCode = statusCode
        self.body = body
    }

    func data(for request: URLRequest) async throws -> (Data, URLResponse) {
        lastRequest = request
        let url = request.url ?? URL(string: "https://matrix.example")!
        let response = HTTPURLResponse(
            url: url,
            statusCode: statusCode,
            httpVersion: nil,
            headerFields: nil
        )!
        return (Data(body.utf8), response)
    }
}
