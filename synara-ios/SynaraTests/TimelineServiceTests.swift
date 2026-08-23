@testable import Synara
import XCTest

final class TimelineServiceTests: XCTestCase {
    func testRoomTimelineScrollPolicyOnlyFollowsAnEstablishedLiveEnd() {
        XCTAssertTrue(
            RoomTimelineScrollPolicy.shouldFollowLiveAppend(
                position: .placingInitial,
                isBottomVisible: false,
                focusedEventID: nil
            )
        )
        XCTAssertTrue(
            RoomTimelineScrollPolicy.shouldFollowLiveAppend(
                position: .followingLive,
                isBottomVisible: false,
                focusedEventID: nil
            )
        )
        XCTAssertTrue(
            RoomTimelineScrollPolicy.shouldFollowLiveAppend(
                position: .readingHistory,
                isBottomVisible: true,
                focusedEventID: nil
            )
        )
        XCTAssertFalse(
            RoomTimelineScrollPolicy.shouldFollowLiveAppend(
                position: .readingHistory,
                isBottomVisible: false,
                focusedEventID: nil
            )
        )
        XCTAssertFalse(
            RoomTimelineScrollPolicy.shouldFollowLiveAppend(
                position: .focusedEvent,
                isBottomVisible: true,
                focusedEventID: "$focused"
            )
        )
    }

    func testRoomTimelineScrollPolicyStopsFollowingWhenUserDragsTowardHistory() {
        XCTAssertEqual(
            RoomTimelineScrollPolicy.positionDuringUserDrag(
                current: .followingLive,
                translationHeight: 1,
                focusedEventID: nil
            ),
            .readingHistory
        )
        XCTAssertEqual(
            RoomTimelineScrollPolicy.positionAfterUserDrag(
                isBottomVisible: false,
                focusedEventID: nil
            ),
            .readingHistory
        )
    }

    func testRoomTimelineScrollPolicyResumesOnlyAtVisibleBottom() {
        XCTAssertEqual(
            RoomTimelineScrollPolicy.positionAfterUserDrag(
                isBottomVisible: true,
                focusedEventID: nil
            ),
            .followingLive
        )
        XCTAssertEqual(
            RoomTimelineScrollPolicy.positionDuringUserDrag(
                current: .followingLive,
                translationHeight: -20,
                focusedEventID: nil
            ),
            .followingLive
        )
        XCTAssertEqual(
            RoomTimelineScrollPolicy.positionAfterUserDrag(
                isBottomVisible: true,
                focusedEventID: "$focused"
            ),
            .focusedEvent
        )
    }

    func testRoomTypingPresentationUsesCompactMatrixNames() {
        XCTAssertEqual(RoomTypingPresentation.text(for: ["@automation:matrix.org"]), "automation is typing...")
        XCTAssertEqual(
            RoomTypingPresentation.text(for: ["@automation:matrix.org", "@alice:matrix.org"]),
            "alice and automation are typing..."
        )
        XCTAssertEqual(
            RoomTypingPresentation.text(
                for: ["@automation:matrix.org", "@alice:matrix.org", "@bob:matrix.org"]
            ),
            "alice, automation, and 1 more are typing..."
        )
        XCTAssertNil(RoomTypingPresentation.text(for: []))
    }

    func testRoomTimelineSnapshotPolicyPreservesLastGoodRowsAcrossTransientEmptyUpdates() {
        XCTAssertTrue(
            RoomTimelineSnapshotPolicy.shouldPreserveCurrentSnapshot(
                currentItemCount: 40,
                incomingItemCount: 0
            )
        )
        XCTAssertFalse(
            RoomTimelineSnapshotPolicy.shouldPreserveCurrentSnapshot(
                currentItemCount: 0,
                incomingItemCount: 0
            )
        )
        XCTAssertFalse(
            RoomTimelineSnapshotPolicy.shouldPreserveCurrentSnapshot(
                currentItemCount: 40,
                incomingItemCount: 1
            )
        )
    }

    func testRoomTimelinePaginationPolicyRequiresUserInteraction() {
        XCTAssertFalse(
            RoomTimelinePaginationPolicy.shouldLoadOlderHistory(
                rowIndex: 0,
                topThreshold: 3,
                hasUserInteractedWithTimeline: false,
                hasPositionedInitialTimeline: true,
                isJumpingToLatest: false,
                isPaginating: false,
                hasReachedOldestMessages: false
            )
        )
    }

    func testRoomTimelinePaginationPolicyAllowsUserRequestedTopRowsOnlyWhenStable() {
        XCTAssertTrue(
            RoomTimelinePaginationPolicy.shouldLoadOlderHistory(
                rowIndex: 0,
                topThreshold: 3,
                hasUserInteractedWithTimeline: true,
                hasPositionedInitialTimeline: true,
                isJumpingToLatest: false,
                isPaginating: false,
                hasReachedOldestMessages: false
            )
        )
        XCTAssertFalse(
            RoomTimelinePaginationPolicy.shouldLoadOlderHistory(
                rowIndex: 4,
                topThreshold: 3,
                hasUserInteractedWithTimeline: true,
                hasPositionedInitialTimeline: true,
                isJumpingToLatest: false,
                isPaginating: false,
                hasReachedOldestMessages: false
            )
        )
        XCTAssertFalse(
            RoomTimelinePaginationPolicy.shouldLoadOlderHistory(
                rowIndex: 0,
                topThreshold: 3,
                hasUserInteractedWithTimeline: true,
                hasPositionedInitialTimeline: false,
                isJumpingToLatest: false,
                isPaginating: false,
                hasReachedOldestMessages: false
            )
        )
        XCTAssertFalse(
            RoomTimelinePaginationPolicy.shouldLoadOlderHistory(
                rowIndex: 0,
                topThreshold: 3,
                hasUserInteractedWithTimeline: true,
                hasPositionedInitialTimeline: true,
                isJumpingToLatest: true,
                isPaginating: false,
                hasReachedOldestMessages: false
            )
        )
        XCTAssertFalse(
            RoomTimelinePaginationPolicy.shouldLoadOlderHistory(
                rowIndex: 0,
                topThreshold: 3,
                hasUserInteractedWithTimeline: true,
                hasPositionedInitialTimeline: true,
                isJumpingToLatest: false,
                isPaginating: true,
                hasReachedOldestMessages: false
            )
        )
        XCTAssertFalse(
            RoomTimelinePaginationPolicy.shouldLoadOlderHistory(
                rowIndex: 0,
                topThreshold: 3,
                hasUserInteractedWithTimeline: true,
                hasPositionedInitialTimeline: true,
                isJumpingToLatest: false,
                isPaginating: false,
                hasReachedOldestMessages: true
            )
        )
    }

