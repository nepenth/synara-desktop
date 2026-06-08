import Foundation
@preconcurrency import MatrixRustSDK

enum EventActionType: Equatable {
    case reply
    case edit
    case redact
    case react(String)
}

struct EventActionAvailability: Equatable {
    let canReply: Bool
    let canEdit: Bool
    let canRedact: Bool
    let canReact: Bool
}

enum EventActionError: LocalizedError, Equatable {
    case signedOut
    case failed

    var errorDescription: String? {
        switch self {
        case .signedOut:
            return "Sign in before changing messages."
        case .failed:
            return "That action could not be completed. Try again."
        }
    }
}

protocol EventActionServicing {
    func availability(for item: TimelineItem, currentUserID: String) -> EventActionAvailability
    func apply(_ action: EventActionType, to item: TimelineItem, currentUserID: String, roomID: String) async throws -> TimelineItem
}

struct MockEventActionService: EventActionServicing {
    func availability(for item: TimelineItem, currentUserID: String) -> EventActionAvailability {
        switch item.kind {
        case .mediaPlaceholder(let resource) where resource.isEncrypted:
            return EventActionAvailability(canReply: false, canEdit: false, canRedact: false, canReact: false)
        case .redacted, .encryptedPlaceholder:
            return EventActionAvailability(canReply: false, canEdit: false, canRedact: false, canReact: false)
        default:
            return EventActionAvailability(
                canReply: true,
                canEdit: item.senderID == currentUserID,
                canRedact: item.senderID == currentUserID,
                canReact: true
            )
        }
    }

    func apply(_ action: EventActionType, to item: TimelineItem, currentUserID: String, roomID: String) async throws -> TimelineItem {
        switch action {
        case .reply:
            return item
        case .edit:
            return TimelineItem(
                id: item.id,
                eventID: item.eventID,
                senderID: item.senderID,
                timestamp: item.timestamp,
                kind: item.kind,
                replyToEventID: item.replyToEventID,
                isEdited: true,
                reactions: item.reactions,
                isEncrypted: item.isEncrypted
            )
        case .redact:
            return TimelineItem(
                id: item.id,
                eventID: item.eventID,
                senderID: item.senderID,
                timestamp: item.timestamp,
                kind: .redacted,
                replyToEventID: item.replyToEventID,
                isEdited: item.isEdited,
                reactions: [:],
                isEncrypted: item.isEncrypted
            )
        case .react(let reaction):
            var reactions = item.reactions
            reactions[reaction, default: 0] += 1
            return TimelineItem(
                id: item.id,
                eventID: item.eventID,
                senderID: item.senderID,
                timestamp: item.timestamp,
                kind: item.kind,
                replyToEventID: item.replyToEventID,
                isEdited: item.isEdited,
                reactions: reactions,
                isEncrypted: item.isEncrypted
            )
        }
    }
}

final class MatrixRustSDKEventActionService: EventActionServicing {
    private let sessionStore: AppSessionStore
    private let clientStore: MatrixRustSDKClientStore

    init(sessionStore: AppSessionStore, clientStore: MatrixRustSDKClientStore) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
    }

    func availability(for item: TimelineItem, currentUserID: String) -> EventActionAvailability {
        MockEventActionService().availability(for: item, currentUserID: currentUserID)
    }

    func apply(_ action: EventActionType, to item: TimelineItem, currentUserID: String, roomID: String) async throws -> TimelineItem {
        guard case .signedIn(let session) = sessionStore.currentState else {
            throw EventActionError.signedOut
        }

        guard let room = try await clientStore.room(roomID: roomID, session: session) else {
            throw EventActionError.failed
        }
        let timeline = try await room.timeline()
        let eventID = EventOrTransactionId.eventId(eventId: item.eventID)

        switch action {
        case .reply, .edit:
            return item
        case .redact:
            try await timeline.redactEvent(eventOrTransactionId: eventID, reason: nil)
        case .react(let reaction):
            _ = try await timeline.toggleReaction(itemId: eventID, key: reaction)
        }

        // The SDK timeline listener owns local echo and remote echo updates.
        return item
    }
}
