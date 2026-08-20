import Combine
import Foundation

/// Product policy for iOS outbound text reliability.
///
/// Native `sendText` fails closed instead of leaving work in the SDK send
/// queue. This layer accepts the compose, keeps a session-scoped pending
/// list, and flushes when sync is running/connected.
enum OutgoingSendPolicy {
    static func isSendReady(_ status: MatrixSyncStatus) -> Bool {
        switch status {
        case .connected, .syncing:
            return true
        case .starting, .reconnecting, .disconnected, .restoreFailed, .stopped, .failed:
            return false
        }
    }

    static func becameSendReady(previous: MatrixSyncStatus, current: MatrixSyncStatus) -> Bool {
        isSendReady(current) && isSendReady(previous) == false
    }

    static func initialDeliveryStatus(isSendReady: Bool) -> TimelineDeliveryStatus {
        isSendReady ? .sending : .queued
    }

    static func deliveryStatusAfterFailure(isSendReady: Bool) -> TimelineDeliveryStatus {
        isSendReady ? .failed : .queued
    }

    /// Retry is text-only. Non-text kinds return nil so the chip can no-op
    /// without looking like it sent.
    static func retryBody(for item: TimelineItem) -> String? {
        guard item.deliveryStatus == .failed else {
            return nil
        }
        return TimelinePendingReconciler.messageBody(for: item)
    }

    static func canRetry(_ item: TimelineItem) -> Bool {
        retryBody(for: item) != nil
    }

    static func isFlushable(_ item: OutgoingQueuedMessage) -> Bool {
        switch item.deliveryStatus {
        case .queued, .failed:
            return item.body.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
        case .sending, .sent:
            return false
        }
    }
}

struct OutgoingQueuedMessage: Equatable, Identifiable {
    let id: String
    let roomID: String
    let body: String
    let formattedBody: String?
    let replyToEventID: String?
    let senderID: String
    let timestamp: Date
    var deliveryStatus: TimelineDeliveryStatus

    func asTimelineItem() -> TimelineItem {
        TimelineItem.pendingMessage(
            localID: id,
            body: body,
            formattedBody: formattedBody,
            senderID: senderID,
            replyToEventID: replyToEventID,
            deliveryStatus: deliveryStatus,
            timestamp: timestamp
        )
    }

    func sendRequest() -> MessageSendRequest {
        MessageSendRequest(
            roomID: roomID,
            body: body,
            formattedBody: formattedBody,
            replyToEventID: replyToEventID,
            editEventID: nil
        )
    }
}

/// Session-scoped pending list. Survives leaving a room; cleared on wipe.
final class OutgoingMessageQueue: ObservableObject {
    @Published private(set) var items: [OutgoingQueuedMessage] = []

    func item(id: String) -> OutgoingQueuedMessage? {
        items.first(where: { $0.id == id })
    }

    func items(in roomID: String) -> [OutgoingQueuedMessage] {
        items.filter { $0.roomID == roomID }
    }

    func timelineItems(in roomID: String) -> [TimelineItem] {
        items(in: roomID).map { $0.asTimelineItem() }
    }

    func flushableItems() -> [OutgoingQueuedMessage] {
        items.filter(OutgoingSendPolicy.isFlushable)
    }

    func upsert(_ message: OutgoingQueuedMessage) {
        mutate { items in
            if let index = items.firstIndex(where: { $0.id == message.id }) {
                items[index] = message
            } else {
                items.append(message)
            }
        }
    }

    func updateDeliveryStatus(id: String, _ status: TimelineDeliveryStatus) {
        mutate { items in
            guard let index = items.firstIndex(where: { $0.id == id }) else {
                return
            }
            items[index].deliveryStatus = status
        }
    }

    func remove(id: String) {
        mutate { items in
            items.removeAll { $0.id == id }
        }
    }

    func clear() {
        mutate { items in
            items.removeAll()
        }
    }

    private func mutate(_ body: (inout [OutgoingQueuedMessage]) -> Void) {
        if Thread.isMainThread {
            body(&items)
        } else {
            DispatchQueue.main.sync {
                body(&items)
            }
        }
    }
}