    func testRoomTimelineFocusPolicyOpensCaughtUpRoomsLive() {
        XCTAssertEqual(
            RoomTimelineFocusPolicy.initialMode(
                focusedEventID: nil,
                hasUnreadMessages: false,
                fullyReadEventID: "$synthetic-1:matrix.org",
                liveItems: focusPolicyItems(receiptIndex: 2)
            ),
            .live
        )
    }

    func testRoomTimelineFocusPolicyUsesNewerReceiptInsteadOfOlderFullyReadMarker() {
        XCTAssertEqual(
            RoomTimelineFocusPolicy.initialMode(
                focusedEventID: nil,
                hasUnreadMessages: true,
                fullyReadEventID: "$synthetic-0:matrix.org",
                liveItems: focusPolicyItems(receiptIndex: 2)
            ),
            .unread(markerEventID: "$synthetic-2:matrix.org")
        )
    }

    func testRoomTimelineFocusPolicyUsesNewerComparableFullyReadMarker() {
        XCTAssertEqual(
            RoomTimelineFocusPolicy.initialMode(
                focusedEventID: nil,
                hasUnreadMessages: true,
                fullyReadEventID: "$synthetic-2:matrix.org",
                liveItems: focusPolicyItems(receiptIndex: 0)
            ),
            .unread(markerEventID: "$synthetic-2:matrix.org")
        )
    }

    func testRoomTimelineFocusPolicyFallsBackLiveForMarkerOutsideBoundedGraph() {
        XCTAssertEqual(
            RoomTimelineFocusPolicy.initialMode(
                focusedEventID: nil,
                hasUnreadMessages: true,
                fullyReadEventID: "$prior-day",
                liveItems: focusPolicyItems(receiptIndex: nil)
            ),
            .live
        )
    }

    func testRoomTimelineFocusPolicyUsesReceiptInsideGraphWhenFullyReadMarkerIsOutside() {
        XCTAssertEqual(
            RoomTimelineFocusPolicy.initialMode(
                focusedEventID: nil,
                hasUnreadMessages: true,
                fullyReadEventID: "$prior-day",
                liveItems: focusPolicyItems(receiptIndex: 1)
            ),
            .unread(markerEventID: "$synthetic-1:matrix.org")
        )
    }

    func testRoomTimelineFocusPolicyTreatsReceiptAtLiveTailAsCaughtUp() {
        XCTAssertEqual(
            RoomTimelineFocusPolicy.initialMode(
                focusedEventID: nil,
                hasUnreadMessages: true,
                fullyReadEventID: "$prior-day",
                liveItems: focusPolicyItems(receiptIndex: 3)
            ),
            .live
        )
    }

    func testRoomTimelineFocusPolicyExplicitEventWinsOverUnreadState() {
        XCTAssertEqual(
            RoomTimelineFocusPolicy.initialMode(
                focusedEventID: "$deep-link",
                hasUnreadMessages: true,
                fullyReadEventID: "$synthetic-1:matrix.org",
                liveItems: focusPolicyItems(receiptIndex: 2)
            ),
            .focused(eventID: "$deep-link")
        )
    }

    func testMatrixTimelineReadReceiptPolicyMatchesOnlySignedInUser() {
        let users = ["@alice:matrix.example", "@operator:matrix.example"]

        XCTAssertTrue(
            MatrixTimelineReadReceiptPolicy.hasCurrentUserReceipt(
                readReceiptUserIDs: users,
                currentUserID: "@operator:matrix.example"
            )
        )
        XCTAssertFalse(
            MatrixTimelineReadReceiptPolicy.hasCurrentUserReceipt(
                readReceiptUserIDs: users,
                currentUserID: "@other:matrix.example"
            )
        )
        XCTAssertFalse(
            MatrixTimelineReadReceiptPolicy.hasCurrentUserReceipt(
                readReceiptUserIDs: users,
                currentUserID: nil
            )
        )
    }

    func testUnreadPresentationKeepsLiveProviderGenerationAndSnapshot() async throws {
        let service = MockTimelineService(items: focusPolicyItems(receiptIndex: 1))
        let session = RoomTimelineSession(roomID: "!room:matrix.example", service: service)
        let openedFeed = await session.open(mode: .live)
        let liveFeed = try XCTUnwrap(openedFeed)

        let unreadFeed = liveFeed.presenting(mode: .unread(markerEventID: "$synthetic-1:matrix.org"))
        let currentGeneration = await session.currentGeneration()

        XCTAssertEqual(unreadFeed.generation, liveFeed.generation)
        XCTAssertEqual(unreadFeed.initialOutcome, liveFeed.initialOutcome)
        XCTAssertEqual(unreadFeed.mode, .unread(markerEventID: "$synthetic-1:matrix.org"))
        XCTAssertTrue(liveFeed.providerIsLive)
        XCTAssertTrue(unreadFeed.providerIsLive)
        XCTAssertEqual(currentGeneration, liveFeed.generation)
    }

