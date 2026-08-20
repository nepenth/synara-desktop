import Foundation
import SynaraCore

/// Cold-start and foreground bootstrap: restore → attach → start.
///
/// Login retains a Client first, so restore is skipped only on the dedicated
/// already-restored code. Any other restore/attach/start failure is fail-closed
/// and must not look like a live session. NSE still cannot start sync.
///
/// Idle after `start()` is not live. The sequencer observes until
/// running/offline or a bounded timeout, then fail-closes.
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
            case .restoreFailed:
                return .restoreFailed
            case .attachFailed, .startFailed:
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
    static let defaultObserveAttempts = 30
    static let defaultObserveDelayNanoseconds: UInt64 = 100_000_000

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
            engine: SharedCoreLiveSessionEngine(core: core),
            observeAttempts: defaultObserveAttempts,
            observeDelayNanoseconds: defaultObserveDelayNanoseconds
        )
    }

    static func prepareLiveSession(
        userID: String,
        homeserverURL: String,
        storeRoot: URL,
        engine: any LiveSessionEngine,
        observeAttempts: Int = defaultObserveAttempts,
        observeDelayNanoseconds: UInt64 = defaultObserveDelayNanoseconds
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
                applyStartResult(try await engine.startSync(), to: &outcome)
                if outcome.started {
                    break
                }
            } catch {
                outcome.started = false
            }
        }

        if outcome.started == false, isIdleReadiness(outcome.readiness) {
            for _ in 0..<max(observeAttempts, 0) {
                if observeDelayNanoseconds > 0 {
                    try? await Task.sleep(nanoseconds: observeDelayNanoseconds)
                }
                do {
                    applyStartResult(try await engine.observeSync(), to: &outcome)
                } catch {
                    continue
                }
                if outcome.started || isTerminalReadiness(outcome.readiness) {
                    break
                }
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

    static func isProductLiveReadiness(_ readiness: String?) -> Bool {
        switch readiness {
        case "running", "offline":
            return true
        default:
            return false
        }
    }

    static func isIdleReadiness(_ readiness: String?) -> Bool {
        readiness == "idle"
    }

    static func isTerminalReadiness(_ readiness: String?) -> Bool {
        switch readiness {
        case "failed", "terminated", "unconfigured":
            return true
        default:
            return false
        }
    }

    private static func applyStartResult(_ dto: StartResult, to outcome: inout Outcome) {
        outcome.readiness = dto.readiness
        outcome.started = dto.started || isProductLiveReadiness(dto.readiness)
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
    func observeSync() async throws -> SharedCoreSessionBootstrap.StartResult
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
        return startResult(started: dto.started, readiness: dto.readiness)
    }

    func observeSync() async throws -> SharedCoreSessionBootstrap.StartResult {
        let dto = try await SharedCoreSessionStatus.syncStatus(core: core)
        return startResult(started: false, readiness: dto.readiness)
    }

    private func startResult(started: Bool, readiness: String) -> SharedCoreSessionBootstrap.StartResult {
        SharedCoreSessionBootstrap.StartResult(
            started: started || SharedCoreSessionBootstrap.isProductLiveReadiness(readiness),
            readiness: readiness
        )
    }
}
