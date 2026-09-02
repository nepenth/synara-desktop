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
        if item.isLocalPending {
            return EventActionAvailability(
                canReply: false,
                canEdit: Self.canEditLocalPending(item, currentUserID: currentUserID),
                canRedact: false,
                canReact: false
            )
        }

        if let capabilities = item.actionCapabilities {
            return EventActionAvailability(
                canReply: capabilities.canReply,
                canEdit: capabilities.canEdit,
                canRedact: capabilities.canRedact,
                canReact: capabilities.canReact
            )
        }

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

    private static func canEditLocalPending(_ item: TimelineItem, currentUserID: String) -> Bool {
        guard item.deliveryStatus == .failed else {
            return false
        }
        guard item.senderID == currentUserID else {
            return false
        }
        return TimelinePendingReconciler.messageBody(for: item) != nil
    }

    func apply(_ action: EventActionType, to item: TimelineItem, currentUserID: String, roomID: String) async throws -> TimelineItem {
        switch action {
        case .reply:
            return item
        case .edit:
            return TimelineItem(
                id: item.id,
                eventID: item.eventID,
                serverEventID: item.serverEventID,
                senderID: item.senderID,
                senderProfileDisplayName: item.senderProfileDisplayName,
                senderAvatarURL: item.senderAvatarURL,
                timestamp: item.timestamp,
                kind: item.kind,
                replyToEventID: item.replyToEventID,
                threadRootEventID: item.threadRootEventID,
                replyPreview: item.replyPreview,
                threadSummary: item.threadSummary,
                poll: item.poll,
                actionCapabilities: item.actionCapabilities,
                isEdited: true,
                isAgentApproval: item.isAgentApproval,
                reactions: item.reactions,
                reactionOwnership: item.reactionOwnership,
                isEncrypted: item.isEncrypted,
                deliveryStatus: item.deliveryStatus,
                hasCurrentUserReadReceipt: item.hasCurrentUserReadReceipt
            )
        case .redact:
            return TimelineItem(
                id: item.id,
                eventID: item.eventID,
                serverEventID: item.serverEventID,
                senderID: item.senderID,
                senderProfileDisplayName: item.senderProfileDisplayName,
                senderAvatarURL: item.senderAvatarURL,
                timestamp: item.timestamp,
                kind: .redacted,
                replyToEventID: item.replyToEventID,
                threadRootEventID: item.threadRootEventID,
                replyPreview: item.replyPreview,
                threadSummary: item.threadSummary,
                actionCapabilities: item.actionCapabilities,
                isEdited: item.isEdited,
                isAgentApproval: false,
                reactions: [:],
                reactionOwnership: .known([]),
                isEncrypted: item.isEncrypted,
                deliveryStatus: item.deliveryStatus,
                hasCurrentUserReadReceipt: item.hasCurrentUserReadReceipt
            )
        case .react(let reaction):
            var reactions = item.reactions
            reactions[reaction, default: 0] += 1
            let reactionOwnership: TimelineReactionOwnership
            switch item.reactionOwnership {
            case .unknown:
                reactionOwnership = .unknown
            case .known(var ownKeys):
                ownKeys.insert(reaction)
                reactionOwnership = .known(ownKeys)
            }
            return TimelineItem(
                id: item.id,
                eventID: item.eventID,
                serverEventID: item.serverEventID,
                senderID: item.senderID,
                senderProfileDisplayName: item.senderProfileDisplayName,
                senderAvatarURL: item.senderAvatarURL,
                timestamp: item.timestamp,
                kind: item.kind,
                replyToEventID: item.replyToEventID,
                threadRootEventID: item.threadRootEventID,
                replyPreview: item.replyPreview,
                threadSummary: item.threadSummary,
                poll: item.poll,
                actionCapabilities: item.actionCapabilities,
                isEdited: item.isEdited,
                isAgentApproval: item.isAgentApproval,
                reactions: reactions,
                reactionOwnership: reactionOwnership,
                isEncrypted: item.isEncrypted,
                deliveryStatus: item.deliveryStatus,
                hasCurrentUserReadReceipt: item.hasCurrentUserReadReceipt
            )
        }
    }
}
