import XCTest
@testable import Synara

final class AgentActionResolverTests: XCTestCase {
    func testOpenURLActionProducesOpenURLPlan() throws {
        let action = try SynaraAgentCardAction(
            id: "open",
            title: "Open",
            kind: "open_url",
            url: "https://artifacts.example.org/report.html"
        )

        guard case .success(.openURL(let url)) = SynaraAgentCardActionResolver.plan(for: action) else {
            XCTFail("Expected open URL plan")
            return
        }

        XCTAssertEqual(url.absoluteString, "https://artifacts.example.org/report.html")
    }

    func testUnsafeURLActionIsRejected() throws {
        XCTAssertThrowsError(try SynaraAgentCardAction(
            id: "open",
            title: "Open",
            kind: "open",
            url: "http://127.0.0.1/report"
        ))
    }

    func testCopyPromptActionCopiesPrompt() throws {
        let action = try SynaraAgentCardAction(
            id: "copy",
            title: "Copy",
            kind: "copy_prompt",
            prompt: "prompt body"
        )

        guard case .success(.copyText("prompt body")) = SynaraAgentCardActionResolver.plan(for: action) else {
            XCTFail("Expected copy text plan")
            return
        }
    }

    func testCopyMarkdownActionCopiesMarkdown() throws {
        let action = try SynaraAgentCardAction(
            id: "copy-markdown",
            title: "Copy Markdown",
            kind: "copy_markdown",
            markdown: "**hello**"
        )

        guard case .success(.copyText("**hello**")) = SynaraAgentCardActionResolver.plan(for: action) else {
            XCTFail("Expected markdown copy plan")
            return
        }
    }

    func testCopyJSONActionReturnsJSONPayload() throws {
        let action = try SynaraAgentCardAction(
            id: "copy-json",
            title: "Copy JSON",
            kind: "copy_json",
            prompt: "prompt body"
        )

        guard case .success(.copyText(let copied)) = SynaraAgentCardActionResolver.plan(for: action) else {
            XCTFail("Expected copy JSON plan")
            return
        }

        XCTAssertTrue(copied.contains("\"id\""))
        XCTAssertTrue(copied.contains("copy-json"))
    }

    func testUnsupportedKindsCanRenderAsBlockedState() throws {
        let action = try SynaraAgentCardAction(
            id: "unknown",
            title: "Unknown",
            kind: "unknown-kind",
            prompt: "unsupported"
        )

        XCTAssertFalse(SynaraAgentCardActionResolver.shouldRender(action))
        guard case .failure(.unsupportedKind("unknown-kind")) = SynaraAgentCardActionResolver.plan(for: action) else {
            XCTFail("Expected unsupported kind failure")
            return
        }
    }

    func testApprovalActionsProduceSubmissionPlan() throws {
        let action = try SynaraAgentCardAction(
            id: "approve",
            title: "Approve",
            kind: "approve",
            prompt: "approve request"
        )

        guard case .success(.submitApproval(.approve)) = SynaraAgentCardActionResolver.plan(for: action) else {
            XCTFail("Expected approve submission plan")
            return
        }
    }

    func testActionsWithoutKindDefaultToPromptCopy() throws {
        let action = try SynaraAgentCardAction(
            id: "fallback",
            title: "Fallback",
            prompt: "fallback prompt"
        )

        guard case .success(.copyText("fallback prompt")) = SynaraAgentCardActionResolver.plan(for: action) else {
            XCTFail("Expected fallback copy plan")
            return
        }
    }
}
