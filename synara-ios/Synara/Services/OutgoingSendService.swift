import Combine
import Foundation

/// Product policy for iOS outbound text reliability.
///
/// Every send is handed to Core at once, whatever the sync state: Core puts
/// it in the SDK's persisted send queue, which keeps it across relaunches and
/// retries it. Until Core has accepted a message it lives in the durable
/// outbox of `ComposerLocalStore`; only a session that is not ready yet keeps
/// it here (`queued`), and it is handed over when the session is.
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

    /// Retry is text-only. Non-text kinds return nil so the chip can no-op
    /// without looking like it sent.
    static func retryBody(for item: TimelineItem) -> String? {
        guard item.deliveryStatus == .failed else {
            return nil
        }
        return TimelinePendingReconciler.messageBody(for: item)
    }

    static func canRetry(_ item: TimelineItem) -> Bool {
        if item.isLocalPending {
            return retryBody(for: item) != nil
        }
        return item.transactionID != nil && item.localEcho?.canRetry == true
    }

    /// Discard applies to unsent own messages: a pending row Core never
    /// accepted, or an SDK local echo that is still sending or failed.
    static func canDiscard(_ item: TimelineItem) -> Bool {
        if item.isLocalPending {
            return item.deliveryStatus == .failed || item.deliveryStatus == .queued
        }
        return item.transactionID != nil && item.localEcho != nil
    }

    /// Messages still waiting for Core: not yet accepted and not failed.
    static func needsHandoff(_ item: OutgoingQueuedMessage) -> Bool {
        guard item.transactionID == nil else {
            return false
        }
        switch item.deliveryStatus {
        case .queued, .sending:
            return item.body.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
        case .sent, .failed:
            return false
        }
    }
}

/// VoiceOver for failed local echoes: keep the Retry chip as the touch
/// target, but expose it as a child and as a named row action.
enum TimelineRowAccessibility {
    static func containsChildren(
        deliveryStatus: TimelineDeliveryStatus?,
        kind: TimelineItem.Kind,
        replyCount: Int,
        hasApprovalPrompt: Bool,
        hasInteractivePoll: Bool = false
    ) -> Bool {
        if deliveryStatus == .failed {
            return true
        }
        if hasApprovalPrompt {
            return true
        }
        if hasInteractivePoll {
            return true
        }
        if case .agentCard = kind {
            return true
        }
        return replyCount > 0
    }

    static func retryActionTitle(deliveryStatus: TimelineDeliveryStatus?) -> String? {
        deliveryStatus == .failed ? String(localized: "Retry") : nil
    }
}

/// Applies session-scoped pending rows onto the current timeline presentation.
/// Send while `.loading` / `.failed` still surfaces the local echo.
enum OutgoingQueueTimelineMerge {
    enum Presentation: Equatable {
        case idle
        case loading
        case empty
        case failed
        case loaded([TimelineItem], isPaginating: Bool)
    }

    static func applying(
        pendingItems: [TimelineItem],
        to presentation: Presentation
    ) -> Presentation {
        switch presentation {
        case let .loaded(items, isPaginating):
            var updated = items
            var ids = Set(items.map(\.id))
            for pending in pendingItems {
                if let index = updated.firstIndex(where: { $0.id == pending.id }) {
                    var refreshed = updated[index].withDeliveryStatus(pending.deliveryStatus)
                    refreshed.transactionID = pending.transactionID
                    updated[index] = refreshed
                } else if ids.insert(pending.id).inserted {
                    updated.append(pending)
                }
            }
            return .loaded(updated, isPaginating: isPaginating)
        case .empty, .idle, .loading, .failed:
            if pendingItems.isEmpty == false {
                return .loaded(pendingItems, isPaginating: false)
            }
            return presentation
        }
    }
}

