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

    func testAgentApprovalReactionMatrixEventUsesMatrixReactionRelation() throws {
        let data = try encodeAgentApprovalReactionMatrixEvent(
            SynaraAgentApprovalReactionRequest(
                roomID: "!room:matrix.org",
                sourceEventID: "$approval:matrix.org",
                reactionKey: "✅"
            )
        )
        let payload = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
        let relation = try XCTUnwrap(payload["m.relates_to"] as? [String: Any])

        XCTAssertEqual(relation["rel_type"] as? String, "m.annotation")
        XCTAssertEqual(relation["event_id"] as? String, "$approval:matrix.org")
        XCTAssertEqual(relation["key"] as? String, "✅")
    }

    func testMockAgentApprovalReactionServiceRecordsReactionRequests() async throws {
        let service = MockAgentApprovalReactionService()
        let request = SynaraAgentApprovalReactionRequest(
            roomID: "!room:matrix.org",
            sourceEventID: "$approval:matrix.org",
            reactionKey: "❌"
        )

        try await service.submitReaction(request)

        XCTAssertEqual(service.submitted, [request])
    }

    func testAgentApprovalPromptDetectorExtractsForgePrompt() throws {
        let prompt = try XCTUnwrap(
            SynaraAgentApprovalPromptDetector.detect(
                body: """
                ⚠️ Dangerous command requires approval

                Code

                Copy
                set -euo pipefail
                curl -fsS http://camofox-browser.whyland.com:9377/openapi.json -o /tmp/camofox_openapi.json

                Reason: Security scan - [HIGH] Plain HTTP URL in execution context.

                Reply !approve to execute, !approve always to approve permanently, or !deny to cancel.

                You can also react to this prompt:
                ✅ = approve once
                ♾️ = approve always
                ❌ = deny
                """
            )
        )

        XCTAssertEqual(prompt.title, "Approval Required: Dangerous Command")
        XCTAssertEqual(prompt.body, "Security scan - [HIGH] Plain HTTP URL in execution context.")
        XCTAssertEqual(prompt.commandPreview, "set -euo pipefail")
        XCTAssertTrue(try XCTUnwrap(prompt.command).contains("curl -fsS http://camofox-browser.whyland.com:9377/openapi.json"))
    }

    func testAgentApprovalPromptDetectorUsesFormattedHTMLCandidate() throws {
        let item = TimelineItem(
            id: "$approval-html",
            eventID: "$approval-html",
            senderID: "@forge:matrix.org",
            timestamp: Date(timeIntervalSince1970: 1_770_000_000),
            kind: .formattedText(
                body: "",
                html: #"""
                <p><strong>Dangerous command requires approval</strong></p>
                <pre><code>printf 'ship it'</code></pre>
                <p>Reason: operator approval required.</p>
                """#
            ),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )

        let prompt = try XCTUnwrap(SynaraAgentApprovalPromptDetector.detect(in: item))

        XCTAssertEqual(prompt.commandPreview, "printf 'ship it'")
        XCTAssertEqual(prompt.body, "operator approval required.")
    }

    func testAgentApprovalPromptDetectorIgnoresNonApprovalMessages() {
        XCTAssertNil(
            SynaraAgentApprovalPromptDetector.detect(
                body: "Security scan completed for the dangerous command run."
            )
        )
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
