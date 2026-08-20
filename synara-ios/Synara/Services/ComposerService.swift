import Foundation

struct MessageSendRequest: Equatable {
    let roomID: String
    let body: String
    let formattedBody: String?
    let replyToEventID: String?
    let editEventID: String?

    init(
        roomID: String,
        body: String,
        formattedBody: String? = nil,
        replyToEventID: String?,
        editEventID: String?
    ) {
        self.roomID = roomID
        self.body = body
        self.formattedBody = formattedBody
        self.replyToEventID = replyToEventID
        self.editEventID = editEventID
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

struct StickerSendRequest: Equatable {
    let roomID: String
    let body: String
    let mxc: String
    let width: UInt64?
    let height: UInt64?
    let mimetype: String?
    let size: UInt64?
    let replyToEventID: String?
    let threadRoot: String?
}

protocol MessageSending {
    func send(_ request: MessageSendRequest) async throws -> TimelineItem
    func sendSticker(_ request: StickerSendRequest) async throws -> TimelineItem
}

extension MessageSending {
    func sendSticker(_ request: StickerSendRequest) async throws -> TimelineItem {
        _ = request
        throw MessageSendError.failed
    }
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
    let editEventID: String?
    let retrying: TimelineItem?
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
        session: ComposerEditSession?
    ) -> ComposerSendIntent {
        if let retrying = session?.retryingItem {
            return ComposerSendIntent(
                body: body,
                replyToEventID: retrying.replyToEventID,
                editEventID: nil,
                retrying: retrying
            )
        }

        return ComposerSendIntent(
            body: body,
            replyToEventID: replyToEventID,
            editEventID: session?.remoteEditEventID,
            retrying: nil
        )
    }
}