    func testLiveProviderResumesFollowAndAcknowledgementAtNaturalBottom() {
        let unreadMode = RoomTimelineMode.unread(markerEventID: "$marker")

        XCTAssertEqual(
            RoomTimelineProviderPresentationPolicy.modeWhenPinned(
                providerIsLive: true,
                currentMode: unreadMode
            ),
            .live
        )
        XCTAssertNil(
            RoomTimelineProviderPresentationPolicy.focusedEventID(
                providerIsLive: true,
                currentMode: unreadMode
            )
        )
    }

    func testFocusedProviderRetainsFocusedPresentationAtBottom() {
        let focusedMode = RoomTimelineMode.focused(eventID: "$focused")

        XCTAssertEqual(
            RoomTimelineProviderPresentationPolicy.modeWhenPinned(
                providerIsLive: false,
                currentMode: focusedMode
            ),
            focusedMode
        )
        XCTAssertEqual(
            RoomTimelineProviderPresentationPolicy.focusedEventID(
                providerIsLive: false,
                currentMode: focusedMode
            ),
            "$focused"
        )
    }

    func testTimelineWindowCapsInitialAndStreamingSnapshotsAtThreeHundredEvents() async throws {
        let initial = TimelineFixtures.largeTimeline(count: 400)
        let streamed = TimelineFixtures.largeTimeline(count: 500)
        let service = MockTimelineService(items: initial)
        service.updateOutcomes = [.loaded(streamed)]
        let session = RoomTimelineSession(roomID: "!room:matrix.org", service: service)

        let openedFeed = await session.open(mode: .live)
        let feed = try XCTUnwrap(openedFeed)
        let initialItems = try loadedItems(from: feed.initialOutcome)
        var iterator = feed.updates.makeAsyncIterator()
        let nextStreamedOutcome = await iterator.next()
        let streamedOutcome = try XCTUnwrap(nextStreamedOutcome)
        let streamedItems = try loadedItems(from: streamedOutcome)

        XCTAssertEqual(initialItems.count, TimelineWindowPolicy.stableEventLimit)
        XCTAssertEqual(initialItems.first?.eventID, "$synthetic-100:matrix.org")
        XCTAssertEqual(streamedItems.count, TimelineWindowPolicy.stableEventLimit)
        XCTAssertEqual(streamedItems.first?.eventID, "$synthetic-200:matrix.org")
        XCTAssertEqual(streamedItems.last?.eventID, "$synthetic-499:matrix.org")
    }

    func testTimelineSessionRejectsInitialLoadFromInvalidatedGeneration() async {
        let service = MockTimelineService(items: TimelineFixtures.largeTimeline(count: 20))
        service.loadDelayNanoseconds = 100_000_000
        let session = RoomTimelineSession(roomID: "!room:matrix.org", service: service)

        let opening = Task {
            await session.open(mode: .focused(eventID: "$synthetic-10:matrix.org"))
        }
        try? await Task.sleep(nanoseconds: 10_000_000)
        await session.invalidate()

        let staleFeed = await opening.value
        XCTAssertNil(staleFeed)
    }

    func testTimelineSessionRejectsLateUpdateFromInvalidatedGeneration() async throws {
        let service = MockTimelineService(items: TimelineFixtures.largeTimeline(count: 20))
        service.updateOutcomes = [.loaded(TimelineFixtures.largeTimeline(count: 40))]
        service.updateDelayNanoseconds = 100_000_000
        let session = RoomTimelineSession(roomID: "!room:matrix.org", service: service)
        let openedFeed = await session.open(mode: .live)
        let feed = try XCTUnwrap(openedFeed)

        let lateUpdate = Task {
            var iterator = feed.updates.makeAsyncIterator()
            return await iterator.next()
        }
        await session.invalidate()

        let rejectedOutcome = await lateUpdate.value
        XCTAssertNil(rejectedOutcome)
    }

    func testTimelineSessionPreservesHistoricalGenerationWhenJumpFails() async throws {
        let service = MockTimelineService(items: TimelineFixtures.largeTimeline(count: 30))
        service.latestOutcome = .failed("Latest unavailable")
        let session = RoomTimelineSession(roomID: "!room:matrix.org", service: service)
        let openedFeed = await session.open(mode: .focused(eventID: "$synthetic-10:matrix.org"))
        _ = try XCTUnwrap(openedFeed)
        let historicalGeneration = await session.currentGeneration()

        let transition = await session.transitionToLive()

        guard case let .failed(message) = transition else {
            XCTFail("Expected failed live transition")
            return
        }
        XCTAssertEqual(message, "Latest unavailable")
        let generationAfterFailure = await session.currentGeneration()
        XCTAssertEqual(generationAfterFailure, historicalGeneration)
    }

    func testTimelineSessionJumpReplacesHistoryWithCleanLiveProvider() async throws {
        let history = Array(TimelineFixtures.largeTimeline(count: 200).prefix(100))
        let live = Array(TimelineFixtures.largeTimeline(count: 500).suffix(75))
        let service = MockTimelineService(items: history)
        service.latestOutcome = .loaded(live)
        let session = RoomTimelineSession(roomID: "!room:matrix.org", service: service)
        let openedFeed = await session.open(mode: .focused(eventID: history[50].eventID))
        _ = try XCTUnwrap(openedFeed)

        let transition = await session.transitionToLive()

        guard case let .succeeded(feed) = transition else {
            XCTFail("Expected successful live transition")
            return
        }
        let items = try loadedItems(from: feed.initialOutcome)
        XCTAssertEqual(feed.mode, .live)
        XCTAssertEqual(items, live)
        XCTAssertFalse(items.contains(where: { $0.eventID == history.first?.eventID }))
    }

