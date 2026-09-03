import Foundation

protocol LocalWiping {
    func logoutAndWipe() async throws
}

enum LocalWipeError: LocalizedError, Equatable {
    case pusherCleanupFailed
    case sessionDeleteFailed

    var errorDescription: String? {
        switch self {
        case .pusherCleanupFailed:
            "Could not remove this device's push registration. Try signing out again."
        case .sessionDeleteFailed:
            "Could not clear local session state."
        }
    }

    /// Converts the bounded logout failure contract into safe, actionable UI
    /// copy. Unknown implementation errors deliberately collapse to the local
    /// deletion message rather than exposing an arbitrary localized payload.
    static func displayMessage(for error: Error) -> String {
        switch error as? LocalWipeError {
        case .pusherCleanupFailed:
            return LocalWipeError.pusherCleanupFailed.localizedDescription
        case .sessionDeleteFailed, .none:
            return LocalWipeError.sessionDeleteFailed.localizedDescription
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

    func logoutAndWipe() async throws {
        let activeSession = await MainActor.run { () -> AuthenticatedSession? in
            if case .signedIn(let signedInSession) = session.currentState {
                return signedInSession
            }
            return nil
        }

        // Remove the remote pusher first, through its retained account-bound
        // Core owner. If cleanup fails, preserve both the owner and local
        // session so a later sign-out attempt can retry with valid credentials.
        guard await push.clearRegistrationState() else {
            throw LocalWipeError.pusherCleanupFailed
        }

        // Only after remote pusher cleanup succeeds may the persisted Matrix
        // device session be deleted. If Keychain deletion fails, the running
        // Core session remains available and no destructive local cleanup runs.
        do {
            try await MainActor.run {
                try session.signOut()
            }
        } catch {
            push.cancelRegistrationTeardown()
            throw LocalWipeError.sessionDeleteFailed
        }
        push.completeRegistrationTeardown()

        // Server logout remains best effort after durable local sign-out.
        if let activeSession {
            _ = await matrix.revokeServerSession(activeSession)
        }
        await matrix.stop()
        await matrix.resetLocalState(for: activeSession)
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
