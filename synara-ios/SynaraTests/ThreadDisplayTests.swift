import SynaraCore
import XCTest
@testable import Synara

final class ThreadDisplayTests: XCTestCase {
    private let root = "$root:matrix.org"

    private func item(
        _ eventID: String,
        at seconds: TimeInterval,
        serverEventID: String? = nil,
        threadRoot: String? = nil
    ) -> TimelineItem {
        TimelineItem(
            id: eventID,
            eventID: eventID,
            serverEventID: serverEventID,
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(seconds),
            kind: .text(eventID),
            replyToEventID: nil,
            threadRootEventID: threadRoot,
            actionCapabilities: TimelineRowActionCapabilities(
                canReact: true, canReply: true, canEdit: false, canRedact: false,
                canReport: true, canPin: true, canForward: true, canVote: false, canDeclineCall: false
            ),
            isEdited: false,
            reactions: [:]
        )
    }

    func testThreadDisplayDefaultsToInlineAndIgnoresUnknownValues() {
        XCTAssertEqual(ThreadDisplayMode.defaultMode, .inline)
        XCTAssertEqual(ThreadDisplayMode.resolve(nil), .inline)
        XCTAssertEqual(ThreadDisplayMode.resolve("side"), .inline)
        XCTAssertEqual(ThreadDisplayMode.resolve("sheet"), .sheet)
        XCTAssertEqual(ThreadDisplayMode.resolve("full"), .full)
        XCTAssertEqual(ThreadDisplayMode.allCases.map(\.title), ["Inline", "Sheet", "Full Screen"])
    }

    func testThreadDisplayPersistsThroughSettingsStore() {
        let settings = InMemorySettingsStore()
        settings.setString(ThreadDisplayMode.full.rawValue, for: ThreadDisplayMode.settingsKey)
        XCTAssertEqual(ThreadDisplayMode.resolve(settings.string(for: ThreadDisplayMode.settingsKey)), .full)
    }

    func testThreadFocusAnchorScrollsToTheRoutedReply() {
        let items = [item(root, at: 0), item("$r1", at: 1, serverEventID: "$server-r1", threadRoot: root)]
        XCTAssertEqual(
            ThreadFocusAnchorPolicy.nextStep(
                focusedEventID: "$server-r1",
                items: items,
                canLoadEarlier: false,
                earlierPagesLoaded: 0
            ),
            .scroll(itemID: "$r1")
        )
    }

    func testThreadFocusAnchorLoadsEarlierRepliesWithinItsBudget() {
        let items = [item("$r9", at: 9, threadRoot: root)]
        XCTAssertEqual(
            ThreadFocusAnchorPolicy.nextStep(
                focusedEventID: "$r2",
                items: items,
                canLoadEarlier: true,
                earlierPagesLoaded: 0
            ),
            .loadEarlier
        )
        XCTAssertEqual(
            ThreadFocusAnchorPolicy.nextStep(
                focusedEventID: "$r2",
                items: items,
                canLoadEarlier: true,
                earlierPagesLoaded: ThreadFocusAnchorPolicy.maximumEarlierPages
            ),
            .settle
        )
        XCTAssertEqual(
            ThreadFocusAnchorPolicy.nextStep(
                focusedEventID: "$r2",
                items: items,
                canLoadEarlier: false,
                earlierPagesLoaded: 0
            ),
            .settle
        )
    }

    func testCoreEventRouteMapsToTheThreadRoute() {
        XCTAssertEqual(
            SharedCoreTimeline.eventRoute(from: TimelineEventRouteDto(kind: .thread, threadRootEventId: root)),
            .thread(rootEventID: root)
        )
        XCTAssertEqual(
            SharedCoreTimeline.eventRoute(from: TimelineEventRouteDto(kind: .thread, threadRootEventId: nil)),
            .room
        )
        XCTAssertEqual(
            SharedCoreTimeline.eventRoute(from: TimelineEventRouteDto(kind: .missing, threadRootEventId: nil)),
            .missing
        )
        XCTAssertEqual(
            SharedCoreTimeline.eventRoute(from: TimelineEventRouteDto(kind: .unreachable, threadRootEventId: nil)),
            .unreachable
        )
        XCTAssertEqual(
            SharedCoreTimeline.eventRoute(from: TimelineEventRouteDto(kind: .room, threadRootEventId: nil)),
            .room
        )
    }