    func testTimelineSessionPaginationRetainsBoundedHistoricalWindow() async throws {
        let allItems = TimelineFixtures.largeTimeline(count: 400)
        let current = Array(allItems.suffix(300))
        let older = Array(allItems.prefix(100))
        let service = MockTimelineService(items: current)
        service.olderOutcome = .loaded(older)
        let session = RoomTimelineSession(roomID: "!room:matrix.org", service: service)
        let openedFeed = await session.open(mode: .live)
        _ = try XCTUnwrap(openedFeed)

        let loadedOutcome = await session.loadOlder(before: current[0].eventID)
        let outcome = try XCTUnwrap(loadedOutcome)
        let items = try loadedItems(from: outcome)

        XCTAssertEqual(items.count, TimelineWindowPolicy.stableEventLimit)
        XCTAssertEqual(items.first?.eventID, "$synthetic-0:matrix.org")
        XCTAssertEqual(items.last?.eventID, "$synthetic-299:matrix.org")
    }

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

    func testMatrixDisplayMarkdownCompactsLooseListsForMobileTimeline() {
        let markdown = """
        Specifically:

        - **Service:** synara-push-gateway

        - **Code:** /srv/example-service

        - **Binary:** /usr/local/bin/example-service
        """

        XCTAssertEqual(
            MatrixDisplayMarkdown.normalize(markdown),
            """
            Specifically:
            - **Service:** synara-push-gateway
            - **Code:** /srv/example-service
            - **Binary:** /usr/local/bin/example-service
            """
        )
    }

    func testMatrixDisplayMarkdownPreservesParagraphBreaks() {
        let markdown = """
        First paragraph.

        Second paragraph.
        """

        XCTAssertEqual(
            MatrixDisplayMarkdown.normalize(markdown),
            """
            First paragraph.

            Second paragraph.
            """
        )
    }

