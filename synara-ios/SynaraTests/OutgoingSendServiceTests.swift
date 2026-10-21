import XCTest
@testable import Synara

final class OutgoingSendServiceTests: XCTestCase {
    private var temporaryFiles: [URL] = []

    override func tearDown() {
        for url in temporaryFiles {
            try? FileManager.default.removeItem(at: url)
        }
        temporaryFiles = []
        super.tearDown()
    }

    private func temporaryStoreURL() -> URL {
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("composer-\(UUID().uuidString).json")
        temporaryFiles.append(url)
        return url
    }

    private func coordinator(
        _ sender: RecordingMessageSendService,
        status: MatrixSyncStatus = .connected,
        store: ComposerLocalStore = ComposerLocalStore(fileURL: nil)
    ) -> OutgoingSendCoordinator {
        let connection = ConnectionStatusStore(reconnectingHold: 0)
        connection.update(status)
        return OutgoingSendCoordinator(
            messageSender: sender,
            connectionStatus: connection,
            queue: OutgoingMessageQueue(store: store)
        )
    }

    @discardableResult
    private func enqueue(
        _ coordinator: OutgoingSendCoordinator,
        id: String,
        body: String,
        roomID: String = "!room:matrix.org",
        clearingDraft: Bool = false
    ) -> OutgoingQueuedMessage {
        coordinator.enqueue(
            localID: id,
            roomID: roomID,
            body: body,
            formattedBody: nil,
            replyToEventID: nil,
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            clearingDraft: clearingDraft
        )
    }

    /// I23: the queue changed itself with `DispatchQueue.main.sync` from the
    /// cooperative pool, which deadlocks while the main thread waits on that
    /// work, and its list was read off-main without a lock.
    @MainActor
    func testQueueChangesFromOtherThreadsNeverWaitOnTheMainThread() async {
        let sender = RecordingMessageSendService()
        let coordinator = coordinator(sender)
        let queue = coordinator.queue
        // The main thread blocks until a background change finishes. The old
        // main-queue hop would never return here.
        DispatchQueue.global().sync {
            enqueue(coordinator, id: "local-blocked", body: "from a worker")
        }
        XCTAssertEqual(queue.currentItems.map(\.id), ["local-blocked"])

        await withTaskGroup(of: Void.self) { group in
            for index in 0..<50 {
                group.addTask {
                    queue.updateDeliveryStatus(id: "local-blocked", index.isMultiple(of: 2) ? .queued : .sending)
                    _ = queue.item(id: "local-blocked")
                    _ = queue.handoffItems()
                }
            }
        }
        XCTAssertEqual(queue.currentItems.count, 1)
        // SwiftUI's copy catches up on the main queue.
        await Task.yield()
        let published = expectation(description: "published")
        DispatchQueue.main.async { published.fulfill() }
        await fulfillment(of: [published], timeout: 2)
        XCTAssertEqual(queue.items.map(\.id), ["local-blocked"])
    }

    @MainActor
    func testSendWhileNotSendReadyIsHandedToCoreImmediately() async throws {
        // A send during launch, reconnect or offline goes to Core's
        // persistent send queue at once; nothing waits in memory for sync.
        for status in [MatrixSyncStatus.starting, .reconnecting, .disconnected, .stopped] {
            let sender = RecordingMessageSendService()
            let coordinator = coordinator(sender, status: status)
            let message = enqueue(coordinator, id: "$pending-\(status)", body: "On the subway")
            XCTAssertEqual(message.deliveryStatus, .sending)

            await coordinator.flush()

            XCTAssertEqual(sender.requests.map(\.body), ["On the subway"], "status \(status)")
            XCTAssertEqual(sender.requests.first?.transactionID, message.clientTransactionID)
            let handedOff = try XCTUnwrap(coordinator.queue.item(id: message.id))
            XCTAssertEqual(handedOff.transactionID, "sdk-txn-1")
            XCTAssertEqual(handedOff.deliveryStatus, .sending, "Core queued means still sending")
        }
    }

