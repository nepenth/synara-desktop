import Foundation

struct MessageSendRequest: Equatable {
    let roomID: String
    let body: String
    let formattedBody: String?
    let replyToEventID: String?
    let editEventID: String?
    let threadRootEventID: String?

    init(
        roomID: String,
        body: String,
        formattedBody: String? = nil,
        replyToEventID: String?,
        editEventID: String?,
        threadRootEventID: String? = nil
    ) {
        self.roomID = roomID
        self.body = body
        self.formattedBody = formattedBody
        self.replyToEventID = replyToEventID
        self.editEventID = editEventID
        self.threadRootEventID = threadRootEventID
    }
}

enum MessageSendError: LocalizedError, Equatable {
    case emptyMessage
    case failed

    var errorDescription: String? {
        switch self {
        case .emptyMessage:
            return "Enter a message before sending."
        case .failed:
            return "Message could not be sent. Try again."
        }
    }
}

protocol MessageSending {
    func send(_ request: MessageSendRequest) async throws -> TimelineItem
}

final class DraftStore {
    private var drafts: [String: String] = [:]

    func draft(roomID: String) -> String {
        drafts[roomID] ?? ""
    }

    func setDraft(_ draft: String, roomID: String) {
        drafts[roomID] = draft
    }

    func clearDraft(roomID: String) {
        drafts.removeValue(forKey: roomID)
    }

    func clearAll() {
        drafts.removeAll()
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
