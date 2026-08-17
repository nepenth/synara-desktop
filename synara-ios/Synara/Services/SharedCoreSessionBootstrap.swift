import Foundation
import SynaraCore

/// P4-S13 cold-start bootstrap: restore → attach → start.
///
/// Uses an already-constructed SharedCore. The caller owns the core so
/// UniFFI does not free the retained Client. Each step is fail-closed
/// and independent: a retained login client skips restore, an already
/// attached owner set skips attach, then start runs. NSE still cannot
/// start sync. This is not iOS-on-engine and not P4 acceptance.
enum SharedCoreSessionBootstrap {
    struct Outcome: Equatable {
        var restored: Bool
        var attached: Bool
        var started: Bool
        var readiness: String?
    }

    static func prepareLiveSession(
        userID: String,
        homeserverURL: String,
        storeRoot: URL,
        core: SharedCore
    ) async -> Outcome {
        var outcome = Outcome(
            restored: false,
            attached: false,
            started: false,
            readiness: nil
        )
        if (try? await SharedCoreSessionRestore.restorePersistedSession(
            userID: userID,
            homeserverURL: homeserverURL,
            storeRoot: storeRoot,
            core: core
        )) != nil {
            outcome.restored = true
        }
        if (try? await SharedCoreSessionAttach.attachSessionOwners(core: core)) != nil {
            outcome.attached = true
        }
        if let dto = try? await SharedCoreSyncStart.startSync(core: core) {
            outcome.started = dto.started
            outcome.readiness = dto.readiness
        }
        return outcome
    }
}
