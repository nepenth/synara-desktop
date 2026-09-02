import XCTest
@testable import Synara

final class TimelineReplyPreviewTests: XCTestCase {
    func testSnippetUsesPlainTextBody() {
        let snippet = TimelineReplyPreview.snippet(for: .text("Hello from the timeline"))

        XCTAssertEqual(snippet, "Hello from the timeline")
    }

    func testSnippetUsesFormattedTextFallback() {
        let snippet = TimelineReplyPreview.snippet(
            for: .formattedText(
                body: "- **Ship it**\n- Review fallback",
                html: #"<ul><li><strong>Ship it</strong></li><li>Review fallback</li></ul>"#
            )
        )

        XCTAssertEqual(snippet, "- **Ship it** - Review fallback")
    }

    func testSnippetUsesSafePlaceholdersForSpecialKinds() throws {
        XCTAssertEqual(
            TimelineReplyPreview.snippet(for: .encryptedPlaceholder),
            "Encrypted message"
        )
        XCTAssertEqual(
            TimelineReplyPreview.snippet(for: .redacted),
            "Message deleted"
        )
        XCTAssertEqual(
            TimelineReplyPreview.snippet(
                for: .mediaPlaceholder(
                    MediaResource(
                        id: "mxc://example/abc",
                        filename: "permissions-spec.pdf",
                        authenticatedURL: nil,
                        requiresAuthentication: false
                    )
                )
            ),
            "permissions-spec.pdf"
        )
        XCTAssertEqual(
            TimelineReplyPreview.snippet(
                for: .agentCard(
                    try SynaraAgentCard(
                        title: "Approval required",
                        status: "pending",
                        summary: "Review the proposed action.",
                        actions: []
                    )
                )
            ),
            "Review the proposed action."
        )
    }

    func testTruncatedSnippetLimitsLength() {
        let longText = String(repeating: "a", count: 120)
        let snippet = TimelineReplyPreview.truncatedSnippet(longText)

        XCTAssertEqual(snippet.count, TimelineReplyPreview.maxSnippetLength)
        XCTAssertTrue(snippet.hasSuffix("…"))
    }

    func testComposerRelationTargetBuildsReplyPreview() {
        let item = TimelineItem(
            id: "1",
            eventID: "$event",
            senderID: "@mina:matrix.org",
            timestamp: Date(),
            kind: .text("Can I take a pass on the reviewer roles?"),
            replyToEventID: nil,
            threadRootEventID: "$thread-root",
            isEdited: false,
            reactions: [:]
        )

        let target = ComposerRelationTarget(item: item, kind: .reply, currentUserID: "@local:matrix.org")

        XCTAssertEqual(target.eventID, "$event")
        XCTAssertEqual(target.threadRootEventID, "$thread-root")
        XCTAssertEqual(target.senderName, "Mina")
        XCTAssertEqual(target.snippet, "Can I take a pass on the reviewer roles?")
        XCTAssertEqual(target.bannerTitle, "Replying to Mina")
    }

    func testComposerRelationTargetBuildsEditPreviewForOwnMessage() {
        let item = TimelineItem(
            id: "2",
            eventID: "$edit",
            senderID: "@local:matrix.org",
            timestamp: Date(),
            kind: .text("Updated draft copy"),
            replyToEventID: nil,
            isEdited: true,
            reactions: [:]
        )

        let target = ComposerRelationTarget(item: item, kind: .edit, currentUserID: "@local:matrix.org")

        XCTAssertEqual(target.senderName, "You")
        XCTAssertEqual(target.bannerTitle, "Editing your message")
        XCTAssertEqual(target.snippet, "Updated draft copy")
        XCTAssertFalse(target.isLocalPending)
    }

    func testComposerRelationTargetBuildsEditPreviewForFailedLocalMessage() {
        let item = TimelineItem.pendingMessage(
            localID: "$pending-failed",
            body: "Retry after a typo",
            senderID: "@local:matrix.org",
            replyToEventID: nil,
            deliveryStatus: .failed
        )

        let target = ComposerRelationTarget(item: item, kind: .edit, currentUserID: "@local:matrix.org")

        XCTAssertEqual(target.bannerTitle, "Editing unsent message")
        XCTAssertTrue(target.isLocalPending)
        XCTAssertEqual(target.snippet, "Retry after a typo")
    }
}
