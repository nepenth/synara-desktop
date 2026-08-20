import XCTest
#if canImport(UIKit)
    import UIKit
#endif
@testable import Synara

final class TimelineMessageCopyTests: XCTestCase {
    func testCopyPayloadIsThePlainMessageBody() {
        let item = makeItem(kind: .text("Hello from the timeline"))

        XCTAssertEqual(TimelineMessageCopy.payload(for: item), "Hello from the timeline")
    }

    func testCopyPayloadUsesFormattedVisibleBodyNotRawHTML() {
        let html = #"<p>Visible <em>body</em> with <a href="https://example.org">link</a></p>"#
        let item = makeItem(kind: .formattedText(body: "Visible body with link", html: html))

        XCTAssertEqual(TimelineMessageCopy.payload(for: item), "Visible body with link")
        XCTAssertFalse(TimelineMessageCopy.payload(for: item)?.contains("<p>") == true)
        XCTAssertFalse(TimelineMessageCopy.payload(for: item)?.contains("<em>") == true)
    }

    func testFailedLocalSendStillHasCopyPayload() {
        let item = TimelineItem.pendingMessage(
            body: "Retry me",
            senderID: "@alice:matrix.org",
            replyToEventID: nil,
            deliveryStatus: .failed
        )

        XCTAssertEqual(TimelineMessageCopy.payload(for: item), "Retry me")
    }

    func testNonTextKindsHaveNoCopyPayload() throws {
        XCTAssertNil(TimelineMessageCopy.payload(for: makeItem(kind: .redacted)))
        XCTAssertNil(TimelineMessageCopy.payload(for: makeItem(kind: .encryptedPlaceholder)))
        XCTAssertNil(
            TimelineMessageCopy.payload(
                for: makeItem(
                    kind: .mediaPlaceholder(
                        MediaResource(
                            id: "mxc://example/abc",
                            filename: "permissions-spec.pdf",
                            authenticatedURL: nil,
                            requiresAuthentication: false
                        )
                    )
                )
            )
        )
        XCTAssertNil(TimelineMessageCopy.payload(for: makeItem(kind: .text(""))))
    }

    func testCopyWritesPlainTextToPasteboard() {
        #if canImport(UIKit)
            let unique = "synara-copy-payload-\(UUID().uuidString)"
            TimelineMessageCopy.copyToPasteboard(unique)
            XCTAssertEqual(UIPasteboard.general.string, unique)
        #else
            XCTFail("UIPasteboard is required for iOS message copy")
        #endif
    }

    func testTimelineRowContextMenuIncludesCopyAndKeepsExistingActions() throws {
        let source = try Self.contents(of: "synara-ios/Synara/Features/RoomTimelineView.swift")

        XCTAssertTrue(source.contains(".contextMenu {"), "Timeline rows must expose a long-press menu")
        XCTAssertTrue(
            source.contains("TimelineMessageCopy.payload(for: item)"),
            "Copy must use the plain-body helper, not raw HTML"
        )
        XCTAssertTrue(
            source.contains("Button(\"Copy\")"),
            "The long-press menu must include Copy"
        )
        XCTAssertTrue(
            source.contains("TimelineItemCopy-\\(item.eventID)"),
            "Copy must use the TimelineItemCopy accessibility identifier"
        )
        XCTAssertTrue(source.contains("Button(\"Reply\", action: onReply)"))
        XCTAssertTrue(source.contains("Button(\"Edit\", action: onEdit)"))
        XCTAssertTrue(source.contains("Button(\"React\", action: onReact)"))
        XCTAssertTrue(source.contains("Button(\"Redact\", role: .destructive, action: onRedact)"))
    }

    func testFailedSendRowsKeepContextMenuByUsingRetryChipNotFullRowButton() throws {
        let timeline = try Self.contents(of: "synara-ios/Synara/Features/RoomTimelineView.swift")
        let bubble = try Self.contents(of: "synara-ios/Synara/SharedUI/SynaraMessageBubble.swift")

        XCTAssertFalse(
            timeline.contains("Button(action: onRetryFailedSend)"),
            "Wrapping the whole failed row in a Button swallows the long-press Copy menu"
        )
        XCTAssertTrue(
            bubble.contains("retryChip"),
            "Failed sends must retry from the delivery-status chip"
        )
        XCTAssertTrue(bubble.contains("TimelineItemRetry"))
        XCTAssertTrue(timeline.contains("retryAccessibilityIdentifier: \"TimelineItemRetry-\\(item.eventID)\""))
    }

    func testMessageBodyEnablesTextSelectionAsSecondarySubstringCopy() throws {
        let bubble = try Self.contents(of: "synara-ios/Synara/SharedUI/SynaraMessageBubble.swift")
        let timeline = try Self.contents(of: "synara-ios/Synara/Features/RoomTimelineView.swift")

        // Primary copy path is the row contextMenu Copy action (full plain body).
        // `.textSelection(.enabled)` on the message body is the substring path for
        // long messages. If those gestures conflict on a given iOS version, keep
        // contextMenu Copy; selection is best-effort and must not replace it.
        XCTAssertTrue(bubble.contains(".textSelection(.enabled)"))
        XCTAssertTrue(timeline.contains(".textSelection(.enabled)"))
        XCTAssertTrue(timeline.contains("TimelineMessageCopy.copyToPasteboard(copyText)"))
    }

    private func makeItem(kind: TimelineItem.Kind) -> TimelineItem {
        TimelineItem(
            id: "$event:matrix.org",
            eventID: "$event:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: kind,
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
    }

    private static func contents(of relativePath: String) throws -> String {
        try String(contentsOfFile: "\(repositoryRoot())/\(relativePath)", encoding: .utf8)
    }

    private static func repositoryRoot() -> String {
        var url = URL(fileURLWithPath: #filePath)
        while url.pathComponents.count > 1 {
            url.deleteLastPathComponent()
            if FileManager.default.fileExists(atPath: url.appendingPathComponent("synara-ios").path) {
                return url.path
            }
        }
        return url.path
    }
}
