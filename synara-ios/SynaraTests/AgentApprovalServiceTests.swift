import XCTest
@testable import Synara

final class AgentApprovalServiceTests: XCTestCase {
    func testMatrixAgentApprovalServiceSendsAuthenticatedApprovalEvent() async throws {
        let client = RecordingAgentApprovalHTTPClient(
            data: Data(#"{"event_id":"$approval:matrix.org"}"#.utf8),
            statusCode: 200
        )
        let service = MatrixAgentApprovalService(
            sessionStore: AppSessionStore(currentState: .signedIn(makeSession())),
            httpClient: client
        )
        let action = try SynaraAgentCardAction(
            id: "deploy",
            title: "Deploy",
            kind: "approve",
            prompt: "approve deployment"
        )

        try await service.submit(
            SynaraAgentApprovalRequest(
                roomID: "!room:matrix.org",
                sourceEventID: "$source:matrix.org",
                action: action,
                decision: .approve
            )
        )

        let request = try XCTUnwrap(client.lastRequest)
        XCTAssertEqual(request.httpMethod, "PUT")
        XCTAssertTrue(request.url?.path.contains("/rooms/!room:matrix.org/send/m.room.message/") == true)
        XCTAssertEqual(request.value(forHTTPHeaderField: "Authorization"), "Bearer token")

        let body = try XCTUnwrap(request.httpBody)
        let payload = try JSONSerialization.jsonObject(with: body) as? [String: Any]
        XCTAssertEqual(payload?["msgtype"] as? String, "m.notice")
        XCTAssertEqual(payload?["body"] as? String, "Approved agent action: Deploy")

        let approval = try XCTUnwrap(payload?["in.synara.agent.action"] as? [String: Any])
        XCTAssertEqual(approval["version"] as? Int, 1)
        XCTAssertEqual(approval["action_id"] as? String, "deploy")
        XCTAssertEqual(approval["action_title"] as? String, "Deploy")
        XCTAssertEqual(approval["decision"] as? String, "approve")
        XCTAssertEqual(approval["source_event_id"] as? String, "$source:matrix.org")
        XCTAssertNotNil(approval["created_at"] as? Int)
    }

    func testMatrixAgentApprovalServiceRejectsSignedOutState() async throws {
        let service = MatrixAgentApprovalService(
            sessionStore: AppSessionStore(),
            httpClient: RecordingAgentApprovalHTTPClient()
        )
        let action = try SynaraAgentCardAction(
            id: "reject",
            title: "Reject",
            kind: "reject",
            prompt: "reject"
        )

        await XCTAssertThrowsErrorAsync(
            try await service.submit(
                SynaraAgentApprovalRequest(
                    roomID: "!room:matrix.org",
                    sourceEventID: nil,
                    action: action,
                    decision: .reject
                )
            )
        ) { error in
            XCTAssertEqual(error as? SynaraAgentApprovalError, .signedOut)
        }
    }

    private func makeSession() -> AuthenticatedSession {
        AuthenticatedSession(
            userID: "@alice:matrix.org",
            deviceID: "DEVICE",
            homeserverURL: URL(string: "https://matrix.org")!,
            accessToken: "token"
        )
    }
}

private final class RecordingAgentApprovalHTTPClient: AuthHTTPClient {
    private(set) var lastRequest: URLRequest?
    private let data: Data
    private let statusCode: Int

    init(data: Data = Data(), statusCode: Int = 200) {
        self.data = data
        self.statusCode = statusCode
    }

    func data(for request: URLRequest) async throws -> (Data, URLResponse) {
        lastRequest = request
        let response = HTTPURLResponse(
            url: request.url!,
            statusCode: statusCode,
            httpVersion: nil,
            headerFields: nil
        )!
        return (data, response)
    }
}

private func XCTAssertThrowsErrorAsync(
    _ expression: @autoclosure () async throws -> Void,
    _ errorHandler: (Error) -> Void,
    file: StaticString = #filePath,
    line: UInt = #line
) async {
    do {
        try await expression()
        XCTFail("Expected error", file: file, line: line)
    } catch {
        errorHandler(error)
    }
}
