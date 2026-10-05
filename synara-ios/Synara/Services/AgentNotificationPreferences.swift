import Foundation
import SynaraCore

struct SynaraAgentNotificationPreferences: Equatable {
    var agentUserIDs: [String]
    var notifyToolActivity: Bool
    var notifyCommentary: Bool
    var notifyFinalResponses: Bool

    init(dto: AgentNotificationPreferencesDto) {
        agentUserIDs = dto.agentUserIds
        notifyToolActivity = dto.notifyToolActivity
        notifyCommentary = dto.notifyCommentary
        notifyFinalResponses = dto.notifyFinalResponses
    }

    var dto: AgentNotificationPreferencesDto {
        AgentNotificationPreferencesDto(
            schemaVersion: 1,
            agentUserIds: agentUserIDs,
            notifyToolActivity: notifyToolActivity,
            notifyCommentary: notifyCommentary,
            notifyFinalResponses: notifyFinalResponses
        )
    }
}

enum SynaraAgentNotificationPreferencesError: LocalizedError {
    case unavailable

    var errorDescription: String? {
        "Agent notification settings are unavailable. Check your connection and try again."
    }
}

extension MatrixClientServicing {
    func agentNotificationEventAllowed(roomID: String, eventID: String, session: AuthenticatedSession) async -> Bool {
        true
    }

    func agentNotificationPreferenceUpdates() -> AsyncStream<Void> {
        AsyncStream { $0.finish() }
    }

    func agentNotificationPreferences() async throws -> SynaraAgentNotificationPreferences {
        throw SynaraAgentNotificationPreferencesError.unavailable
    }

    func setAgentNotificationPreferences(
        _ preferences: SynaraAgentNotificationPreferences
    ) async throws -> SynaraAgentNotificationPreferences {
        throw SynaraAgentNotificationPreferencesError.unavailable
    }
}

enum SharedCoreAgentNotificationPreferences {
    static func invalidations<Element>(_ updates: AsyncStream<Element>) -> AsyncStream<Void> {
        AsyncStream(bufferingPolicy: .bufferingNewest(1)) { continuation in
            let task = Task {
                for await _ in updates {
                    guard !Task.isCancelled else { break }
                    continuation.yield(())
                }
                continuation.finish()
            }
            continuation.onTermination = { _ in task.cancel() }
        }
    }

    static func snapshot(core: SharedCore) async throws -> SynaraAgentNotificationPreferences {
        let dto = try await core.agentNotificationPreferencesSnapshot()
        guard dto.schemaVersion == 1 else { throw SynaraAgentNotificationPreferencesError.unavailable }
        return SynaraAgentNotificationPreferences(dto: dto)
    }

    static func set(
        core: SharedCore,
        preferences: SynaraAgentNotificationPreferences
    ) async throws -> SynaraAgentNotificationPreferences {
        let dto = try await core.agentNotificationPreferencesSet(preferences: preferences.dto)
        guard dto.schemaVersion == 1 else { throw SynaraAgentNotificationPreferencesError.unavailable }
        return SynaraAgentNotificationPreferences(dto: dto)
    }
}
