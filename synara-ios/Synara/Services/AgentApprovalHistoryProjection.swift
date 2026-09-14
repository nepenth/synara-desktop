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

    /// Recent = account-data history ∪ inbox items whose status ≠ pending. Account data wins.
    static func unionRecent(
        inboxItems: [AgentApprovalInboxRecord],
        historyItems: [AgentApprovalHistoryRecord]
    ) -> [AgentApprovalInboxRecord] {
        var merged: [String: AgentApprovalInboxRecord] = [:]
        for item in inboxItems where item.status != .pending {
            merged[item.identity] = item
        }
        for item in historyItems {
            merged[identity(roomId: item.roomId, eventId: item.eventId)] = inboxItem(from: item)
        }
        return merged.values.sorted(by: compareRecent)
    }
}
