import Foundation
#if canImport(UIKit)
import UIKit
#endif

struct MessageSendRequest: Equatable {
    let roomID: String
    let body: String
    let formattedBody: String?
    let replyToEventID: String?
    let editEventID: String?
    let threadRootEventID: String?
    /// Caller transaction id of a text send. Core answers a repeat with the
    /// same id with the request it already enqueued, never a second event.
    let transactionID: String?

    init(
        roomID: String,
        body: String,
        formattedBody: String? = nil,
        replyToEventID: String?,
        editEventID: String?,
        threadRootEventID: String? = nil,
        transactionID: String? = nil
    ) {
        self.roomID = roomID
        self.body = body
        self.formattedBody = formattedBody
        self.replyToEventID = replyToEventID
        self.editEventID = editEventID
        self.threadRootEventID = threadRootEventID
        self.transactionID = transactionID
    }
}

enum MessageSendError: LocalizedError, Equatable {
    case emptyMessage
    case failed
    /// Core has no session or does not know the room yet (launch, restore).
    /// The message stays queued and is handed over when the session is ready.
    case notReady

    var errorDescription: String? {
        switch self {
        case .emptyMessage:
            return "Enter a message before sending."
        case .failed:
            return "Message could not be sent. Try again."
        case .notReady:
            return "Message will be sent when Synara reconnects."
        }
    }
}

/// Core's answer to a text send: the SDK transaction id the send queue holds
/// the message under (its timeline row carries the same id), and the server
/// event id once known.
struct MessageSendReceipt: Equatable {
    let transactionID: String?
    let eventID: String?
    /// True while the SDK send queue still holds the message.
    let isQueued: Bool
}

protocol MessageSending {
    func send(_ request: MessageSendRequest) async throws -> TimelineItem
    /// Hands one text message to Core's persistent send queue and returns
    /// as soon as it is queued.
    func sendText(_ request: MessageSendRequest) async throws -> MessageSendReceipt
    /// Retries an unsent SDK local echo under its transaction id.
    func retryLocalEcho(roomID: String, transactionID: String) async throws
    /// Discards an unsent SDK local echo. False when it was already sent.
    func discardLocalEcho(roomID: String, transactionID: String) async throws -> Bool
}

extension MessageSending {
    func sendText(_ request: MessageSendRequest) async throws -> MessageSendReceipt {
        let item = try await send(request)
        return MessageSendReceipt(
            transactionID: request.transactionID,
            eventID: item.serverEventID ?? item.eventID,
            isQueued: false
        )
    }

    func retryLocalEcho(roomID _: String, transactionID _: String) async throws {
        throw MessageSendError.failed
    }

    func discardLocalEcho(roomID _: String, transactionID _: String) async throws -> Bool {
        throw MessageSendError.failed
    }
}

/// Unsent composer state that must survive the app being terminated: draft
/// text per room, and messages the user sent that Core has not accepted yet.
/// One small JSON file in the app's Application Support, written atomically
/// and protected until first unlock; bound to one account and removed on
/// sign-out. A store without a file (tests, previews) is memory-only.
final class ComposerLocalStore: @unchecked Sendable {
    struct OutboxMessage: Codable, Equatable {
        let id: String
        let clientTransactionID: String
        let roomID: String
        let body: String
        let formattedBody: String?
        let replyToEventID: String?
        let threadRootEventID: String?
        let senderID: String
        let timestamp: Date
        var failed: Bool
    }

    struct Snapshot: Codable, Equatable {
        var version = 1
        var accountID: String?
        var drafts: [String: String] = [:]
        var outbox: [OutboxMessage] = []
    }

    private let fileURL: URL?
    private let lock = NSLock()
    private var snapshot: Snapshot
    private var draftWriteScheduled = false
    private let writeQueue = DispatchQueue(label: "com.whylandcreative.synara.composer-store")
    private var backgroundObserver: NSObjectProtocol?
    /// Coalesces keystroke writes; the outbox is always written at once.
    private let draftWriteDelay: TimeInterval

    init(fileURL: URL?, draftWriteDelay: TimeInterval = 0.4) {
        self.fileURL = fileURL
        self.draftWriteDelay = draftWriteDelay
        snapshot = fileURL.flatMap(Self.read(from:)) ?? Snapshot()
        #if canImport(UIKit)
        if fileURL != nil {
            backgroundObserver = NotificationCenter.default.addObserver(
                forName: UIApplication.didEnterBackgroundNotification,
                object: nil,
                queue: nil
            ) { [weak self] _ in
                self?.flush()
            }
        }
        #endif
    }

