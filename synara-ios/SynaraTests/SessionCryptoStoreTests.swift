import XCTest
@testable import Synara

final class SessionCryptoStoreTests: XCTestCase {
    private let unverified = SessionCryptoStatus(
        verification: .unverified,
        recovery: .disabled,
        backup: .unavailable,
        hasDevicesToVerifyAgainst: false,
        isLastDevice: true,
        unableToDecryptCount: nil
    )
    private let verified = SessionCryptoStatus(
        verification: .verified,
        recovery: .enabled,
        backup: .enabled,
        hasDevicesToVerifyAgainst: true,
        isLastDevice: false,
        unableToDecryptCount: nil
    )

    @MainActor
    func testConcurrentReadsShareOneCoreReadAndFreshResultsAreReused() async {
        let crypto = CountingCryptoStatusService(status: unverified)
        let store = SessionCryptoStore(crypto: crypto, freshness: 60)
        crypto.holdReads()

        async let first = store.read()
        async let second = store.refresh()
        async let third = store.read()
        try? await waitUntil { crypto.readCount == 1 }
        crypto.releaseReads()
        let results = await [first, second, third]

        XCTAssertEqual(results, [unverified, unverified, unverified])
        XCTAssertEqual(crypto.readCount, 1, "concurrent screens share one read")
        _ = await store.read()
        XCTAssertEqual(crypto.readCount, 1, "a fresh result is reused")
        _ = await store.refresh()
        XCTAssertEqual(crypto.readCount, 2, "Retry and screen open read again")
    }

    @MainActor
    func testUpdatesPublishOnlyChangesAndCoalesceAWakeupBurst() async throws {
        let crypto = CountingCryptoStatusService(status: unverified)
        let store = SessionCryptoStore(crypto: crypto, freshness: 60, wakeupSpacing: 0.2)
        let received = ReceivedStatuses()
        let updates = store.updates()
        let observing = Task { @MainActor in
            for await status in updates {
                received.values.append(status)
            }
        }
        await store.refresh()
        try await waitUntil { received.values == [self.unverified] }

        // A burst that changes nothing reads once and publishes nothing.
        let readsBeforeBurst = crypto.readCount
        crypto.wake(times: 50)
        try await waitUntil { store.wakeupRevision >= 1 }
        try await Task.sleep(nanoseconds: 400_000_000)
        XCTAssertLessThanOrEqual(crypto.readCount - readsBeforeBurst, 2)
        XCTAssertEqual(received.values, [unverified], "an unchanged status is not published again")
        XCTAssertEqual(store.status, unverified)

        // Verification completing elsewhere is picked up by the next wakeup.
        crypto.status = verified
        crypto.wake(times: 1)
        try await waitUntil { received.values == [self.unverified, self.verified] }
        XCTAssertEqual(store.status, verified)

        crypto.finishWakeups()
        await observing.value
    }

    @MainActor
    func testWakeupDuringAReadIsFollowedByOneMoreRead() async throws {
        let crypto = CountingCryptoStatusService(status: unverified)
        let store = SessionCryptoStore(crypto: crypto, freshness: 60, wakeupSpacing: 0)
        let received = ReceivedStatuses()
        let updates = store.updates()
        let observing = Task { @MainActor in
            for await status in updates {
                received.values.append(status)
            }
        }
        crypto.holdReads()
        let initial = Task { @MainActor in await store.refresh() }
        try await waitUntil { crypto.readCount == 1 }
        // The change lands while the first read is already in flight.
        crypto.status = verified
        crypto.wake(times: 3)
        try await Task.sleep(nanoseconds: 50_000_000)
        crypto.releaseReads()
        _ = await initial.value
        try await waitUntil { received.values == [self.unverified, self.verified] }
        XCTAssertEqual(crypto.readCount, 2)

        crypto.finishWakeups()
        await observing.value
    }

    @MainActor
    func testResetForgetsTheSignedOutAccountsStatusAndFreshness() async {
        let crypto = CountingCryptoStatusService(status: verified)
        let store = SessionCryptoStore(crypto: crypto, freshness: 60)
        await store.read()
        XCTAssertEqual(store.status, verified)
        XCTAssertEqual(crypto.readCount, 1)

        store.reset()
        XCTAssertEqual(store.status, .unknown)

        crypto.status = unverified
        let next = await store.read()
        XCTAssertEqual(crypto.readCount, 2, "a reset store reads again inside the freshness window")
        XCTAssertEqual(next, unverified)
        XCTAssertEqual(store.status, unverified)
    }

