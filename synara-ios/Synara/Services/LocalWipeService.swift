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

    func logoutAndWipe() async throws {
        let activeSession: AuthenticatedSession?
        if case .signedIn(let signedInSession) = session.currentState {
            activeSession = signedInSession
        } else {
            activeSession = nil
        }

        await matrix.stop()
        await push.clearRegistrationState()
        await matrix.resetLocalState()
        roomList.clearCache()
        timeline.clearSessionCaches()
        drafts.clearAll()
        router.resetNavigationPathsForAccountChange()

        do {
            try session.signOut()
        } catch {
            throw LocalWipeError.sessionDeleteFailed
        }

        _ = activeSession
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