    deinit {
        if let backgroundObserver {
            NotificationCenter.default.removeObserver(backgroundObserver)
        }
    }

    static func live(fileManager: FileManager = .default) -> ComposerLocalStore {
        let directory = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("SynaraComposer", isDirectory: true)
        try? fileManager.createDirectory(at: directory, withIntermediateDirectories: true)
        return ComposerLocalStore(fileURL: directory.appendingPathComponent("composer-v1.json"))
    }

    var current: Snapshot {
        lock.lock()
        defer { lock.unlock() }
        return snapshot
    }

    /// Mutates the snapshot. `durable` writes before returning (an outbox
    /// change, so a message is on disk before its draft is cleared); draft
    /// keystrokes are written shortly after.
    func update(durable: Bool, _ body: (inout Snapshot) -> Void) {
        lock.lock()
        body(&snapshot)
        let toWrite = snapshot
        lock.unlock()
        if durable {
            write(toWrite)
        } else {
            scheduleDraftWrite()
        }
    }

    /// Drops everything another account left behind. Content written before
    /// any account was bound (this launch) belongs to this one.
    func bindAccount(_ accountID: String) {
        guard current.accountID != accountID else {
            return
        }
        update(durable: true) { snapshot in
            if snapshot.accountID == nil {
                snapshot.accountID = accountID
            } else {
                snapshot = Snapshot(accountID: accountID)
            }
        }
    }

    /// Sign-out: forget drafts and unsent messages, and delete the file.
    func removeAll() {
        lock.lock()
        snapshot = Snapshot()
        lock.unlock()
        guard let fileURL else {
            return
        }
        writeQueue.sync {
            try? FileManager.default.removeItem(at: fileURL)
        }
    }

    func flush() {
        write(current)
    }

    private func scheduleDraftWrite() {
        guard fileURL != nil else {
            return
        }
        lock.lock()
        let alreadyScheduled = draftWriteScheduled
        draftWriteScheduled = true
        lock.unlock()
        guard alreadyScheduled == false else {
            return
        }
        writeQueue.asyncAfter(deadline: .now() + draftWriteDelay) { [weak self] in
            guard let self else { return }
            self.lock.lock()
            self.draftWriteScheduled = false
            let toWrite = self.snapshot
            self.lock.unlock()
            self.writeNow(toWrite)
        }
    }

    private func write(_ snapshot: Snapshot) {
        guard fileURL != nil else {
            return
        }
        writeQueue.sync {
            writeNow(snapshot)
        }
    }

    /// Runs on `writeQueue`.
    private func writeNow(_ snapshot: Snapshot) {
        guard let fileURL else {
            return
        }
        guard snapshot.drafts.isEmpty == false || snapshot.outbox.isEmpty == false
            || snapshot.accountID != nil
        else {
            try? FileManager.default.removeItem(at: fileURL)
            return
        }
        guard let data = try? JSONEncoder().encode(snapshot) else {
            return
        }
        #if os(iOS)
        try? data.write(to: fileURL, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
        #else
        try? data.write(to: fileURL, options: [.atomic])
        #endif
    }

    private static func read(from url: URL) -> Snapshot? {
        guard let data = try? Data(contentsOf: url) else {
            return nil
        }
        return try? JSONDecoder().decode(Snapshot.self, from: data)
    }
}

/// Composer draft text per room, kept across launches in the
/// `ComposerLocalStore` and cleared on sign-out.
final class DraftStore {
    let store: ComposerLocalStore

    init(store: ComposerLocalStore = ComposerLocalStore(fileURL: nil)) {
        self.store = store
    }

    func draft(roomID: String) -> String {
        store.current.drafts[roomID] ?? ""
    }

    func setDraft(_ draft: String, roomID: String) {
        guard store.current.drafts[roomID, default: ""] != draft else {
            return
        }
        store.update(durable: false) { snapshot in
            if draft.isEmpty {
                snapshot.drafts.removeValue(forKey: roomID)
            } else {
                snapshot.drafts[roomID] = draft
            }
        }
    }