    func testMatrixHTMLRendererExtractsDetailsCodeBlocks() throws {
        let html = #"""
        <details open>
          <summary>🛠️ Tool activity (4 updates)</summary>
          <pre><code>🧠 memory: "~memory: &quot;Worker multi-model &quot;"&#10;✅ memory completed (0.0s)</code></pre>
        </details>
        <p><strong>Practical decision:</strong></p>
        <ul><li>Do not use it for active/default bounty pipelines.</li></ul>
        """#

        let block = try XCTUnwrap(MatrixHTMLRenderer.detailsBlocks(html: html).first)

        XCTAssertEqual(block.summary, "🛠️ Tool activity (4 updates)")
        XCTAssertEqual(
            block.code,
            "🧠 memory: \"~memory: \"Worker multi-model \"\"\n✅ memory completed (0.0s)"
        )
        XCTAssertTrue(block.body.isEmpty)
        XCTAssertEqual(
            MatrixHTMLRenderer.markdownExcludingDetails(body: "", html: html),
            "**Practical decision:**\n\n- Do not use it for active/default bounty pipelines."
        )
        XCTAssertEqual(
            MatrixHTMLRenderer.segments(body: "", html: html),
            [
                .details(block),
                .markdown("**Practical decision:**\n\n- Do not use it for active/default bounty pipelines."),
            ]
        )
    }

    func testMatrixHTMLRendererSegmentsCodeBlocksOutsideDetails() {
        let html = #"""
        <p>Plan:</p>
        <pre><code>let value = 1&#10;print(value)</code></pre>
        <ul><li><strong>Ship</strong></li><li>Verify</li></ul>
        """#

        XCTAssertEqual(
            MatrixHTMLRenderer.segments(body: "fallback", html: html),
            [
                .markdown("Plan:"),
                .code("let value = 1\nprint(value)"),
                .markdown("- **Ship**\n- Verify"),
            ]
        )
    }

    func testMatrixHTMLRendererCountsCodeBlockLines() {
        XCTAssertEqual(MatrixHTMLRenderer.codeLineCount(""), 1)
        XCTAssertEqual(MatrixHTMLRenderer.codeLineCount("print(1)"), 1)
        XCTAssertEqual(MatrixHTMLRenderer.codeLineCount("let value = 1\nprint(value)"), 2)
        XCTAssertEqual(MatrixHTMLRenderer.codeLineCount("one\n\nthree\n"), 3)
    }

    func testMatrixHTMLRendererSegmentsHeadingsAndBlockquotes() {
        let html = #"""
        <h2>App-agent handoff</h2>
        <p>I wrote a copyable handoff file here:</p>
        <blockquote><p>TestFlight <strong>MUST</strong> use production APNs.</p></blockquote>
        """#

        XCTAssertEqual(
            MatrixHTMLRenderer.segments(body: "fallback", html: html),
            [
                .markdown("**App-agent handoff**\n\nI wrote a copyable handoff file here:"),
                .quote("TestFlight **MUST** use production APNs."),
            ]
        )
    }

    func testMatrixHTMLRendererSegmentsTablesAsReadableRows() {
        let html = #"""
        <p>Models</p>
        <table>
          <tr><th>Stage</th><th>Actual</th><th>Proof</th></tr>
          <tr><td>Alpha</td><td>stealth/ox-alpha</td><td><code>content_chars=2702</code></td></tr>
          <tr><td>Parent</td><td>grok-4.6</td><td>orchestrator only</td></tr>
        </table>
        <p>Verdicts</p>
        """#

        XCTAssertEqual(
            MatrixHTMLRenderer.segments(body: "fallback", html: html),
            [
                .markdown("Models"),
                .table(
                    .init(rows: [
                        .init(cells: ["Stage", "Actual", "Proof"], isHeader: true),
                        .init(cells: ["Alpha", "stealth/ox-alpha", "content_chars=2702"], isHeader: false),
                        .init(cells: ["Parent", "grok-4.6", "orchestrator only"], isHeader: false),
                    ])
                ),
                .markdown("Verdicts"),
            ]
        )
    }

    func testMatrixHTMLRendererPreservesAgentRichFormattingContract() {
        let html = #"""
        <h3>Verification plan</h3>
        <p><strong>Status:</strong> <em>ready</em> with <code>TimelineServiceTests</code>.</p>
        <ol><li>Send the message</li><li>Confirm <s>plain</s> rich output</li></ol>
        <ul><li><a href="https://matrix.org">Matrix link</a></li><li>Unsafe <a href="javascript:alert(1)">link</a></li></ul>
        <hr>
        <table><tr><th>Case</th><th>Expected</th></tr><tr><td>code</td><td>preserved</td></tr></table>
        """#

        XCTAssertEqual(
            MatrixHTMLRenderer.sanitizedMarkdown(body: "fallback", html: html),
            """
            **Verification plan**

            **Status:** *ready* with `TimelineServiceTests`.

            1. Send the message
            2. Confirm ~~plain~~ rich output

            - [Matrix link](https://matrix.org)
            - Unsafe link

            ---

            | Case | Expected |
            | --- | --- |
            | code | preserved |
            """
        )
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

    func testAgentCardPayloadParserReadsBoundedProjectedPayload() throws {
        let payload = #"{"title":"Projected approval","status":"pending","summary":"Review","actions":[]}"#
        let card = try XCTUnwrap(SynaraAgentCardPayloadParser.parse(payloadJSON: payload))

        XCTAssertEqual(card.title, "Projected approval")
        XCTAssertEqual(card.status, "pending")
        XCTAssertNil(SynaraAgentCardPayloadParser.parse(payloadJSON: "ordinary text"))
        XCTAssertNil(
            SynaraAgentCardPayloadParser.parse(
                payloadJSON: String(repeating: "x", count: 200_001)
            )
        )
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
                ),
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

        if case let .agentCard(mapped) = TimelineMapper.map(event).kind {
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

        guard case let .mediaPlaceholder(resource) = item.kind else {
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
        guard case let .loaded(initial) = initialOutcome else {
            XCTFail("Expected loaded initial timeline")
            return
        }

        let olderOutcome = await service.loadOlderTimeline(roomID: "!room:matrix.org", before: initial[0].eventID)
        guard case let .loaded(older) = olderOutcome else {
            XCTFail("Expected loaded older timeline")
            return
        }

        XCTAssertEqual(initial.count, 6)
        XCTAssertEqual(older.count, 5)
        XCTAssertEqual(initial[0].senderID, "@mina:matrix.org")
        XCTAssertEqual(initial[0].reactions["👍"], 3)
        XCTAssertEqual(initial[4].replyToEventID, "$security:!room:matrix.org")
    }

    func testMockFocusedTimelineReturnsRoomSpecificMarkerContextAndSuccessor() async {
        let service = MockTimelineService()
        let roomID = "!context:matrix.org"

        let outcome = await service.loadInitialTimeline(
            roomID: roomID,
            focusedEventID: "$security:\(roomID)"
        )

        guard case let .loaded(items) = outcome else {
            XCTFail("Expected focused context")
            return
        }
        let markerIndex = items.firstIndex { $0.eventID == "$security:\(roomID)" }
        let successorIndex = items.firstIndex { $0.eventID == "$thread-reply:\(roomID)" }
        XCTAssertNotNil(markerIndex)
        XCTAssertEqual(successorIndex, markerIndex.map { $0 + 1 })
        XCTAssertFalse(items.contains { $0.eventID.contains("!project:matrix.org") })
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
            .loaded([firstItem, secondItem]),
        ]

        var outcomes: [TimelineLoadOutcome] = []
        for await outcome in service.timelineUpdates(roomID: "!room:matrix.org", focusedEventID: nil) {
            outcomes.append(outcome)
        }

        XCTAssertEqual(outcomes.count, 2)
        guard case let .loaded(firstBatch) = outcomes[0],
              case let .loaded(secondBatch) = outcomes[1]
        else {
            XCTFail("Expected loaded timeline outcomes")
            return
        }
        XCTAssertEqual(firstBatch.count, 1)
        XCTAssertEqual(secondBatch.count, 2)
    }

    func testMockTypingStreamYieldsLiveUpdates() async {
        let service = MockTimelineService()
        service.typingUserUpdates = [
            ["@automation:matrix.org"],
            [],
        ]

        var updates: [[String]] = []
        for await userIDs in service.typingUsers(roomID: "!room:matrix.org") {
            updates.append(userIDs)
        }

        XCTAssertEqual(updates, [["@automation:matrix.org"], []])
    }

    func testLargeTimelineFixtureHasStableIdentity() {
        let items = TimelineFixtures.largeTimeline()

        XCTAssertEqual(items.count, 10000)
        XCTAssertEqual(Set(items.map(\.id)).count, 10000)
    }

    func testPendingMessageFactoryMarksLocalDeliveryState() {
        let pending = TimelineItem.pendingMessage(
            localID: "$pending-test",
            body: "Hello world",
            senderID: "@alice:matrix.org",
            replyToEventID: "$parent:matrix.org"
        )

        XCTAssertTrue(pending.isLocalPending)
        XCTAssertEqual(pending.deliveryStatus, .sending)
        XCTAssertNil(pending.serverEventID)
        XCTAssertEqual(pending.kind, .text("Hello world"))
        XCTAssertEqual(pending.replyToEventID, "$parent:matrix.org")
    }

    func testServerAndTransactionTimelineIdentitiesRemainDistinct() {
        let server = TimelineItem(
            id: "$server:matrix.org",
            eventID: "$server:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Confirmed"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
        let transaction = TimelineItem(
            id: "transaction-123",
            eventID: "transaction-123",
            serverEventID: nil,
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Local echo"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:],
            deliveryStatus: .sent
        )

        XCTAssertEqual(server.serverEventID, "$server:matrix.org")
        XCTAssertNil(transaction.serverEventID)
        XCTAssertTrue(transaction.isLocalPending)
    }

    func testAvatarEnrichmentPreservesTransactionIdentityAndDeliveryState() throws {
        let transactionItem = TimelineItem(
            id: "txn-local-echo",
            eventID: "txn-local-echo",
            serverEventID: nil,
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Sending"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:],
            deliveryStatus: .sent
        )
        let avatarURL = try XCTUnwrap(URL(string: "mxc://matrix.org/alice"))

        let enrichedItem = transactionItem.withSenderAvatarURL(avatarURL)

        XCTAssertEqual(enrichedItem.senderAvatarURL, avatarURL)
        XCTAssertNil(enrichedItem.serverEventID)
        XCTAssertEqual(enrichedItem.deliveryStatus, .sent)
        XCTAssertTrue(enrichedItem.isLocalPending)
    }

    func testPendingReconcilerPreservesAuthoritativeServerVectorOrder() {
        func item(_ id: String, offset: TimeInterval) -> Synara.TimelineItem {
            Synara.TimelineItem(
                id: id,
                eventID: id,
                senderID: "@server:matrix.org",
                timestamp: TimelineFixtures.baseDate.addingTimeInterval(offset),
                kind: .text(id),
                replyToEventID: nil,
                isEdited: false,
                reactions: [:]
            )
        }
        let serverItems = [item("$one", offset: 100), item("$two", offset: 0), item("$three", offset: 50)]
        let pending = TimelineItem.pendingMessage(
            localID: "$pending-order",
            body: "Pending",
            senderID: "@alice:matrix.org",
            replyToEventID: nil,
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(25)
        )

        let merged = TimelinePendingReconciler.merge(
            streamItems: serverItems,
            localItems: [pending],
            currentUserID: "@alice:matrix.org"
        )

        XCTAssertEqual(merged.filter { !$0.isLocalPending }.map(\.eventID), ["$one", "$two", "$three"])
    }

    func testTimelineCollectorAndInteractiveFreshnessPoliciesAreBounded() {
        XCTAssertEqual(MatrixTimelineCollectorPolicy.retainedSuffixCount(itemCount: 5000, limit: 1200), 1200)
        XCTAssertEqual(MatrixTimelineCollectorPolicy.droppedPrefixCount(itemCount: 5000, limit: 1200), 3800)
        XCTAssertEqual(
            MatrixTimelineCollectorPolicy.droppedPrefixCountAfterPopBack(
                retainedCount: 0,
                droppedPrefixCount: 25
            ),
            24
        )

        let now = Date()
        XCTAssertFalse(MatrixInteractiveFreshnessPolicy.shouldPerformSync(
            hasActiveSyncService: true,
            lastSuccessfulSyncAt: nil,
            now: now,
            maximumAge: 2
        ))
        XCTAssertFalse(MatrixInteractiveFreshnessPolicy.shouldPerformSync(
            hasActiveSyncService: false,
            lastSuccessfulSyncAt: now.addingTimeInterval(-1),
            now: now,
            maximumAge: 2
        ))
        XCTAssertTrue(MatrixInteractiveFreshnessPolicy.shouldPerformSync(
            hasActiveSyncService: false,
            lastSuccessfulSyncAt: now.addingTimeInterval(-3),
            now: now,
            maximumAge: 2
        ))
        XCTAssertTrue(MatrixInteractiveFreshnessPolicy.ownsInstalledOperation(
            installedGeneration: 4,
            currentGeneration: 4
        ))
        XCTAssertFalse(MatrixInteractiveFreshnessPolicy.ownsInstalledOperation(
            installedGeneration: 4,
            currentGeneration: 5
        ))
    }

    func testPendingReconcilerDropsMatchedLocalEchoes() {
        let pending = TimelineItem.pendingMessage(
            localID: "$pending-test",
            body: "Ship it",
            senderID: "@alice:matrix.org",
            replyToEventID: nil,
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(30)
        )
        let confirmed = TimelineItem(
            id: "$server:matrix.org",
            eventID: "$server:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(31),
            kind: .text("Ship it"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )

        let merged = TimelinePendingReconciler.merge(
            streamItems: [confirmed],
            localItems: [pending],
            currentUserID: "@alice:matrix.org"
        )

        XCTAssertEqual(merged.count, 1)
        XCTAssertEqual(merged.first?.eventID, "$server:matrix.org")
        XCTAssertNil(merged.first?.deliveryStatus)
    }

    func testPendingReconcilerDropsMatchedSentLocalEchoes() {
        let pending = TimelineItem.pendingMessage(
            localID: "$pending-sent",
            body: "Acknowledged locally",
            senderID: "@alice:matrix.org",
            replyToEventID: nil,
            deliveryStatus: .sent,
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(35)
        )
        let confirmed = TimelineItem(
            id: "$server-sent:matrix.org",
            eventID: "$server-sent:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(36),
            kind: .text("Acknowledged locally"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )

        let merged = TimelinePendingReconciler.merge(
            streamItems: [confirmed],
            localItems: [pending],
            currentUserID: "@alice:matrix.org"
        )

        XCTAssertEqual(merged.count, 1)
        XCTAssertEqual(merged.first?.eventID, "$server-sent:matrix.org")
        XCTAssertNil(merged.first?.deliveryStatus)
    }

    func testPendingReconcilerKeepsFailedAndUnmatchedPendingItems() {
        let failed = TimelineItem.pendingMessage(
            localID: "$pending-failed",
            body: "Retry me",
            senderID: "@alice:matrix.org",
            replyToEventID: nil,
            deliveryStatus: .failed,
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(40)
        )
        let stillSending = TimelineItem.pendingMessage(
            localID: "$pending-open",
            body: "Still sending",
            senderID: "@alice:matrix.org",
            replyToEventID: nil,
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(50)
        )
        let server = TimelineItem(
            id: "$other:matrix.org",
            eventID: "$other:matrix.org",
            senderID: "@bob:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Earlier"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )

        let merged = TimelinePendingReconciler.merge(
            streamItems: [server],
            localItems: [failed, stillSending],
            currentUserID: "@alice:matrix.org"
        )

        XCTAssertEqual(merged.count, 3)
        XCTAssertTrue(merged.contains(where: { $0.id == failed.id && $0.deliveryStatus == .failed }))
        XCTAssertTrue(merged.contains(where: { $0.id == stillSending.id && $0.deliveryStatus == .sending }))
    }

    func testPendingReconcilerKeepsQueuedItemsUntilConfirmed() {
        let queued = TimelineItem.pendingMessage(
            localID: "$pending-queued",
            body: "Waiting for network",
            senderID: "@alice:matrix.org",
            replyToEventID: nil,
            deliveryStatus: .queued,
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(40)
        )
        let confirmedSameBody = TimelineItem(
            id: "$server:matrix.org",
            eventID: "$server:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(41),
            kind: .text("Waiting for network"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )

        let merged = TimelinePendingReconciler.merge(
            streamItems: [confirmedSameBody],
            localItems: [queued],
            currentUserID: "@alice:matrix.org"
        )

        XCTAssertEqual(merged.count, 2)
        XCTAssertTrue(merged.contains(where: { $0.id == queued.id && $0.deliveryStatus == .queued }))
    }

    func testPendingReconcilerCombinesStoredPendingStatuses() {
        let local = TimelineItem.pendingMessage(
            localID: "$pending-local",
            body: "Hello",
            senderID: "@alice:matrix.org",
            replyToEventID: nil,
            deliveryStatus: .sending
        )
        let stored = TimelineItem.pendingMessage(
            localID: "$pending-local",
            body: "Hello",
            senderID: "@alice:matrix.org",
            replyToEventID: nil,
            deliveryStatus: .queued
        )
        let extra = TimelineItem.pendingMessage(
            localID: "$pending-extra",
            body: "Later",
            senderID: "@alice:matrix.org",
            replyToEventID: nil,
            deliveryStatus: .failed
        )

        let combined = TimelinePendingReconciler.combining(
            localItems: [local],
            storedPending: [stored, extra]
        )

        XCTAssertEqual(combined.count, 2)
        XCTAssertEqual(combined.first(where: { $0.id == local.id })?.deliveryStatus, .queued)
        XCTAssertTrue(combined.contains(where: { $0.id == extra.id && $0.deliveryStatus == .failed }))
    }

    func testMessageBodyIsTextOnly() {
        let text = TimelineItem.pendingMessage(
            body: "Retry me",
            senderID: "@alice:matrix.org",
            replyToEventID: nil
        )
        let formatted = TimelineItem.pendingMessage(
            body: "Retry me",
            formattedBody: "<p>Retry me</p>",
            senderID: "@alice:matrix.org",
            replyToEventID: nil
        )
        let media = TimelineItem(
            id: "$media",
            eventID: "$media",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .mediaPlaceholder(
                MediaResource(
                    id: "$media",
                    filename: "photo.jpg",
                    authenticatedURL: URL(string: "mxc://matrix.org/photo")!,
                    requiresAuthentication: true
                )
            ),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )

        XCTAssertEqual(TimelinePendingReconciler.messageBody(for: text), "Retry me")
        XCTAssertEqual(TimelinePendingReconciler.messageBody(for: formatted), "Retry me")
        XCTAssertEqual(TimelinePendingReconciler.formattedBody(for: formatted), "<p>Retry me</p>")
        XCTAssertNil(TimelinePendingReconciler.messageBody(for: media))
        XCTAssertNil(TimelinePendingReconciler.formattedBody(for: text))
    }

    func testServerWindowReplacementDropsHistoricalContext() {
        let older = TimelineItem(
            id: "$older:matrix.org",
            eventID: "$older:matrix.org",
            senderID: "@bob:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Older"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
        let readMarkerContext = TimelineItem(
            id: "$read-marker:matrix.org",
            eventID: "$read-marker:matrix.org",
            senderID: "@bob:matrix.org",
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(60),
            kind: .text("Read marker context"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
        let latest = TimelineItem(
            id: "$latest:matrix.org",
            eventID: "$latest:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(120),
            kind: .text("Latest"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )

        let merged = TimelineWindowPolicy.replacingServerWindow([latest])

        XCTAssertEqual(merged.map(\.eventID), ["$latest:matrix.org"])
        XCTAssertFalse(merged.contains(older))
        XCTAssertFalse(merged.contains(readMarkerContext))
    }

    func testServerWindowReplacementUsesIncomingEventRevision() {
        let original = TimelineItem(
            id: "$event:matrix.org",
            eventID: "$event:matrix.org",
            senderID: "@bob:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Original"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
        let updated = TimelineItem(
            id: "$event:matrix.org",
            eventID: "$event:matrix.org",
            senderID: "@bob:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Edited"),
            replyToEventID: nil,
            isEdited: true,
            reactions: ["👍": 1]
        )

        let merged = TimelineWindowPolicy.replacingServerWindow([updated])

        XCTAssertEqual(merged.count, 1)
        XCTAssertNotEqual(merged.first, original)
        XCTAssertEqual(merged.first?.kind, .text("Edited"))
        XCTAssertEqual(merged.first?.isEdited, true)
        XCTAssertEqual(merged.first?.reactions["👍"], 1)
    }

    func testBoundedServerWindowKeepsOnlyUnmatchedLocalEchoes() {
        let pending = TimelineItem.pendingMessage(
            localID: "$pending-local",
            body: "Ship it",
            senderID: "@alice:matrix.org",
            replyToEventID: nil,
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(30)
        )
        let confirmed = TimelineItem(
            id: "$server:matrix.org",
            eventID: "$server:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(31),
            kind: .text("Ship it"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )

        let merged = TimelinePendingReconciler.merge(
            streamItems: TimelineWindowPolicy.replacingServerWindow([confirmed]),
            localItems: [pending],
            currentUserID: "@alice:matrix.org"
        )

        XCTAssertEqual(merged.count, 1)
        XCTAssertEqual(merged.first?.eventID, "$server:matrix.org")
        XCTAssertNil(merged.first?.deliveryStatus)
    }

    private func loadedItems(from outcome: TimelineLoadOutcome) throws -> [Synara.TimelineItem] {
        let items: [Synara.TimelineItem]?
        if case let .loaded(loadedItems) = outcome {
            items = loadedItems
        } else {
            items = nil
        }
        return try XCTUnwrap(items)
    }

    private func focusPolicyItems(receiptIndex: Int?) -> [Synara.TimelineItem] {
        TimelineFixtures.largeTimeline(count: 4).enumerated().map { index, item in
            Synara.TimelineItem(
                id: item.id,
                eventID: item.eventID,
                serverEventID: item.serverEventID,
                senderID: item.senderID,
                senderAvatarURL: item.senderAvatarURL,
                timestamp: item.timestamp,
                kind: item.kind,
                replyToEventID: item.replyToEventID,
                isEdited: item.isEdited,
                reactions: item.reactions,
                isEncrypted: item.isEncrypted,
                deliveryStatus: item.deliveryStatus,
                hasCurrentUserReadReceipt: index == receiptIndex
            )
        }
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
                        dueTs: now + 3_600_000,
                        completedAt: 9000
                    ),
                    "b": .init(
                        id: "b",
                        kind: .reminder,
                        roomId: "!room:example.org",
                        eventId: "$two",
                        createdAt: 6,
                        dueTs: now - 10000,
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
                    ),
                ]
            )
        } catch {
            XCTFail("Failed fixture: \(error)")
            return
        }

        let sorted = SynaraLaterListItem.sorted(items: items, now: now)

        XCTAssertEqual(sorted.map(\.id), ["b", "c", "a"])
    }

    func testLaterDueUrgencyClassifiesOverdueSoonAndFuture() {
        let now = 1_760_000_000_000

        XCTAssertEqual(
            LaterDueUrgency.classify(dueTs: now - 1, isCompleted: false, now: now),
            .overdue
        )
        XCTAssertEqual(
            LaterDueUrgency.classify(dueTs: now + 3_600_000, isCompleted: false, now: now),
            .dueSoon
        )
        XCTAssertEqual(
            LaterDueUrgency.classify(dueTs: now + (25 * 60 * 60 * 1000), isCompleted: false, now: now),
            .future
        )
        XCTAssertEqual(
            LaterDueUrgency.classify(dueTs: now + 3_600_000, isCompleted: true, now: now),
            .none
        )
    }

    func testLaterContentCompletingItemSetsCompletedAt() throws {
        let content = try SynaraLaterContent(
            version: 1,
            items: [
                "saved": .init(
                    id: "saved",
                    kind: .saved,
                    roomId: "!room:example.org",
                    eventId: "$one",
                    createdAt: 1
                ),
            ]
        )

        let completed = try content.completingItem(id: "saved", at: 9999)

        XCTAssertEqual(completed.items["saved"]?.completedAt, 9999)
    }

    func testMockLaterServiceCompletesActiveItem() async {
        let service = MockLaterService(
            items: [
                SynaraLaterListItem(
                    id: "saved",
                    roomID: "!room:example.org",
                    eventID: "$one",
                    kind: .saved,
                    dueTs: nil,
                    completedAt: nil,
                    createdAt: 1,
                    isCompleted: false
                ),
            ],
            now: { 9999 }
        )

        let result = await service.completeItem(id: "saved")
        let loaded = await service.loadItems()

        XCTAssertEqual(result, .success(true))
        guard case let .success((items, _)) = loaded else {
            XCTFail("Expected loaded items")
            return
        }
        XCTAssertEqual(items.first?.completedAt, 9999)
        XCTAssertTrue(items.first?.isCompleted == true)
    }

    func testTimelineSearchFilterMatchesMessageBody() {
        let items = [
            TimelineItem(
                id: "$one",
                eventID: "$one",
                senderID: "@mina:matrix.org",
                timestamp: TimelineFixtures.baseDate,
                kind: .text("Ship the release notes"),
                replyToEventID: nil,
                isEdited: false,
                reactions: [:]
            ),
            TimelineItem(
                id: "$two",
                eventID: "$two",
                senderID: "@alex:matrix.org",
                timestamp: TimelineFixtures.baseDate.addingTimeInterval(1),
                kind: .text("Review the design draft"),
                replyToEventID: nil,
                isEdited: false,
                reactions: [:]
            ),
        ]

        let filtered = TimelineSearchFilter.applySearchQuery("release", to: items)

        XCTAssertEqual(filtered.map(\.eventID), ["$one"])
    }

    func testTimelineSearchFilterMatchesSenderID() {
        let items = [
            TimelineItem(
                id: "$one",
                eventID: "$one",
                senderID: "@mina:matrix.org",
                timestamp: TimelineFixtures.baseDate,
                kind: .text("Hello"),
                replyToEventID: nil,
                isEdited: false,
                reactions: [:]
            ),
        ]

        let filtered = TimelineSearchFilter.applySearchQuery("mina", to: items)

        XCTAssertEqual(filtered.map(\.eventID), ["$one"])
    }

    func testTimelineSearchFilterReturnsAllItemsForEmptyQuery() {
        let items = TimelineFixtures.largeTimeline(count: 3)

        let filtered = TimelineSearchFilter.applySearchQuery("   ", to: items)

        XCTAssertEqual(filtered, items)
    }
}
