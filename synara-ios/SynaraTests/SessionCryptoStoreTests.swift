import XCTest
import UIKit
@testable import Synara
import SynaraCore

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

/// Security → Set Up Recovery: the flow's states, and the Core-to-Swift
/// status and error mapping it relies on.
@MainActor
final class RecoverySetupFlowTests: XCTestCase {
    func testNoPasswordServerGoesStraightToTheKeyShownOnce() async {
        let crypto = ScriptedRecoveryService(setup: [.ready], enrollment: .enabled(recoveryKey: "EsTj 3yST y93F"))
        let flow = RecoverySetupFlow()

        await flow.start(passphrase: "correct horse", confirmation: "correct horse", crypto: crypto)

        XCTAssertEqual(flow.phase, .showingKey("EsTj 3yST y93F"))
        XCTAssertNil(flow.errorMessage)
        XCTAssertEqual(crypto.enrolledPassphrases, ["correct horse"])
        flow.acknowledgeKey()
        XCTAssertEqual(flow.phase, .finished, "the key leaves memory once acknowledged")
    }

    func testPasswordStageThenRejectedPasswordThenSuccess() async {
        let crypto = ScriptedRecoveryService(
            setup: [.passwordRequired],
            authentication: [.passwordRejected, .ready],
            enrollment: .enabled(recoveryKey: "key")
        )
        let flow = RecoverySetupFlow()

        await flow.start(passphrase: "pass", confirmation: "pass", crypto: crypto)
        XCTAssertEqual(flow.phase, .passwordRequired)
        XCTAssertTrue(crypto.enrolledPassphrases.isEmpty, "no enrolment before cross-signing")

        await flow.authenticate(password: "", crypto: crypto)
        XCTAssertEqual(flow.errorMessage, "Enter your account password.")
        XCTAssertTrue(crypto.passwords.isEmpty)

        await flow.authenticate(password: "wrong", crypto: crypto)
        XCTAssertEqual(flow.phase, .passwordRequired)
        XCTAssertEqual(flow.errorMessage, RecoverySetupCopy.passwordRejected)

        await flow.authenticate(password: "right", crypto: crypto)
        XCTAssertEqual(flow.phase, .showingKey("key"))
        XCTAssertNil(flow.errorMessage)
        XCTAssertEqual(crypto.passwords, ["wrong", "right"])
        XCTAssertEqual(crypto.enrolledPassphrases, ["pass"])
    }

    func testMismatchedOrEmptyPassphraseNeverCallsCore() async {
        let crypto = ScriptedRecoveryService(setup: [.ready], enrollment: .enabled(recoveryKey: "key"))
        let flow = RecoverySetupFlow()

        await flow.start(passphrase: "one", confirmation: "two", crypto: crypto)
        XCTAssertEqual(flow.errorMessage, "Recovery passphrases do not match.")
        await flow.start(passphrase: "  ", confirmation: "  ", crypto: crypto)
        XCTAssertEqual(flow.errorMessage, "Choose a recovery passphrase.")

        XCTAssertEqual(flow.phase, .idle)
        XCTAssertEqual(crypto.setupCalls, 0)
    }

    func testAlreadyEnabledFinishesWithoutAKeyAndFailuresReturnToIdle() async {
        let already = ScriptedRecoveryService(setup: [.ready], enrollment: .alreadyEnabled)
        let flow = RecoverySetupFlow()
        await flow.start(passphrase: "p", confirmation: "p", crypto: already)
        XCTAssertEqual(flow.phase, .finished)

        let partial = ScriptedRecoveryService(setup: [.failed("partial identity")], enrollment: .alreadyEnabled)
        let failing = RecoverySetupFlow()
        await failing.start(passphrase: "p", confirmation: "p", crypto: partial)
        XCTAssertEqual(failing.phase, .idle)
        XCTAssertEqual(failing.errorMessage, "partial identity")
        XCTAssertTrue(partial.enrolledPassphrases.isEmpty)

        let enrolFails = ScriptedRecoveryService(setup: [.ready], enrollment: .failed("existing backup"))
        let retry = RecoverySetupFlow()
        await retry.start(passphrase: "p", confirmation: "p", crypto: enrolFails)
        XCTAssertEqual(retry.phase, .idle)
        XCTAssertEqual(retry.errorMessage, "existing backup")
    }

    func testCancellingThePasswordStepEnrolsNothing() async {
        let crypto = ScriptedRecoveryService(setup: [.passwordRequired], enrollment: .enabled(recoveryKey: "key"))
        let flow = RecoverySetupFlow()
        await flow.start(passphrase: "p", confirmation: "p", crypto: crypto)
        flow.cancel()
        XCTAssertEqual(flow.phase, .idle)
        await flow.authenticate(password: "right", crypto: crypto)
        XCTAssertTrue(crypto.passwords.isEmpty)
        XCTAssertTrue(crypto.enrolledPassphrases.isEmpty)
    }