    @MainActor
    func testCoreQueuedIsSendingAndOnlyAnAcknowledgedEventIsSent() async throws {
        let sender = RecordingMessageSendService()
        let coordinator = coordinator(sender)
        let queued = enqueue(coordinator, id: "$pending-queued", body: "Queued")
        await coordinator.flush()
        XCTAssertEqual(coordinator.queue.item(id: queued.id)?.deliveryStatus, .sending)
        XCTAssertNil(coordinator.queue.item(id: queued.id)?.serverEventID)

        sender.acknowledgeWithEventID = "$acked:matrix.org"
        let acked = enqueue(coordinator, id: "$pending-acked", body: "Acked")
        await coordinator.flush()
        XCTAssertEqual(coordinator.queue.item(id: acked.id)?.deliveryStatus, .sent)
        XCTAssertEqual(coordinator.queue.item(id: acked.id)?.serverEventID, "$acked:matrix.org")
    }

    @MainActor
    func testCoreNotReadyKeepsTheMessageQueuedInOrderAndRetriesWithTheSameTransaction() async throws {
        let sender = RecordingMessageSendService()
        sender.error = MessageSendError.notReady
        let coordinator = coordinator(sender, status: .starting)
        let first = enqueue(coordinator, id: "$pending-1", body: "first")
        let second = enqueue(coordinator, id: "$pending-2", body: "second")

        await coordinator.flush()
        // The room waits behind its first message: the second is not tried.
        XCTAssertEqual(sender.requests.map(\.body), ["first"])
        XCTAssertEqual(coordinator.queue.item(id: first.id)?.deliveryStatus, .queued)
        XCTAssertEqual(coordinator.queue.item(id: second.id)?.deliveryStatus, .sending)

        sender.error = nil
        await coordinator.flush()
        XCTAssertEqual(sender.requests.map(\.body), ["first", "first", "second"])
        XCTAssertEqual(
            sender.requests[0].transactionID,
            sender.requests[1].transactionID,
            "a repeated hand-off reuses the caller transaction id, so Core never makes a second event"
        )
        XCTAssertNotEqual(sender.requests[1].transactionID, sender.requests[2].transactionID)
    }

    @MainActor
    func testRejectedSendIsFailedAndRetryReusesItsTransaction() async throws {
        let sender = RecordingMessageSendService()
        sender.error = MessageSendError.failed
        let coordinator = coordinator(sender)
        let message = enqueue(coordinator, id: "$pending-hard-fail", body: "Retry later")
        await coordinator.flush()

        let failedItem = try XCTUnwrap(coordinator.queue.item(id: message.id)?.asTimelineItem())
        XCTAssertEqual(failedItem.deliveryStatus, .failed)
        XCTAssertTrue(OutgoingSendPolicy.canRetry(failedItem))
        XCTAssertTrue(OutgoingSendPolicy.canDiscard(failedItem))

        sender.error = nil
        let retried = try XCTUnwrap(
            coordinator.retry(failedItem, roomID: "!room:matrix.org", senderID: "@alice:matrix.org")
        )
        XCTAssertEqual(retried.clientTransactionID, message.clientTransactionID)
        await coordinator.flush()

        XCTAssertEqual(sender.requests.map(\.transactionID), [message.clientTransactionID, message.clientTransactionID])
        XCTAssertEqual(coordinator.queue.item(id: message.id)?.deliveryStatus, .sending)
    }

    @MainActor
    func testRetryNoOpsForNonTextKinds() async {
        let sender = RecordingMessageSendService()
        let coordinator = coordinator(sender)
        let media = TimelineItem(
            id: "$pending-media",
            eventID: "$pending-media",
            senderID: "@alice:matrix.org",
            timestamp: Date(),
            kind: .mediaPlaceholder(
                MediaResource(
                    id: "$pending-media",
                    filename: "photo.jpg",
                    authenticatedURL: URL(string: "mxc://matrix.org/photo")!,
                    requiresAuthentication: true
                )
            ),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:],
            deliveryStatus: .failed
        )

