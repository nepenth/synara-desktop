import Foundation
import SynaraCore

/// Typed native agent-approval write. The Rust owner constructs and sends the
/// Matrix event; arbitrary event JSON never crosses the Swift/UniFFI boundary.
enum SharedCoreAgentApprovals {
    static func send(
        core: SharedCore,
        roomId: String,
        actionId: String,
        actionTitle: String,
        decision: String,
        sourceEventId: String?,
        createdAt: UInt64
    ) async throws -> AgentApprovalSendDto {
        try await core.sendAgentApproval(
            roomId: roomId,
            actionId: actionId,
            actionTitle: actionTitle,
            decision: decision,
            sourceEventId: sourceEventId,
            createdAt: createdAt
        )
    }
}
