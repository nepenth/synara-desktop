import Foundation

protocol LocalWiping {
    func logoutAndWipe() async throws
}

enum LocalWipeError: LocalizedError, Equatable {
    case pusherCleanupFailed
    case sessionDeleteFailed
    case timedOut

    var errorDescription: String? {
        switch self {
        case .pusherCleanupFailed:
            "Could not remove this device's push registration. Try signing out again."
        case .sessionDeleteFailed:
            "Could not clear local session state."
        case .timedOut:
            "Signing out is taking too long. Try signing out again."
        }
    }

    /// Converts the bounded logout failure contract into safe, actionable UI
    /// copy. Unknown implementation errors deliberately collapse to the local
    /// deletion message rather than exposing an arbitrary localized payload.
    static func displayMessage(for error: Error) -> String {
        switch error as? LocalWipeError {
        case .pusherCleanupFailed:
            return LocalWipeError.pusherCleanupFailed.localizedDescription
        case .timedOut:
            return LocalWipeError.timedOut.localizedDescription
        case .sessionDeleteFailed, .none:
            return LocalWipeError.sessionDeleteFailed.localizedDescription
        }
    }
}

/// Bounds one sign-out attempt without relying on cancellation.
///
/// A task group waits for every child, so it cannot bound an await that does
/// not check for cancellation. Here the work runs in its own task and races a
/// timer; whichever finishes first resumes the caller exactly once. On a
/// timeout the work keeps running, and the caller may wait on the same task
/// again rather than starting a second logout.
enum BoundedSignOut {
    static let timeoutNanoseconds: UInt64 = 15_000_000_000

    static func wait(
        for work: Task<Void, Error>,
        timeoutNanoseconds: UInt64 = timeoutNanoseconds
    ) async throws {
        let gate = ResumeOnce()
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            Task {
                let result: Result<Void, Error>
                do {
                    try await work.value
                    result = .success(())
                } catch {
                    result = .failure(error)
                }
                if gate.claim() {
                    continuation.resume(with: result)
                }
            }
            Task {
                try? await Task.sleep(nanoseconds: timeoutNanoseconds)
                if gate.claim() {
                    continuation.resume(throwing: LocalWipeError.timedOut)
                }
            }
        }
    }

    private final class ResumeOnce: @unchecked Sendable {
        private let lock = NSLock()
        private var claimed = false

        func claim() -> Bool {
            lock.withLock {
                if claimed {
                    return false
                }
                claimed = true
                return true
            }
        }
    }
}

struct AppLocalWipeService: LocalWiping {
    let session: AppSessionStore
    let matrix: MatrixClientServicing
    let roomList: RoomListServicing
    let timeline: TimelineServicing
    let drafts: DraftStore
    let push: PushServicing
    let router: AppRouter
    var outgoingSends: OutgoingSendCoordinator? = nil
    var sessionReadiness: SignedInSessionReadinessServicing = ImmediateSignedInSessionReadiness()

    func logoutAndWipe() async throws {
        let activeSession = await MainActor.run { () -> AuthenticatedSession? in
            if case .signedIn(let signedInSession) = session.currentState {
                return signedInSession
            }
            return nil
        }

        // Attempt account-bound remote cleanup while credentials are usable.
        // Offline cleanup must not prevent durable local sign-out.
        _ = await push.clearRegistrationState()
        if let activeSession {
            _ = await matrix.revokeServerSession(activeSession)
        }

        do {
            if let activeSession {
                try await matrix.forgetPersistedSession(activeSession)
            }
            try await MainActor.run {
                try session.signOut()
            }
        } catch {
            push.cancelRegistrationTeardown()
            throw LocalWipeError.sessionDeleteFailed
        }
        push.completeRegistrationTeardown()

        await matrix.stop()
        await matrix.resetLocalState(for: activeSession)
        // Core owners are gone; the startup gate must forget this identity or a
        // same-device re-login is considered already prepared and never restarts
        // the Matrix client.
        await sessionReadiness.resetForSignOut()
        roomList.clearCache()
        timeline.clearSessionCaches()
        drafts.clearAll()
        outgoingSends?.queue.clear()
        await MainActor.run {
            router.resetNavigationPathsForAccountChange()
        }
    }
}

final class MockLocalWipeService: LocalWiping {
    private(set) var wipeCallCount = 0
    var error: Error?

    func logoutAndWipe() async throws {
        wipeCallCount += 1
        if let error {
            throw error
        }
    }
}
