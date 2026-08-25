import Foundation

protocol SignedInSessionReadinessServicing: Sendable {
    /// Returns true only to the caller that owns startup for this session.
    func claimPreparation(for session: AuthenticatedSession) async -> Bool
    @discardableResult
    func markPrepared(for session: AuthenticatedSession) async -> Bool
    func cancelPreparation(for session: AuthenticatedSession) async
    /// Returns false when another signed-in identity superseded this startup.
    func waitUntilPrepared(for session: AuthenticatedSession) async -> Bool
}

/// Gates room-list loading until `SessionCoordinator` finishes preparing the Matrix client.
actor SignedInSessionReadiness: SignedInSessionReadinessServicing {
    private var preparingToken: String?
    private var preparedToken: String?
    private var waiters: [String: [CheckedContinuation<Bool, Never>]] = [:]
    private var invalidatedTokens: Set<String> = []

    func claimPreparation(for session: AuthenticatedSession) async -> Bool {
        let token = readinessToken(for: session)
        if preparedToken == token || preparingToken == token {
            return false
        }
        if let preparingToken, preparingToken != token {
            rememberInvalidated(preparingToken)
        }
        invalidatedTokens.remove(token)
        preparingToken = token
        preparedToken = nil
        let supersededTokens = waiters.keys.filter { $0 != token }
        for supersededToken in supersededTokens {
            rememberInvalidated(supersededToken)
            let superseded = waiters.removeValue(forKey: supersededToken) ?? []
            for waiter in superseded {
                waiter.resume(returning: false)
            }
        }
        return true
    }

    func markPrepared(for session: AuthenticatedSession) async -> Bool {
        let token = readinessToken(for: session)
        guard preparingToken == token else { return preparedToken == token }
        preparingToken = nil
        preparedToken = token
        invalidatedTokens.remove(token)
        let pending = waiters.removeValue(forKey: token) ?? []
        for waiter in pending {
            waiter.resume(returning: true)
        }
        return true
    }

    func cancelPreparation(for session: AuthenticatedSession) async {
        let token = readinessToken(for: session)
        guard preparingToken == token else { return }
        preparingToken = nil
        rememberInvalidated(token)
        let pending = waiters.removeValue(forKey: token) ?? []
        for waiter in pending {
            waiter.resume(returning: false)
        }
    }

    func waitUntilPrepared(for session: AuthenticatedSession) async -> Bool {
        let token = readinessToken(for: session)
        if preparedToken == token {
            return true
        }
        if invalidatedTokens.contains(token) {
            return false
        }

        // Room content may begin waiting a few scheduler turns before the
        // root shell claims startup. Only a *different* identity fails this
        // waiter closed; an unclaimed matching startup is allowed to wait.
        guard preparingToken == nil || preparingToken == token else { return false }

        return await withCheckedContinuation { continuation in
            waiters[token, default: []].append(continuation)
        }
    }

    private func readinessToken(for session: AuthenticatedSession) -> String {
        "\(session.userID)|\(session.deviceID)"
    }

    private func rememberInvalidated(_ token: String) {
        invalidatedTokens.insert(token)
    }
}

struct ImmediateSignedInSessionReadiness: SignedInSessionReadinessServicing {
    func claimPreparation(for session: AuthenticatedSession) async -> Bool {
        _ = session
        return true
    }

    func markPrepared(for session: AuthenticatedSession) async -> Bool {
        _ = session
        return true
    }

    func cancelPreparation(for session: AuthenticatedSession) async {
        _ = session
    }

    func waitUntilPrepared(for session: AuthenticatedSession) async -> Bool {
        _ = session
        return true
    }
}