    func testRoutedThreadCarriesItsReplyThroughEveryPresentation() {
        XCTAssertEqual(
            ThreadSheetTarget(rootEventID: root, rootTitle: nil, focusedEventID: "$reply").focusedEventID,
            "$reply"
        )
        let route = AppRoute.thread(roomID: "!room:matrix.org", rootEventID: root, focusedEventID: "$reply")
        XCTAssertTrue(route.isConversation)
        XCTAssertEqual(route.conversationRoomID, "!room:matrix.org")
        XCTAssertNotEqual(route, AppRoute.thread(roomID: "!room:matrix.org", rootEventID: root))
    }

    func testThreadStreamsUseThreadOpenKindAndTheirOwnKey() {
        let roomKey = SharedCoreTimelineStreamKey.room("!room:matrix.org")
        let threadKey = SharedCoreTimelineStreamKey.thread(roomID: "!room:matrix.org", rootEventID: root)
        let otherThreadKey = SharedCoreTimelineStreamKey.thread(roomID: "!room:matrix.org", rootEventID: "$other")
        XCTAssertNotEqual(roomKey, threadKey)
        XCTAssertNotEqual(threadKey, otherThreadKey)

        let threadPosition = SharedCoreTimelineStreamKey.threadPosition(rootEventID: root)
        XCTAssertEqual(threadPosition.kind, .thread)
        XCTAssertEqual(threadPosition.eventId, root)

        XCTAssertEqual(SharedCoreTimelineStreamKey.roomPosition(focusedEventID: nil).kind, .liveBottom)
        XCTAssertEqual(SharedCoreTimelineStreamKey.roomPosition(focusedEventID: "$e").kind, .focused)
        XCTAssertTrue(SharedCoreTimelineUpdateBootstrap.shouldRefreshThreadStream)
    }

    func testSentRepliesStayVisibleUntilTheStreamDeliversThem() {
        let rootItem = item(root, at: 0)
        let sent = item("$sent", at: 10, threadRoot: root)

        let beforeEcho = ThreadTimelineMerge.merged(snapshot: [rootItem], pending: [sent])
        XCTAssertEqual(beforeEcho.map(\.eventID), [root, "$sent"])

        let echoed = item("$sent", at: 10, threadRoot: root)
        XCTAssertEqual(ThreadTimelineMerge.unresolved(pending: [sent], snapshot: [rootItem, echoed]), [])
        XCTAssertEqual(
            ThreadTimelineMerge.merged(snapshot: [rootItem, echoed], pending: [sent]).map(\.eventID),
            [root, "$sent"]
        )

        let localEcho = item("local-1", at: 11, serverEventID: "$server", threadRoot: root)
        let serverCopy = item("$server", at: 11, threadRoot: root)
        XCTAssertEqual(ThreadTimelineMerge.unresolved(pending: [localEcho], snapshot: [serverCopy]), [])
    }

    func testInlineWindowShowsNewestRepliesWithoutTheRoot() {
        let items = [item(root, at: 0)] + (1...8).map { item("$r\($0)", at: TimeInterval($0), threadRoot: root) }
        let replies = InlineThreadWindow.replies(in: items.shuffled(), rootEventID: root)
        XCTAssertEqual(replies.count, 8)
        XCTAssertFalse(replies.contains { $0.eventID == root })
        XCTAssertEqual(
            InlineThreadWindow.visible(replies, limit: InlineThreadWindow.initialReplies).map(\.eventID),
            ["$r4", "$r5", "$r6", "$r7", "$r8"]
        )
        XCTAssertEqual(InlineThreadWindow.visible(replies, limit: 100).count, 8)
    }

    func testReplyInThreadFocusRequestIsConsumedOnce() {
        let requests = ThreadComposerFocusRequests()
        XCTAssertFalse(requests.consume(roomID: "!room", rootEventID: root))
        requests.request(roomID: "!room", rootEventID: root)
        XCTAssertFalse(requests.consume(roomID: "!room", rootEventID: "$other"))
        XCTAssertTrue(requests.consume(roomID: "!room", rootEventID: root))
        XCTAssertFalse(requests.consume(roomID: "!room", rootEventID: root))
    }
}