struct OutgoingQueuedMessage: Equatable, Identifiable {
    let id: String
    /// Stable caller transaction id for Core: a repeated hand-off (retry,
    /// relaunch, a second flush) resolves to the same SDK request.
    let clientTransactionID: String
    let roomID: String
    let body: String
    let formattedBody: String?
    let replyToEventID: String?
    let threadRootEventID: String?
    let senderID: String
    var senderAvatarURL: URL?
    let timestamp: Date
    var deliveryStatus: TimelineDeliveryStatus
    /// The SDK transaction id Core enqueued this message under. Its SDK local
    /// echo and its remote echo carry the same id; the row is matched by it.
    var transactionID: String?
    /// Server event id when Core already knew it.
    var serverEventID: String?

    init(
        id: String,
        clientTransactionID: String? = nil,
        roomID: String,
        body: String,
        formattedBody: String?,
        replyToEventID: String?,
        threadRootEventID: String?,
        senderID: String,
        senderAvatarURL: URL? = nil,
        timestamp: Date,
        deliveryStatus: TimelineDeliveryStatus,
        transactionID: String? = nil,
        serverEventID: String? = nil
    ) {
        self.id = id
        self.clientTransactionID = clientTransactionID ?? Self.newClientTransactionID()
        self.roomID = roomID
        self.body = body
        self.formattedBody = formattedBody
        self.replyToEventID = replyToEventID
        self.threadRootEventID = threadRootEventID
        self.senderID = senderID
        self.senderAvatarURL = senderAvatarURL
        self.timestamp = timestamp
        self.deliveryStatus = deliveryStatus
        self.transactionID = transactionID
        self.serverEventID = serverEventID
    }

    static func newClientTransactionID() -> String {
        "synara-text-\(UUID().uuidString.lowercased())"
    }

    func asTimelineItem() -> TimelineItem {
        var item = TimelineItem(
            id: id,
            eventID: id,
            serverEventID: serverEventID,
            senderID: senderID,
            senderAvatarURL: senderAvatarURL,
            timestamp: timestamp,
            kind: formattedBody.map { .formattedText(body: body, html: $0) } ?? .text(body),
            replyToEventID: replyToEventID,
            threadRootEventID: threadRootEventID,
            isEdited: false,
            reactions: [:],
            deliveryStatus: deliveryStatus
        )
        item.transactionID = transactionID
        return item
    }

    func sendRequest() -> MessageSendRequest {
        MessageSendRequest(
            roomID: roomID,
            body: body,
            formattedBody: formattedBody,
            replyToEventID: replyToEventID,
            editEventID: nil,
            threadRootEventID: threadRootEventID,
            transactionID: clientTransactionID
        )
    }

    fileprivate var outboxMessage: ComposerLocalStore.OutboxMessage {
        ComposerLocalStore.OutboxMessage(
            id: id,
            clientTransactionID: clientTransactionID,
            roomID: roomID,
            body: body,
            formattedBody: formattedBody,
            replyToEventID: replyToEventID,
            threadRootEventID: threadRootEventID,
            senderID: senderID,
            timestamp: timestamp,
            failed: deliveryStatus == .failed
        )
    }

    /// A message restored after relaunch waits for its hand-off again under
    /// the same caller transaction id.
    fileprivate init(restoring message: ComposerLocalStore.OutboxMessage) {
        self.init(
            id: message.id,
            clientTransactionID: message.clientTransactionID,
            roomID: message.roomID,
            body: message.body,
            formattedBody: message.formattedBody,
            replyToEventID: message.replyToEventID,
            threadRootEventID: message.threadRootEventID,
            senderID: message.senderID,
            timestamp: message.timestamp,
            deliveryStatus: message.failed ? .failed : .queued
        )
    }
}

