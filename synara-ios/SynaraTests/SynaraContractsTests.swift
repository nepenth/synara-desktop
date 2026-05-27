import XCTest
@testable import Synara

final class SynaraContractsTests: XCTestCase {
    func testDecodesNotificationSummary() throws {
        let data = try XCTUnwrap(validNotificationSummaryJSON.data(using: .utf8))
        let summary = try JSONDecoder().decode(SynaraNotificationSummary.self, from: data)

        XCTAssertEqual(summary.appBadgeCount, 10)
        XCTAssertEqual(summary.inboxBadgeCount, 8)
        XCTAssertEqual(summary.laterActiveCount, 5)
        XCTAssertEqual(summary.unreadCount, 3)
    }

    func testRejectsNegativeNotificationCounts() {
        let data = try? invalidNotificationSummaryJSON.data(using: .utf8)
        XCTAssertNotNil(data)
        XCTAssertThrowsError(try JSONDecoder().decode(SynaraNotificationSummary.self, from: data!))
    }

    func testDecodesLaterContentAndKinds() throws {
        let data = try XCTUnwrap(validLaterContentJSON.data(using: .utf8))
        let content = try JSONDecoder().decode(SynaraLaterContent.self, from: data)

        XCTAssertEqual(content.version, 1)
        XCTAssertEqual(content.items.count, 2)
        XCTAssertEqual(content.items["!room:example.org\n$event"]?.kind, .saved)
        XCTAssertEqual(content.items["!room:example.org\n$reminder"]?.kind, .reminder)
    }

    func testRejectsInvalidLaterKind() {
        let data = try? invalidLaterKindJSON.data(using: .utf8)
        XCTAssertNotNil(data)
        XCTAssertThrowsError(try JSONDecoder().decode(SynaraLaterContent.self, from: data!))
    }

    func testRejectsMissingLaterVersion() {
        let data = try? missingLaterVersionJSON.data(using: .utf8)
        XCTAssertNotNil(data)
        XCTAssertThrowsError(try JSONDecoder().decode(SynaraLaterContent.self, from: data!))
    }

    func testDecodesAgentActionOpenURL() throws {
        let action = try JSONDecoder().decode(SynaraAgentAction.self, from: validAgentActionOpenURLJSON.data(using: .utf8)!)

        XCTAssertEqual(action.id, "export")
        XCTAssertEqual(action.title, "Export")
        XCTAssertEqual(action.kind, .openURL)
        XCTAssertEqual(action.url, "https://artifacts.example.org/report.html")
    }

    func testRejectsUnsafeAgentURL() {
        XCTAssertThrowsError(try JSONDecoder().decode(SynaraAgentAction.self, from: invalidAgentActionURLJSON.data(using: .utf8)!))
    }

    func testInvalidAgentActionWithoutPayloadIsRejected() {
        XCTAssertThrowsError(try JSONDecoder().decode(SynaraAgentAction.self, from: invalidAgentActionPayloadJSON.data(using: .utf8)!))
    }

    func testDecodesAgentCardFromValidPayload() throws {
        let card = try JSONDecoder().decode(SynaraAgentCard.self, from: validAgentCardJSON.data(using: .utf8)!)

        XCTAssertEqual(card.title, "Agent summary")
        XCTAssertEqual(card.status, "ok")
        XCTAssertEqual(card.summary, "Plan complete.")
        XCTAssertEqual(card.actions.count, 1)
        XCTAssertEqual(card.artifacts.count, 1)
        XCTAssertEqual(card.actions.first?.id, "continue")
        XCTAssertEqual(card.artifacts.first?.title, "Report")
    }

    func testDecodesAgentCardActionWithMarkdownPayload() throws {
        let action = try JSONDecoder().decode(SynaraAgentCardAction.self, from: validAgentCardActionMarkdownJSON.data(using: .utf8)!)

        XCTAssertEqual(action.id, "copy-markdown")
        XCTAssertEqual(action.title, "Copy Markdown")
        XCTAssertEqual(action.kind, "copy_markdown")
        XCTAssertEqual(action.markdown, "# heading")
    }

    func testRejectsAgentCardActionWithOnlyUnsupportedPayloadAndUnknownKind() {
        XCTAssertThrowsError(try SynaraAgentCardAction(
            id: "bad",
            title: "Bad",
            kind: "copy_markdown",
            prompt: nil,
            url: nil,
            markdown: nil
        ))
    }

