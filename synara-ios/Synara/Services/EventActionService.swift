import Foundation

enum EventActionType: Equatable {
    case reply
    case edit
    case redact
    case react(String)
    case report(reason: String?)
    case forward(
        targetRoomID: String,
        asQuote: Bool,
        confirmedEncryptionDowngrade: Bool
    )
    case pollVote(answerIDs: [String])
    case declineCall

    var inFlightKey: String {
        switch self {
        case .reply: return "reply"
        case .edit: return "edit"
        case .redact: return "redact"
        case .react(let key): return "react:\(key)"
        case .report: return "report"
        case .forward: return "forward"
        case .pollVote: return "poll-vote"
        case .declineCall: return "decline-call"
        }
    }
}

enum TimelineActionReadbackPolicy {
    static let schemaVersion: UInt32 = 1

    static func accepts(
        schemaVersion: UInt32,
        action: String,
        roomID: String,
        eventID: String,
        status: String,
        expectedAction: String,
        expectedRoomID: String,
        expectedStatus: String,
        expectedEventID: String? = nil
    ) -> Bool {
        schemaVersion == Self.schemaVersion
            && action == expectedAction
            && roomID == expectedRoomID
            && eventID.isEmpty == false
            && status == expectedStatus
            && (expectedEventID == nil || eventID == expectedEventID)
    }
}

enum TimelineReactionReadbackPolicy {
    static func acceptsToggle(
        roomID: String,
        targetEventID: String,
        key: String,
        mutation: String,
        readbackKey: String?,
        readbackOwnsReaction: Bool?,
        expectedRoomID: String,
        expectedTargetEventID: String,
        expectedKey: String,
        expectedOwn: Bool
    ) -> Bool {
        guard roomID == expectedRoomID,
              targetEventID == expectedTargetEventID,
              key == expectedKey
        else {
            return false
        }
        switch mutation {
        case "added":
            guard expectedOwn else { return false }
            return readbackKey == nil
                ? readbackOwnsReaction == nil
                : readbackKey == key && readbackOwnsReaction == true
        case "removed":
            guard expectedOwn == false else { return false }
            return readbackKey == nil
                ? readbackOwnsReaction == nil
                : readbackKey == key && readbackOwnsReaction == false
        default:
            return false
        }
    }
}

struct EventActionAvailability: Equatable {
    let canReply: Bool
    let canEdit: Bool
    let canRedact: Bool
    let canReact: Bool
    let canReport: Bool
    let canForward: Bool
    let canVote: Bool
    let canDeclineCall: Bool

    init(
        canReply: Bool,
        canEdit: Bool,
        canRedact: Bool,
        canReact: Bool,
        canReport: Bool = false,
        canForward: Bool = false,
        canVote: Bool = false,
        canDeclineCall: Bool = false
    ) {
        self.canReply = canReply
        self.canEdit = canEdit
        self.canRedact = canRedact
        self.canReact = canReact
        self.canReport = canReport
        self.canForward = canForward
        self.canVote = canVote
        self.canDeclineCall = canDeclineCall
    }
}

enum EventActionError: LocalizedError, Equatable {
    case signedOut
    case alreadyInProgress
    case failed

    var errorDescription: String? {
        switch self {
        case .signedOut:
            return "Sign in before changing messages."
        case .alreadyInProgress:
            return "That action is already in progress."
        case .failed:
            return "That action could not be completed. Try again."
        }
    }
}

/// Owns duplicate-submit exclusion for the lifetime of the signed-in product
/// services, rather than for the lifetime of a transient room or sheet view.
/// Keys identify one Matrix room/event/action class; user-entered payload is
/// deliberately excluded so changing a reason, target, or poll answer cannot
/// create a concurrent write for the same action.
final class TimelineActionInFlightCoordinator: @unchecked Sendable {
    private struct PendingPoll {
        let answerIDs: Set<String>
        var dispatchSettled = false
        var projectionObserved = false
    }

    private struct PendingReaction {
        let reactionKey: String
        let expectedOwn: Bool
        var dispatchSettled = false
        var projectionObserved = false
    }

    private let lock = NSLock()
    private var sessionEpoch: Int?
    private var keys: Set<String> = []
    private var pendingPolls: [String: PendingPoll] = [:]
    private var pendingReactions: [String: PendingReaction] = [:]

    func bindSession(epoch: Int) {
        lock.lock()
        defer { lock.unlock() }
        guard sessionEpoch != epoch else { return }
        sessionEpoch = epoch
        keys.removeAll()
        pendingPolls.removeAll()
        pendingReactions.removeAll()
    }