        XCTAssertNil(OutgoingSendPolicy.retryBody(for: media))
        XCTAssertFalse(OutgoingSendPolicy.canRetry(media))
        XCTAssertNil(coordinator.retry(media, roomID: "!room:matrix.org", senderID: "@alice:matrix.org"))

        await coordinator.flush()

        XCTAssertTrue(sender.requests.isEmpty)
        XCTAssertTrue(coordinator.queue.currentItems.isEmpty)
    }

    @MainActor
    func testQueuedThreadReplyPreservesChildAndRootThroughHandOff() async {
        let sender = RecordingMessageSendService()
        let coordinator = coordinator(sender)
        let queued = coordinator.enqueue(
            localID: "$pending-thread-reply",
            roomID: "!room:matrix.org",
            body: "Reply to a child",
            formattedBody: nil,
            replyToEventID: "$child",
            threadRootEventID: "$root",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate
        )

        XCTAssertEqual(queued.asTimelineItem().replyToEventID, "$child")
        XCTAssertEqual(queued.asTimelineItem().threadRootEventID, "$root")

        await coordinator.flush()

        XCTAssertEqual(sender.requests.first?.replyToEventID, "$child")
        XCTAssertEqual(sender.requests.first?.threadRootEventID, "$root")
    }

    @MainActor
    func testConcurrentFlushRequestsRunOneFlusherInOrder() async {
        let sender = RecordingMessageSendService()
        sender.delayNanoseconds = 20_000_000
        let coordinator = coordinator(sender)
        enqueue(coordinator, id: "$pending-a", body: "a")
        enqueue(coordinator, id: "$pending-b", body: "b")

        // Reconnect, launch and a new send all ask for a flush at once.
        async let first: Void = coordinator.flush()
        async let second: Void = coordinator.flush()
        enqueue(coordinator, id: "$pending-c", body: "c")
        async let third: Void = coordinator.flush()
        _ = await (first, second, third)

        XCTAssertEqual(sender.requests.map(\.body), ["a", "b", "c"], "each message exactly once, in order")
        XCTAssertEqual(sender.maxConcurrentSends, 1, "never two hand-offs at the same time")
    }

    @MainActor
    func testTwoIdenticalOksAreTwoMessagesMatchedOnlyByTheirOwnTransactions() async throws {
        let sender = RecordingMessageSendService()
        let coordinator = coordinator(sender)
        let first = enqueue(coordinator, id: "$pending-ok-1", body: "ok")
        let second = enqueue(coordinator, id: "$pending-ok-2", body: "ok")
        await coordinator.flush()
        XCTAssertNotEqual(first.clientTransactionID, second.clientTransactionID)
        XCTAssertEqual(sender.requests.count, 2)

        // Only the first "ok" has reached the timeline (as its remote echo).
        var firstEcho = TimelineItem(
            id: "$ok-1:matrix.org", eventID: "$ok-1:matrix.org", senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate, kind: .text("ok"), replyToEventID: nil,
            isEdited: false, reactions: [:]
        )
        firstEcho.transactionID = coordinator.queue.item(id: first.id)?.transactionID
        coordinator.dropConfirmed(matching: [firstEcho])

        XCTAssertNil(coordinator.queue.item(id: first.id))
        XCTAssertNotNil(
            coordinator.queue.item(id: second.id),
            "an identical earlier ok must not drop the in-flight one"
        )
    }

    @MainActor
    func testUnsentMessageSurvivesRelaunchAndIsHandedOffWithItsOriginalTransaction() async throws {
        let url = temporaryStoreURL()
        let firstLaunch = coordinator(
            RecordingMessageSendService(),
            status: .starting,
            store: ComposerLocalStore(fileURL: url)
        )
        let message = enqueue(firstLaunch, id: "$pending-durable", body: "Before the kill")
        // Terminated before Core accepted it: nothing else ran.

        let sender = RecordingMessageSendService()
        let relaunched = coordinator(sender, status: .starting, store: ComposerLocalStore(fileURL: url))
        let restored = try XCTUnwrap(relaunched.queue.item(id: message.id))
        XCTAssertEqual(restored.body, "Before the kill")
        XCTAssertEqual(restored.deliveryStatus, .queued)
        XCTAssertEqual(restored.clientTransactionID, message.clientTransactionID)

        await relaunched.flush()
        XCTAssertEqual(sender.requests.map(\.transactionID), [message.clientTransactionID])

        // Accepted by Core: the SDK owns it, so it leaves the durable outbox.
        let third = coordinator(RecordingMessageSendService(), store: ComposerLocalStore(fileURL: url))
        XCTAssertTrue(third.queue.currentItems.isEmpty)
    }

    @MainActor
    func testEnqueueClearsTheDraftOnlyInTheSameDurableWriteAsTheMessage() {
        let url = temporaryStoreURL()
        let store = ComposerLocalStore(fileURL: url, draftWriteDelay: 60)
        let drafts = DraftStore(store: store)
        let coordinator = coordinator(RecordingMessageSendService(), store: store)
        drafts.setDraft("Hold this", roomID: "!room:matrix.org")
        store.flush()

        enqueue(coordinator, id: "$pending-draft", body: "Hold this", clearingDraft: true)

        let onDisk = ComposerLocalStore(fileURL: url).current
        XCTAssertNil(onDisk.drafts["!room:matrix.org"])
        XCTAssertEqual(onDisk.outbox.map(\.body), ["Hold this"])
        XCTAssertEqual(drafts.draft(roomID: "!room:matrix.org"), "")
    }

    @MainActor
    func testDiscardRemovesAPendingRowAndAbortsAnSdkLocalEcho() async throws {
        let sender = RecordingMessageSendService()
        sender.error = MessageSendError.failed
        let coordinator = coordinator(sender)
        let message = enqueue(coordinator, id: "$pending-discard", body: "drop me")
        await coordinator.flush()
        let failed = try XCTUnwrap(coordinator.queue.item(id: message.id)?.asTimelineItem())
        let discardedPending = await coordinator.discard(failed, roomID: "!room:matrix.org")
        XCTAssertEqual(discardedPending, true)
        XCTAssertNil(coordinator.queue.item(id: message.id))
        XCTAssertTrue(sender.discarded.isEmpty, "Core never held it")

        var wedged = TimelineItem(
            id: "sdk-item", eventID: "sdk-item", senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate, kind: .text("too big"), replyToEventID: nil,
            isEdited: false, reactions: [:]
        )
        wedged.transactionID = "sdk-txn-wedged"
        wedged.localEcho = .wedged(cause: "too_large")
        XCTAssertTrue(OutgoingSendPolicy.canDiscard(wedged))
        XCTAssertFalse(OutgoingSendPolicy.canRetry(wedged), "a 413 cannot succeed on retry")
        let discardedEcho = await coordinator.discard(wedged, roomID: "!room:matrix.org")
        XCTAssertEqual(discardedEcho, true)
        XCTAssertEqual(sender.discarded, ["sdk-txn-wedged"])
    }

    @MainActor
    func testRetryOnAnSdkLocalEchoGoesThroughCoreWithoutANewSend() async {
        let sender = RecordingMessageSendService()
        let coordinator = coordinator(sender)
        var parked = TimelineItem(
            id: "sdk-item", eventID: "sdk-item", senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate, kind: .text("later"), replyToEventID: nil,
            isEdited: false, reactions: [:]
        )
        parked.transactionID = "sdk-txn-parked"
        parked.localEcho = .failed(cause: "network")
        XCTAssertEqual(parked.presentedDeliveryStatus, .failed)
        XCTAssertTrue(OutgoingSendPolicy.canRetry(parked))
        XCTAssertNil(coordinator.retry(parked, roomID: "!room:matrix.org", senderID: "@alice:matrix.org"))

        let retried = await coordinator.retryLocalEcho(parked, roomID: "!room:matrix.org")

        XCTAssertTrue(retried)
        XCTAssertEqual(sender.retried, ["sdk-txn-parked"])
        XCTAssertTrue(sender.requests.isEmpty, "Retry must not add a second event")
    }

    func testFailedRowUsesContainmentAndNamedRetryAction() {
        XCTAssertTrue(
            TimelineRowAccessibility.containsChildren(
                deliveryStatus: .failed,
                kind: .text("Retry me"),
                replyCount: 0,
                hasApprovalPrompt: false
            )
        )
        XCTAssertEqual(
            TimelineRowAccessibility.retryActionTitle(deliveryStatus: .failed),
            "Retry"
        )
        XCTAssertFalse(
            TimelineRowAccessibility.containsChildren(
                deliveryStatus: .sending,
                kind: .text("Hello"),
                replyCount: 0,
                hasApprovalPrompt: false
            )
        )
        XCTAssertNil(TimelineRowAccessibility.retryActionTitle(deliveryStatus: .queued))
        XCTAssertNil(TimelineRowAccessibility.retryActionTitle(deliveryStatus: .sending))
        XCTAssertTrue(
            TimelineRowAccessibility.containsChildren(
                deliveryStatus: nil,
                kind: .text("Hello"),
                replyCount: 1,
                hasApprovalPrompt: false
            )
        )
        XCTAssertTrue(
            TimelineRowAccessibility.containsChildren(
                deliveryStatus: nil,
                kind: .text("Poll"),
                replyCount: 0,
                hasApprovalPrompt: false,
                hasInteractivePoll: true
            )
        )
    }

    func testSdkLocalEchoStatesProjectSendingAndFailedWithRetryAndDiscard() {
        XCTAssertEqual(TimelineLocalEchoState(coreState: "sending", cause: nil), .sending)
        XCTAssertEqual(
            TimelineLocalEchoState(coreState: "failed", cause: "network"),
            .failed(cause: "network")
        )
        XCTAssertEqual(
            TimelineLocalEchoState(coreState: "wedged", cause: "forbidden"),
            .wedged(cause: "forbidden")
        )
        XCTAssertNil(TimelineLocalEchoState(coreState: "sent", cause: nil))
        XCTAssertNil(TimelineLocalEchoState(coreState: nil, cause: nil))

        XCTAssertEqual(TimelineLocalEchoState.sending.deliveryStatus, .sending)
        XCTAssertEqual(TimelineLocalEchoState.wedged(cause: nil).deliveryStatus, .failed)
        XCTAssertTrue(TimelineLocalEchoState.wedged(cause: "forbidden").canRetry)
        XCTAssertFalse(TimelineLocalEchoState.wedged(cause: "too_large").canRetry)
        XCTAssertFalse(TimelineLocalEchoState.sending.canRetry)

        var sending = TimelineItem(
            id: "sdk-item", eventID: "sdk-item", senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate, kind: .text("on its way"), replyToEventID: nil,
            isEdited: false, reactions: [:]
        )
        sending.transactionID = "sdk-txn"
        sending.localEcho = .sending
        XCTAssertFalse(sending.isLocalPending, "an SDK echo stays a stream row")
        XCTAssertEqual(sending.presentedDeliveryStatus, .sending)
        XCTAssertTrue(OutgoingSendPolicy.canDiscard(sending))
        XCTAssertFalse(OutgoingSendPolicy.canRetry(sending))
    }

    func testPendingItemsSurfaceWhileTimelineIsLoadingOrFailed() {
        let pending = TimelineItem.pendingMessage(
            localID: "$pending-visible",
            body: "Hold this",
            senderID: "@alice:matrix.org",
            replyToEventID: nil,
            deliveryStatus: .queued
        )

        XCTAssertEqual(
            OutgoingQueueTimelineMerge.applying(pendingItems: [pending], to: .loading),
            .loaded([pending], isPaginating: false)
        )
        XCTAssertEqual(
            OutgoingQueueTimelineMerge.applying(pendingItems: [pending], to: .failed),
            .loaded([pending], isPaginating: false)
        )
        XCTAssertEqual(
            OutgoingQueueTimelineMerge.applying(pendingItems: [pending], to: .empty),
            .loaded([pending], isPaginating: false)
        )
        XCTAssertEqual(
            OutgoingQueueTimelineMerge.applying(pendingItems: [], to: .loading),
            .loading
        )
        XCTAssertEqual(
            OutgoingQueueTimelineMerge.applying(pendingItems: [], to: .failed),
            .failed
        )

        let existing = TimelineItem(
            id: "$server",
            eventID: "$server",
            senderID: "@bob:matrix.org",
            timestamp: Date(),
            kind: .text("Earlier"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
        let merged = OutgoingQueueTimelineMerge.applying(
            pendingItems: [pending],
            to: .loaded([existing], isPaginating: true)
        )
        guard case let .loaded(items, isPaginating) = merged else {
            return XCTFail("Expected loaded timeline")
        }
        XCTAssertTrue(isPaginating)
        XCTAssertEqual(items.map(\.id), ["$server", "$pending-visible"])
        XCTAssertEqual(items.last?.deliveryStatus, .queued)
    }

    func testRoomTimelineShowsRetryAndDiscardOnTheBubbleWithOneFlusher() throws {
        let root = Self.repositoryRoot()
        let timeline = try String(
            contentsOfFile: "\(root)/synara-ios/Synara/Features/RoomTimelineView.swift",
            encoding: .utf8
        )
        let bubble = try String(
            contentsOfFile: "\(root)/synara-ios/Synara/SharedUI/SynaraMessageBubble.swift",
            encoding: .utf8
        )
        let shell = try String(
            contentsOfFile: "\(root)/synara-ios/Synara/App/RootShellView.swift",
            encoding: .utf8
        )

        XCTAssertFalse(timeline.contains("Button(action: onRetryFailedSend)"))
        XCTAssertFalse(timeline.contains("TimelineItemRetry-\\(item.eventID)"))
        XCTAssertTrue(timeline.contains("retryFailedMessage(item)"))
        XCTAssertTrue(timeline.contains("retryFailedMessage(eventRow.item)"))
        XCTAssertTrue(timeline.contains("discardFailedMessage(item)"))
        XCTAssertTrue(timeline.contains("discardFailedMessage(eventRow.item)"))
        XCTAssertTrue(timeline.contains("accessibilityAction(named: Text(\"Retry\"), onRetryFailedSend)"))
        XCTAssertTrue(timeline.contains("accessibilityAction(named: Text(\"Discard\"), onDiscardFailedSend)"))
        XCTAssertTrue(timeline.contains("deliveryStatus: item.presentedDeliveryStatus"))
        XCTAssertTrue(timeline.contains("TimelineRowAccessibility.containsChildren("))
        XCTAssertTrue(timeline.contains("case .idle, .loading, .empty, .failed:"))
        XCTAssertTrue(bubble.contains("TimelineItemRetry"))
        XCTAssertTrue(bubble.contains("TimelineItemDiscard"))
        XCTAssertTrue(bubble.contains("TimelineItemQueued"))
        XCTAssertTrue(bubble.contains("TimelineItemSending"))
        XCTAssertTrue(bubble.contains("accessibilityLabel(\"Queued\")"))
        XCTAssertTrue(bubble.contains("accessibilityLabel(\"Sending\")"))
        // One flusher: the shell's send-ready edge, plus hand-off on send
        // (room and thread). The timeline's connection observer only retries
        // a failed load; it never flushes.
        XCTAssertFalse(timeline.contains("flushWhenSendReady"))
        XCTAssertEqual(timeline.components(separatedBy: "outgoingSends.flush()").count - 1, 2)
        XCTAssertTrue(shell.contains("outgoingSends.flush()"))
    }

    @MainActor
    func testThreadReplyShowsOnceAndOnlyInItsThread() async throws {
        // I6: a thread reply goes through the outbox like a room message, so
        // its queued row gives way to the SDK echo with the same transaction
        // id instead of staying beside it.
        let root = "$root:matrix.org"
        let sender = RecordingMessageSendService()
        let coordinator = coordinator(sender)
        let reply = coordinator.enqueue(
            localID: "$pending-thread",
            roomID: "!room:matrix.org",
            body: "In the thread",
            formattedBody: nil,
            replyToEventID: nil,
            threadRootEventID: root,
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate
        )
        XCTAssertEqual(
            coordinator.queue.timelineItems(in: "!room:matrix.org", threadRootEventID: nil),
            [],
            "the room timeline never shows a thread reply"
        )
        XCTAssertEqual(
            coordinator.queue.timelineItems(in: "!room:matrix.org", threadRootEventID: root).map(\.id),
            [reply.id]
        )

        await coordinator.flush()
        XCTAssertEqual(sender.requests.first?.threadRootEventID, root)
        let pending = coordinator.queue.timelineItems(in: "!room:matrix.org", threadRootEventID: root)
        XCTAssertEqual(pending.first?.transactionID, "sdk-txn-1")

        let rootItem = TimelineItem(
            id: root,
            eventID: root,
            senderID: "@bob:matrix.org",
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(-60),
            kind: .text("Root"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
        var sdkEcho = TimelineItem(
            id: "sdk-local-1",
            eventID: "sdk-local-1",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate.addingTimeInterval(1),
            kind: .text("In the thread"),
            replyToEventID: nil,
            threadRootEventID: root,
            isEdited: false,
            reactions: [:]
        )
        sdkEcho.transactionID = "sdk-txn-1"
        sdkEcho.serverEventID = nil
        let snapshot = [rootItem, sdkEcho]

        XCTAssertEqual(
            ThreadTimelineMerge.merged(snapshot: snapshot, pending: pending).map(\.id),
            [root, "sdk-local-1"],
            "one row for the reply"
        )
        coordinator.dropConfirmed(matching: snapshot)
        XCTAssertTrue(coordinator.queue.currentItems.isEmpty)
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

private final class RecordingMessageSendService: MessageSending, @unchecked Sendable {
    private let lock = NSLock()
    private(set) var requests: [MessageSendRequest] = []
    private(set) var retried: [String] = []
    private(set) var discarded: [String] = []
    private(set) var maxConcurrentSends = 0
    private var inFlight = 0
    var error: Error?
    var acknowledgeWithEventID: String?
    var delayNanoseconds: UInt64 = 0

    func send(_ request: MessageSendRequest) async throws -> TimelineItem {
        throw MessageSendError.failed
    }

    func sendText(_ request: MessageSendRequest) async throws -> MessageSendReceipt {
        lock.lock()
        requests.append(request)
        inFlight += 1
        maxConcurrentSends = max(maxConcurrentSends, inFlight)
        let count = requests.count
        lock.unlock()
        if delayNanoseconds > 0 {
            try? await Task.sleep(nanoseconds: delayNanoseconds)
        }
        lock.lock()
        inFlight -= 1
        lock.unlock()
        if let error {
            throw error
        }
        return MessageSendReceipt(
            transactionID: "sdk-txn-\(count)",
            eventID: acknowledgeWithEventID,
            isQueued: acknowledgeWithEventID == nil
        )
    }

    func retryLocalEcho(roomID _: String, transactionID: String) async throws {
        retried.append(transactionID)
    }

    func discardLocalEcho(roomID _: String, transactionID: String) async throws -> Bool {
        discarded.append(transactionID)
        return true
    }
}