/// Pending own text messages. Messages Core has not accepted are mirrored to
/// the durable outbox, so a message sent while the app is terminated is
/// handed over on the next launch; accepted ones belong to the SDK and stay
/// here only until their timeline row appears. Cleared on wipe.
///
/// The send coordinator reads and changes it from the cooperative thread
/// pool and SwiftUI from the main thread, so the authoritative list lives
/// behind `lock` and is read through `currentItems`. `items` is the copy
/// SwiftUI observes: set on the main thread, one main-queue hop after a
/// change made elsewhere.
final class OutgoingMessageQueue: ObservableObject, @unchecked Sendable {
    @Published private(set) var items: [OutgoingQueuedMessage] = []
    private let store: ComposerLocalStore
    private let lock = NSLock()
    private var storage: [OutgoingQueuedMessage]

    init(store: ComposerLocalStore = ComposerLocalStore(fileURL: nil)) {
        self.store = store
        storage = store.current.outbox.map(OutgoingQueuedMessage.init(restoring:))
        items = storage
    }

    /// The current list, from any thread.
    var currentItems: [OutgoingQueuedMessage] {
        lock.withLock { storage }
    }

    func item(id: String) -> OutgoingQueuedMessage? {
        currentItems.first(where: { $0.id == id })
    }

    func items(in roomID: String) -> [OutgoingQueuedMessage] {
        currentItems.filter { $0.roomID == roomID }
    }

    func timelineItems(in roomID: String) -> [TimelineItem] {
        items(in: roomID).map { $0.asTimelineItem() }
    }

    /// Pending rows of one timeline: the room's main timeline (`nil`) or one
    /// thread. The room timeline never shows thread replies, so a thread's
    /// sends are shown, and confirmed, by its thread screen.
    func timelineItems(in roomID: String, threadRootEventID: String?) -> [TimelineItem] {
        items(in: roomID)
            .filter { $0.threadRootEventID == threadRootEventID }
            .map { $0.asTimelineItem() }
    }

    func handoffItems() -> [OutgoingQueuedMessage] {
        currentItems.filter(OutgoingSendPolicy.needsHandoff)
    }

