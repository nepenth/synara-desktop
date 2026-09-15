import Foundation

enum AgentApprovalHistoryDecision: String, Equatable {
    case approveOnce = "approve_once"
    case approveAlways = "approve_always"
    case deny
}

enum AgentApprovalInboxStatus: String, Equatable {
    case pending
    case decided
    case expired
}

struct AgentApprovalHistoryRecord: Equatable {
    let roomId: String
    let eventId: String
    let sender: String
    let decision: AgentApprovalHistoryDecision
    let decidedAt: Double
    let originServerTs: Double
    let expiresAt: Double
    let summary: String
}

struct AgentApprovalInboxRecord: Equatable {
    let roomId: String
    let eventId: String
    let sender: String
    let body: String
    let canSendReaction: Bool
    let bodyTruncated: Bool
    let originServerTs: Double
    let expiresAt: Double
    let status: AgentApprovalInboxStatus
    var decision: AgentApprovalHistoryDecision?
    var decidedAt: Double?
    var summary: String?

    var identity: String { AgentApprovalHistoryProjection.identity(roomId: roomId, eventId: eventId) }
}

enum AgentApprovalHistoryProjection {
    static let emptySummary = "Command not recorded"

    static func identity(roomId: String, eventId: String) -> String {
        "\(roomId)\u{0000}\(eventId)"
    }

    static func decisionLabel(
        decision: AgentApprovalHistoryDecision?,
        status: AgentApprovalInboxStatus
    ) -> String {
        if status == .expired {
            return "Expired"
        }
        switch decision {
        case .approveOnce:
            return "Approved once"
        case .approveAlways:
            return "Approved always"
        case .deny:
            return "Denied"
        case nil:
            return status == .decided ? "Decided" : "Expired"
        }
    }

    static func recordedSummary(_ value: String) -> String {
        value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? emptySummary : value
    }

    static func inboxItem(from history: AgentApprovalHistoryRecord) -> AgentApprovalInboxRecord {
        let summary = recordedSummary(history.summary)
        return AgentApprovalInboxRecord(
            roomId: history.roomId,
            eventId: history.eventId,
            sender: history.sender,
            body: summary,
            canSendReaction: false,
            bodyTruncated: false,
            originServerTs: history.originServerTs,
            expiresAt: history.expiresAt,
            status: .decided,
            decision: history.decision,
            decidedAt: history.decidedAt,
            summary: summary
        )
    }

    /// Newest decidedAt/originServerTs first, then roomId, then eventId.
    static func compareRecent(_ a: AgentApprovalInboxRecord, _ b: AgentApprovalInboxRecord) -> Bool {
        let left = a.decidedAt ?? a.originServerTs
        let right = b.decidedAt ?? b.originServerTs
        if left != right {
            return left > right
        }
        if a.roomId != b.roomId {
            return a.roomId < b.roomId
        }
        if a.eventId != b.eventId {
            return a.eventId < b.eventId
        }
        return false
    }

    /// Recent = account-data history ∪ inbox items whose status ≠ pending.
    /// History-only rows stay visible (cross-device). Inbox `decision` wins a
    /// conflict. A decided inbox row without `decision` does not inherit the
    /// history decision. An expired inbox row may still show the history decision.
    static func unionRecent(
        inboxItems: [AgentApprovalInboxRecord],
        historyItems: [AgentApprovalHistoryRecord]
    ) -> [AgentApprovalInboxRecord] {
        var merged: [String: AgentApprovalInboxRecord] = [:]
        for item in historyItems {
            merged[identity(roomId: item.roomId, eventId: item.eventId)] = inboxItem(from: item)
        }
        for item in inboxItems where item.status != .pending {
            if let history = merged[item.identity] {
                merged[item.identity] = mergeInboxOverHistory(inbox: item, history: history)
            } else {
                merged[item.identity] = item
            }
        }
        return merged.values.sorted(by: compareRecent)
    }

    private static func historyPreview(_ item: AgentApprovalInboxRecord) -> String? {
        guard let summary = item.summary?.trimmingCharacters(in: .whitespacesAndNewlines),
              !summary.isEmpty,
              summary != emptySummary
        else {
            return nil
        }
        return item.summary
    }

    /// Inbox proof wins a conflict. Account data is not a reaction.
    private static func mergeInboxOverHistory(
        inbox: AgentApprovalInboxRecord,
        history: AgentApprovalInboxRecord
    ) -> AgentApprovalInboxRecord {
        let inboxNamedDecision = inbox.decision != nil
        let hideUnprovenHistoryDecision = inbox.status == .decided && !inboxNamedDecision
        let decision: AgentApprovalHistoryDecision?
        if inboxNamedDecision {
            decision = inbox.decision
        } else if hideUnprovenHistoryDecision {
            decision = nil
        } else {
            decision = history.decision
        }
        let status: AgentApprovalInboxStatus =
            decision != nil || hideUnprovenHistoryDecision ? .decided : inbox.status
        let summary = historyPreview(history) ?? inbox.summary ?? inbox.body
        return AgentApprovalInboxRecord(
            roomId: inbox.roomId,
            eventId: inbox.eventId,
            sender: inbox.sender,
            body: summary,
            canSendReaction: inbox.canSendReaction,
            bodyTruncated: inbox.bodyTruncated,
            originServerTs: inbox.originServerTs,
            expiresAt: inbox.expiresAt,
            status: status,
            decision: decision,
            decidedAt: inbox.decidedAt ?? history.decidedAt,
            summary: summary
        )
    }
}
