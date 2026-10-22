import Foundation

protocol SignedInSessionReadinessServicing: Sendable {
    /// Returns true only to the caller that owns startup for this session.
    func claimPreparation(for session: AuthenticatedSession) async -> Bool
    @discardableResult
    func markPrepared(for session: AuthenticatedSession) async -> Bool
    func cancelPreparation(for session: AuthenticatedSession) async
    /// Returns false when another signed-in identity superseded this startup.
    func waitUntilPrepared(for session: AuthenticatedSession) async -> Bool
    /// Forgets every prepared or in-flight identity. Sign-out must call this:
    /// the crypto device is reused on re-login, so the next session carries the
    /// same `user|device` token and would otherwise be treated as already
    /// prepared, skipping `matrix.start` and leaving the room list without a
    /// live Core session until the process is relaunched.
    func resetForSignOut() async
    /// The Matrix stores are closing for background suspension. A prepared
    /// session stops counting as prepared, so a room or timeline opened
    /// meanwhile waits for the foreground resume instead of failing on
    /// closed stores.
    func suspend() async
    /// A foreground (or background-mutation) start for `session` finished.
    /// Releases everything parked since `suspend()`.
    func resume(for session: AuthenticatedSession) async
}

/// Gates room-list loading until `SessionCoordinator` finishes preparing the Matrix client.
actor SignedInSessionReadiness: SignedInSessionReadinessServicing {
    private var preparingToken: String?
    private var preparedToken: String?
    /// Prepared before (or while) the stores closed for suspension; it becomes
    /// prepared again on `resume(for:)`.
    private var suspendedToken: String?
    private var isSuspended = false
    private var waiters: [String: [CheckedContinuation<Bool, Never>]] = [:]
    private var invalidatedTokens: Set<String> = []

    func claimPreparation(for session: AuthenticatedSession) async -> Bool {
        let token = readinessToken(for: session)
        // A suspended session is resumed by the lifecycle path, not prepared again.
        if preparedToken == token || preparingToken == token || suspendedToken == token {
            return false
        }
        if let preparingToken, preparingToken != token {
            rememberInvalidated(preparingToken)
        }
        if let suspendedToken {
            rememberInvalidated(suspendedToken)
            self.suspendedToken = nil
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
        guard preparingToken == token else { return preparedToken == token || suspendedToken == token }
        preparingToken = nil
        invalidatedTokens.remove(token)
        if isSuspended {
            // `matrix.start` returned without opening the stores (suppressed in
            // the background, or paused meanwhile). Waiters keep waiting for
            // the foreground resume instead of opening against closed stores.
            suspendedToken = token
            return true
        }
        preparedToken = token
        releaseWaiters(for: token, returning: true)
        return true
    }

    func suspend() async {
        isSuspended = true
        if let preparedToken {
            suspendedToken = preparedToken
            self.preparedToken = nil
        }
    }

    func resume(for session: AuthenticatedSession) async {
        isSuspended = false
        guard let suspendedToken else { return }
        self.suspendedToken = nil
        let token = readinessToken(for: session)
        guard suspendedToken == token else {
            // A different identity resumed; the suspended one is gone.
            rememberInvalidated(suspendedToken)
            releaseWaiters(for: suspendedToken, returning: false)
            return
        }
        preparedToken = token
        releaseWaiters(for: token, returning: true)
    }

    func cancelPreparation(for session: AuthenticatedSession) async {
        let token = readinessToken(for: session)
        guard preparingToken == token else { return }
        preparingToken = nil
        rememberInvalidated(token)
        releaseWaiters(for: token, returning: false)
    }

    func resetForSignOut() async {
        preparingToken = nil
        preparedToken = nil
        suspendedToken = nil
        isSuspended = false
        invalidatedTokens.removeAll()
        let pending = waiters
        waiters.removeAll()
        for continuations in pending.values {
            for waiter in continuations {
                waiter.resume(returning: false)
            }
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

    /// Test hook: number of continuations currently parked in `waitUntilPrepared`.
    func pendingWaiterCount() -> Int {
        waiters.values.reduce(0) { $0 + $1.count }
    }

    private func readinessToken(for session: AuthenticatedSession) -> String {
        "\(session.userID)|\(session.deviceID)"
    }

    private func rememberInvalidated(_ token: String) {
        invalidatedTokens.insert(token)
    }

    private func releaseWaiters(for token: String, returning prepared: Bool) {
        let pending = waiters.removeValue(forKey: token) ?? []
        for waiter in pending {
            waiter.resume(returning: prepared)
        }
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

    func resetForSignOut() async {}

    func suspend() async {}

    func resume(for session: AuthenticatedSession) async {
        _ = session
    }
}