    /// Adds or replaces a message and writes the outbox before returning.
    /// `clearingDraftOf` clears that room's draft in the same write, so the
    /// text is never only in memory.
    func upsert(_ message: OutgoingQueuedMessage, clearingDraftOf draftRoomID: String? = nil) {
        mutate(clearingDraftOf: draftRoomID) { items in
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

    /// Core accepted the message: the SDK's persisted queue owns it now.
    func markHandedOff(id: String, receipt: MessageSendReceipt) {
        mutate { items in
            guard let index = items.firstIndex(where: { $0.id == id }) else {
                return
            }
            items[index].transactionID = receipt.transactionID
            if let eventID = receipt.eventID, eventID.isEmpty == false {
                items[index].serverEventID = eventID
            }
            items[index].deliveryStatus = receipt.isQueued ? .sending : .sent
        }
    }

    func hydrateSenderAvatarURL(senderID: String, avatarURL: URL) {
        let changed = currentItems.contains { $0.senderID == senderID && $0.senderAvatarURL != avatarURL }
        guard changed else {
            return
        }
        mutate { items in
            for index in items.indices where items[index].senderID == senderID {
                items[index].senderAvatarURL = avatarURL
            }
        }
    }

    func remove(id: String) {
        mutate { items in
            items.removeAll { $0.id == id }
        }
    }

    func remove(where predicate: (OutgoingQueuedMessage) -> Bool) {
        guard currentItems.contains(where: predicate) else {
            return
        }
        mutate { items in
            items.removeAll(where: predicate)
        }
    }

    func clear() {
        mutate { items in
            items.removeAll()
        }
    }

    /// Binds the durable store to the signed-in account. Unsent messages a
    /// different account left on disk are dropped, never sent from this one.
    func bindAccount(_ accountID: String) {
        let previous = store.current.accountID
        store.bindAccount(accountID)
        if let previous, previous != accountID {
            mutate { items in
                items.removeAll { $0.transactionID == nil }
            }
        }
    }

    /// Changes the list and writes the outbox under `lock`, from any thread,
    /// so messages Core has not accepted are durable before this returns. It
    /// never blocks on the main thread (it used to `DispatchQueue.main.sync`
    /// from the cooperative pool).
    private func mutate(
        clearingDraftOf draftRoomID: String? = nil,
        _ body: (inout [OutgoingQueuedMessage]) -> Void
    ) {
        lock.withLock {
            body(&storage)
            let outbox = storage.filter { $0.transactionID == nil }.map(\.outboxMessage)
            guard outbox != store.current.outbox || draftRoomID != nil else {
                return
            }
            store.update(durable: true) { snapshot in
                snapshot.outbox = outbox
                if let draftRoomID {
                    snapshot.drafts.removeValue(forKey: draftRoomID)
                }
            }
        }
        publish()
    }

    /// Copies the latest list into `items` on the main thread. Reading it at
    /// delivery time means a late hop can never publish an older list.
    private func publish() {
        if Thread.isMainThread {
            items = currentItems
        } else {
            DispatchQueue.main.async { [weak self] in
                guard let self else {
                    return
                }
                self.items = self.currentItems
            }
        }
    }
}

/// The one owner of text hand-off to Core. Enqueue persists the message and
/// requests a flush; the flusher hands messages over in order, one at a
/// time. A second request while a flush runs only asks it to run again, so
/// a reconnect, a launch and a new message never run two flushers.
final class OutgoingSendCoordinator {
    let queue: OutgoingMessageQueue
    private let messageSender: MessageSending
    private let connectionStatus: ConnectionStatusStore
    private let flushLock = NSLock()
    private var isFlushing = false
    private var flushRequested = false

    init(
        messageSender: MessageSending,
        connectionStatus: ConnectionStatusStore,
        queue: OutgoingMessageQueue = OutgoingMessageQueue()
    ) {
        self.messageSender = messageSender
        self.connectionStatus = connectionStatus
        self.queue = queue
    }

    /// Records the message durably (clearing `roomID`'s draft in the same
    /// write when asked). Call `flush()` afterwards to hand it to Core.
    @discardableResult
    func enqueue(
        localID: String,
        clientTransactionID: String? = nil,
        roomID: String,
        body: String,
        formattedBody: String?,
        replyToEventID: String?,
        threadRootEventID: String? = nil,
        senderID: String,
        senderAvatarURL: URL? = nil,
        timestamp: Date,
        clearingDraft: Bool = false
    ) -> OutgoingQueuedMessage {
        let message = OutgoingQueuedMessage(
            id: localID,
            clientTransactionID: clientTransactionID,
            roomID: roomID,
            body: body,
            formattedBody: formattedBody,
            replyToEventID: replyToEventID,
            threadRootEventID: threadRootEventID,
            senderID: senderID,
            senderAvatarURL: senderAvatarURL,
            timestamp: timestamp,
            deliveryStatus: .sending
        )
        queue.upsert(message, clearingDraftOf: clearingDraft ? roomID : nil)
        return message
    }

    /// Re-queues a pending row Core rejected, under the same caller
    /// transaction id. Returns nil for non-text (or non-failed) rows so retry
    /// is a true no-op. SDK local echo rows retry through `retryLocalEcho`.
    func retry(_ item: TimelineItem, roomID: String, senderID: String) -> OutgoingQueuedMessage? {
        guard let body = OutgoingSendPolicy.retryBody(for: item) else {
            return nil
        }
        return enqueue(
            localID: item.id,
            clientTransactionID: queue.item(id: item.id)?.clientTransactionID,
            roomID: roomID,
            body: body,
            formattedBody: TimelinePendingReconciler.formattedBody(for: item),
            replyToEventID: item.replyToEventID,
            threadRootEventID: item.threadRootEventID,
            senderID: senderID,
            senderAvatarURL: item.senderAvatarURL,
            timestamp: item.timestamp
        )
    }

    func bindAccount(_ accountID: String) {
        queue.bindAccount(accountID)
    }

    /// Retries an SDK local echo (wedged or parked) without a new event.
    func retryLocalEcho(_ item: TimelineItem, roomID: String) async -> Bool {
        guard let transactionID = item.transactionID, item.localEcho != nil else {
            return false
        }
        do {
            try await messageSender.retryLocalEcho(roomID: roomID, transactionID: transactionID)
            return true
        } catch {
            return false
        }
    }

    /// Discards an unsent message: a pending row Core never accepted is
    /// dropped here; an SDK local echo is aborted in the SDK queue. True when
    /// discarded, false when the SDK had already sent it, nil on failure.
    func discard(_ item: TimelineItem, roomID: String) async -> Bool? {
        if item.isLocalPending, queue.item(id: item.id)?.transactionID == nil {
            queue.remove(id: item.id)
            return true
        }
        guard let transactionID = item.transactionID else {
            return nil
        }
        do {
            let aborted = try await messageSender.discardLocalEcho(
                roomID: roomID,
                transactionID: transactionID
            )
            if aborted {
                queue.remove { $0.transactionID == transactionID }
            }
            return aborted
        } catch {
            return nil
        }
    }

    /// Reconciles optimistic rows created before the account profile finished
    /// loading. The queue remains the single owner, so leaving or switching a
    /// room cannot race an unstructured view task into a stale local echo.
    func hydrateSenderAvatarURL(senderID: String, avatarURL: URL) {
        queue.hydrateSenderAvatarURL(senderID: senderID, avatarURL: avatarURL)
    }

    /// Hands every waiting message to Core, in order. Messages in a room
    /// whose earlier message is still waiting stay behind it.
    func flush() async {
        guard beginFlush() else {
            return
        }
        repeat {
            var blockedRooms = Set<String>()
            for message in queue.handoffItems() where blockedRooms.contains(message.roomID) == false {
                if await handOff(message) == false {
                    blockedRooms.insert(message.roomID)
                }
            }
        } while continueFlush()
    }

    /// Hand-off of one message. False when the room must wait (Core not
    /// ready); a rejection marks the row failed with Retry and Discard.
    private func handOff(_ message: OutgoingQueuedMessage) async -> Bool {
        if message.deliveryStatus != .sending {
            queue.updateDeliveryStatus(id: message.id, .sending)
        }
        do {
            let receipt = try await messageSender.sendText(message.sendRequest())
            queue.markHandedOff(id: message.id, receipt: receipt)
            return true
        } catch MessageSendError.notReady {
            queue.updateDeliveryStatus(id: message.id, .queued)
            return false
        } catch {
            queue.updateDeliveryStatus(id: message.id, .failed)
            return true
        }
    }

    /// Forgets accepted messages whose timeline row arrived: the SDK local
    /// echo or the remote echo with the same transaction id, or the server
    /// event id Core reported. Never by content.
    func dropConfirmed(matching streamItems: [TimelineItem]) {
        let transactions = Set(streamItems.filter { $0.isLocalPending == false }.compactMap(\.transactionID))
        let events = Set(streamItems.filter { $0.isLocalPending == false }.compactMap(\.serverEventID))
        queue.remove { pending in
            if let transactionID = pending.transactionID, transactions.contains(transactionID) {
                return true
            }
            if let serverEventID = pending.serverEventID, events.contains(serverEventID) {
                return true
            }
            return false
        }
    }

    private func beginFlush() -> Bool {
        flushLock.lock()
        defer { flushLock.unlock() }
        if isFlushing {
            flushRequested = true
            return false
        }
        isFlushing = true
        flushRequested = false
        return true
    }

    private func continueFlush() -> Bool {
        flushLock.lock()
        defer { flushLock.unlock() }
        if flushRequested {
            flushRequested = false
            return true
        }
        isFlushing = false
        return false
    }
}
