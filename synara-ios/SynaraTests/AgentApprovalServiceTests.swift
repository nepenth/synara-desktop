import XCTest
@testable import Synara

final class AgentApprovalServiceTests: XCTestCase {
    func testAgentApprovalMatrixEventMatchesSharedContractFixture() throws {
        let action = try SynaraAgentCardAction(
            id: "deploy",
            title: "Deploy",
            kind: "approve",
            prompt: "approve deployment"
        )
        let request = SynaraAgentApprovalRequest(
            roomID: "!room:matrix.org",
            sourceEventID: "$source:example.org",
            action: action,
            decision: .approve
        )

        let data = try encodeAgentApprovalMatrixEvent(
            request,
            createdAt: 1_770_000_000_000
        )
        let payload = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
        let approval = try XCTUnwrap(payload["in.synara.agent.action"] as? [String: Any])
        let fixture = try loadSharedAgentApprovalFixture(name: "approve")

        XCTAssertEqual(payload["msgtype"] as? String, "m.notice")
        XCTAssertEqual(payload["body"] as? String, "Approved agent action: Deploy")
        XCTAssertEqual(approval as NSDictionary, fixture as NSDictionary)
    }

    func testMockAgentApprovalServiceRejectsConfiguredFailure() async throws {
        let service = MockAgentApprovalService(error: .signedOut)
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

    private func loadSharedAgentApprovalFixture(name: String) throws -> [String: Any] {
        let testFile = URL(fileURLWithPath: #filePath)
        let repositoryRoot = testFile
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let fixtureURL = repositoryRoot
            .appendingPathComponent("synara")
            .appendingPathComponent("docs")
            .appendingPathComponent("contracts")
            .appendingPathComponent("fixtures")
            .appendingPathComponent("synara-agent-approval-action.json")
        let data = try Data(contentsOf: fixtureURL)
        let fixtures = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
        let valid = try XCTUnwrap(fixtures["valid"] as? [String: Any])
        return try XCTUnwrap(valid[name] as? [String: Any])
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
