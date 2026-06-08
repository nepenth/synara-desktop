import XCTest
@preconcurrency import MatrixRustSDK
@testable import Synara

final class TimelineServiceTests: XCTestCase {
    func testMatrixHTMLSanitizerDropsUnsafeLinksAndKeepsFallback() {
        let markdown = MatrixHTMLRenderer.sanitizedMarkdown(
            body: "fallback",
            html: #"<em>Hi</em> <a href="javascript:alert(1)">tap</a> <code>&lt;safe&gt;</code>"#
        )

        XCTAssertEqual(markdown, "*Hi* tap `<safe>`")
    }

    func testMatrixHTMLRendererConvertsListsAndStrongText() {
        let markdown = MatrixHTMLRenderer.sanitizedMarkdown(
            body: "- **Ship it**\n- Review fallback",
            html: #"<ul><li><strong>Ship it</strong></li><li>Review fallback</li></ul>"#
        )

        XCTAssertEqual(markdown, "- **Ship it**\n- Review fallback")

        let attributed = MatrixHTMLRenderer.attributedString(
            body: "- **Ship it**\n- Review fallback",
            html: #"<ul><li><strong>Ship it</strong></li><li>Review fallback</li></ul>"#
        )
        XCTAssertEqual(String(attributed.characters), "- Ship it\n- Review fallback")
    }