    func testRejectsInvalidAgentCardWithoutContent() {
        XCTAssertThrowsError(try JSONDecoder().decode(SynaraAgentCard.self, from: invalidAgentCardMissingContentJSON.data(using: .utf8)!))
    }

    func testParsesInboxRoutes() throws {
        let route = try SynaraRoutePath(rawValue: "/inbox/later/")
        XCTAssertEqual(route.rawValue, "/inbox/later/")

        XCTAssertThrowsError(try SynaraRoutePath(rawValue: "https://example.org/home"))
        XCTAssertThrowsError(try SynaraRoutePath(rawValue: "/inbox/unknown/"))
    }

    func testRejectsEmptyLaterItemFields() {
        XCTAssertThrowsError(try SynaraLaterItem(
            id: "",
            kind: .saved,
            roomId: "!room:example.org",
            eventId: "$event:example.org",
            createdAt: 1770000000000
        ))
    }

    func testRejectsEmptyRoute() {
        XCTAssertThrowsError(try SynaraRoutePath(rawValue: ""))
    }

    private let validNotificationSummaryJSON = #"""
    {
      "appBadgeCount": 10,
      "inboxBadgeCount": 8,
      "laterActiveCount": 5,
      "inviteCount": 2,
      "agentApprovalCount": 1,
      "highlightCount": 2,
      "unreadCount": 3
    }
    """

    private let invalidNotificationSummaryJSON = #"""
    {
      "appBadgeCount": -1,
      "inboxBadgeCount": 0,
      "laterActiveCount": 0,
      "inviteCount": 0,
      "agentApprovalCount": 0,
      "highlightCount": 0,
      "unreadCount": 0
    }
    """

    private let validLaterContentJSON = #"""
    {
        "version": 1,
        "items": {
        "!room:example.org\n$event": {
          "id": "!room:example.org\n$event",
          "kind": "saved",
          "roomId": "!room:example.org",
          "eventId": "$event",
          "createdAt": 1770000000000
        },
        "!room:example.org\n$reminder": {
          "id": "!room:example.org\n$reminder",
          "kind": "reminder",
          "roomId": "!room:example.org",
          "eventId": "$reminder",
          "createdAt": 1770000000000,
          "dueTs": 1770003600000
        }
      }
    }
    """

    private let invalidLaterKindJSON = #"""
    {
      "version": 1,
      "items": {
        "!room:example.org\n$event": {
          "id": "!room:example.org\n$event",
          "kind": "todo",
          "roomId": "!room:example.org",
          "eventId": "$event",
          "createdAt": 1770000000000
        }
      }
    }
    """

    private let missingLaterVersionJSON = #"""
    {
      "items": {
        "!room:example.org\n$event": {
          "id": "!room:example.org\n$event",
          "kind": "saved",
          "roomId": "!room:example.org",
          "eventId": "$event",
          "createdAt": 1770000000000
        }
      }
    }
    """

    private let validAgentActionOpenURLJSON = #"""
    {
      "id": "export",
      "title": "Export",
      "kind": "open_url",
      "url": "https://artifacts.example.org/report.html",
      "markdown": "# Thread"
    }
    """

    private let invalidAgentActionURLJSON = #"""
    {
      "id": "unsafe",
      "title": "Open",
      "kind": "open_url",
      "url": "http://127.0.0.1/report.html"
    }
    """

    private let invalidAgentActionPayloadJSON = #"""
    {
      "id": "missing",
      "title": "No payload"
    }
    """

    private let validAgentCardJSON = #"""
    {
      "title": "Agent summary",
      "status": "ok",
      "summary": "Plan complete.",
      "actions": [
        {
          "id": "continue",
          "title": "Continue",
          "kind": "agent",
          "prompt": "Continue from the latest checkpoint."
        }
      ],
      "artifacts": [
        {
          "title": "Report",
          "type": "html",
          "url": "https://artifacts.example.org/report.html",
          "summary": "Build report."
        }
      ]
    }
    """

    private let invalidAgentCardMissingContentJSON = #"""
    {
      "title": "Empty"
    }
    """

    private let validAgentCardActionMarkdownJSON = #"""
    {
      "id": "copy-markdown",
      "title": "Copy Markdown",
      "kind": "copy_markdown",
      "markdown": "# heading"
    }
    """
}
