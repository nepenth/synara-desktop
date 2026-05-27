import Foundation

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

protocol EventActionServicing {
    func availability(for item: TimelineItem, currentUserID: String) -> EventActionAvailability
    func apply(_ action: EventActionType, to item: TimelineItem, currentUserID: String) async -> TimelineItem
}

struct MockEventActionService: EventActionServicing {
    func availability(for item: TimelineItem, currentUserID: String) -> EventActionAvailability {
        switch item.kind {
        case .redacted:
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

    func apply(_ action: EventActionType, to item: TimelineItem, currentUserID: String) async -> TimelineItem {
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
                reactions: item.reactions
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
                reactions: [:]
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
                reactions: reactions
            )
        }
    }
}
