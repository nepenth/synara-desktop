import Foundation

protocol SignedInSessionReadinessServicing: Sendable {
    func beginPreparing(for session: AuthenticatedSession) async
    func markPrepared(for session: AuthenticatedSession) async
    func waitUntilPrepared(for session: AuthenticatedSession) async
}

/// Gates room-list loading until `SessionCoordinator` finishes preparing the Matrix client.
actor SignedInSessionReadiness: SignedInSessionReadinessServicing {
    private var preparedToken: String?
    private var waiters: [CheckedContinuation<Void, Never>] = []

    func beginPreparing(for session: AuthenticatedSession) async {
        preparedToken = nil
    }

    func markPrepared(for session: AuthenticatedSession) async {
        preparedToken = readinessToken(for: session)
        let pending = waiters
        waiters.removeAll()
        for waiter in pending {
            waiter.resume()
        }
    }

    func waitUntilPrepared(for session: AuthenticatedSession) async {
        let token = readinessToken(for: session)
        if preparedToken == token {
            return
        }

        await withCheckedContinuation { continuation in
            waiters.append(continuation)
        }

        if preparedToken != token {
            await waitUntilPrepared(for: session)
        }
    }

    private func readinessToken(for session: AuthenticatedSession) -> String {
        "\(session.userID)|\(session.deviceID)"
    }
}

struct ImmediateSignedInSessionReadiness: SignedInSessionReadinessServicing {
    func beginPreparing(for session: AuthenticatedSession) async {
        _ = session
    }

    func markPrepared(for session: AuthenticatedSession) async {
        _ = session
    }

    func waitUntilPrepared(for session: AuthenticatedSession) async {
        _ = session
    }
}