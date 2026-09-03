@testable import Synara
import Foundation
import XCTest

private struct MessageFormatCorpus: Decodable {
    let schemaVersion: Int
    let presentationFormattedBodyMaxBytes: Int
    let coverage: [String: [String]]
    let cases: [MessageFormatCorpusCase]
}

private struct MessageFormatCorpusCase: Decodable {
    struct Generator: Decodable {
        let kind: String
        let tag: String?
        let count: Int
        let text: String?
        let prefix: String?
        let unit: String?
        let suffix: String?
    }

    struct Expectation: Decodable {
        let accepted: Bool
        let textContains: [String]
        let textExcludes: [String]
        let linkSchemes: [String]
        let containsSpoiler: Bool
        let forbiddenFragments: [String]
        let semanticKinds: [String]?
        let mentionTargets: [String]?
        let spoilerReasons: [String]?
        let inlineCode: [String]?
        let codeBlocks: [String]?
        let orderedListStarts: [Int]?
    }

    let id: String
    let body: String
    let formattedBody: String
    let generator: Generator?
    let expect: Expectation

    var expandedFormattedBody: String {
        guard let generator else { return formattedBody }
        switch generator.kind {
        case "nestedTag":
            let tag = generator.tag ?? "span"
            return String(repeating: "<\(tag)>", count: generator.count)
                + (generator.text ?? "")
                + String(repeating: "</\(tag)>", count: generator.count)
        case "repeatedText":
            return (generator.prefix ?? "")
                + String(repeating: generator.unit ?? "", count: generator.count)
                + (generator.suffix ?? "")
        default:
            return formattedBody
        }
    }
}

private func matrixMessageHasSemanticKind(
    _ kind: String,
    segments: [MatrixHTMLRenderer.Segment],
    projection: MatrixHTMLRenderer.SelectionProjection
) -> Bool {
    switch kind {
    case "bold":
        return projection.richText.runs.contains { $0.style.contains(.bold) }
    case "heading":
        return segments.contains { if case .heading = $0 { return true }; return false }
    case "inlineCode":
        return projection.richText.runs.contains { $0.style.contains(.code) }
    case "orderedList":
        return projection.richText.plainText.range(
            of: #"(?m)^(?:  )*-?\d+\. "#,
            options: .regularExpression
        ) != nil
    case "preformattedCode":
        return matrixCodeBlocks(in: segments).isEmpty == false
    case "spoiler":
        return projection.containsSpoilers
    case "strikethrough":
        return projection.richText.runs.contains { $0.style.contains(.strikethrough) }
    case "table":
        return segments.contains { if case .table = $0 { return true }; return false }
    case "unorderedList":
        return projection.richText.plainText.range(
            of: #"(?m)^(?:  )*• "#,
            options: .regularExpression
        ) != nil
    default:
        return false
    }
}

private func matrixCodeBlocks(in segments: [MatrixHTMLRenderer.Segment]) -> [String] {
    segments.flatMap { segment -> [String] in
        switch segment {
        case let .code(block):
            return [block.code]
        case let .details(block):
            return matrixCodeBlocks(in: block.content)
        default:
            return []
        }
    }
}

private func matrixSpoilerReasons(in segments: [MatrixHTMLRenderer.Segment]) -> [String] {
    segments.flatMap { segment -> [String] in
        switch segment {
        case let .spoiler(block):
            return block.reason.map { [$0] } ?? []
        case let .inline(group):
            return group.pieces.compactMap { piece in
                if case let .spoiler(block) = piece { return block.reason }
                return nil
            }
        case let .details(block):
            return matrixSpoilerReasons(in: block.content)
        case let .table(block):
            let captionReasons: [String] = block.captionInlineContent?.pieces.compactMap { piece in
                if case let .spoiler(spoiler) = piece { return spoiler.reason }
                return nil
            } ?? []
            let cellReasons: [String] = block.rows.flatMap(\.cells).flatMap { cell -> [String] in
                cell.inlineContent?.pieces.compactMap { piece in
                    if case let .spoiler(spoiler) = piece { return spoiler.reason }
                    return nil
                } ?? []
            }
            return captionReasons + cellReasons
        default:
            return []
        }
    }
}

private func matrixRenderedOrderedListOrdinals(in plainText: String) -> [Int] {
    guard let regex = try? NSRegularExpression(
        pattern: #"(?m)^(?:  )*(-?\d+)\. "#
    ) else { return [] }
    let source = plainText as NSString
    return regex.matches(
        in: plainText,
        range: NSRange(location: 0, length: source.length)
    ).compactMap {
        guard $0.numberOfRanges == 2 else { return nil }
        return Int(source.substring(with: $0.range(at: 1)))
    }
}

private func matrixRenderedListLines(in plainText: String) -> [String] {
    plainText.split(separator: "\n", omittingEmptySubsequences: false).compactMap { line in
        let rendered = String(line)
        guard rendered.range(
            of: #"^(?:  )*(?:(?:-?\d+)\.|•) .+$"#,
            options: .regularExpression
        ) != nil else { return nil }
        return rendered
    }
}

final class TimelineServiceTests: XCTestCase {
    private actor RecoveryInvocationCounter {
        private(set) var value = 0

        func increment() {
            value += 1
        }
    }

    func testSharedMatrixAndHermesMessageFormatCorpus() throws {
        let repositoryRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let corpusURL = repositoryRoot
            .appendingPathComponent("docs/future-projects/rust-ownership-expansion/fixtures/message-format/corpus.json")
        let corpus = try JSONDecoder().decode(
            MessageFormatCorpus.self,
            from: Data(contentsOf: corpusURL)
        )

        XCTAssertEqual(corpus.schemaVersion, 1)
        XCTAssertEqual(corpus.presentationFormattedBodyMaxBytes, 256 * 1_024)
        let fixtureIDs = Set(corpus.cases.map(\.id))
        XCTAssertEqual(fixtureIDs.count, corpus.cases.count)
        XCTAssertEqual(
            Set(corpus.coverage.keys),
            Set([
                "executable-content",
                "formatted-reply-fallback",
                "inline-code",
                "links",
                "lists",
                "malformed-html",
                "mentions",
                "plaintext-fallback",
                "preformatted-code",
                "presentation-size-boundary",
                "remote-resource-blocking",
                "spoilers",
                "tables",
            ]),
            "shared corpus coverage register drifted"
        )
        for (area, ids) in corpus.coverage {
            XCTAssertFalse(ids.isEmpty, "coverage area has no fixtures: \(area)")
            for id in ids {
                XCTAssertTrue(fixtureIDs.contains(id), "unknown \(area) fixture: \(id)")
            }
        }

        for fixture in corpus.cases {
            let html = fixture.expandedFormattedBody
            let sanitized = MatrixHTMLRenderer.sanitizedHTMLForClipboard(html: html)
            XCTAssertEqual(sanitized != nil, fixture.expect.accepted, fixture.id)
            let segments = MatrixHTMLRenderer.segments(body: fixture.body, html: html)
            let projection = MatrixHTMLRenderer.selectionProjection(
                body: fixture.body,
                html: html,
                revealingSpoilers: true
            )
            let text = projection.richText.plainText

            for expected in fixture.expect.textContains {
                XCTAssertTrue(text.contains(expected), "\(fixture.id): missing text \(expected)")
            }
            for excluded in fixture.expect.textExcludes {
                XCTAssertFalse(text.contains(excluded), "\(fixture.id): exposed text \(excluded)")
            }
            let schemes = projection.richText.runs.compactMap { $0.link?.scheme?.lowercased() }.sorted()
            XCTAssertEqual(schemes, fixture.expect.linkSchemes.sorted(), fixture.id)
            XCTAssertEqual(projection.containsSpoilers, fixture.expect.containsSpoiler, fixture.id)
            for forbidden in fixture.expect.forbiddenFragments {
                XCTAssertFalse((sanitized ?? "").contains(forbidden), "\(fixture.id): retained \(forbidden)")
            }
            let expectedKinds = fixture.expect.semanticKinds ?? []
            for kind in expectedKinds {
                XCTAssertTrue(
                    matrixMessageHasSemanticKind(
                        kind,
                        segments: segments,
                        projection: projection
                    ),
                    "\(fixture.id): missing semantic kind \(kind)"
                )
            }
            XCTAssertEqual(
                projection.richText.runs.compactMap(\.link).map(\.absoluteString)
                    .filter { $0.hasPrefix("https://matrix.to/#/@") },
                fixture.expect.mentionTargets ?? [],
                "\(fixture.id): mention targets"
            )
            XCTAssertEqual(
                matrixSpoilerReasons(in: segments),
                fixture.expect.spoilerReasons ?? [],
                "\(fixture.id): spoiler reasons"
            )
            for expected in fixture.expect.inlineCode ?? [] {
                XCTAssertTrue(
                    projection.richText.runs.contains {
                        $0.text == expected && $0.style.contains(.code)
                    },
                    "\(fixture.id): missing inline code \(expected)"
                )
            }
            XCTAssertEqual(
                matrixCodeBlocks(in: segments),
                fixture.expect.codeBlocks ?? [],
                "\(fixture.id): code blocks"
            )
            let renderedOrdinals = matrixRenderedOrderedListOrdinals(
                in: projection.richText.plainText
            )
            for expectedStart in fixture.expect.orderedListStarts ?? [] {
                XCTAssertTrue(
                    renderedOrdinals.contains(expectedStart),
                    "\(fixture.id): missing rendered ordered-list start \(expectedStart)"
                )
            }
            if expectedKinds.contains("orderedList") || expectedKinds.contains("unorderedList") {
                XCTAssertEqual(
                    matrixRenderedListLines(in: projection.richText.plainText),
                    matrixRenderedListLines(in: fixture.body),
                    "\(fixture.id): rendered list ordinals, nesting, or bullets drifted"
                )
            }
        }
    }