    func clearDraft(roomID: String) {
        guard store.current.drafts[roomID] != nil else {
            return
        }
        store.update(durable: true) { snapshot in
            snapshot.drafts.removeValue(forKey: roomID)
        }
    }

    func clearAll() {
        store.removeAll()
    }
}

struct MockMessageSendService: MessageSending {
    func send(_ request: MessageSendRequest) async throws -> TimelineItem {
        let body = request.body.trimmingCharacters(in: .whitespacesAndNewlines)
        guard body.isEmpty == false else {
            throw MessageSendError.emptyMessage
        }

        let eventID = request.editEventID ?? "$local-\(UUID().uuidString)"
        return TimelineItem(
            id: eventID,
            eventID: eventID,
            senderID: "@local:matrix.org",
            timestamp: Date(),
            kind: request.formattedBody.map { .formattedText(body: body, html: $0) } ?? .text(body),
            replyToEventID: request.replyToEventID,
            threadRootEventID: request.threadRootEventID,
            isEdited: request.editEventID != nil,
            reactions: [:]
        )
    }
}

struct ComposerEditSession: Equatable {
    let draft: String
    let previousDraft: String
    let editTarget: ComposerRelationTarget
    let retryingItem: TimelineItem?

    var remoteEditEventID: String? {
        retryingItem == nil ? editTarget.eventID : nil
    }
}

/// A send that has finished, handed to the composer so it can drop the text
/// it sent. The keyboard stays up after a send, so the user may have typed
/// more while an upload or edit save was in flight; that text survives.
struct ComposerSentDraft: Equatable {
    /// Distinguishes two sends of the same text.
    let id: UInt64
    /// The composer text when the send began.
    let sentText: String

    /// The composer text to keep once the send finishes: nothing if it
    /// still holds the sent text, what was typed after the sent text if the
    /// user only added to it, else everything (they replaced it).
    static func remainingText(current: String, sent: String) -> String {
        if current == sent {
            return ""
        }
        if current.hasPrefix(sent) {
            let added = current.dropFirst(sent.count)
            return String(added.drop(while: { $0.isWhitespace }))
        }
        return current
    }
}

struct ComposerSendIntent: Equatable {
    let body: String
    let replyToEventID: String?
    let threadRootEventID: String?
    let editEventID: String?
    let retrying: TimelineItem?

    init(
        body: String,
        replyToEventID: String?,
        threadRootEventID: String? = nil,
        editEventID: String?,
        retrying: TimelineItem?
    ) {
        self.body = body
        self.replyToEventID = replyToEventID
        self.threadRootEventID = threadRootEventID
        self.editEventID = editEventID
        self.retrying = retrying
    }

    /// An edit or failed-local retry must remain a distinct text operation.
    /// Attachment captions cannot represent either Matrix mutation.
    var requiresStandaloneTextSend: Bool {
        editEventID != nil || retrying != nil
    }

    func replacingBody(with body: String) -> ComposerSendIntent {
        ComposerSendIntent(
            body: body,
            replyToEventID: replyToEventID,
            threadRootEventID: threadRootEventID,
            editEventID: editEventID,
            retrying: retrying
        )
    }
}

enum ComposerEditFlow {
    static func begin(
        item: TimelineItem,
        currentUserID: String,
        currentDraft: String
    ) -> ComposerEditSession {
        ComposerEditSession(
            draft: TimelinePendingReconciler.messageBody(for: item) ?? currentDraft,
            previousDraft: currentDraft,
            editTarget: ComposerRelationTarget(item: item, kind: .edit, currentUserID: currentUserID),
            retryingItem: item.isLocalPending ? item : nil
        )
    }

    static func cancel(_ session: ComposerEditSession) -> String {
        session.previousDraft
    }

    static func sendIntent(
        body: String,
        replyToEventID: String?,
        threadRootEventID: String? = nil,
        session: ComposerEditSession?
    ) -> ComposerSendIntent {
        if let retrying = session?.retryingItem {
            return ComposerSendIntent(
                body: body,
                replyToEventID: retrying.replyToEventID,
                threadRootEventID: retrying.threadRootEventID,
                editEventID: nil,
                retrying: retrying
            )
        }

        return ComposerSendIntent(
            body: body,
            replyToEventID: replyToEventID,
            threadRootEventID: session?.editTarget.threadRootEventID ?? threadRootEventID,
            editEventID: session?.remoteEditEventID,
            retrying: nil
        )
    }
}
