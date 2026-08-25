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

    func testMockNativeDecisionRejectsApproveAlwaysLikeProductionCore() async {
        let service = MockAgentApprovalReactionService()

        do {
            try await service.submitNativeDecision(
                roomID: "!room:matrix.org",
                eventID: "$approval:matrix.org",
                actionIdentifier: SynaraAgentApprovalNotificationActionID.approveAlways.rawValue
            )
            XCTFail("Approve always must require in-app confirmation")
        } catch let error as SynaraAgentApprovalError {
            XCTAssertEqual(error, .unsupportedAction)
        } catch {
            XCTFail("Unexpected error: \(error)")
        }
        XCTAssertTrue(service.submitted.isEmpty)
    }

    func testAgentApprovalPromptDetectorExtractsAutomationPrompt() throws {
        let prompt = try XCTUnwrap(
            SynaraAgentApprovalPromptDetector.detect(
                body: """
                ⚠️ Dangerous command requires approval

                Code

                Copy
                set -euo pipefail
                curl -fsS http://browser-control.example.com:9377/openapi.json -o /tmp/browser_openapi.json

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
        XCTAssertTrue(try XCTUnwrap(prompt.command).contains("curl -fsS http://browser-control.example.com:9377/openapi.json"))
        XCTAssertTrue(try XCTUnwrap(prompt.replyInstructions).contains("Reply !approve"))
        XCTAssertTrue(try XCTUnwrap(prompt.sourceContext).contains("You can also react"))
    }

    func testAgentApprovalPromptDetectorKeepsHeredocAndInvalidHostnameContext() throws {
        let prompt = try XCTUnwrap(
            SynaraAgentApprovalPromptDetector.detect(
                body: """
                ⚠️ Dangerous command requires approval

                Code

                Copy
                python3 - <<'PY'
                import socket
                hostname = socket.gethostname()
                if hostname not in {"worker-a", "worker-b"}:
                    raise SystemExit(f"invalid hostname: {hostname}")
                print("ok", hostname)
                PY

                Reason: Security scan — [HIGH] Invalid hostname: target host is not in the allowlist for remote execution.

                Reply !approve to execute, !approve always to approve permanently, or !deny to cancel.

                You can also react to this prompt:
                ✅ = approve once
                ♾️ = approve always
                ❌ = deny
                """
            )
        )

        XCTAssertTrue(prompt.body.contains("Invalid hostname"))
        XCTAssertEqual(prompt.commandPreview, "python3 - <<'PY'")
        let command = try XCTUnwrap(prompt.command)
        XCTAssertTrue(command.contains("python3 - <<'PY'"))
        XCTAssertTrue(command.contains("socket.gethostname"))
        XCTAssertTrue(command.contains("raise SystemExit"))
        XCTAssertFalse(command.contains("Reason:"))
        XCTAssertTrue(try XCTUnwrap(prompt.replyInstructions).contains("approve always"))
        XCTAssertTrue(try XCTUnwrap(prompt.sourceContext).contains("Code"))
    }

    func testNativeActionValidatorRequiresResolvedApprovalPrompt() {
        let approvalBody = """
        ⚠️ Dangerous command requires approval

        Code
        Copy
        echo ship-it
        Reason: operator approval required.

        Reply !approve to execute or !deny to cancel.
        """
        let approvalItem = TimelineItem(
            id: "$approval",
            eventID: "$approval",
            senderID: "@automation:matrix.org",
            timestamp: Date(),
            kind: .text(approvalBody),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
        let ordinaryItem = TimelineItem(
            id: "$ordinary",
            eventID: "$ordinary",
            senderID: "@automation:matrix.org",
            timestamp: Date(),
            kind: .text("hello world"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )

        let unresolved = SynaraAgentApprovalNativeActionValidator.validate(
            items: [ordinaryItem],
            eventID: "$missing"
        )
        XCTAssertFalse(unresolved.shouldSubmitReaction)
        XCTAssertEqual(unresolved.reason, "event-unresolved")

        let notPrompt = SynaraAgentApprovalNativeActionValidator.validate(
            items: [ordinaryItem],
            eventID: "$ordinary"
        )
        XCTAssertFalse(notPrompt.shouldSubmitReaction)
        XCTAssertEqual(notPrompt.reason, "not-approval-prompt")

        let valid = SynaraAgentApprovalNativeActionValidator.validate(
            items: [approvalItem],
            eventID: "$approval"
        )
        XCTAssertTrue(valid.shouldSubmitReaction)
        XCTAssertTrue(valid.isApprovalPrompt)
        XCTAssertEqual(valid.reason, "validated")

        let expired = SynaraAgentApprovalNativeActionValidator.validate(
            items: [
                TimelineItem(
                    id: "$stale",
                    eventID: "$stale",
                    senderID: "@automation:matrix.org",
                    timestamp: Date().addingTimeInterval(-(SynaraNotificationActionContract.nativeActionTTL + 30)),
                    kind: .text(approvalBody),
                    replyToEventID: nil,
                    isEdited: false,
                    reactions: [:]
                )
            ],
            eventID: "$stale"
        )
        XCTAssertFalse(expired.shouldSubmitReaction)
        XCTAssertEqual(expired.reason, "expired-ttl")
    }

    func testAgentApprovalPromptDetectorUsesFormattedHTMLCandidate() throws {
        let item = TimelineItem(
            id: "$approval-html",
            eventID: "$approval-html",
            senderID: "@automation:matrix.org",
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