    func testTimelineRecoveryGateCoalescesConcurrentRecoveryForOneRoom() async {
        let gate = SharedCoreTimelineRecoveryGate()
        let counter = RecoveryInvocationCounter()

        async let first = gate.run(roomID: "!room:example.org") {
            await counter.increment()
            try? await Task.sleep(nanoseconds: 50_000_000)
            return TimelineLoadOutcome.empty
        }
        async let second = gate.run(roomID: "!room:example.org") {
            await counter.increment()
            try? await Task.sleep(nanoseconds: 50_000_000)
            return TimelineLoadOutcome.empty
        }

        let outcomes = await (first, second)
        let recoveryCount = await counter.value
        XCTAssertEqual(outcomes.0, .empty)
        XCTAssertEqual(outcomes.1, .empty)
        XCTAssertEqual(recoveryCount, 1)
    }

    func testTimelineAvailabilityFailureClearsOnlyAfterSuccessfulRecovery() {
        let failure = TimelineLoadFailure(
            kind: .temporarilyUnavailable,
            diagnosticCode: "timeline-temporarily-unavailable"
        )
        var availability = RoomTimelineAvailabilityState()

        availability.recordFailure(failure, preservingRows: true)
        XCTAssertEqual(availability.failure, failure)

        availability.recordSuccess()
        XCTAssertNil(availability.failure)
    }

    func testTimelineAvailabilityDoesNotDuplicateFullScreenFailure() {
        let failure = TimelineLoadFailure(
            kind: .viewUnavailable,
            diagnosticCode: "v-timeline-view-not-open"
        )
        var availability = RoomTimelineAvailabilityState()

        availability.recordFailure(failure, preservingRows: false)

        XCTAssertNil(availability.failure)
    }
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

