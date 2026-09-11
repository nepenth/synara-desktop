import Foundation

/// Drives the real sheet and presenter for local UI tests, without a server or
/// changes to a user's encryption identity. Only selected by --ui-testing.
final class VerificationUITestService: CryptoStatusServicing {
    private let lock = NSLock()
    private var current: CryptoVerificationSnapshot?
    private var subscribers: [UUID: AsyncStream<CryptoVerificationSnapshot?>.Continuation] = [:]
    private let verified: Bool
    private let identity = CryptoVerificationSnapshot.ID(sessionGeneration: 1, flowID: "ui-verification")

    init(verified: Bool) {
        self.verified = verified
        current = .init(id: identity, state: .requestReceived(.init(
            userID: "@alice:matrix.org", displayName: nil,
            deviceID: "DESKTOP", deviceDisplayName: "Synara on Mac", flowID: identity.flowID
        )))
    }

    func verificationUpdates() -> AsyncStream<CryptoVerificationSnapshot?> {
        AsyncStream { continuation in
            let id = UUID()
            lock.lock()
            subscribers[id] = continuation
            continuation.yield(current)
            lock.unlock()
            continuation.onTermination = { [weak self] _ in self?.removeSubscriber(id) }
        }
    }

    private func removeSubscriber(_ id: UUID) {
        lock.lock()
        subscribers.removeValue(forKey: id)
        lock.unlock()
    }

    private func publish(_ state: CryptoVerificationState?) {
        lock.lock()
        current = state.map { .init(id: identity, state: $0) }
        for subscriber in subscribers.values { subscriber.yield(current) }
        lock.unlock()
    }

    func dismissVerification(flowID: String) async -> CryptoActionResult {
        guard flowID == identity.flowID else { return .failed("Wrong flow") }
        publish(nil)
        // Reproduce buffered native done events arriving after acknowledgement.
        Task {
            try? await Task.sleep(nanoseconds: 250_000_000)
            publish(.finished)
        }
        return .completed("Closed")
    }

    func acceptVerificationRequest() async -> CryptoActionResult {
        publish(.emojis([
            .init(symbol: "🐢", description: "Turtle"), .init(symbol: "🎁", description: "Gift"),
            .init(symbol: "⌛", description: "Hourglass"), .init(symbol: "🔨", description: "Hammer"),
            .init(symbol: "🚲", description: "Bicycle"), .init(symbol: "🐇", description: "Rabbit"),
            .init(symbol: "😀", description: "Smiley")
        ]))
        return .completed("Accepted")
    }
    func approveVerification() async -> CryptoActionResult { publish(.finished); return .completed("Confirmed") }
    func declineVerification() async -> CryptoActionResult { publish(.mismatched); return .completed("Declined") }
    func cancelVerification() async -> CryptoActionResult { publish(.cancelled); return .completed("Cancelled") }
    func startSasVerification() async -> CryptoActionResult { await acceptVerificationRequest() }
    func requestDeviceVerification(deviceId: String?) async -> CryptoActionResult { .completed("Requested") }
    func roomStatus(roomID: String) async -> RoomCryptoStatus { .unknown }
    func sessionStatus() async -> SessionCryptoStatus {
        .init(verification: verified ? .verified : .unverified, recovery: .incomplete, backup: .unavailable,
              hasDevicesToVerifyAgainst: true, isLastDevice: false, unableToDecryptCount: 0)
    }
    func retryDecryption(roomID: String) async -> CryptoActionResult { .unavailable("Fixture") }
    func recover(recoveryKey: String) async -> CryptoActionResult { .unavailable("Fixture") }
}
