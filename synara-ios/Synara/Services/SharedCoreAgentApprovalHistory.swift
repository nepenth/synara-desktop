import Foundation
import SynaraCore

/// Typed native agent-approval history snapshot. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the registered history snapshot only. Summaries may cross.
/// Failed errors stay static. It is not a generic `Core.command` FFI.
enum SharedCoreAgentApprovalHistory {
    static func snapshot(core: SharedCore) async throws -> AgentApprovalHistorySnapshotDto {
        try await core.agentApprovalHistorySnapshot()
    }

    static func inbox(core: SharedCore, discoveryActive: Bool) async throws -> AgentApprovalInboxDto {
        try await core.agentApprovalsList(discoveryActive: discoveryActive)
    }
}

enum AgentApprovalHistoryError: Error, LocalizedError, Equatable {
    case noSession
    case unavailable

    var errorDescription: String? {
        switch self {
        case .noSession:
            return "Sign in to load approval history."
        case .unavailable:
            return "Approval history could not be loaded."
        }
    }
}

protocol AgentApprovalHistoryServicing {
    func loadInbox(discoveryActive: Bool) async -> Result<[AgentApprovalInboxRecord], AgentApprovalHistoryError>
    func loadHistory() async -> Result<[AgentApprovalHistoryRecord], AgentApprovalHistoryError>
    func historyUpdates() -> AsyncStream<Void>
}

final class MockAgentApprovalHistoryService: AgentApprovalHistoryServicing {
    var inboxItems: [AgentApprovalInboxRecord]
    var historyItems: [AgentApprovalHistoryRecord]
    var error: AgentApprovalHistoryError?

    init(
        inboxItems: [AgentApprovalInboxRecord] = [],
        historyItems: [AgentApprovalHistoryRecord] = [],
        error: AgentApprovalHistoryError? = nil
    ) {
        self.inboxItems = inboxItems
        self.historyItems = historyItems
        self.error = error
    }

    func loadInbox(discoveryActive: Bool) async -> Result<[AgentApprovalInboxRecord], AgentApprovalHistoryError> {
        _ = discoveryActive
        if let error {
            return .failure(error)
        }
        return .success(inboxItems)
    }

    func loadHistory() async -> Result<[AgentApprovalHistoryRecord], AgentApprovalHistoryError> {
        if let error {
            return .failure(error)
        }
        return .success(historyItems)
    }

    func historyUpdates() -> AsyncStream<Void> {
        AsyncStream { continuation in
            continuation.finish()
        }
    }
}

final class SharedCoreAgentApprovalHistoryService: AgentApprovalHistoryServicing {
    private let host: SharedCoreProductHost

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func loadInbox(discoveryActive: Bool) async -> Result<[AgentApprovalInboxRecord], AgentApprovalHistoryError> {
        guard case .signedIn = host.sessionStore.currentState else {
            return .failure(.noSession)
        }
        do {
            let snapshot = try await SharedCoreAgentApprovalHistory.inbox(
                core: host.core,
                discoveryActive: discoveryActive
            )
            return .success(snapshot.items.compactMap(Self.inboxRecord(from:)))
        } catch {
            return .failure(.unavailable)
        }
    }

    func loadHistory() async -> Result<[AgentApprovalHistoryRecord], AgentApprovalHistoryError> {
        guard case .signedIn = host.sessionStore.currentState else {
            return .failure(.noSession)
        }
        do {
            let snapshot = try await SharedCoreAgentApprovalHistory.snapshot(core: host.core)
            return .success(snapshot.items.compactMap(Self.historyRecord(from:)))
        } catch {
            return .failure(.unavailable)
        }
    }

    func historyUpdates() -> AsyncStream<Void> {
        let updates = host.livePoller.ownerSignals(
            families: ["agent_approval_history"],
            bufferingPolicy: .bufferingNewest(1)
        )
        return SharedCoreSessionDeviceInvalidations.stream(updates)
    }

    static func inboxRecord(from item: AgentApprovalInboxItemDto) -> AgentApprovalInboxRecord? {
        guard let status = AgentApprovalInboxStatus(rawValue: item.status) else {
            return nil
        }
        return AgentApprovalInboxRecord(
            roomId: item.roomId,
            eventId: item.eventId,
            sender: item.sender,
            body: item.body,
            canSendReaction: item.canSendReaction,
            bodyTruncated: item.bodyTruncated,
            originServerTs: Double(item.originServerTs),
            expiresAt: Double(item.expiresAt),
            status: status
        )
    }

    static func historyRecord(from item: AgentApprovalHistoryItemDto) -> AgentApprovalHistoryRecord? {
        guard let decision = AgentApprovalHistoryDecision(rawValue: item.decision) else {
            return nil
        }
        return AgentApprovalHistoryRecord(
            roomId: item.roomId,
            eventId: item.eventId,
            sender: item.sender,
            decision: decision,
            decidedAt: item.decidedAt,
            originServerTs: item.originServerTs,
            expiresAt: item.expiresAt,
            summary: item.summary
        )
    }
}