    func begin(_ key: String) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return keys.insert(key).inserted
    }

    func end(_ key: String) {
        lock.lock()
        defer { lock.unlock() }
        keys.remove(key)
        pendingPolls.removeValue(forKey: key)
        pendingReactions.removeValue(forKey: key)
    }

    /// Claims a poll mutation and records the desired projection before the
    /// SDK call can emit a local echo. This prevents an early projection from
    /// being lost while the async command is suspended.
    func beginPoll(_ key: String, answerIDs: [String]) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        guard keys.insert(key).inserted else { return false }
        pendingPolls[key] = PendingPoll(answerIDs: Set(answerIDs))
        return true
    }

    /// Records the typed command readback. The lock is released only after
    /// both this acknowledgement and the exact authoritative projection have
    /// arrived, regardless of their order.
    func settlePollDispatch(_ key: String) {
        lock.lock()
        defer { lock.unlock() }
        guard var pending = pendingPolls[key] else { return }
        pending.dispatchSettled = true
        if pending.projectionObserved {
            pendingPolls.removeValue(forKey: key)
            keys.remove(key)
        } else {
            pendingPolls[key] = pending
        }
    }

    func observePollProjection(_ key: String, ownAnswerIDs: [String]) {
        lock.lock()
        defer { lock.unlock() }
        guard var pending = pendingPolls[key], pending.answerIDs == Set(ownAnswerIDs) else { return }
        pending.projectionObserved = true
        if pending.dispatchSettled {
            pendingPolls.removeValue(forKey: key)
            keys.remove(key)
        } else {
            pendingPolls[key] = pending
        }
    }

    func beginReaction(_ key: String, reactionKey: String, expectedOwn: Bool) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        guard keys.insert(key).inserted else { return false }
        pendingReactions[key] = PendingReaction(
            reactionKey: reactionKey,
            expectedOwn: expectedOwn
        )
        return true
    }

    func settleReactionDispatch(_ key: String) {
        lock.lock()
        defer { lock.unlock() }
        guard var pending = pendingReactions[key] else { return }
        pending.dispatchSettled = true
        if pending.projectionObserved {
            pendingReactions.removeValue(forKey: key)
            keys.remove(key)
        } else {
            pendingReactions[key] = pending
        }
    }

    func observeReactionProjection(_ actionPrefix: String, ownership: TimelineReactionOwnership) {
        guard case let .known(ownKeys) = ownership else { return }
        lock.lock()
        defer { lock.unlock() }
        for key in pendingReactions.keys.filter({ $0.hasPrefix(actionPrefix) }) {
            guard var pending = pendingReactions[key],
                  ownKeys.contains(pending.reactionKey) == pending.expectedOwn
            else {
                continue
            }
            pending.projectionObserved = true
            if pending.dispatchSettled {
                pendingReactions.removeValue(forKey: key)
                keys.remove(key)
            } else {
                pendingReactions[key] = pending
            }
        }
    }

    func contains(prefix: String) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return keys.contains(where: { $0.hasPrefix(prefix) })
    }

    func contains(_ key: String) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return keys.contains(key)
    }
}

enum TimelineForwardSecurityDecision: Equatable {
    case unavailable
    case confirmDowngrade
    case proceed
}

enum TimelineForwardSecurityPolicy {
    static func decision(
        sourceEncryption: SynaraRoomEncryptionStatus,
        targetEncryption: SynaraRoomEncryptionStatus
    ) -> TimelineForwardSecurityDecision {
        switch (sourceEncryption, targetEncryption) {
        case (.unknown, _), (.unavailable, _), (_, .unknown), (_, .unavailable):
            return .unavailable
        case (.encrypted, .notEncrypted):
            return .confirmDowngrade
        case (.encrypted, .encrypted), (.notEncrypted, .encrypted), (.notEncrypted, .notEncrypted):
            return .proceed
        }
    }
}

protocol EventActionServicing {
    func availability(for item: TimelineItem, currentUserID: String) -> EventActionAvailability
    func availability(for item: TimelineItem, currentUserID: String, roomID: String) -> EventActionAvailability
    func apply(_ action: EventActionType, to item: TimelineItem, currentUserID: String, roomID: String) async throws -> TimelineItem
}

extension EventActionServicing {
    func availability(
        for item: TimelineItem,
        currentUserID: String,
        roomID: String
    ) -> EventActionAvailability {
        _ = roomID
        return availability(for: item, currentUserID: currentUserID)
    }
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
                canReact: capabilities.canReact,
                canReport: capabilities.canReport,
                canForward: capabilities.canForward && item.forwardTransport != .unavailable,
                canVote: capabilities.canVote,
                canDeclineCall: capabilities.canDeclineCall
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
                forwardTransport: item.forwardTransport,
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
                forwardTransport: item.forwardTransport,
                isEdited: item.isEdited,
                isAgentApproval: item.isAgentApproval,
                reactions: reactions,
                reactionOwnership: reactionOwnership,
                isEncrypted: item.isEncrypted,
                deliveryStatus: item.deliveryStatus,
                hasCurrentUserReadReceipt: item.hasCurrentUserReadReceipt
            )
        case .report, .forward, .pollVote, .declineCall:
            // Mock/local timelines have no Matrix side effect. Keep the row
            // stable while exercising the native presenter action path.
            return item
        }
    }
}
