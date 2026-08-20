import XCTest
@testable import Synara

final class OutgoingSendServiceTests: XCTestCase {
    @MainActor
    func testRetryWithTextBodyRequeuesAndSends() async {
        let sender = RecordingMessageSendService()
        let connection = ConnectionStatusStore(reconnectingHold: 0)
        connection.update(.connected)
        let coordinator = OutgoingSendCoordinator(messageSender: sender, connectionStatus: connection)
        let failed = TimelineItem.pendingMessage(
            localID: "$pending-retry",
            body: "Ship it",
            formattedBody: "<p>Ship it</p>",
            senderID: "@alice:matrix.org",
            replyToEventID: "$parent",
            deliveryStatus: .failed
        )

        let queued = coordinator.retry(failed, roomID: "!room:matrix.org", senderID: "@alice:matrix.org")

        XCTAssertEqual(queued?.body, "Ship it")
        XCTAssertEqual(queued?.deliveryStatus, .sending)
        XCTAssertEqual(OutgoingSendPolicy.retryBody(for: failed), "Ship it")
        XCTAssertTrue(OutgoingSendPolicy.canRetry(failed))

        await coordinator.transmitIfNeeded(id: "$pending-retry")

        XCTAssertEqual(sender.requests.count, 1)
        XCTAssertEqual(sender.requests.first?.body, "Ship it")
        XCTAssertEqual(sender.requests.first?.roomID, "!room:matrix.org")
        XCTAssertEqual(sender.requests.first?.replyToEventID, "$parent")
        XCTAssertNil(sender.requests.first?.editEventID)
        XCTAssertEqual(coordinator.queue.item(id: "$pending-retry")?.deliveryStatus, .sent)
    }

    @MainActor
    func testRetryNoOpsForNonTextKinds() async {
        let sender = RecordingMessageSendService()
        let connection = ConnectionStatusStore(reconnectingHold: 0)
        connection.update(.connected)
        let coordinator = OutgoingSendCoordinator(messageSender: sender, connectionStatus: connection)
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
        let redacted = TimelineItem(
            id: "$pending-redacted",
            eventID: "$pending-redacted",
            senderID: "@alice:matrix.org",
            timestamp: Date(),
            kind: .redacted,
            replyToEventID: nil,
            isEdited: false,
            reactions: [:],
            deliveryStatus: .failed
        )

        XCTAssertNil(OutgoingSendPolicy.retryBody(for: media))
        XCTAssertNil(OutgoingSendPolicy.retryBody(for: redacted))
        XCTAssertFalse(OutgoingSendPolicy.canRetry(media))
        XCTAssertNil(coordinator.retry(media, roomID: "!room:matrix.org", senderID: "@alice:matrix.org"))
        XCTAssertNil(coordinator.retry(redacted, roomID: "!room:matrix.org", senderID: "@alice:matrix.org"))

        await coordinator.flushWhenSendReady()

        XCTAssertTrue(sender.requests.isEmpty)
        XCTAssertTrue(coordinator.queue.items.isEmpty)
    }

    @MainActor
    func testOfflineSendMarksQueuedNotDropped() async {
        let sender = RecordingMessageSendService()
        let connection = ConnectionStatusStore(reconnectingHold: 0)
        connection.update(.disconnected)
        let coordinator = OutgoingSendCoordinator(messageSender: sender, connectionStatus: connection)

        let queued = coordinator.enqueue(
            localID: "$pending-offline",
            roomID: "!room:matrix.org",
            body: "Hold this",
            formattedBody: nil,
            replyToEventID: nil,
            senderID: "@alice:matrix.org",
            timestamp: Date()
        )

        XCTAssertEqual(queued.deliveryStatus, .queued)
        XCTAssertEqual(OutgoingSendPolicy.initialDeliveryStatus(isSendReady: false), .queued)

        await coordinator.transmitIfNeeded(queued)

        XCTAssertTrue(sender.requests.isEmpty)
        XCTAssertEqual(coordinator.queue.item(id: "$pending-offline")?.deliveryStatus, .queued)
        XCTAssertEqual(coordinator.queue.timelineItems(in: "!room:matrix.org").count, 1)
    }

    @MainActor
    func testFlushWhenConnectedSendsQueuedAndFailedText() async {
        let sender = RecordingMessageSendService()
        let connection = ConnectionStatusStore(reconnectingHold: 0)
        connection.update(.disconnected)
        let coordinator = OutgoingSendCoordinator(messageSender: sender, connectionStatus: connection)
        coordinator.enqueue(
            localID: "$pending-queued",
            roomID: "!room:matrix.org",
            body: "Queued body",
            formattedBody: nil,
            replyToEventID: nil,
            senderID: "@alice:matrix.org",
            timestamp: Date()
        )
        let failed = coordinator.enqueue(
            localID: "$pending-failed",
            roomID: "!other:matrix.org",
            body: "Failed body",
            formattedBody: nil,
            replyToEventID: nil,
            senderID: "@alice:matrix.org",
            timestamp: Date()
        )
        coordinator.queue.updateDeliveryStatus(id: failed.id, .failed)

        await coordinator.flushWhenSendReady()
        XCTAssertTrue(sender.requests.isEmpty)

        connection.update(.connected)
        XCTAssertTrue(OutgoingSendPolicy.becameSendReady(previous: .disconnected, current: .connected))
        await coordinator.flushWhenSendReady()

        XCTAssertEqual(sender.requests.map(\.body), ["Queued body", "Failed body"])
        XCTAssertEqual(coordinator.queue.item(id: "$pending-queued")?.deliveryStatus, .sent)
        XCTAssertEqual(coordinator.queue.item(id: "$pending-failed")?.deliveryStatus, .sent)
    }

