import XCTest
#if canImport(UIKit)
    import UIKit
#endif
@testable import Synara

final class TimelineMessageCopyTests: XCTestCase {
    func testCopyPayloadIsThePlainMessageBody() {
        let item = makeItem(kind: .text("Hello from the timeline"))

        XCTAssertEqual(
            TimelineMessageCopy.payload(for: item),
            .init(plainText: "Hello from the timeline", html: nil)
        )
    }

    func testCopyPayloadCarriesPlainFallbackAndSafeFormattedHTML() throws {
        let html = #"<p>Visible <em>body</em> with <a href="https://example.org">link</a></p>"#
        let item = makeItem(kind: .formattedText(body: "Visible body with link", html: html))
        let payload = try XCTUnwrap(TimelineMessageCopy.payload(for: item))

        XCTAssertEqual(payload.plainText, "Visible body with link")
        XCTAssertEqual(payload.html, html)
    }

    func testCopyPayloadRemovesExecutableContentAndUnsafeLinks() throws {
        let item = makeItem(
            kind: .formattedText(
                body: "Visible safe bad",
                html: #"<p>Visible <strong>safe</strong><script>alert(1)</script><a href="javascript:alert(2)" onclick="alert(3)">bad</a></p>"#
            )
        )
        let payload = try XCTUnwrap(TimelineMessageCopy.payload(for: item))
        let safeHTML = try XCTUnwrap(payload.html)

        XCTAssertEqual(payload.plainText, "Visible safe bad")
        XCTAssertTrue(safeHTML.contains("<strong>safe</strong>"))
        XCTAssertTrue(safeHTML.contains("<a>bad</a>"))
        XCTAssertFalse(safeHTML.localizedCaseInsensitiveContains("script"))
        XCTAssertFalse(safeHTML.localizedCaseInsensitiveContains("javascript"))
        XCTAssertFalse(safeHTML.localizedCaseInsensitiveContains("onclick"))
    }