/// Enqueues text sends, transmits when connected, and auto-flushes queued
/// plus failed-sendable items when sync becomes ready.
final class OutgoingSendCoordinator {
    let queue: OutgoingMessageQueue
    private let messageSender: MessageSending
    private let connectionStatus: ConnectionStatusStore
    private var inFlight = Set<String>()
    private let inFlightLock = NSLock()

    init(
        messageSender: MessageSending,
        connectionStatus: ConnectionStatusStore,
        queue: OutgoingMessageQueue = OutgoingMessageQueue()
    ) {
        self.messageSender = messageSender
        self.connectionStatus = connectionStatus
        self.queue = queue
    }

    @discardableResult
    func enqueue(
        localID: String,
        roomID: String,
        body: String,
        formattedBody: String?,
        replyToEventID: String?,
        senderID: String,
        timestamp: Date
    ) -> OutgoingQueuedMessage {
        let message = OutgoingQueuedMessage(
            id: localID,
            roomID: roomID,
            body: body,
            formattedBody: formattedBody,
            replyToEventID: replyToEventID,
            senderID: senderID,
            timestamp: timestamp,
            deliveryStatus: OutgoingSendPolicy.initialDeliveryStatus(
                isSendReady: OutgoingSendPolicy.isSendReady(connectionStatus.status)
            )
        )
        queue.upsert(message)
        return message
    }

    /// Rebuilds a failed text row onto the same local id. Returns nil for
    /// non-text (or non-failed) rows so retry is a true no-op.
    func retry(_ item: TimelineItem, roomID: String, senderID: String) -> OutgoingQueuedMessage? {
        guard let body = OutgoingSendPolicy.retryBody(for: item) else {
            return nil
        }
        return enqueue(
            localID: item.id,
            roomID: roomID,
            body: body,
            formattedBody: TimelinePendingReconciler.formattedBody(for: item),
            replyToEventID: item.replyToEventID,
            senderID: senderID,
            timestamp: item.timestamp
        )
    }

    func transmitIfNeeded(id: String) async {
        guard let message = queue.item(id: id) else {
            return
        }
        await transmitIfNeeded(message)
    }

    func transmitIfNeeded(_ message: OutgoingQueuedMessage) async {
        guard OutgoingSendPolicy.isSendReady(connectionStatus.status) else {
            if message.deliveryStatus != .queued {
                queue.updateDeliveryStatus(id: message.id, .queued)
            }
            return
        }
        guard beginFlight(message.id) else {
            return
        }
        defer { endFlight(message.id) }

        queue.updateDeliveryStatus(id: message.id, .sending)
        do {
            _ = try await messageSender.send(message.sendRequest())
            queue.updateDeliveryStatus(id: message.id, .sent)
        } catch {
            queue.updateDeliveryStatus(
                id: message.id,
                OutgoingSendPolicy.deliveryStatusAfterFailure(
                    isSendReady: OutgoingSendPolicy.isSendReady(connectionStatus.status)
                )
            )
        }
    }

    func flushWhenSendReady() async {
        guard OutgoingSendPolicy.isSendReady(connectionStatus.status) else {
            return
        }
        for message in queue.flushableItems() {
            await transmitIfNeeded(message)
        }
    }

    func dropConfirmed(matching streamItems: [TimelineItem], currentUserID: String) {
        for pending in queue.items {
            let timeline = pending.asTimelineItem()
            let matched = streamItems.contains { serverItem in
                TimelinePendingReconciler.matchesPending(timeline, serverItem: serverItem)
                    && serverItem.senderID == currentUserID
            }
            if matched {
                queue.remove(id: pending.id)
            }
        }
    }

    private func beginFlight(_ id: String) -> Bool {
        inFlightLock.lock()
        defer { inFlightLock.unlock() }
        return inFlight.insert(id).inserted
    }

    private func endFlight(_ id: String) {
        inFlightLock.lock()
        inFlight.remove(id)
        inFlightLock.unlock()
    }
}