    func testDeliveryStatusAfterFailureDependsOnReadiness() {
        XCTAssertEqual(OutgoingSendPolicy.deliveryStatusAfterFailure(isSendReady: true), .failed)
        XCTAssertEqual(OutgoingSendPolicy.deliveryStatusAfterFailure(isSendReady: false), .queued)
        XCTAssertTrue(OutgoingSendPolicy.isSendReady(.connected))
        XCTAssertTrue(OutgoingSendPolicy.isSendReady(.syncing))
        XCTAssertFalse(OutgoingSendPolicy.isSendReady(.disconnected))
        XCTAssertFalse(OutgoingSendPolicy.isSendReady(.reconnecting))
    }

    @MainActor
    func testFailedSendWhileConnectedStaysFailedAndRetryable() async {
        let sender = RecordingMessageSendService()
        sender.error = MessageSendError.failed
        let connection = ConnectionStatusStore(reconnectingHold: 0)
        connection.update(.connected)
        let coordinator = OutgoingSendCoordinator(messageSender: sender, connectionStatus: connection)
        let message = coordinator.enqueue(
            localID: "$pending-hard-fail",
            roomID: "!room:matrix.org",
            body: "Retry later",
            formattedBody: nil,
            replyToEventID: nil,
            senderID: "@alice:matrix.org",
            timestamp: Date()
        )
        await coordinator.transmitIfNeeded(message)

        let failedItem = try XCTUnwrap(coordinator.queue.item(id: message.id)?.asTimelineItem())
        XCTAssertEqual(failedItem.deliveryStatus, .failed)
        XCTAssertTrue(OutgoingSendPolicy.canRetry(failedItem))

        sender.error = nil
        let retried = try XCTUnwrap(coordinator.retry(failedItem, roomID: "!room:matrix.org", senderID: "@alice:matrix.org"))
        await coordinator.transmitIfNeeded(retried)

        XCTAssertEqual(sender.requests.last?.body, "Retry later")
        XCTAssertEqual(coordinator.queue.item(id: message.id)?.deliveryStatus, .sent)
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

    func testRoomTimelineDoesNotWrapFailedRowsInRetryButton() throws {
        let root = Self.repositoryRoot()
        let timeline = try String(
            contentsOfFile: "\(root)/synara-ios/Synara/Features/RoomTimelineView.swift",
            encoding: .utf8
        )
        let bubble = try String(
            contentsOfFile: "\(root)/synara-ios/Synara/SharedUI/SynaraMessageBubble.swift",
            encoding: .utf8
        )

        XCTAssertFalse(timeline.contains("Button(action: onRetryFailedSend)"))
        XCTAssertFalse(timeline.contains("TimelineItemRetry-\\(item.eventID)"))
        XCTAssertTrue(timeline.contains("retryFailedMessage(item)"))
        XCTAssertTrue(timeline.contains("retryFailedMessage(eventRow.item)"))
        XCTAssertTrue(timeline.contains("accessibilityAction(named: Text(\"Retry\"), onRetryFailedSend)"))
        XCTAssertTrue(timeline.contains("TimelineRowAccessibility.containsChildren("))
        XCTAssertTrue(timeline.contains("case .idle, .loading, .empty, .failed:"))
        XCTAssertTrue(bubble.contains("TimelineItemRetry"))
        XCTAssertTrue(bubble.contains("TimelineItemQueued"))
        XCTAssertTrue(bubble.contains("TimelineItemSending"))
        XCTAssertTrue(bubble.contains("accessibilityLabel(\"Queued\")"))
        XCTAssertTrue(bubble.contains("accessibilityLabel(\"Sending\")"))
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

private final class RecordingMessageSendService: MessageSending {
    private(set) var requests: [MessageSendRequest] = []
    var error: Error?

    func send(_ request: MessageSendRequest) async throws -> TimelineItem {
        requests.append(request)
        if let error {
            throw error
        }
        let body = request.body.trimmingCharacters(in: .whitespacesAndNewlines)
        let eventID = "$sent-\(requests.count)"
        return TimelineItem(
            id: eventID,
            eventID: eventID,
            senderID: "@local:matrix.org",
            timestamp: Date(),
            kind: .text(body),
            replyToEventID: request.replyToEventID,
            isEdited: false,
            reactions: [:]
        )
    }
}
