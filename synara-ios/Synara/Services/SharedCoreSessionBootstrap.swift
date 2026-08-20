import Foundation
import SynaraCore

/// Cold-start and foreground bootstrap: restore → attach → start.
///
/// Login retains a Client first, so restore is skipped only on the dedicated
/// already-restored code. Any other restore/attach/start failure is fail-closed
/// and must not look like a live session. NSE still cannot start sync.
enum SharedCoreSessionBootstrap {
    struct Outcome: Equatable {
        var restored: Bool
        var skippedRestore: Bool
        var attached: Bool
        var skippedAttach: Bool
        var started: Bool
        var readiness: String?
        var failure: Failure?

        var hasLiveClient: Bool {
            restored || skippedRestore
        }

        var hasAttachedOwners: Bool {
            attached || skippedAttach
        }
    }

    enum Failure: Equatable {
        case restoreFailed
        case attachFailed
        case startFailed

        var syncStatus: MatrixSyncStatus {
            switch self {
            case .restoreFailed, .attachFailed:
                return .restoreFailed
            case .startFailed:
                return .disconnected
            }
        }
    }

    enum StepError: Error, Equatable {
        case failed(code: String)
    }

    struct StartResult: Equatable {
        var started: Bool
        var readiness: String
    }

    static let alreadyRestoredCode = "p4-s3b-session-already-restored"
    static let alreadyAttachedCode = "p4-s3d-already-attached"

    static func prepareLiveSession(
        userID: String,
        homeserverURL: String,
        storeRoot: URL,
        core: SharedCore
    ) async -> Outcome {
        await prepareLiveSession(
            userID: userID,
            homeserverURL: homeserverURL,
            storeRoot: storeRoot,
            engine: SharedCoreLiveSessionEngine(core: core)
        )
    }

    static func prepareLiveSession(
        userID: String,
        homeserverURL: String,
        storeRoot: URL,
        engine: any LiveSessionEngine
    ) async -> Outcome {
        var outcome = Outcome(
            restored: false,
            skippedRestore: false,
            attached: false,
            skippedAttach: false,
            started: false,
            readiness: nil,
            failure: nil
        )

        do {
            try await engine.restorePersistedSession(
                userID: userID,
                homeserverURL: homeserverURL,
                storeRoot: storeRoot
            )
            outcome.restored = true
        } catch {
            if errorCode(from: error) == alreadyRestoredCode {
                outcome.skippedRestore = true
            } else {
                outcome.failure = .restoreFailed
                return outcome
            }
        }

        do {
            try await engine.attachSessionOwners()
            outcome.attached = true
        } catch {
            if errorCode(from: error) == alreadyAttachedCode {
                outcome.skippedAttach = true
            } else {
                outcome.failure = .attachFailed
                return outcome
            }
        }

        for _ in 0..<2 {
            do {
                let dto = try await engine.startSync()
                applyStartResult(dto, to: &outcome)
                if outcome.started {
                    break
                }
            } catch {
                outcome.started = false
            }
        }
        if outcome.started == false {
            outcome.failure = .startFailed
        }

        return outcome
    }

    static func errorCode(from error: Error) -> String? {
        if case let StepError.failed(code) = error {
            return code
        }
        if case let SessionRestoreError.Failed(code, _) = error {
            return code
        }
        if case let SessionAttachError.Failed(code, _) = error {
            return code
        }
        if case let SyncStartError.Failed(code, _) = error {
            return code
        }
        return nil
    }

    static func isLiveReadiness(_ readiness: String?) -> Bool {
        switch readiness {
        case "running", "offline", "idle":
            return true
        default:
            return false
        }
    }

    private static func applyStartResult(_ dto: StartResult, to outcome: inout Outcome) {
        outcome.readiness = dto.readiness
        outcome.started = dto.started || isLiveReadiness(dto.readiness)
    }
}

protocol LiveSessionEngine: Sendable {
    func restorePersistedSession(
        userID: String,
        homeserverURL: String,
        storeRoot: URL
    ) async throws
    func attachSessionOwners() async throws
    func startSync() async throws -> SharedCoreSessionBootstrap.StartResult
}

struct SharedCoreLiveSessionEngine: LiveSessionEngine, @unchecked Sendable {
    let core: SharedCore

    func restorePersistedSession(
        userID: String,
        homeserverURL: String,
        storeRoot: URL
    ) async throws {
        _ = try await SharedCoreSessionRestore.restorePersistedSession(
            userID: userID,
            homeserverURL: homeserverURL,
            storeRoot: storeRoot,
            core: core
        )
    }

    func attachSessionOwners() async throws {
        _ = try await SharedCoreSessionAttach.attachSessionOwners(core: core)
    }

    func startSync() async throws -> SharedCoreSessionBootstrap.StartResult {
        let dto = try await SharedCoreSyncStart.startSync(core: core)
        return SharedCoreSessionBootstrap.StartResult(
            started: dto.started,
            readiness: dto.readiness
        )
    }
}
