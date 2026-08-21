import Foundation

protocol LocalWiping {
    func logoutAndWipe() async throws
}

enum LocalWipeError: LocalizedError, Equatable {
    case sessionDeleteFailed

    var errorDescription: String? {
        "Could not clear local session state."
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
        let activeSession: AuthenticatedSession?
        if case .signedIn(let signedInSession) = session.currentState {
            activeSession = signedInSession
        } else {
            activeSession = nil
        }

        // Remove the persisted Matrix device session before deleting its SDK
        // crypto store or stopping reversible services. If Keychain deletion
        // fails, retaining both prevents the next launch from rebuilding a fresh
        // store under the same device ID and leaves the running session intact.
        do {
            try session.signOut()
        } catch {
            throw LocalWipeError.sessionDeleteFailed
        }

        // These remote operations are best effort after durable local sign-out.
        // Clear the pusher while the captured access token is still valid, then
        // revoke that token/device session without reconstructing a client/store.
        await push.clearRegistrationState()
        if let activeSession {
            _ = await matrix.revokeServerSession(activeSession)
        }
        await matrix.stop()
        await matrix.resetLocalState(for: activeSession)
        roomList.clearCache()
        timeline.clearSessionCaches()
        drafts.clearAll()
        outgoingSends?.queue.clear()
        router.resetNavigationPathsForAccountChange()
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