    func testMatrixHTMLRendererExtractsDetailsCodeBlocks() throws {
        let html = #"""
        <details open>
          <summary>🛠️ Tool activity (4 updates)</summary>
          <pre><code>🧠 memory: "~memory: &quot;Spectre multi-model &quot;"&#10;✅ memory completed (0.0s)</code></pre>
        </details>
        <p><strong>Practical decision:</strong></p>
        <ul><li>Do not use it for active/default bounty pipelines.</li></ul>
        """#

        let block = try XCTUnwrap(MatrixHTMLRenderer.detailsBlocks(html: html).first)

        XCTAssertEqual(block.summary, "🛠️ Tool activity (4 updates)")
        XCTAssertEqual(
            block.code,
            "🧠 memory: \"~memory: \"Spectre multi-model \"\"\n✅ memory completed (0.0s)"
        )
        XCTAssertTrue(block.body.isEmpty)
        XCTAssertEqual(
            MatrixHTMLRenderer.markdownExcludingDetails(body: "", html: html),
            "**Practical decision:**\n\n- Do not use it for active/default bounty pipelines."
        )
    }

    func testMatrixRustSDKMapperPreservesFormattedTextMessages() {
        let content = MsgLikeContent(
            kind: .message(
                content: MessageContent(
                    msgType: .text(
                        content: TextMessageContent(
                            body: "- **Ship it**\n- Review fallback",
                            formatted: FormattedBody(
                                format: .html,
                                body: #"<ul><li><strong>Ship it</strong></li><li>Review fallback</li></ul>"#
                            )
                        )
                    ),
                    body: "- **Ship it**\n- Review fallback",
                    isEdited: false,
                    mentions: nil
                )
            ),
            reactions: [],
            inReplyTo: nil,
            threadRoot: nil,
            threadSummary: nil
        )

        let kind = MatrixRustSDKTimelineMessageMapper.mapMessageLike(content, eventTypeRaw: "m.room.message")

        if case .formattedText(let body, let html) = kind {
            XCTAssertEqual(body, "- **Ship it**\n- Review fallback")
            XCTAssertEqual(html, #"<ul><li><strong>Ship it</strong></li><li>Review fallback</li></ul>"#)
        } else {
            XCTFail("Expected formatted text, got \(kind)")
        }
    }

    func testAgentCardPayloadParserReadsHermesJSONMessageBody() throws {
        let body = #"""
        {
          "hermes": true,
          "payload": {
            "title": "Approval required",
            "status": "pending",
            "summary": "Review the proposed action.",
            "actions": [
              {
                "id": "approve",
                "title": "Approve",
                "kind": "approve",
                "prompt": "approve request"
              }
            ]
          }
        }
        """#

        let card = try XCTUnwrap(SynaraAgentCardPayloadParser.parse(body: body))

        XCTAssertEqual(card.title, "Approval required")
        XCTAssertEqual(card.status, "pending")
        XCTAssertEqual(card.actions.first?.id, "approve")
        XCTAssertEqual(card.actions.first?.kind, "approve")
    }

    func testMapperMapsAgentCardKind() {
        let card = try! SynaraAgentCard(
            title: "Agent summary",
            status: "ok",
            summary: "Plan complete.",
            actions: [
                try! SynaraAgentCardAction(
                    id: "continue",
                    title: "Continue",
                    prompt: "continue"
                )
            ]
        )
        let event = RawTimelineEvent(
            eventID: "$agent:matrix.org",
            senderID: "@agent:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            type: "org.hermes.agent",
            body: nil,
            replyToEventID: nil,
            isEdited: false,
            mediaURL: nil,
            agentCard: card
        )

        if case .agentCard(let mapped) = TimelineMapper.map(event).kind {
            XCTAssertEqual(mapped, card)
        } else {
            XCTFail("Expected agent card mapped kind")
        }
    }

    func testMapperKeepsStableIdentityAndMetadata() {
        let event = RawTimelineEvent(
            eventID: "$event:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            type: "m.room.message",
            body: "Hello",
            replyToEventID: "$parent:matrix.org",
            isEdited: true,
            mediaURL: nil
        )

        let item = TimelineMapper.map(event)

        XCTAssertEqual(item.id, "$event:matrix.org")
        XCTAssertEqual(item.eventID, "$event:matrix.org")
        XCTAssertEqual(item.senderID, "@alice:matrix.org")
        XCTAssertEqual(item.kind, .text("Hello"))
        XCTAssertEqual(item.replyToEventID, "$parent:matrix.org")
        XCTAssertTrue(item.isEdited)
    }

    func testUnknownEventsRenderAsSafePlaceholders() {
        let event = RawTimelineEvent(
            eventID: "$unknown:matrix.org",
            senderID: "@agent:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            type: "synara.agent.card",
            body: nil,
            replyToEventID: nil,
            isEdited: false,
            mediaURL: nil
        )

        XCTAssertEqual(TimelineMapper.map(event).kind, .unknown(type: "synara.agent.card"))
    }

    func testEncryptedEventsRenderAsSafePlaceholders() {
        let event = RawTimelineEvent(
            eventID: "$encrypted:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            type: "m.room.encrypted",
            body: nil,
            replyToEventID: nil,
            isEdited: false,
            mediaURL: nil
        )

        let item = TimelineMapper.map(event)

        XCTAssertEqual(item.kind, .encryptedPlaceholder)
        XCTAssertTrue(item.isEncrypted)
    }

    func testMediaEventsUseSafeResourceDescription() throws {
        let mediaURL = try XCTUnwrap(URL(string: "mxc://matrix.org/private-media-id"))
        let event = RawTimelineEvent(
            eventID: "$media:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            type: "m.room.media",
            body: "photo.jpg",
            replyToEventID: nil,
            isEdited: false,
            mediaURL: mediaURL
        )

        let item = TimelineMapper.map(event)

        guard case .mediaPlaceholder(let resource) = item.kind else {
            XCTFail("Expected media placeholder")
            return
        }
        XCTAssertEqual(resource.safeDescription, "photo.jpg")
        XCTAssertFalse(resource.safeDescription.contains("matrix.org"))
        XCTAssertTrue(resource.requiresAuthentication)
    }

    func testMockTimelineCanLoadInitialAndOlderEvents() async {
        let service = MockTimelineService()

        let initialOutcome = await service.loadInitialTimeline(roomID: "!room:matrix.org")
        guard case .loaded(let initial) = initialOutcome else {
            XCTFail("Expected loaded initial timeline")
            return
        }

        let olderOutcome = await service.loadOlderTimeline(roomID: "!room:matrix.org", before: initial[0].eventID)
        guard case .loaded(let older) = olderOutcome else {
            XCTFail("Expected loaded older timeline")
            return
        }

        XCTAssertEqual(initial.count, 6)
        XCTAssertEqual(older.count, 5)
        XCTAssertEqual(initial[0].senderID, "@mina:matrix.org")
        XCTAssertEqual(initial[0].reactions["👍"], 3)
        XCTAssertEqual(initial[4].replyToEventID, "$security:!project:matrix.org")
    }

    func testMockTimelineStreamYieldsMultipleOutcomes() async {
        let firstItem = TimelineItem(
            id: "$first",
            eventID: "$first",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("First"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
        let secondItem = TimelineItem(
            id: "$second",
            eventID: "$second",
            senderID: "@bob:matrix.org",
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(1),
            kind: .text("Second"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
        let service = MockTimelineService()
        service.updateOutcomes = [
            .loaded([firstItem]),
            .loaded([firstItem, secondItem])
        ]

        var outcomes: [TimelineLoadOutcome] = []
        for await outcome in service.timelineUpdates(roomID: "!room:matrix.org", focusedEventID: nil) {
            outcomes.append(outcome)
        }

        XCTAssertEqual(outcomes.count, 2)
        guard case .loaded(let firstBatch) = outcomes[0],
              case .loaded(let secondBatch) = outcomes[1] else {
            XCTFail("Expected loaded timeline outcomes")
            return
        }
        XCTAssertEqual(firstBatch.count, 1)
        XCTAssertEqual(secondBatch.count, 2)
    }

    func testLargeTimelineFixtureHasStableIdentity() {
        let items = TimelineFixtures.largeTimeline()

        XCTAssertEqual(items.count, 10_000)
        XCTAssertEqual(Set(items.map(\.id)).count, 10_000)
    }

    func testTimelineReplyCounterCountsRepliesByRootEvent() {
        let root = TimelineItem(
            id: "$root",
            eventID: "$root",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Root"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
        let firstReply = TimelineItem(
            id: "$reply-1",
            eventID: "$reply-1",
            senderID: "@bob:matrix.org",
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(1),
            kind: .text("Reply one"),
            replyToEventID: "$root",
            isEdited: false,
            reactions: [:]
        )
        let secondReply = TimelineItem(
            id: "$reply-2",
            eventID: "$reply-2",
            senderID: "@carol:matrix.org",
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(2),
            kind: .text("Reply two"),
            replyToEventID: "$root",
            isEdited: false,
            reactions: [:]
        )

        XCTAssertEqual(TimelineReplyCounter.replyCounts(for: [root, firstReply, secondReply]), ["$root": 2])
    }

    func testSynaraLaterListSortingPrioritizesActiveItems() {
        let now = 1_760_000_000_000
        let items: SynaraLaterContent
        do {
            items = try SynaraLaterContent(
                version: 1,
                items: [
                    "a": .init(
                        id: "a",
                        kind: .saved,
                        roomId: "!room:example.org",
                        eventId: "$one",
                        createdAt: 5,
                        dueTs: now + 3600_000,
                        completedAt: 9_000
                    ),
                    "b": .init(
                        id: "b",
                        kind: .reminder,
                        roomId: "!room:example.org",
                        eventId: "$two",
                        createdAt: 6,
                        dueTs: now - 10_000,
                        completedAt: nil
                    ),
                    "c": .init(
                        id: "c",
                        kind: .saved,
                        roomId: "!room:example.org",
                        eventId: "$three",
                        createdAt: 7,
                        dueTs: now + 1000,
                        completedAt: nil
                    )
                ]
            )
        } catch {
            XCTFail("Failed fixture: \(error)")
            return
        }

        let sorted = SynaraLaterListItem.sorted(items: items, now: now)

        XCTAssertEqual(sorted.map(\.id), ["b", "c", "a"])
    }

}
