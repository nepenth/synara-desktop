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
    let push: PushServicing

    func logoutAndWipe() async throws {
        await matrix.stop()
        matrix.resetLocalState()
        roomList.clearCache()
        push.clearRegistrationState()

        do {
            try session.signOut()
        } catch {
            throw LocalWipeError.sessionDeleteFailed
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