    @MainActor
    func testReadInFlightDuringResetIsNotPublished() async throws {
        let crypto = CountingCryptoStatusService(status: verified)
        let store = SessionCryptoStore(crypto: crypto, freshness: 60)
        crypto.holdReads()
        let stale = Task { @MainActor in await store.refresh() }
        try await waitUntil { crypto.readCount == 1 }

        store.reset()
        crypto.releaseReads()
        _ = await stale.value
        XCTAssertEqual(store.status, .unknown, "the previous account's read must not land")

        // The next read is a new one, not the dropped read.
        crypto.status = unverified
        await store.read()
        XCTAssertEqual(crypto.readCount, 2)
        XCTAssertEqual(store.status, unverified)
    }

    func testRoomsObserveSessionUpdatesOnlyWhileUnverifiedOrUndecryptable() {
        func room(_ verification: SynaraCryptoVerificationStatus, utds: Int) -> RoomCryptoStatus {
            RoomCryptoStatus(
                encryption: .encrypted,
                verification: verification,
                recovery: .enabled,
                backup: .enabled,
                unableToDecryptCount: utds
            )
        }
        XCTAssertTrue(RoomCryptoObservationPolicy.needsSessionUpdates(room(.unverified, utds: 0)))
        XCTAssertTrue(RoomCryptoObservationPolicy.needsSessionUpdates(room(.unknown, utds: 0)))
        XCTAssertTrue(RoomCryptoObservationPolicy.needsSessionUpdates(room(.verified, utds: 2)))
        XCTAssertFalse(RoomCryptoObservationPolicy.needsSessionUpdates(room(.verified, utds: 0)))
    }
}

@MainActor
private final class ReceivedStatuses {
    var values: [SessionCryptoStatus] = []
}

@MainActor
private func waitUntil(
    timeoutNanoseconds: UInt64 = 2_000_000_000,
    condition: @escaping () -> Bool
) async throws {
    let started = DispatchTime.now().uptimeNanoseconds
    while condition() == false {
        if DispatchTime.now().uptimeNanoseconds - started >= timeoutNanoseconds {
            XCTFail("Timed out waiting for the session crypto store")
            return
        }
        try await Task.sleep(nanoseconds: 5_000_000)
    }
}

private final class CountingCryptoStatusService: CryptoStatusServicing {
    private let lock = NSLock()
    private var current: SessionCryptoStatus
    private var reads = 0
    private var holding = false
    private var held: [CheckedContinuation<Void, Never>] = []
    private var wakeups: AsyncStream<Void>.Continuation?

    init(status: SessionCryptoStatus) {
        current = status
    }

    var status: SessionCryptoStatus {
        get { lock.withLock { current } }
        set { lock.withLock { current = newValue } }
    }

    var readCount: Int {
        lock.withLock { reads }
    }

    func holdReads() {
        lock.withLock { holding = true }
    }

    func releaseReads() {
        let waiting = lock.withLock {
            holding = false
            defer { held.removeAll() }
            return held
        }
        waiting.forEach { $0.resume() }
    }

    func wake(times: Int) {
        let continuation = lock.withLock { wakeups }
        for _ in 0..<times {
            continuation?.yield(())
        }
    }

    func finishWakeups() {
        lock.withLock { wakeups }?.finish()
    }

    func sessionStatus() async -> SessionCryptoStatus {
        let (shouldHold, snapshot) = lock.withLock {
            reads += 1
            return (holding, current)
        }
        if shouldHold {
            await withCheckedContinuation { continuation in
                let resumeNow = lock.withLock {
                    guard holding else { return true }
                    held.append(continuation)
                    return false
                }
                if resumeNow { continuation.resume() }
            }
        }
        return snapshot
    }

    func sessionDeviceUpdates() -> AsyncStream<Void> {
        AsyncStream(bufferingPolicy: .bufferingNewest(1)) { continuation in
            lock.withLock { wakeups = continuation }
        }
    }

    func roomStatus(roomID: String) async -> RoomCryptoStatus { .unknown }
    func verificationUpdates() -> AsyncStream<CryptoVerificationSnapshot?> { AsyncStream { $0.finish() } }
    func retryDecryption(roomID: String) async -> CryptoActionResult { .unavailable("unused") }
    func requestDeviceVerification(deviceId: String?) async -> CryptoActionResult { .unavailable("unused") }
    func acceptVerificationRequest() async -> CryptoActionResult { .unavailable("unused") }
    func startSasVerification() async -> CryptoActionResult { .unavailable("unused") }
    func approveVerification() async -> CryptoActionResult { .unavailable("unused") }
    func declineVerification() async -> CryptoActionResult { .unavailable("unused") }
    func cancelVerification() async -> CryptoActionResult { .unavailable("unused") }
    func recover(recoveryKey: String) async -> CryptoActionResult { .unavailable("unused") }
}