    func testRoomTimelineFailurePresentationExposesStaticRetryWhilePreservingRows() {
        let failure = TimelineLoadFailure(
            kind: .temporarilyUnavailable,
            diagnosticCode: "p4-s6-open-failed"
        )

        XCTAssertEqual(
            RoomTimelineFailurePresentationPolicy.retryMessage(
                for: failure,
                preservedItemCount: 40
            ),
            "Messages are temporarily unavailable. Try again."
        )
        XCTAssertNil(
            RoomTimelineFailurePresentationPolicy.retryMessage(
                for: failure,
                preservedItemCount: 0
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

    func testTimelineSessionSuppressesIdenticalSnapshotsBeforePublishingChange() async throws {
        let initial = TimelineFixtures.largeTimeline(count: 20)
        let changed = TimelineFixtures.largeTimeline(count: 21)
        let service = MockTimelineService(items: initial)
        service.updateOutcomes = [.loaded(initial), .loaded(changed)]
        let session = RoomTimelineSession(roomID: "!room:matrix.org", service: service)

        let openedFeed = await session.open(mode: .live)
        let feed = try XCTUnwrap(openedFeed)
        var iterator = feed.updates.makeAsyncIterator()
        let nextOutcome = await iterator.next()
        let outcome = try XCTUnwrap(nextOutcome)
        let items = try loadedItems(from: outcome)
        let finishedOutcome = await iterator.next()

        XCTAssertEqual(items, changed)
        XCTAssertNil(finishedOutcome)
    }

    func testTimelineSessionPublishesIdenticalSnapshotAfterFailureAsRecoveryHeartbeat() async throws {
        let initial = TimelineFixtures.largeTimeline(count: 20)
        let failure = TimelineLoadFailure(
            kind: .temporarilyUnavailable,
            diagnosticCode: "timeline-temporarily-unavailable"
        )
        let service = MockTimelineService(items: initial)
        service.updateOutcomes = [.failed(failure), .loaded(initial)]
        let session = RoomTimelineSession(roomID: "!room:matrix.org", service: service)

        let openedFeed = await session.open(mode: .live)
        let feed = try XCTUnwrap(openedFeed)
        var iterator = feed.updates.makeAsyncIterator()
        let failedUpdate = await iterator.next()
        let recoveredUpdate = await iterator.next()
        let finishedUpdate = await iterator.next()

        XCTAssertEqual(failedUpdate, .failed(failure))
        XCTAssertEqual(recoveredUpdate, .loaded(initial))
        XCTAssertNil(finishedUpdate)
    }

    func testTimelineSessionPublishesRecoveryHeartbeatAfterPaginationFailure() async throws {
        let initial = TimelineFixtures.largeTimeline(count: 20)
        let failure = TimelineLoadFailure(
            kind: .temporarilyUnavailable,
            diagnosticCode: "timeline-pagination-unavailable"
        )
        let service = MockTimelineService(items: initial)
        service.olderOutcome = .failed(failure)
        let session = RoomTimelineSession(roomID: "!room:matrix.org", service: service)
        let openedFeed = await session.open(mode: .live)
        let feed = try XCTUnwrap(openedFeed)

        let paginationOutcome = await session.loadOlder(before: initial[0].eventID)
        var iterator = feed.updates.makeAsyncIterator()
        let recoveredUpdate = await iterator.next()

        XCTAssertEqual(paginationOutcome, .failed(failure))
        XCTAssertEqual(recoveredUpdate, .loaded(initial))
    }

    func testTimelineSessionPublishesRecoveryHeartbeatAfterFailedOrEmptyLiveTransition() async throws {
        let initial = TimelineFixtures.largeTimeline(count: 20)
        let failure = TimelineLoadFailure(
            kind: .temporarilyUnavailable,
            diagnosticCode: "timeline-live-unavailable"
        )

        for latestOutcome in [TimelineLoadOutcome.failed(failure), .empty] {
            let service = MockTimelineService(items: initial)
            service.latestOutcome = latestOutcome
            let session = RoomTimelineSession(roomID: "!room:matrix.org", service: service)
            let openedFeed = await session.open(mode: .focused(eventID: initial[5].eventID))
            let feed = try XCTUnwrap(openedFeed)

            _ = await session.transitionToLive()
            var iterator = feed.updates.makeAsyncIterator()
            let recoveredUpdate = await iterator.next()

            XCTAssertEqual(recoveredUpdate, .loaded(initial))
        }
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
        let failure = TimelineLoadFailure(
            kind: .temporarilyUnavailable,
            diagnosticCode: "timeline-test-unavailable"
        )
        service.latestOutcome = .failed(failure)
        let session = RoomTimelineSession(roomID: "!room:matrix.org", service: service)
        let openedFeed = await session.open(mode: .focused(eventID: "$synthetic-10:matrix.org"))
        _ = try XCTUnwrap(openedFeed)
        let historicalGeneration = await session.currentGeneration()

        let transition = await session.transitionToLive()

        guard case let .failed(receivedFailure) = transition else {
            XCTFail("Expected failed live transition")
            return
        }
        XCTAssertEqual(receivedFailure, failure)
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

        let richText = MatrixHTMLRenderer.richText(
            body: "- **Ship it**\n- Review fallback",
            html: #"<ul><li><strong>Ship it</strong></li><li>Review fallback</li></ul>"#
        )
        XCTAssertEqual(richText.plainText, "• Ship it\n• Review fallback")
        XCTAssertTrue(richText.runs.first { $0.text == "Ship it" }?.style.contains(.bold) == true)
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
            block.code?.code,
            "🧠 memory: \"~memory: \"Worker multi-model \"\"\n✅ memory completed (0.0s)"
        )
        XCTAssertNil(block.code?.language)
        XCTAssertTrue(block.body.isEmpty)
        XCTAssertEqual(
            MatrixHTMLRenderer.markdownExcludingDetails(body: "", html: html),
            "**Practical decision:**\n\n- Do not use it for active/default bounty pipelines."
        )
        let segments = MatrixHTMLRenderer.segments(body: "", html: html)
        XCTAssertEqual(segments.count, 2)
        XCTAssertEqual(segments.first, .details(block))
        let text = try XCTUnwrap(richText(from: segments.last))
        XCTAssertEqual(
            text.plainText,
            "Practical decision:\n\n• Do not use it for active/default bounty pipelines."
        )
        XCTAssertTrue(text.runs.first { $0.text.contains("Practical decision:") }?.style.contains(.bold) == true)
    }

    func testMatrixHTMLRendererSegmentsCodeBlocksOutsideDetails() throws {
        let html = #"""
        <p>Plan:</p>
        <pre><code>let value = 1&#10;print(value)</code></pre>
        <ul><li><strong>Ship</strong></li><li>Verify</li></ul>
        """#

        let segments = MatrixHTMLRenderer.segments(body: "fallback", html: html)
        XCTAssertEqual(segments.count, 3)
        XCTAssertEqual(try XCTUnwrap(richText(from: segments[0])).plainText, "Plan:")
        XCTAssertEqual(segments[1], .code(.init(code: "let value = 1\nprint(value)", language: nil)))
        let list = try XCTUnwrap(richText(from: segments[2]))
        XCTAssertEqual(list.plainText, "• Ship\n• Verify")
        XCTAssertTrue(list.runs.first { $0.text == "Ship" }?.style.contains(.bold) == true)
    }

    func testMatrixHTMLRendererCountsCodeBlockLines() {
        XCTAssertEqual(MatrixHTMLRenderer.codeLineCount(""), 1)
        XCTAssertEqual(MatrixHTMLRenderer.codeLineCount("print(1)"), 1)
        XCTAssertEqual(MatrixHTMLRenderer.codeLineCount("let value = 1\nprint(value)"), 2)
        XCTAssertEqual(MatrixHTMLRenderer.codeLineCount("one\n\nthree\n"), 3)
    }

    func testMatrixHTMLRendererSegmentsHeadingsAndBlockquotes() throws {
        let html = #"""
        <h2>App-agent handoff</h2>
        <p>I wrote a copyable handoff file here:</p>
        <blockquote><p>TestFlight <strong>MUST</strong> use production APNs.</p></blockquote>
        """#

        let segments = MatrixHTMLRenderer.segments(body: "fallback", html: html)
        XCTAssertEqual(segments.count, 3)
        guard case let .heading(heading) = segments[0] else {
            return XCTFail("Expected a semantic heading segment")
        }
        XCTAssertEqual(heading.level, 2)
        XCTAssertEqual(heading.content.plainText, "App-agent handoff")
        XCTAssertTrue(heading.content.runs.first?.style.contains(.heading2) == true)
        XCTAssertEqual(try XCTUnwrap(richText(from: segments[1])).plainText, "I wrote a copyable handoff file here:")
        guard case let .quote(quote) = segments[2] else {
            return XCTFail("Expected semantic quote segment")
        }
        XCTAssertEqual(quote.plainText, "TestFlight MUST use production APNs.")
        XCTAssertTrue(quote.runs.first { $0.text == "MUST" }?.style.contains(.bold) == true)
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

        let segments = MatrixHTMLRenderer.segments(body: "fallback", html: html)
        XCTAssertEqual(segments.count, 3)
        XCTAssertEqual(try XCTUnwrap(richText(from: segments[0])).plainText, "Models")
        guard case let .table(table) = segments[1] else {
            return XCTFail("Expected semantic table segment")
        }
        XCTAssertEqual(table.rows.map { $0.cells.map(\.plainText) }, [
            ["Stage", "Actual", "Proof"],
            ["Alpha", "stealth/ox-alpha", "content_chars=2702"],
            ["Parent", "grok-4.6", "orchestrator only"],
        ])
        XCTAssertEqual(table.rows.map(\.isHeader), [true, false, false])
        XCTAssertTrue(table.rows[1].cells[2].content.runs.first?.style.contains(.code) == true)
        XCTAssertEqual(try XCTUnwrap(richText(from: segments[2])).plainText, "Verdicts")
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

    func testMatrixHTMLRendererNeverReinterpretsLiteralMarkdownInsideFormattedHTML() throws {
        let text = MatrixHTMLRenderer.richText(
            body: "fallback",
            html: #"<p>literal **bold** and ~~removed~~ and `code`; <strong>real bold</strong>, <del>real strike</del>, <code>real code</code></p>"#
        )

        XCTAssertEqual(
            text.plainText,
            "literal **bold** and ~~removed~~ and `code`; real bold, real strike, real code"
        )
        let literal = try XCTUnwrap(text.runs.first { $0.text.contains("literal **bold**") })
        XCTAssertTrue(literal.style.isEmpty)
        XCTAssertTrue(text.runs.first { $0.text == "real bold" }?.style.contains(.bold) == true)
        XCTAssertTrue(text.runs.first { $0.text == "real strike" }?.style.contains(.strikethrough) == true)
        XCTAssertTrue(text.runs.first { $0.text == "real code" }?.style.contains(.code) == true)
    }

    func testMatrixHTMLRendererPreservesExactHermesMentionAndRejectsUnsafeLink() throws {
        let text = MatrixHTMLRenderer.richText(
            body: "fallback",
            html: #"<p>Hello <a href="https://matrix.to/#/@alice:example.org">@alice:example.org</a>; unsafe <a href="javascript:alert(1)">tap</a></p>"#
        )

        XCTAssertEqual(text.plainText, "Hello @alice:example.org; unsafe tap")
        let mention = try XCTUnwrap(text.runs.first { $0.text == "@alice:example.org" })
        XCTAssertEqual(mention.link?.absoluteString, "https://matrix.to/#/@alice:example.org")
        XCTAssertNil(text.runs.first { $0.text == "tap" }?.link)
    }

    func testMatrixHTMLRendererUsesInlineImageAltTextWithoutImportingAResource() {
        let text = MatrixHTMLRenderer.richText(
            body: "fallback",
            html: #"<p>Status <img data-mx-emoticon src="https://invalid.example/tracker.png" alt="✅"> ready</p>"#
        )

        XCTAssertEqual(text.plainText, "Status ✅ ready")
    }

    func testMatrixHTMLRendererStrictlyAllowlistsTagsAndIsQuoteAware() {
        let text = MatrixHTMLRenderer.richText(
            body: "fallback",
            html: #"<p onclick="ignored()" data-probe='><img src="https://invalid.example/tracker.png">'>Safe <unknown>visible</unknown></p><video src="https://invalid.example/video">hidden</video><link rel="stylesheet" href="https://invalid.example/style.css"><p><strong>Done</strong></p>"#
        )

        XCTAssertEqual(text.plainText, "Safe visible\n\nDone")
        XCTAssertTrue(text.runs.allSatisfy { $0.link == nil })
        XCTAssertTrue(text.runs.first { $0.text == "Done" }?.style.contains(.bold) == true)
    }

    func testMatrixHTMLRendererRetainsEverySafeAbsoluteMatrixLinkScheme() throws {
        let text = MatrixHTMLRenderer.richText(
            body: "fallback",
            html: #"<p><a href="https://example.org">https</a> <a href="http://example.org">http</a> <a href="ftp://example.org/file">ftp</a> <a href="mailto:alice@example.org">mail</a> <a href="magnet:?xt=urn:btih:abc">magnet</a> <a href="matrix:u/alice:example.org">matrix</a> <a href="/relative">relative</a></p>"#
        )

        for label in ["https", "http", "ftp", "mail", "magnet"] {
            XCTAssertTrue(
                text.runs.contains { $0.text.contains(label) && $0.link != nil },
                "Expected a typed safe link for \(label): \(text.runs)"
            )
        }
        XCTAssertTrue(text.runs.contains { $0.text.contains("matrix") && $0.link == nil })
        XCTAssertTrue(text.runs.contains { $0.text.contains("relative") && $0.link == nil })
    }

    func testMatrixHTMLRendererParsesUnquotedAbsoluteLinksWithoutTruncatingSlashes() throws {
        let text = MatrixHTMLRenderer.richText(
            body: "fallback",
            html: #"<p><a href=https://example.org/path>safe</a> <a href=javascript:alert(1)>unsafe</a></p>"#
        )

        XCTAssertEqual(
            try XCTUnwrap(text.runs.first { $0.text == "safe" }).link?.absoluteString,
            "https://example.org/path"
        )
        XCTAssertNil(text.runs.first { $0.text == "unsafe" }?.link)
    }

    func testMatrixHTMLRendererPreservesHeadingHierarchyAndScriptSemantics() throws {
        let text = MatrixHTMLRenderer.richText(
            body: "fallback",
            html: #"<h1>Primary</h1><h4>Section</h4><p>x<sup>2</sup> + H<sub>2</sub>O</p>"#
        )

        XCTAssertEqual(text.plainText, "Primary\n\nSection\n\nx2 + H2O")
        XCTAssertTrue(try XCTUnwrap(text.runs.first { $0.text == "Primary" }).style.contains(.heading1))
        XCTAssertTrue(try XCTUnwrap(text.runs.first { $0.text == "Section" }).style.contains(.heading4))
        XCTAssertTrue(try XCTUnwrap(text.runs.first { $0.text == "2" }).style.contains(.superscript))
        XCTAssertTrue(try XCTUnwrap(text.runs.last { $0.text == "2" }).style.contains(.subscriptText))
    }

    func testMatrixHTMLRendererStripsReplyFallbackAndCapsTagNestingAtOneHundred() throws {
        let reply = MatrixHTMLRenderer.richText(
            body: "fallback",
            html: #"<mx-reply><blockquote>old reply</blockquote></mx-reply><p>Current message</p>"#
        )
        XCTAssertEqual(reply.plainText, "Current message")

        let html = String(repeating: "<div>", count: 100)
            + "<strong>deep text</strong>"
            + String(repeating: "</div>", count: 100)
        let nested = MatrixHTMLRenderer.richText(body: "fallback", html: html)
        XCTAssertEqual(nested.plainText, "deep text")
        XCTAssertFalse(try XCTUnwrap(nested.runs.first).style.contains(.bold))
    }

    func testMatrixHTMLRendererConcealsSpoilersAsTypedSegments() throws {
        let segments = MatrixHTMLRenderer.segments(
            body: "Visible secret after reveal",
            html: #"<p>Visible</p><span data-mx-spoiler="deployment detail"><strong>secret</strong></span>"#
        )

        XCTAssertEqual(segments.count, 2)
        guard case let .spoiler(block) = segments[1] else {
            return XCTFail("Expected a typed spoiler segment")
        }
        XCTAssertEqual(block.reason, "deployment detail")
        XCTAssertEqual(block.content.plainText, "secret")
        XCTAssertTrue(block.content.runs.first?.style.contains(.bold) == true)
    }

    func testMatrixHTMLRendererPreservesNestedListsAndOrderedStart() {
        let text = MatrixHTMLRenderer.richText(
            body: "fallback",
            html: #"<ol start="4"><li>Outer<ul><li><strong>Inner</strong></li></ul></li><li>After</li></ol>"#
        )

        XCTAssertEqual(text.plainText, "4. Outer\n  • Inner\n5. After")
        XCTAssertTrue(text.runs.first { $0.text == "Inner" }?.style.contains(.bold) == true)
    }

    func testMatrixHTMLRendererDoesNotInsertBlankLinesForParagraphWrappedListItems() {
        let text = MatrixHTMLRenderer.richText(
            body: "fallback",
            html: #"<ul><li><p>First</p></li><li><p><strong>Second</strong></p></li></ul>"#
        )

        XCTAssertEqual(text.plainText, "• First\n• Second")
        XCTAssertTrue(text.runs.first { $0.text == "Second" }?.style.contains(.bold) == true)
    }

    func testMatrixHTMLRendererPreservesMixedNestedListKindsAndIndependentOrderedStarts() {
        let text = MatrixHTMLRenderer.richText(
            body: "fallback",
            html: #"<ol start="7"><li>Seven<ul><li>Bullet<ol start="11"><li>Eleven</li><li>Twelve</li></ol></li></ul></li><li>Eight</li></ol>"#
        )

        XCTAssertEqual(
            text.plainText,
            """
            7. Seven
              • Bullet
                11. Eleven
                12. Twelve
            8. Eight
            """
        )
    }

    func testMatrixHTMLRendererPreservesWhitespaceAcrossInlineElementBoundaries() {
        let text = MatrixHTMLRenderer.richText(
            body: "fallback",
            html: #"<p><strong>Hello</strong> <em>careful</em> reader; <code>exact</code> tail</p>"#
        )

        XCTAssertEqual(text.plainText, "Hello careful reader; exact tail")
        XCTAssertTrue(text.runs.first { $0.text == "Hello" }?.style.contains(.bold) == true)
        XCTAssertTrue(text.runs.first { $0.text == "careful" }?.style.contains(.italic) == true)
        XCTAssertTrue(text.runs.first { $0.text == "exact" }?.style.contains(.code) == true)
    }

    func testAttributedRichTextKeepsNativeBoldAndCodeIntentsWithSemanticCodePaint() throws {
        let richText = MatrixHTMLRenderer.RichText(runs: [
            .init(text: "Bold", style: [.bold], link: nil),
            .init(text: " and ", style: [], link: nil),
            .init(text: "inlineCode", style: [.code], link: nil),
        ])

        let attributed = attributedRichText(richText)
        XCTAssertEqual(String(attributed.characters), "Bold and inlineCode")
        let runs = Array(attributed.runs)
        let boldRun = try XCTUnwrap(runs.first { run in
            String(attributed[run.range].characters) == "Bold"
        })
        let codeRun = try XCTUnwrap(runs.first { run in
            String(attributed[run.range].characters) == "inlineCode"
        })

        XCTAssertTrue(boldRun.inlinePresentationIntent?.contains(.stronglyEmphasized) == true)
        XCTAssertTrue(codeRun.inlinePresentationIntent?.contains(.code) == true)
        XCTAssertNotNil(codeRun.foregroundColor)
        XCTAssertNotNil(codeRun.backgroundColor)
        XCTAssertEqual(codeRun.underlineStyle, .single)
        XCTAssertNotNil(codeRun.underlineColor)
    }

    func testExplicitlyUnderlinedInlineCodeKeepsAuthoredUnderlineSemantics() throws {
        let attributed = attributedRichText(
            .init(runs: [
                .init(text: "underlinedCode", style: [.code, .underline], link: nil),
            ])
        )
        let run = try XCTUnwrap(attributed.runs.first)

        XCTAssertTrue(run.inlinePresentationIntent?.contains(.code) == true)
        XCTAssertEqual(run.underlineStyle, .single)
        XCTAssertNil(
            run.underlineColor,
            "Authored underline must use the text foreground instead of the adaptive hidden boundary"
        )
        XCTAssertNotNil(run.backgroundColor)
    }

    func testRichFixtureRetainsStructuralSemanticsAndClipboardText() throws {
        let html = #"""
        <h3>Deploy</h3>
        <p><strong>Important</strong> <code>swift test</code> <a href="https://example.org">proof</a></p>
        <blockquote><p>Keep this readable.</p></blockquote>
        <ul><li>First</li><li><em>Second</em></li></ul>
        <span data-mx-spoiler="private">secret</span>
        <table><tr><th>State</th><th>Value</th></tr><tr><td>Build</td><td><code>green</code></td></tr></table>
        """#
        let segments = MatrixHTMLRenderer.segments(body: "fallback", html: html)

        XCTAssertTrue(segments.contains { if case .heading = $0 { return true }; return false })
        XCTAssertTrue(segments.contains { if case .quote = $0 { return true }; return false })
        XCTAssertTrue(segments.contains {
            if case let .inline(group) = $0 {
                return group.pieces.contains { if case .spoiler = $0 { return true }; return false }
            }
            if case .spoiler = $0 { return true }
            return false
        })
        XCTAssertTrue(segments.contains { if case .table = $0 { return true }; return false })

        let projection = MatrixHTMLRenderer.selectionProjection(
            body: "fallback",
            html: html,
            revealingSpoilers: true
        )
        let attributed = attributedRichText(projection.richText, includeLinks: false)
        XCTAssertEqual(String(attributed.characters), projection.richText.plainText)
        XCTAssertTrue(attributed.runs.contains { $0.inlinePresentationIntent?.contains(.stronglyEmphasized) == true })
        XCTAssertTrue(attributed.runs.contains { $0.inlinePresentationIntent?.contains(.code) == true })
    }

    func testMatrixHTMLRendererPreservesOnlyStrictMatrixColorsIncludingNestedOverride() throws {
        let text = MatrixHTMLRenderer.richText(
            body: "fallback",
            html: ##"<p><span data-mx-color="#a1b2c3" data-mx-bg-color="#010203">outer <span data-mx-color="#DDEEFF">inner</span></span> <span data-mx-color="red" data-mx-bg-color="#12345G">invalid</span></p>"##
        )

        let outer = try XCTUnwrap(text.runs.first { $0.text == "outer " })
        XCTAssertEqual(outer.foregroundColorHex, "#A1B2C3")
        XCTAssertEqual(outer.backgroundColorHex, "#010203")
        let inner = try XCTUnwrap(text.runs.first { $0.text == "inner" })
        XCTAssertEqual(inner.foregroundColorHex, "#DDEEFF")
        XCTAssertEqual(inner.backgroundColorHex, "#010203")
        let invalid = try XCTUnwrap(text.runs.first { $0.text.contains("invalid") })
        XCTAssertNil(invalid.foregroundColorHex)
        XCTAssertNil(invalid.backgroundColorHex)
    }

    func testMatrixHTMLRendererNormalizesLegacyFontColorsWithNestedOverrides() throws {
        let html = ##"<p><font color="#a1b2c3" data-mx-bg-color="#010203">legacy <span data-mx-color="#DDEEFF">span</span> <font color="#445566" data-mx-color="#102030" data-mx-bg-color="#F0E0D0">nested</font></font> <font color="red" data-mx-color="#12345G" data-mx-bg-color="blue" style="color: #FFFFFF">invalid</font></p>"##
        let text = MatrixHTMLRenderer.richText(body: "fallback", html: html)

        let legacy = try XCTUnwrap(text.runs.first { $0.text == "legacy " })
        XCTAssertEqual(legacy.foregroundColorHex, "#A1B2C3")
        XCTAssertEqual(legacy.backgroundColorHex, "#010203")

        let span = try XCTUnwrap(text.runs.first { $0.text == "span" })
        XCTAssertEqual(span.foregroundColorHex, "#DDEEFF")
        XCTAssertEqual(span.backgroundColorHex, "#010203")

        let nested = try XCTUnwrap(text.runs.first { $0.text == "nested" })
        XCTAssertEqual(nested.foregroundColorHex, "#102030", "data-mx-color must win over legacy color")
        XCTAssertEqual(nested.backgroundColorHex, "#F0E0D0")

        let invalid = try XCTUnwrap(text.runs.first { $0.text.contains("invalid") })
        XCTAssertNil(invalid.foregroundColorHex)
        XCTAssertNil(invalid.backgroundColorHex)

        let projection = MatrixHTMLRenderer.selectionProjection(
            body: "legacy span nested invalid",
            html: html,
            revealingSpoilers: true
        )
        XCTAssertEqual(projection.richText.runs.first { $0.text == "legacy " }?.foregroundColorHex, "#A1B2C3")
        XCTAssertEqual(projection.richText.runs.first { $0.text == "span" }?.foregroundColorHex, "#DDEEFF")
        XCTAssertEqual(projection.richText.runs.first { $0.text == "nested" }?.foregroundColorHex, "#102030")
    }

    func testMatrixHTMLRendererPreservesMathFallbackAndExplicitInlineImageFallback() {
        let text = MatrixHTMLRenderer.richText(
            body: "fallback",
            html: #"<p>Inline <span data-mx-maths="x &lt; y"></span>.</p><div data-mx-maths="\int_0^1 x dx"></div><p><img src="mxc://example.org/id"></p>"#
        )

        XCTAssertEqual(text.plainText, "Inline x < y.\n\n\\int_0^1 x dx\n\n[Inline image]")
        XCTAssertTrue(text.runs.first { $0.text.contains("x < y") }?.style.contains(.code) == true)
        XCTAssertTrue(text.runs.first { $0.text.contains("\\int") }?.style.contains(.code) == true)
    }

    func testMatrixHTMLRendererPreservesMixedInlineSpoilerOrderAndFormatting() throws {
        let segments = MatrixHTMLRenderer.segments(
            body: "before secret after",
            html: #"<p>before <span data-mx-spoiler="reason"><strong>secret</strong></span> after</p>"#
        )

        guard segments.count == 1, case let .inline(group) = segments[0] else {
            return XCTFail("Expected one wrapping inline presentation group")
        }
        XCTAssertEqual(group.pieces.count, 3)
        guard case let .richText(before) = group.pieces[0],
              case let .spoiler(spoiler) = group.pieces[1],
              case let .richText(after) = group.pieces[2]
        else { return XCTFail("Expected visible, spoiler, visible sibling order") }
        XCTAssertEqual(before.plainText, "before ")
        XCTAssertEqual(spoiler.reason, "reason")
        XCTAssertEqual(spoiler.content.plainText, "secret")
        XCTAssertTrue(spoiler.content.runs.first?.style.contains(.bold) == true)
        XCTAssertEqual(after.plainText, " after")
    }

    func testMatrixSpoilerPreservesOuterAndNestedAuthoredColorsWithoutLeakingHiddenText() throws {
        let html = ##"<p>before <span data-mx-spoiler="private" data-mx-color="#FFFFFF" data-mx-bg-color="#000000">outer <span data-mx-color="#00FF00">inner</span></span> after</p>"##
        let segments = MatrixHTMLRenderer.segments(body: "before outer inner after", html: html)
        guard case let .inline(group)? = segments.first,
              case let .spoiler(spoiler) = group.pieces.first(where: {
                  if case .spoiler = $0 { return true }
                  return false
              })
        else { return XCTFail("Expected inline spoiler content") }

        let outer = try XCTUnwrap(spoiler.content.runs.first { $0.text == "outer " })
        let inner = try XCTUnwrap(spoiler.content.runs.first { $0.text == "inner" })
        XCTAssertEqual(outer.foregroundColorHex, "#FFFFFF")
        XCTAssertEqual(outer.backgroundColorHex, "#000000")
        XCTAssertEqual(inner.foregroundColorHex, "#00FF00", "Nested Matrix color must override the spoiler span")
        XCTAssertEqual(inner.backgroundColorHex, "#000000")

        let hidden = matrixInlineAttributedText(
            group,
            revealedSpoilers: [],
            presentationContext: .otherMessage
        )
        XCTAssertFalse(String(hidden.characters).contains("outer"))
        XCTAssertFalse(String(hidden.characters).contains("inner"))
        XCTAssertTrue(String(hidden.characters).contains("Spoiler: private"))

        let revealed = matrixInlineAttributedText(
            group,
            revealedSpoilers: [0],
            presentationContext: .otherMessage
        )
        XCTAssertEqual(String(revealed.characters), "before outer inner after")
        let revealedOuter = try XCTUnwrap(revealed.runs.first { run in
            String(revealed[run.range].characters) == "outer "
        })
        let revealedInner = try XCTUnwrap(revealed.runs.first { run in
            String(revealed[run.range].characters) == "inner"
        })
        XCTAssertNotNil(revealedOuter.foregroundColor)
        XCTAssertNotNil(revealedOuter.backgroundColor)
        XCTAssertNotNil(revealedInner.foregroundColor)
        XCTAssertNotNil(revealedInner.backgroundColor)

        let projection = MatrixHTMLRenderer.selectionProjection(
            body: "before outer inner after",
            html: html,
            revealingSpoilers: true
        )
        XCTAssertEqual(projection.richText.runs.first { $0.text == "outer " }?.foregroundColorHex, "#FFFFFF")
        XCTAssertEqual(projection.richText.runs.first { $0.text == "inner" }?.foregroundColorHex, "#00FF00")

        let blockSegments = MatrixHTMLRenderer.segments(
            body: "block secret",
            html: ##"<span data-mx-spoiler="block" data-mx-color="#FFFFFF" data-mx-bg-color="#000000">block secret</span>"##
        )
        guard case let .spoiler(block)? = blockSegments.first else {
            return XCTFail("Expected standalone spoiler block")
        }
        XCTAssertEqual(block.content.runs.first?.foregroundColorHex, "#FFFFFF")
        XCTAssertEqual(block.content.runs.first?.backgroundColorHex, "#000000")
    }

    func testMatrixInlineSpoilerPresentationNeverExposesHiddenContentToAccessibilityText() throws {
        let segments = MatrixHTMLRenderer.segments(
            body: "before secret after",
            html: #"<p><strong>before</strong> <span data-mx-spoiler="private"><em>secret</em></span> after</p>"#
        )
        guard case let .inline(group)? = segments.first else {
            return XCTFail("Expected inline spoiler group")
        }

        let hidden = matrixInlineAttributedText(group, revealedSpoilers: [])
        let hiddenCharacters = String(hidden.characters)
        XCTAssertEqual(hiddenCharacters, "before [Spoiler: private · Reveal] after")
        XCTAssertFalse(hiddenCharacters.contains("secret"))
        XCTAssertEqual(
            Array(hidden.runs).compactMap(\.link).map(\.absoluteString),
            [matrixInlineSpoilerURL(index: 0).absoluteString]
        )

        let revealed = matrixInlineAttributedText(group, revealedSpoilers: [0])
        XCTAssertEqual(String(revealed.characters), "before secret after")
        XCTAssertTrue(try XCTUnwrap(matrixInlineSpoilerIndex(matrixInlineSpoilerURL(index: 0))) == 0)
    }

    func testMatrixHTMLRendererPreservesNestedBlockquoteDepthAndInlineStyles() throws {
        let segments = MatrixHTMLRenderer.segments(
            body: "fallback",
            html: #"<blockquote><p>Outer</p><blockquote><p><em>Inner</em></p></blockquote><p>Outer again</p></blockquote>"#
        )

        guard segments.count == 1, case let .quote(quote) = segments[0] else {
            return XCTFail("Expected one semantic outer quote")
        }
        XCTAssertEqual(quote.plainText, "Outer\n\n> Inner\n\nOuter again")
        XCTAssertTrue(quote.runs.first { $0.text == "Inner" }?.style.contains(.italic) == true)
    }

    func testMatrixHTMLRendererKeepsNestedDetailsAndTableContentWithoutRegexTruncation() throws {
        let html = #"<details><summary>Outer</summary><p>Before</p><details><summary>Inner</summary><table><tr><th>A</th><th>B</th></tr><tr><td>1</td><td><code>2</code></td></tr></table></details><p>After</p></details>"#
        let details = MatrixHTMLRenderer.detailsBlocks(html: html)

        XCTAssertEqual(details.map(\.summary), ["Outer", "Inner"])
        let outer = try XCTUnwrap(details.first)
        XCTAssertTrue(outer.body.contains("Before"))
        XCTAssertTrue(outer.body.contains("Inner"))
        XCTAssertTrue(outer.body.contains("A\tB"))
        XCTAssertTrue(outer.body.contains("1\t2"))
        XCTAssertTrue(outer.body.contains("After"))
        XCTAssertEqual(MatrixHTMLRenderer.segments(body: "fallback", html: html), [.details(outer)])

        guard outer.content.count == 3,
              case let .details(inner) = outer.content[1],
              case let .table(table) = inner.content.first
        else { return XCTFail("Expected recursively typed details and table content") }
        XCTAssertEqual(table.rows.map { $0.cells.map(\.plainText) }, [["A", "B"], ["1", "2"]])
    }

    func testMatrixHTMLRendererNeverDropsReadableMalformedSemanticContainerContent() throws {
        let segments = MatrixHTMLRenderer.segments(
            body: "fallback",
            html: #"<p>Before</p><details><p>Missing summary</p><pre><code>exact</code></pre></details><table><caption>Caption only</caption></table><p>After</p>"#
        )

        XCTAssertEqual(segments.count, 5)
        XCTAssertEqual(try XCTUnwrap(richText(from: segments[0])).plainText, "Before")
        XCTAssertEqual(try XCTUnwrap(richText(from: segments[1])).plainText, "Missing summary")
        XCTAssertEqual(segments[2], .code(.init(code: "exact", language: nil)))
        XCTAssertEqual(try XCTUnwrap(richText(from: segments[3])).plainText, "Caption only")
        XCTAssertEqual(try XCTUnwrap(richText(from: segments[4])).plainText, "After")
    }

    func testMatrixHTMLRendererMalformedSemanticFallbackRetainsEnclosingStyleAndLink() throws {
        let segments = MatrixHTMLRenderer.segments(
            body: "fallback",
            html: #"<strong><a href="https://example.org"><details><p>Readable</p></details></a></strong>"#
        )

        guard segments.count == 1, case let .richText(text) = segments[0] else {
            return XCTFail("Expected one readable fallback segment")
        }
        let run = try XCTUnwrap(text.runs.first { $0.text == "Readable" })
        XCTAssertTrue(run.style.contains(.bold))
        XCTAssertEqual(run.link?.absoluteString, "https://example.org")
    }

    func testMatrixHTMLRendererRetainsEverySemanticSegmentInsideDetails() throws {
        let html = #"""
        <details><summary><em>Diagnostics</em></summary>
          <ol start="3"><li>Third<ul><li>Nested</li></ul></li></ol>
          <blockquote><p>Quoted</p></blockquote>
          <pre><code class="language-swift"> let value = 1&#10;</code></pre>
          <span data-mx-spoiler="private"><strong>secret</strong></span>
          <table><tr><th>Key</th><td>Value</td></tr></table>
          <details><summary>Child</summary><p>Child body</p></details>
        </details>
        """#

        let segments = MatrixHTMLRenderer.segments(body: "fallback", html: html)
        guard case let .details(details)? = segments.first else {
            return XCTFail("Expected an outer semantic details segment")
        }
        XCTAssertEqual(details.summary, "Diagnostics")
        XCTAssertTrue(details.summaryContent.runs.first?.style.contains(.italic) == true)
        XCTAssertEqual(details.content.count, 6)

        guard case let .richText(list) = details.content[0],
              case let .quote(quote) = details.content[1],
              case let .code(code) = details.content[2],
              case let .spoiler(spoiler) = details.content[3],
              case let .table(table) = details.content[4],
              case let .details(child) = details.content[5]
        else { return XCTFail("Expected list, quote, code, spoiler, table, and nested details in source order") }

        XCTAssertEqual(list.plainText, "3. Third\n  • Nested")
        XCTAssertEqual(quote.plainText, "Quoted")
        XCTAssertEqual(code, .init(code: " let value = 1\n", language: "swift"))
        XCTAssertEqual(spoiler.reason, "private")
        XCTAssertEqual(spoiler.content.plainText, "secret")
        XCTAssertTrue(spoiler.content.runs.first?.style.contains(.bold) == true)
        XCTAssertEqual(table.rows.first?.cells.map(\.plainText), ["Key", "Value"])
        XCTAssertEqual(child.summary, "Child")
        XCTAssertEqual(child.body, "Child body")
    }

    func testMatrixHTMLRendererDoesNotPromoteNestedTableRowsIntoParentTable() throws {
        let html = #"<table><tr><th>Outer</th><td><table><tr><td>Nested A</td><td>Nested B</td></tr></table></td></tr></table>"#
        let table = try XCTUnwrap(MatrixHTMLRenderer.tableBlock(html: html))

        XCTAssertEqual(table.rows.count, 1)
        XCTAssertEqual(table.rows[0].cells.count, 2)
        XCTAssertEqual(table.rows[0].cells[0].plainText, "Outer")
        XCTAssertEqual(table.rows[0].cells[1].plainText, "Nested A\tNested B")
    }

    func testMatrixHTMLRendererPreservesExactPreCodeWhitespaceBreaksAndEntities() {
        let html = "<pre><code class=\"language-sh\">  lead\t&amp;&copy;<br>middle&#10;&#x20;tail  \n\n</code></pre>"

        XCTAssertEqual(
            MatrixHTMLRenderer.segments(body: "fallback", html: html),
            [.code(.init(code: "  lead\t&©\nmiddle\n tail  \n\n", language: "sh"))]
        )
    }

    func testMatrixHTMLRendererDecodesNamedEntitiesWithoutHTMLDocumentImport() {
        let html = "<pre><code>&amp;&copy;&Alpha;&euro;&trade;&apos;&bogus;&#10;&#x1F642;</code></pre>"

        XCTAssertEqual(
            MatrixHTMLRenderer.segments(body: "fallback", html: html),
            [.code(.init(code: "&©Α€™'&bogus;\n🙂", language: nil))]
        )
    }

    func testMatrixHTMLRendererBalancesMalformedOverlapAndPreservesText() {
        let text = MatrixHTMLRenderer.richText(
            body: "fallback",
            html: #"<p><strong>bold <em>both</strong> italic tail</em> plain</p>"#
        )

        XCTAssertEqual(text.plainText, "bold both italic tail plain")
        XCTAssertTrue(text.runs.first { $0.text == "bold " }?.style.contains(.bold) == true)
        let both = text.runs.first { $0.text == "both" }?.style
        XCTAssertTrue(both?.contains(.bold) == true)
        XCTAssertTrue(both?.contains(.italic) == true)
        XCTAssertTrue(text.runs.first { $0.text.contains("italic tail") }?.style.isEmpty == true)
    }

    func testMatrixHTMLRendererDropsExecutableBlocksAndPreservesFollowingText() {
        let text = MatrixHTMLRenderer.richText(
            body: "fallback",
            html: #"<script>alert(1)</script><style>p{display:none}</style><p>Visible &amp; safe</p><iframe>hidden</iframe>"#
        )

        XCTAssertEqual(text.plainText, "Visible & safe")
    }

    func testMatrixHTMLRendererFailsToBoundedPlainBodyForOversizeHTML() {
        let html = "<strong>" + String(repeating: "x", count: 256 * 1_024) + "</strong>"
        let text = MatrixHTMLRenderer.richText(body: "bounded fallback", html: html)

        XCTAssertEqual(text, .init(runs: [.init(text: "bounded fallback", style: [], link: nil)]))
        XCTAssertEqual(
            MatrixHTMLRenderer.segments(body: "bounded fallback", html: html),
            [.richText(.init(runs: [.init(text: "bounded fallback", style: [], link: nil)]))]
        )
    }

    func testMatrixHTMLRendererPreservesExactHermesTableSections() {
        let html = #"""
        <table>
        <thead>
        <tr><th>Stage</th><th>Requested</th><th>Actual</th><th>Proof</th></tr>
        </thead>
        <tbody>
        <tr><td>Ox Alpha</td><td>stealth/ox-alpha max</td><td>stealth/ox-alpha</td><td><code>content_chars=2702</code>, finish=stop</td></tr>
        <tr><td>qwen</td><td>qwen3.8-max / alibaba</td><td>qwen3.8-max (self-report)</td><td>EXIT 0</td></tr>
        </tbody>
        </table>
        """#

        let segments = MatrixHTMLRenderer.segments(body: "fallback", html: html)
        guard segments.count == 1, case let .table(table) = segments[0] else {
            return XCTFail("Expected exact Hermes table segment")
        }
        XCTAssertEqual(table.rows.map { $0.cells.map(\.plainText) }, [
            ["Stage", "Requested", "Actual", "Proof"],
            ["Ox Alpha", "stealth/ox-alpha max", "stealth/ox-alpha", "content_chars=2702, finish=stop"],
            ["qwen", "qwen3.8-max / alibaba", "qwen3.8-max (self-report)", "EXIT 0"],
        ])
        XCTAssertEqual(table.rows.map(\.isHeader), [true, false, false])
        XCTAssertTrue(table.rows[1].cells[3].content.runs.first?.style.contains(.code) == true)
    }

    func testMatrixHTMLRendererPreservesTableCaptionAndMixedHeaderDataCellOrder() throws {
        let html = #"<table><caption><strong>Build proof</strong></caption><tbody><tr><th>Stage</th><td><code>green</code></td></tr></tbody></table>"#

        let segments = MatrixHTMLRenderer.segments(body: "fallback", html: html)
        guard segments.count == 1, case let .table(table) = segments[0] else {
            return XCTFail("Expected table segment")
        }
        let caption = try XCTUnwrap(table.caption)
        XCTAssertEqual(caption.plainText, "Build proof")
        XCTAssertTrue(caption.runs.first?.style.contains(.bold) == true)
        XCTAssertEqual(table.rows.map { $0.cells.map(\.plainText) }, [["Stage", "green"]])
        XCTAssertEqual(table.rows.map(\.isHeader), [false])
        XCTAssertEqual(table.rows[0].cells.map(\.isHeader), [true, false])
        XCTAssertTrue(table.rows[0].cells[1].content.runs.first?.style.contains(.code) == true)
    }

    func testMatrixHTMLRendererPreservesExactHermesApprovalPayloadShape() throws {
        let body = #"""
        ⚠️ **Dangerous command requires approval**
        ```
        rm -rf /tmp/example
        ```
        Reason: destructive command

        Reply `!approve` to execute, `!approve session` to approve this pattern for the session, `!approve always` to approve permanently, or `!deny` to cancel.

        You can also react to this prompt:
        ✅ = approve once
        ♾️ = approve always
        ❌ = deny
        """#
        let html = #"""
        <p>⚠️ <strong>Dangerous command requires approval</strong></p>
        <pre><code>rm -rf /tmp/example
        </code></pre>
        <p>Reason: destructive command</p>
        <p>Reply <code>!approve</code> to execute, <code>!approve session</code> to approve this pattern for the session, <code>!approve always</code> to approve permanently, or <code>!deny</code> to cancel.</p>
        <p>You can also react to this prompt:<br>
        ✅ = approve once<br>
        ♾️ = approve always<br>
        ❌ = deny</p>
        """#

        let segments = MatrixHTMLRenderer.segments(body: body, html: html)
        XCTAssertEqual(segments.count, 3)
        let heading = try XCTUnwrap(richText(from: segments[0]))
        XCTAssertEqual(heading.plainText, "⚠️ Dangerous command requires approval")
        XCTAssertTrue(heading.runs.first { $0.text.contains("Dangerous command") }?.style.contains(.bold) == true)
        XCTAssertEqual(segments[1], .code(.init(code: "rm -rf /tmp/example\n", language: nil)))
        let instructions = try XCTUnwrap(richText(from: segments[2]))
        XCTAssertEqual(
            instructions.plainText,
            """
            Reason: destructive command

            Reply !approve to execute, !approve session to approve this pattern for the session, !approve always to approve permanently, or !deny to cancel.

            You can also react to this prompt:
            ✅ = approve once
            ♾️ = approve always
            ❌ = deny
            """
        )
        for command in ["!approve", "!approve session", "!approve always", "!deny"] {
            XCTAssertTrue(
                instructions.runs.contains { $0.text == command && $0.style.contains(.code) },
                "Expected inline code semantics for \(command)"
            )
        }
    }

    func testMatrixHTMLRendererPreservesHermesFencedCodeLanguage() {
        let html = "<pre><code class=\"language-python\">print(&quot;hello&quot;)\n</code></pre>"

        XCTAssertEqual(
            MatrixHTMLRenderer.segments(body: "```python\nprint(\"hello\")\n```", html: html),
            [.code(.init(code: "print(\"hello\")\n", language: "python"))]
        )
    }

    func testMatrixHTMLRendererPreservesExactCodeWhitespaceForCopying() {
        let html = "<pre><code class=\"language-sh\">  printf 'x'  \n\n</code></pre>"

        XCTAssertEqual(
            MatrixHTMLRenderer.segments(body: "fallback", html: html),
            [.code(.init(code: "  printf 'x'  \n\n", language: "sh"))]
        )
        XCTAssertEqual(MatrixHTMLRenderer.codeLineCount("  printf 'x'  \n\n"), 2)
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

    func testMessageGroupingUsesTwoHourWindowAndSenderBoundary() {
        let calendar = Calendar.current
        let baseDate = calendar.date(
            from: DateComponents(year: 2026, month: 8, day: 27, hour: 12)
        )!

        func item(sender: String, offset: TimeInterval) -> TimelineItem {
            TimelineItem(
                id: "\(sender)-\(offset)",
                eventID: "\(sender)-\(offset)",
                senderID: sender,
                timestamp: baseDate.addingTimeInterval(offset),
                kind: .text("Message"),
                replyToEventID: nil,
                isEdited: false,
                reactions: [:]
            )
        }

        let first = item(sender: "@alice:matrix.org", offset: 0)
        XCTAssertTrue(
            TimelineMessageGroupingPolicy.shouldGroup(
                previous: first,
                current: item(sender: "@alice:matrix.org", offset: 2 * 60 * 60 - 1)
            )
        )
        XCTAssertFalse(
            TimelineMessageGroupingPolicy.shouldGroup(
                previous: first,
                current: item(sender: "@alice:matrix.org", offset: 2 * 60 * 60)
            )
        )
        XCTAssertFalse(
            TimelineMessageGroupingPolicy.shouldGroup(
                previous: first,
                current: item(sender: "@bob:matrix.org", offset: 1)
            )
        )

        let late = item(sender: "@alice:matrix.org", offset: 11.5 * 60 * 60)
        let afterMidnight = item(sender: "@alice:matrix.org", offset: 12.5 * 60 * 60)
        XCTAssertFalse(
            TimelineMessageGroupingPolicy.shouldGroup(previous: late, current: afterMidnight)
        )
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

    func testPendingReconcilerRequiresMatchingThreadRoot() {
        let pending = TimelineItem.pendingMessage(
            localID: "$pending-thread-reply",
            body: "Same text",
            senderID: "@alice:matrix.org",
            replyToEventID: "$child",
            threadRootEventID: "$root-a",
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(35)
        )
        let differentThread = TimelineItem(
            id: "$server-other-thread",
            eventID: "$server-other-thread",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(36),
            kind: .text("Same text"),
            replyToEventID: "$child",
            threadRootEventID: "$root-b",
            isEdited: false,
            reactions: [:]
        )
        let sameThread = TimelineItem(
            id: "$server-same-thread",
            eventID: "$server-same-thread",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(36),
            kind: .text("Same text"),
            replyToEventID: "$child",
            threadRootEventID: "$root-a",
            isEdited: false,
            reactions: [:]
        )

        XCTAssertFalse(TimelinePendingReconciler.matchesPending(pending, serverItem: differentThread))
        XCTAssertTrue(TimelinePendingReconciler.matchesPending(pending, serverItem: sameThread))

        let unmatched = TimelinePendingReconciler.merge(
            streamItems: [differentThread],
            localItems: [pending],
            currentUserID: "@alice:matrix.org"
        )
        XCTAssertEqual(unmatched.map(\.id), ["$pending-thread-reply", "$server-other-thread"])

        let reconciled = TimelinePendingReconciler.merge(
            streamItems: [sameThread],
            localItems: [pending],
            currentUserID: "@alice:matrix.org"
        )
        XCTAssertEqual(reconciled.map(\.id), ["$server-same-thread"])
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

    func testCoreRelationPresentationWinsOverLoadedWindowFallback() {
        let authoritativePreview = TimelineReplyPreview(senderName: "Remote Alice", snippet: "Core preview")
        let item = TimelineItem(
            id: "$reply",
            eventID: "$reply",
            senderID: "@bob:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Reply"),
            replyToEventID: "$root",
            replyPreview: authoritativePreview,
            threadSummary: TimelineThreadSummary(
                rootEventID: "$reply",
                replyCount: 9,
                latestEventID: "$latest"
            ),
            isEdited: false,
            reactions: [:]
        )
        let localPreview = TimelineReplyPreview(senderName: "Stale", snippet: "Local fallback")

        XCTAssertEqual(
            TimelineRelationPresentation.replyPreview(
                for: item,
                locallyResolvedByEventID: ["$root": localPreview]
            ),
            authoritativePreview
        )
        XCTAssertEqual(
            TimelineRelationPresentation.replyCount(
                for: item,
                locallyCountedByRootID: ["$reply": 1]
            ),
            9
        )
    }

    func testRelationPresentationFallsBackForLocalAndMockItems() {
        let item = TimelineItem(
            id: "$reply",
            eventID: "$reply",
            senderID: "@bob:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Reply"),
            replyToEventID: "$root",
            isEdited: false,
            reactions: [:]
        )
        let localPreview = TimelineReplyPreview(senderName: "Alice", snippet: "Loaded root")

        XCTAssertEqual(
            TimelineRelationPresentation.replyPreview(
                for: item,
                locallyResolvedByEventID: ["$root": localPreview]
            ),
            localPreview
        )
        XCTAssertEqual(
            TimelineRelationPresentation.replyCount(
                for: item,
                locallyCountedByRootID: ["$reply": 2]
            ),
            2
        )
    }

    func testCoreRowWithoutThreadSummaryDoesNotInferThreadFromClassicReplies() {
        let item = TimelineItem(
            id: "$root",
            eventID: "$root",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Classic message"),
            replyToEventID: nil,
            actionCapabilities: TimelineRowActionCapabilities(
                canReact: true,
                canReply: true,
                canEdit: false,
                canRedact: false,
                canReport: false,
                canPin: true,
                canForward: true,
                canVote: false,
                canDeclineCall: false
            ),
            isEdited: false,
            reactions: [:]
        )

        XCTAssertEqual(
            TimelineRelationPresentation.replyCount(
                for: item,
                locallyCountedByRootID: ["$root": 3]
            ),
            0
        )
    }

    func testThreadMembershipUsesCoreThreadRootInsteadOfNestedReplyTarget() {
        let capabilities = TimelineRowActionCapabilities(
            canReact: true,
            canReply: true,
            canEdit: false,
            canRedact: false,
            canReport: false,
            canPin: true,
            canForward: true,
            canVote: false,
            canDeclineCall: false
        )
        let nestedThreadReply = TimelineItem(
            id: "$child-two",
            eventID: "$child-two",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Nested reply"),
            replyToEventID: "$child-one",
            threadRootEventID: "$root",
            actionCapabilities: capabilities,
            isEdited: false,
            reactions: [:]
        )
        let classicReply = TimelineItem(
            id: "$classic",
            eventID: "$classic",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Classic reply"),
            replyToEventID: "$root",
            actionCapabilities: capabilities,
            isEdited: false,
            reactions: [:]
        )

        XCTAssertTrue(TimelineThreadMembership.contains(nestedThreadReply, rootEventID: "$root"))
        XCTAssertFalse(TimelineThreadMembership.contains(nestedThreadReply, rootEventID: "$other"))
        XCTAssertFalse(TimelineThreadMembership.contains(classicReply, rootEventID: "$root"))
    }

    func testTimelineItemUsesCoreSenderDisplayNameBeforeLocalpartFallback() {
        let item = TimelineItem(
            id: "$message",
            eventID: "$message",
            senderID: "@alice:matrix.org",
            senderProfileDisplayName: "Alice Example",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Hello"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )

        XCTAssertEqual(item.senderDisplayName, "Alice Example")
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

    func testRoomNoteOrderingMovesOrdinaryNoteWithoutChangingEditTimestamp() {
        let timestamp = Date(timeIntervalSince1970: 1_800_000_000)
        let notes = [
            SynaraRoomNoteItem(
                id: "third", kind: .note, roomID: "!room:example.org", createdAt: timestamp,
                updatedAt: timestamp, body: "Third", completedAt: nil, order: 300,
                eventID: nil, eventTimestamp: nil, senderID: nil
            ),
            SynaraRoomNoteItem(
                id: "second", kind: .note, roomID: "!room:example.org", createdAt: timestamp,
                updatedAt: timestamp, body: "Second", completedAt: nil, order: 200,
                eventID: nil, eventTimestamp: nil, senderID: nil
            ),
            SynaraRoomNoteItem(
                id: "first", kind: .note, roomID: "!room:example.org", createdAt: timestamp,
                updatedAt: timestamp, body: "First", completedAt: nil, order: 100,
                eventID: nil, eventTimestamp: nil, senderID: nil
            ),
        ]

        let result = RoomNoteOrdering.moving(itemID: "first", to: 1, in: notes)

        XCTAssertEqual(result?.items.map(\.id), ["third", "first", "second"])
        XCTAssertEqual(result?.order, 250)
        XCTAssertEqual(result?.movedItem.updatedAt, timestamp)
    }

    func testMockRoomNotesServiceRunsRoomScopedCrudAndItemOrderingPath() async {
        let now = Date(timeIntervalSince1970: 1_800_000_000)
        let first = SynaraRoomNoteItem(
            id: "first", kind: .todo, roomID: "!room:example.org", createdAt: now,
            updatedAt: now, body: "First", completedAt: nil, order: 300,
            eventID: nil, eventTimestamp: nil, senderID: nil
        )
        let second = SynaraRoomNoteItem(
            id: "second", kind: .todo, roomID: "!room:example.org", createdAt: now,
            updatedAt: now, body: "Second", completedAt: nil, order: 200,
            eventID: nil, eventTimestamp: nil, senderID: nil
        )
        let otherRoom = SynaraRoomNoteItem(
            id: "other", kind: .note, roomID: "!other:example.org", createdAt: now,
            updatedAt: now, body: "Other", completedAt: nil, order: nil,
            eventID: nil, eventTimestamp: nil, senderID: nil
        )
        let service = MockRoomNotesService(items: [second, otherRoom, first], now: { now })

        guard case let .success(initial) = await service.loadItems(roomID: first.roomID) else {
            return XCTFail("Expected room notes snapshot")
        }
        XCTAssertEqual(initial.map(\.id), ["first", "second"])

        guard case let .success(moved) = await service.moveTodo(
            roomID: first.roomID,
            itemID: second.id,
            direction: .up
        ) else {
            return XCTFail("Expected reordered snapshot")
        }
        XCTAssertEqual(moved.map(\.id), ["second", "first"])

        guard case let .success(completed) = await service.setTodoCompleted(
            roomID: first.roomID,
            itemID: second.id,
            completed: true
        ) else {
            return XCTFail("Expected completed snapshot")
        }
        XCTAssertEqual(completed.first?.id, "first")
        XCTAssertEqual(completed.last?.id, "second")
        XCTAssertTrue(completed.last?.isCompleted == true)

        guard case let .success(added) = await service.addItem(
            roomID: first.roomID,
            kind: .note,
            body: "  Private cross-client note  "
        ) else {
            return XCTFail("Expected added snapshot")
        }
        XCTAssertEqual(added.first(where: { $0.kind == .note })?.body, "Private cross-client note")

        guard let addedNote = added.first(where: { $0.kind == .note }),
              case let .success(updated) = await service.updateItem(
                  addedNote,
                  body: "  Updated private note  "
              )
        else {
            return XCTFail("Expected updated snapshot")
        }
        XCTAssertEqual(updated.first(where: { $0.id == addedNote.id })?.body, "Updated private note")

        guard let updatedNote = updated.first(where: { $0.id == addedNote.id }),
              case let .success(ranked) = await service.setItemOrder(updatedNote, order: 9_999)
        else {
            return XCTFail("Expected reordered note snapshot")
        }
        XCTAssertEqual(ranked.first(where: { $0.kind == .note })?.order, 9_999)

        guard case let .success(deleted) = await service.deleteItem(roomID: first.roomID, itemID: first.id) else {
            return XCTFail("Expected deleted snapshot")
        }
        XCTAssertFalse(deleted.contains(where: { $0.id == first.id }))
        guard case let .success(otherSnapshot) = await service.loadItems(roomID: otherRoom.roomID) else {
            return XCTFail("Expected other room snapshot")
        }
        XCTAssertEqual(otherSnapshot.map(\.id), [otherRoom.id])
    }

    func testMockRoomNotesServicePinsOnlyDurableMessageEvents() async {
        let now = Date(timeIntervalSince1970: 1_800_000_000)
        let service = MockRoomNotesService(now: { now })
        let message = TimelineItem(
            id: "$event", eventID: "$event", senderID: "@mina:example.org", timestamp: now,
            kind: .text("Pinned message preview"), replyToEventID: nil,
            actionCapabilities: TimelineRowActionCapabilities(
                canReact: true,
                canReply: true,
                canEdit: false,
                canRedact: false,
                canReport: true,
                canPin: false,
                canForward: true,
                canVote: false,
                canDeclineCall: false
            ),
            isEdited: false, reactions: [:]
        )

        let availability = TimelinePinActionAvailability.forItem(message)
        XCTAssertTrue(availability.canPinToPrivateNotes)
        XCTAssertFalse(availability.canPinToMatrixRoom)

        guard case let .success(items) = await service.pinMessage(
            roomID: "!room:example.org",
            item: message
        ) else {
            return XCTFail("Expected pinned message snapshot")
        }
        XCTAssertEqual(items.first?.kind, .message)
        XCTAssertEqual(items.first?.eventID, "$event")
        XCTAssertEqual(items.first?.body, "Pinned message preview")

        let pending = TimelineItem.pendingMessage(
            body: "Not durable",
            senderID: "@mina:example.org",
            replyToEventID: nil
        )
        let pendingResult = await service.pinMessage(roomID: "!room:example.org", item: pending)
        XCTAssertEqual(pendingResult, .failure(.invalidItem))

        let redacted = TimelineItem(
            id: "$redacted", eventID: "$redacted", senderID: "@mina:example.org", timestamp: now,
            kind: .redacted, replyToEventID: nil,
            actionCapabilities: TimelineRowActionCapabilities(
                canReact: false,
                canReply: false,
                canEdit: false,
                canRedact: false,
                canReport: true,
                canPin: true,
                canForward: false,
                canVote: false,
                canDeclineCall: false
            ),
            isEdited: false, reactions: [:]
        )
        XCTAssertEqual(
            TimelinePinActionAvailability.forItem(redacted),
            TimelinePinActionAvailability(
                canPinToPrivateNotes: false,
                canPinToMatrixRoom: false
            )
        )
        let redactedResult = await service.pinMessage(
            roomID: "!room:example.org",
            item: redacted
        )
        XCTAssertEqual(redactedResult, .failure(.invalidItem))
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

    private func richText(from segment: MatrixHTMLRenderer.Segment?) -> MatrixHTMLRenderer.RichText? {
        guard case let .richText(text) = segment else {
            return nil
        }
        return text
    }
}