    func testCrossSigningReadinessAndRecoveryAvailabilityMapping() {
        XCTAssertEqual(SharedCoreSessionCrypto.crossSigning(.ready), .ready)
        XCTAssertEqual(SharedCoreSessionCrypto.crossSigning(.setupRequired), .notSetUp)
        XCTAssertEqual(SharedCoreSessionCrypto.crossSigning(.verificationRequired), .needsVerification)
        XCTAssertEqual(SharedCoreSessionCrypto.crossSigning(.recoveryRequired), .needsRecovery)
        XCTAssertEqual(SharedCoreSessionCrypto.crossSigning(.unavailable), .unknown)
        XCTAssertEqual(SharedCoreSessionCrypto.crossSigning(nil), .unknown)
        XCTAssertEqual(SynaraCryptoCrossSigningStatus.ready.settingsDisplayName, "Ready")
        XCTAssertEqual(SynaraCryptoCrossSigningStatus.notSetUp.settingsDisplayName, "Not Set Up")

        XCTAssertEqual(SharedCoreSessionCrypto.recoverySetupAvailable(.bootstrapRequired), true)
        XCTAssertEqual(SharedCoreSessionCrypto.recoverySetupAvailable(.unlockRequired), false)
        XCTAssertEqual(SharedCoreSessionCrypto.recoverySetupAvailable(.noAction), false)
        XCTAssertNil(SharedCoreSessionCrypto.recoverySetupAvailable(nil))
        XCTAssertEqual(SessionCryptoStatus.unknown.crossSigning, .unknown)
        XCTAssertNil(SessionCryptoStatus.unknown.recoverySetupAvailable)
    }

    func testCoreSetupOutcomesAndErrorsMapToFlowResults() {
        XCTAssertEqual(SharedCoreCryptoStatusService.crossSigningSetupResult(.complete), .ready)
        XCTAssertEqual(SharedCoreCryptoStatusService.crossSigningSetupResult(.alreadyConfigured), .ready)
        XCTAssertEqual(
            SharedCoreCryptoStatusService.crossSigningSetupResult(.authenticationRequired),
            .passwordRequired
        )
        func failure(_ code: String) -> SessionStatusError {
            .Failed(kind: .terminal, code: code, description: "static")
        }
        XCTAssertEqual(
            SharedCoreCryptoStatusService.crossSigningSetupFailure(
                failure("v-crypto.2-cross-signing-password-rejected")
            ),
            .passwordRejected
        )
        guard case .failed(let partial) = SharedCoreCryptoStatusService.crossSigningSetupFailure(
            failure("v-crypto.2-cross-signing-partial-private-identity")
        ) else {
            return XCTFail("partial identity must fail")
        }
        XCTAssertTrue(partial.contains("only part"))
        XCTAssertEqual(
            SharedCoreCryptoStatusService.crossSigningSetupFailure(failure("anything-else")),
            .failed(RecoverySetupCopy.setupFailed)
        )
        guard case .failed(let existing) = SharedCoreCryptoStatusService.recoveryEnrollmentFailure(
            failure("v-crypto.4-bootstrap-existing-backup")
        ) else {
            return XCTFail("existing backup must fail")
        }
        XCTAssertTrue(existing.contains("Recover Keys"))
        XCTAssertEqual(
            SharedCoreCryptoStatusService.recoveryEnrollmentFailure(failure("v-crypto.4-bootstrap-failed")),
            .failed(RecoverySetupCopy.enrollmentFailed)
        )
    }

    func testUnreadableLocalDataIsNotReportedAsBadCredentials() {
        XCTAssertEqual(
            SharedCoreLoginErrorMapping.loginError(
                for: SessionLoginError.Failed(kind: .actionRequired, code: "p4-s3c-store-reset-required", description: "static")
            ),
            .localDataUnreadable
        )
        XCTAssertNotEqual(LoginError.localDataUnreadable.errorDescription, LoginError.invalidCredentials.errorDescription)
    }

    func testRecoveryKeyCopyIsLocalOnlyAndExpires() {
        let pasteboard = UIPasteboard.withUniqueName()
        defer { UIPasteboard.remove(withName: pasteboard.name) }
        RecoveryKeyPasteboard.copy("EsTj 3yST", to: pasteboard)
        XCTAssertEqual(pasteboard.string, "EsTj 3yST")
        XCTAssertEqual(RecoveryKeyPasteboard.lifetime, 120)
    }
}

private final class ScriptedRecoveryService: CryptoStatusServicing {
    private var setupResults: [CrossSigningSetupResult]
    private var authenticationResults: [CrossSigningSetupResult]
    private let enrollment: RecoveryEnrollmentResult
    private(set) var setupCalls = 0
    private(set) var passwords: [String] = []
    private(set) var enrolledPassphrases: [String] = []

    init(
        setup: [CrossSigningSetupResult],
        authentication: [CrossSigningSetupResult] = [],
        enrollment: RecoveryEnrollmentResult
    ) {
        setupResults = setup
        authenticationResults = authentication
        self.enrollment = enrollment
    }

    func startCrossSigningSetup() async -> CrossSigningSetupResult {
        setupCalls += 1
        return setupResults.isEmpty ? .failed("unscripted") : setupResults.removeFirst()
    }

    func authenticateCrossSigningSetup(password: String) async -> CrossSigningSetupResult {
        passwords.append(password)
        return authenticationResults.isEmpty ? .failed("unscripted") : authenticationResults.removeFirst()
    }

    func enableRecovery(passphrase: String) async -> RecoveryEnrollmentResult {
        enrolledPassphrases.append(passphrase)
        return enrollment
    }

    func sessionStatus() async -> SessionCryptoStatus { .unknown }
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