    func testCopyPayloadCannotEmbedRemoteResourcesOrCSS() throws {
        let item = makeItem(
            kind: .formattedText(
                body: "Diagram and documentation",
                html: #"<style>@import url(https://tracker.example/style.css)</style><iframe src="https://tracker.example/frame"></iframe><p><img src="https://tracker.example/pixel" onerror="steal()" alt="Diagram"> and <a href="https://docs.example/guide">documentation</a></p>"#
            )
        )
        let payload = try XCTUnwrap(TimelineMessageCopy.payload(for: item))
        let safeHTML = try XCTUnwrap(payload.html)

        XCTAssertTrue(safeHTML.contains("Diagram"), "Inline images must degrade to inert alt text")
        XCTAssertTrue(safeHTML.contains(#"href="https://docs.example/guide""#))
        XCTAssertFalse(safeHTML.localizedCaseInsensitiveContains("<style"))
        XCTAssertFalse(safeHTML.localizedCaseInsensitiveContains("<iframe"))
        XCTAssertFalse(safeHTML.localizedCaseInsensitiveContains("<img"))
        XCTAssertFalse(safeHTML.localizedCaseInsensitiveContains("src="))
        XCTAssertFalse(safeHTML.localizedCaseInsensitiveContains("onerror"))
        XCTAssertFalse(safeHTML.localizedCaseInsensitiveContains("tracker.example"))
    }

    func testCopyPayloadPreservesAllowedListAndInlineFormatting() throws {
        let html = #"<ol start="3"><li><strong>First</strong></li><li><em>Second</em></li></ol>"#
        let payload = try XCTUnwrap(
            TimelineMessageCopy.payload(
                for: makeItem(kind: .formattedText(body: "3. First\n4. Second", html: html))
            )
        )

        XCTAssertEqual(payload.html, html)
        XCTAssertEqual(payload.plainText, "3. First\n4. Second")
    }

    func testSelectionProjectionConcealsSpoilersUntilExplicitReveal() {
        let html = #"<p>Public <span data-mx-spoiler="answer">secret <strong>detail</strong></span> ending</p>"#

        let concealed = MatrixHTMLRenderer.selectionProjection(
            body: "Public secret detail ending",
            html: html,
            revealingSpoilers: false
        )
        XCTAssertTrue(concealed.containsSpoilers)
        XCTAssertTrue(concealed.richText.plainText.contains("Public"))
        XCTAssertTrue(concealed.richText.plainText.contains("Reveal to select"))
        XCTAssertFalse(concealed.richText.plainText.contains("secret"))
        XCTAssertFalse(concealed.richText.plainText.contains("detail"))

        let revealed = MatrixHTMLRenderer.selectionProjection(
            body: "Public secret detail ending",
            html: html,
            revealingSpoilers: true
        )
        XCTAssertTrue(revealed.containsSpoilers)
        XCTAssertTrue(revealed.richText.plainText.contains("secret detail"))
        XCTAssertTrue(
            revealed.richText.runs.contains { run in
                run.text == "detail" && run.style.contains(.bold)
            },
            "Revealing a spoiler must retain formatting inside the selected range"
        )
    }

    func testSanitizedClipboardSpoilerRemainsConcealedBySelectionProjection() throws {
        let rawHTML = #"<p>Public <span data-mx-spoiler="answer">secret <strong>detail</strong></span></p><script>steal()</script>"#
        let sanitizedHTML = try XCTUnwrap(
            MatrixHTMLRenderer.sanitizedHTMLForClipboard(html: rawHTML)
        )
        XCTAssertTrue(sanitizedHTML.contains(#"data-mx-spoiler="answer""#))
        XCTAssertFalse(sanitizedHTML.localizedCaseInsensitiveContains("script"))

        let concealed = MatrixHTMLRenderer.selectionProjection(
            body: "Public secret detail",
            html: sanitizedHTML,
            revealingSpoilers: false
        )
        XCTAssertTrue(concealed.containsSpoilers)
        XCTAssertFalse(concealed.richText.plainText.contains("secret"))
        XCTAssertFalse(concealed.richText.plainText.contains("detail"))

        let revealed = MatrixHTMLRenderer.selectionProjection(
            body: "Public secret detail",
            html: sanitizedHTML,
            revealingSpoilers: true
        )
        XCTAssertTrue(revealed.richText.plainText.contains("secret detail"))
        XCTAssertTrue(
            revealed.richText.runs.contains { run in
                run.text == "detail" && run.style.contains(.bold)
            },
            "The clipboard sanitizer and selection projection must preserve allowed formatting after reveal"
        )
    }

    func testSelectionProjectionRetainsPartialInlineFormatting() {
        let projection = MatrixHTMLRenderer.selectionProjection(
            body: "Bold italic code",
            html: "<p><strong>Bold</strong> <em>italic</em> <code>code</code></p>",
            revealingSpoilers: false
        )

        XCTAssertFalse(projection.containsSpoilers)
        XCTAssertEqual(projection.richText.plainText, "Bold italic code")
        XCTAssertTrue(projection.richText.runs.contains { $0.text == "Bold" && $0.style.contains(.bold) })
        XCTAssertTrue(projection.richText.runs.contains { $0.text == "italic" && $0.style.contains(.italic) })
        XCTAssertTrue(projection.richText.runs.contains { $0.text == "code" && $0.style.contains(.code) })
    }

    func testFailedLocalSendStillHasCopyPayload() {
        let item = TimelineItem.pendingMessage(
            body: "Retry me",
            senderID: "@alice:matrix.org",
            replyToEventID: nil,
            deliveryStatus: .failed
        )

        XCTAssertEqual(TimelineMessageCopy.payload(for: item)?.plainText, "Retry me")
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
            TimelineMessageCopy.copyToPasteboard(.init(plainText: unique, html: nil))
            XCTAssertEqual(UIPasteboard.general.string, unique)
        #else
            XCTFail("UIPasteboard is required for iOS message copy")
        #endif
    }

    func testCopyWritesSafeHTMLAlongsidePlainFallback() throws {
        #if canImport(UIKit)
            let payload = TimelineMessageCopy.Payload(
                plainText: "Formatted message",
                html: "<p><strong>Formatted</strong> message</p>"
            )
            TimelineMessageCopy.copyToPasteboard(payload)

            XCTAssertEqual(UIPasteboard.general.string, payload.plainText)
            let htmlValue = UIPasteboard.general.value(forPasteboardType: "public.html")
            if let htmlData = htmlValue as? Data {
                XCTAssertEqual(String(data: htmlData, encoding: .utf8), payload.html)
            } else {
                XCTAssertEqual(htmlValue as? String, payload.html)
            }
        #else
            XCTFail("UIPasteboard is required for iOS rich message copy")
        #endif
    }

    func testTimelineRowContextMenuIncludesCopyAndKeepsExistingActions() throws {
        let source = try Self.contents(of: "synara-ios/Synara/Features/RoomTimelineView.swift")

        XCTAssertTrue(source.contains(".contextMenu {"), "Timeline rows must expose a long-press menu")
        XCTAssertTrue(
            source.contains("TimelineMessageCopy.payload(for: item)"),
            "Copy must use the sanitized rich/plain clipboard helper"
        )
        XCTAssertTrue(source.contains("Button(\"Copy\", systemImage: \"doc.on.doc\")"))
        XCTAssertTrue(source.contains("Button(\"Select Text\", systemImage: \"text.cursor\")"))
        XCTAssertTrue(
            source.contains("TimelineItemCopy-\\(item.eventID)"),
            "Copy must use the TimelineItemCopy accessibility identifier"
        )
        XCTAssertTrue(source.contains("TimelineItemSelectText-\\(item.eventID)"))
        XCTAssertTrue(source.contains("Button(\"Reply\", systemImage: \"arrowshape.turn.up.left\", action: onReply)"))
        XCTAssertTrue(source.contains("Button(\"Open Thread\", systemImage: \"bubble.left.and.bubble.right\", action: onOpenThread)"))
        XCTAssertTrue(source.contains("Button(\"Edit\", systemImage: \"pencil\", action: onEdit)"))
        XCTAssertTrue(source.contains("Button(\"React\", systemImage: \"face.smiling\", action: onReact)"))
        XCTAssertTrue(source.contains("Button(\"Redact\", systemImage: \"trash\", role: .destructive, action: onRedact)"))
    }

    func testTimelineAndThreadMenusUseDismissibleExplicitSelectionPresentation() throws {
        let source = try Self.contents(of: "synara-ios/Synara/Features/RoomTimelineView.swift")

        XCTAssertGreaterThanOrEqual(
            source.components(separatedBy: "Button(\"Select Text\", systemImage: \"text.cursor\")").count - 1,
            2,
            "Both room and thread message menus must expose Select Text"
        )
        XCTAssertTrue(source.contains("MessageTextSelectionSheet(payload: copyPayload)"))
        XCTAssertTrue(source.contains("Button(\"Done\", action: dismiss.callAsFunction)"))
        XCTAssertTrue(source.contains(".textSelection(.enabled)"))
        XCTAssertTrue(source.contains("Button(\"Copy All\", systemImage: \"doc.on.doc\")"))
        XCTAssertTrue(source.contains("includeLinks: false"), "Selection must never activate remote links")
        XCTAssertTrue(source.contains(".accessibilityLabel(projection.richText.plainText)"))
        XCTAssertFalse(source.contains(".accessibilityLabel(\"Selectable message text\")"))
        XCTAssertTrue(source.contains("revealsSpoilers ? \"Hide Spoilers\" : \"Reveal Spoilers\""))
        XCTAssertTrue(source.contains(".disabled(projection.containsSpoilers && revealsSpoilers == false)"))
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
        XCTAssertTrue(timeline.contains("statusEventID: item.eventID"))
    }

    func testMessageBodyEnablesTextSelectionAsSecondarySubstringCopy() throws {
        let bubble = try Self.contents(of: "synara-ios/Synara/SharedUI/SynaraMessageBubble.swift")
        let timeline = try Self.contents(of: "synara-ios/Synara/Features/RoomTimelineView.swift")

        // Copy transfers the complete rich/plain payload. Select Text opens a
        // dedicated sheet where `.textSelection(.enabled)` owns the substring
        // gesture without competing with the row's long-press menu.
        XCTAssertTrue(bubble.contains(".textSelection(.enabled)"))
        XCTAssertTrue(timeline.contains(".textSelection(.enabled)"))
        XCTAssertTrue(timeline.contains("TimelineMessageCopy.copyToPasteboard(copyPayload)"))
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
