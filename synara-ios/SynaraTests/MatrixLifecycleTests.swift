import XCTest
import UIKit
@testable import Synara
import SynaraCore

final class MatrixLifecycleTests: XCTestCase {
    func testStartSyncAfterLogin() async throws {
        let matrix = MockMatrixClientService()
        let session = try makeSession()

        await matrix.start(session: session)

        XCTAssertEqual(matrix.syncStatus, .syncing)
        XCTAssertEqual(matrix.syncStatusDescription, "Syncing history…")
        XCTAssertEqual(matrix.startedSessions, [session])
    }

    func testStopAndResetSync() async {
        let matrix = MockMatrixClientService(syncStatus: .syncing)

        await matrix.stop()
        await matrix.resetLocalState(for: nil)

        XCTAssertEqual(matrix.syncStatus, .stopped)
        XCTAssertEqual(matrix.stopCallCount, 1)
        XCTAssertEqual(matrix.resetCallCount, 1)
    }

    func testPauseAndResumeBackgroundLifecycle() async throws {
        let matrix = MockMatrixClientService(syncStatus: .syncing)
        let session = try makeSession()

        await matrix.pauseForBackground()
        XCTAssertEqual(matrix.pauseCallCount, 1)
        XCTAssertEqual(matrix.syncStatus, .stopped)

        await matrix.resumeFromForeground(session: session)
        XCTAssertEqual(matrix.resumeCallCount, 1)
        XCTAssertEqual(matrix.resumedSessions, [session])
        XCTAssertEqual(matrix.syncStatus, .syncing)
    }

    @MainActor
    func testBackgroundCoordinatorEndsAssertionAfterNativePause() async throws {
        let matrix = MockMatrixClientService(syncStatus: .syncing)
        let application = MockBackgroundTaskManager()
        let coordinator = SynaraBackgroundSyncCoordinator()

        coordinator.enterBackground(application: application, matrix: matrix)

        try await waitUntil { coordinator.hasPendingPause == false }
        XCTAssertEqual(matrix.pauseCallCount, 1)
        XCTAssertEqual(application.beginNames, ["Quiesce Matrix stores before suspension"])
        XCTAssertEqual(application.endedIdentifiers, [application.issuedIdentifier])
    }

    @MainActor
    func testSequentialDuplicateBackgroundCallbacksDoNotRecloseStores() async throws {
        let matrix = MockMatrixClientService(syncStatus: .syncing)
        let application = MockBackgroundTaskManager()
        let coordinator = SynaraBackgroundSyncCoordinator()

        coordinator.enterBackground(application: application, matrix: matrix)
        try await waitUntil { coordinator.hasPendingPause == false }
        coordinator.enterBackground(application: application, matrix: matrix)
        try await Task.sleep(nanoseconds: 20_000_000)

        XCTAssertEqual(matrix.pauseCallCount, 1)
        XCTAssertEqual(application.beginNames.count, 1)
        XCTAssertEqual(application.endedIdentifiers, [application.issuedIdentifier])
    }

    @MainActor
    func testBackgroundBindQuiescesServiceBeforeDeferredShellStartup() async throws {
        let matrix = MockMatrixClientService(syncStatus: .syncing)
        let application = MockBackgroundTaskManager()
        let coordinator = SynaraBackgroundSyncCoordinator()

        coordinator.bind(
            application: application,
            matrix: matrix,
            foregroundActive: false
        )

        try await waitUntil { coordinator.hasPendingPause == false }
        XCTAssertEqual(matrix.foregroundActivity, [false])
        XCTAssertEqual(matrix.pauseCallCount, 1)
        XCTAssertEqual(application.endedIdentifiers, [application.issuedIdentifier])
    }

    @MainActor
    func testBackgroundCoordinatorDoesNotRestartOnInitialActivation() async throws {
        let matrix = MockMatrixClientService(syncStatus: .syncing)
        let coordinator = SynaraBackgroundSyncCoordinator()
        let session = try makeSession()

        coordinator.enterForeground(matrix: matrix, session: session)
        try await Task.sleep(nanoseconds: 20_000_000)

        XCTAssertEqual(matrix.resumeCallCount, 0)
    }

    @MainActor
    func testForegroundBindRestoresAuthorityAfterPreBindActivation() async throws {
        let matrix = MockMatrixClientService(syncStatus: .stopped)
        let application = MockBackgroundTaskManager()
        let coordinator = SynaraBackgroundSyncCoordinator()

        coordinator.bind(
            application: application,
            matrix: matrix,
            foregroundActive: true
        )

        XCTAssertEqual(matrix.foregroundActivity, [true])
        XCTAssertEqual(matrix.pauseCallCount, 0)
        XCTAssertTrue(application.beginNames.isEmpty)
    }

    @MainActor
    func testBackgroundCoordinatorSerializesPauseBeforeForegroundResume() async throws {
        let matrix = MockMatrixClientService(syncStatus: .syncing)
        matrix.pauseDelayNanoseconds = 50_000_000
        var operations: [String] = []
        matrix.onOperation = { operations.append($0) }
        let application = MockBackgroundTaskManager()
        let coordinator = SynaraBackgroundSyncCoordinator()
        let session = try makeSession()

        coordinator.enterBackground(application: application, matrix: matrix)
        coordinator.enterBackground(application: application, matrix: matrix)
        coordinator.enterForeground(matrix: matrix, session: session)

        try await waitUntil { matrix.resumeCallCount == 1 }
        XCTAssertEqual(application.beginNames.count, 1)
        XCTAssertEqual(
            operations,
            ["matrix-pause-begin", "matrix-pause-end", "matrix-resume"]
        )
        XCTAssertEqual(matrix.resumedSessions, [session])
    }

    @MainActor
    func testBackgroundCoordinatorDoesNotResumeAfterRapidReentryToBackground() async throws {
        let matrix = MockMatrixClientService(syncStatus: .syncing)
        matrix.pauseDelayNanoseconds = 50_000_000
        let application = MockBackgroundTaskManager()
        let coordinator = SynaraBackgroundSyncCoordinator()
        let session = try makeSession()

        coordinator.enterBackground(application: application, matrix: matrix)
        coordinator.enterForeground(matrix: matrix, session: session)
        coordinator.enterBackground(application: application, matrix: matrix)

        try await waitUntil { coordinator.hasPendingPause == false }
        try await Task.sleep(nanoseconds: 20_000_000)
        XCTAssertEqual(matrix.resumeCallCount, 0)

        coordinator.enterForeground(matrix: matrix, session: session)
        try await waitUntil { matrix.resumeCallCount == 1 }
        XCTAssertEqual(application.beginNames.count, 1)
    }

    @MainActor
    func testBackgroundCoordinatorExpirationEndsAssertionExactlyOnce() async throws {
        let matrix = MockMatrixClientService(syncStatus: .syncing)
        matrix.pauseDelayNanoseconds = 50_000_000
        let application = MockBackgroundTaskManager()
        let coordinator = SynaraBackgroundSyncCoordinator()

        coordinator.enterBackground(application: application, matrix: matrix)
        application.expire()

        try await waitUntil { application.endedIdentifiers == [application.issuedIdentifier] }
        try await waitUntil { coordinator.hasPendingPause == false }
        XCTAssertEqual(matrix.pauseCallCount, 1)
        XCTAssertEqual(application.endedIdentifiers, [application.issuedIdentifier])
    }

    func testInactiveScenesKeepStoresOpenAndOnlyBackgroundQuiesces() {
        // Control Center, Notification Center, system alerts and the
        // app-switcher peek report inactive while UIKit is still inactive.
        XCTAssertEqual(
            SynaraScenePhasePolicy.transition(for: .inactive, applicationState: .inactive),
            .inactive
        )
        XCTAssertEqual(
            SynaraScenePhasePolicy.transition(for: .inactive, applicationState: .active),
            .inactive
        )
        XCTAssertEqual(
            SynaraScenePhasePolicy.transition(for: .background, applicationState: .background),
            .background
        )
        XCTAssertEqual(
            SynaraScenePhasePolicy.transition(for: .active, applicationState: .active),
            .foreground
        )
        // A scene sample never overrules UIKit's process-level state.
        XCTAssertEqual(
            SynaraScenePhasePolicy.transition(for: .active, applicationState: .background),
            .background
        )
        XCTAssertEqual(
            SynaraScenePhasePolicy.transition(for: .active, applicationState: .inactive),
            .inactive
        )
        XCTAssertEqual(
            SynaraScenePhasePolicy.transition(for: .inactive, applicationState: .background),
            .background
        )
    }

    @MainActor
    func testInactivePeekDoesNotPauseButBackgroundDoes() async throws {
        try XCTSkipUnless(
            UIApplication.shared.applicationState == .active,
            "the delegate reads UIKit's process state"
        )
        let matrix = MockMatrixClientService(syncStatus: .syncing)
        let delegate = SynaraAppDelegate()
        delegate.updateScenePhase(.active)
        delegate.bind(to: AppEnvironment.mock(matrix: matrix))
        XCTAssertEqual(matrix.foregroundActivity, [true])

        // Control Center down and back up.
        delegate.updateScenePhase(.inactive)
        delegate.updateScenePhase(.active)
        try await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertEqual(matrix.pauseCallCount, 0)
        XCTAssertEqual(matrix.resumeCallCount, 0)

        // Home: inactive, then background.
        delegate.updateScenePhase(.inactive)
        delegate.updateScenePhase(.background)
        try await waitUntil { matrix.pauseCallCount == 1 }
        XCTAssertEqual(matrix.foregroundActivity.last, false)
    }

    func testPathPolicyRestartsOnlyOnARouteChangeOrAReturn() {
        let wifi = ConnectionPathSnapshot(isSatisfied: true, primaryInterface: "wifi")
        let cellular = ConnectionPathSnapshot(isSatisfied: true, primaryInterface: "cellular", isExpensive: true)
        let lowData = ConnectionPathSnapshot(
            isSatisfied: true,
            primaryInterface: "wifi",
            isConstrained: true
        )
        let offline = ConnectionPathSnapshot(isSatisfied: false, primaryInterface: nil)

        // The first sample only describes the route the session started on.
        XCTAssertEqual(ConnectionPathPolicy.action(previous: nil, current: wifi, status: .connected), .none)
        XCTAssertEqual(ConnectionPathPolicy.action(previous: wifi, current: wifi, status: .connected), .none)
        XCTAssertEqual(
            ConnectionPathPolicy.action(previous: wifi, current: cellular, status: .connected),
            .restartSync
        )
        XCTAssertEqual(
            ConnectionPathPolicy.action(previous: wifi, current: lowData, status: .connected),
            .restartSync
        )
        XCTAssertEqual(
            ConnectionPathPolicy.action(previous: wifi, current: offline, status: .connected),
            .markReconnecting
        )
        XCTAssertEqual(
            ConnectionPathPolicy.action(previous: offline, current: wifi, status: .reconnecting),
            .restartSync
        )
        XCTAssertEqual(
            ConnectionPathPolicy.action(previous: offline, current: wifi, status: .disconnected),
            .restartSync
        )
        // Background and failed restores have nothing to restart.
        XCTAssertEqual(ConnectionPathPolicy.action(previous: wifi, current: cellular, status: .stopped), .none)
        XCTAssertEqual(
            ConnectionPathPolicy.action(previous: offline, current: wifi, status: .restoreFailed),
            .none
        )
    }

    func testPathUpdatesApplyOneAtATimeInDeliveryOrder() async throws {
        let recorder = PathRecorder()
        let sequencer = ConnectionPathSequencer { previous, current in
            await recorder.begin(previous: previous, current: current)
            // A slow restart must finish before the next update is judged.
            try? await Task.sleep(nanoseconds: 20_000_000)
            await recorder.end()
        }
        let updates = [
            ConnectionPathSnapshot(isSatisfied: true, primaryInterface: "wifi"),
            ConnectionPathSnapshot(isSatisfied: false, primaryInterface: nil),
            ConnectionPathSnapshot(isSatisfied: true, primaryInterface: "cellular"),
            ConnectionPathSnapshot(isSatisfied: true, primaryInterface: "cellular", isExpensive: true),
        ]
        for update in updates {
            sequencer.receive(update)
        }

        var applied = await recorder.applied
        var attempts = 0
        while applied.count < updates.count, attempts < 200 {
            try await Task.sleep(nanoseconds: 10_000_000)
            applied = await recorder.applied
            attempts += 1
        }
        sequencer.finish()
        let maxConcurrent = await recorder.maxConcurrent
        XCTAssertEqual(applied.map(\.current), updates)
        XCTAssertEqual(applied.map(\.previous), [nil] + updates.dropLast())
        XCTAssertEqual(maxConcurrent, 1)
    }

    func testReconnectRestartsSyncInPlaceAndBootstrapsOnlyWithoutOwners() async {
        var fullStarts = 0
        let restarted = await SharedCoreSyncReconnect.run(
            restartSync: { .running },
            fullStart: { fullStarts += 1 }
        )
        XCTAssertEqual(restarted, .restarted(.running))
        XCTAssertEqual(fullStarts, 0)

        let bootstrapped = await SharedCoreSyncReconnect.run(
            restartSync: {
                throw SyncStartError.Failed(
                    code: "p4-s12-sync-not-attached",
                    description: "Session owners are not attached."
                )
            },
            fullStart: { fullStarts += 1 }
        )
        XCTAssertEqual(bootstrapped, .fullStart)
        XCTAssertEqual(fullStarts, 1)
        XCTAssertEqual(
            ConnectionStatusCopy.fromReadiness(.running, previous: .reconnecting),
            .connected
        )
    }

    func testRoomListKeepsTheLastGoodListWhenAReadFails() throws {
        let rooms = RoomListState.loaded([])
        let suspended = RoomListSnapshotError.Failed(
            code: SharedCoreSessionSuspension.code,
            description: "The session is paused while the app is in the background."
        )
        let failed = RoomListSnapshotError.Failed(code: "p4-s4-snapshot-failed", description: "failed")
        let noSession = RoomListSnapshotError.Failed(
            code: "p2-room-list-snapshot-no-session",
            description: "none"
        )

        XCTAssertEqual(SharedCoreRoomListService.state(afterLoadError: suspended, lastGood: rooms), rooms)
        XCTAssertEqual(SharedCoreRoomListService.state(afterLoadError: failed, lastGood: rooms), rooms)
        XCTAssertEqual(SharedCoreRoomListService.state(afterLoadError: suspended, lastGood: nil), .loading)
        XCTAssertEqual(
            SharedCoreRoomListService.state(afterLoadError: failed, lastGood: nil),
            .failed(SharedCoreRoomListService.loadFailedMessage)
        )
        XCTAssertEqual(SharedCoreRoomListService.state(afterLoadError: noSession, lastGood: nil), .empty)
    }

    func testSuspendedSendStaysQueuedAsNotReady() {
        XCTAssertTrue(SharedCoreSendText.isNotReady(
            SendTextError.Failed(code: SharedCoreSessionSuspension.code, description: "paused")
        ))
        XCTAssertFalse(SharedCoreSendText.isNotReady(
            SendTextError.Failed(code: "p2-send-text-failed", description: "failed")
        ))
    }

    func testBackgroundNotificationSyncReportsResult() async throws {
        let matrix = MockMatrixClientService()
        matrix.backgroundSyncResult = true
        let session = try makeSession()

        let synced = await matrix.syncForBackgroundNotification(session: session)

        XCTAssertTrue(synced)
        XCTAssertEqual(matrix.backgroundSyncCallCount, 1)
    }

    func testNonTerminalVerificationUICannotBeInteractivelyDismissed() {
        let emojis = [CryptoVerificationEmoji(symbol: "🐶", description: "Dog")]
        XCTAssertFalse(CryptoVerificationPresentationPolicy.allowsInteractiveDismiss(.emojis(emojis)))
        XCTAssertFalse(CryptoVerificationPresentationPolicy.allowsInteractiveDismiss(.decimals([1, 2, 3])))
        XCTAssertFalse(CryptoVerificationPresentationPolicy.allowsInteractiveDismiss(.confirmed))
        XCTAssertFalse(CryptoVerificationPresentationPolicy.allowsInteractiveDismiss(.requestReceived(
            CryptoVerificationRequest(
                userID: "@alice:matrix.org",
                displayName: nil,
                deviceID: "OTHER",
                deviceDisplayName: nil,
                flowID: "flow"
            )
        )))
        XCTAssertFalse(CryptoVerificationPresentationPolicy.allowsInteractiveDismiss(.sasStarted))
        XCTAssertTrue(CryptoVerificationPresentationPolicy.allowsInteractiveDismiss(.finished))
        XCTAssertTrue(CryptoVerificationPresentationPolicy.allowsInteractiveDismiss(.cancelled))
        XCTAssertTrue(CryptoVerificationPresentationPolicy.allowsInteractiveDismiss(.mismatched))
        XCTAssertTrue(CryptoVerificationPresentationPolicy.allowsInteractiveDismiss(.failed))
    }

    @MainActor
    func testSecuritySettingsHidesVerifyButtonAfterFinishedTrustRefresh() async {
        let unverified = SessionCryptoStatus(
            verification: .unverified,
            recovery: .unknown,
            backup: .unknown,
            hasDevicesToVerifyAgainst: true,
            isLastDevice: false,
            unableToDecryptCount: 0
        )
        let verified = SessionCryptoStatus(
            verification: .verified,
            recovery: .unknown,
            backup: .unknown,
            hasDevicesToVerifyAgainst: true,
            isLastDevice: false,
            unableToDecryptCount: 0
        )
        XCTAssertTrue(SecuritySettingsVerificationPolicy.showsVerifyThisDevice(unverified))
        XCTAssertTrue(SecuritySettingsVerificationPolicy.enablesVerifyThisDevice(unverified))
        XCTAssertFalse(SecuritySettingsVerificationPolicy.showsVerifyThisDevice(verified))
        let unverifiedWithoutTrustedPeer = SessionCryptoStatus(
            verification: .unverified,
            recovery: .unknown,
            backup: .unknown,
            hasDevicesToVerifyAgainst: false,
            isLastDevice: false,
            unableToDecryptCount: 0
        )
        XCTAssertTrue(SecuritySettingsVerificationPolicy.showsVerifyThisDevice(unverifiedWithoutTrustedPeer))
        XCTAssertFalse(SecuritySettingsVerificationPolicy.enablesVerifyThisDevice(unverifiedWithoutTrustedPeer))
        let lastDevice = SessionCryptoStatus(
            verification: .unverified,
            recovery: .unknown,
            backup: .unknown,
            hasDevicesToVerifyAgainst: false,
            isLastDevice: true,
            unableToDecryptCount: 0
        )
        XCTAssertTrue(SecuritySettingsVerificationPolicy.showsVerifyThisDevice(lastDevice))
        XCTAssertFalse(SecuritySettingsVerificationPolicy.enablesVerifyThisDevice(lastDevice))

        let unknownAuthority = SessionCryptoStatus(
            verification: .unverified,
            recovery: .unknown,
            backup: .unknown,
            hasDevicesToVerifyAgainst: nil,
            isLastDevice: false,
            unableToDecryptCount: 0
        )
        XCTAssertTrue(SecuritySettingsVerificationPolicy.showsVerifyThisDevice(unknownAuthority))
        XCTAssertFalse(SecuritySettingsVerificationPolicy.enablesVerifyThisDevice(unknownAuthority))
        XCTAssertTrue(
            SecuritySettingsVerificationPolicy.availabilityMessage(unknownAuthority)
                .contains("could not check")
        )

        let crypto = SequencingCryptoStatusService(statuses: [unverified, verified])
        let observer = SessionCryptoStatusObserver()
        await observer.start(store: SessionCryptoStore(crypto: crypto))
        XCTAssertEqual(observer.status.verification, .verified)
        XCTAssertFalse(observer.showsVerifyThisDevice)
    }

    @MainActor
    func testSessionCryptoStatusRefreshesDeviceAuthorityWithoutVerificationFlow() async {
        let eligible = SessionCryptoStatus(
            verification: .unverified,
            recovery: .unknown,
            backup: .unknown,
            hasDevicesToVerifyAgainst: true,
            isLastDevice: false,
            unableToDecryptCount: 0
        )
        let crypto = SequencingCryptoStatusService(statuses: [.unknown, eligible])
        let observer = SessionCryptoStatusObserver()

        await observer.start(store: SessionCryptoStore(crypto: crypto))

        XCTAssertEqual(observer.status, eligible)
        XCTAssertTrue(observer.enablesVerifyThisDevice)
    }

    @MainActor
    func testSessionDeviceBurstDuringInitialReadRetainsOneRefresh() async throws {
        let eligible = SessionCryptoStatus(
            verification: .unverified,
            recovery: .unknown,
            backup: .unknown,
            hasDevicesToVerifyAgainst: true,
            isLastDevice: false,
            unableToDecryptCount: 0
        )
        let crypto = SequencingCryptoStatusService(statuses: [.unknown, eligible], suspendInitialRead: true)
        let observer = SessionCryptoStatusObserver()
        let observing = Task { await observer.start(store: SessionCryptoStore(crypto: crypto)) }
        try await waitUntil { crypto.initialReadIsSuspended }
        // Exercise the production forwarding boundary while the initial
        // authority read remains in flight. The inbox is deliberately empty.
        crypto.sendDeviceBurstAndFinish(count: 200)
        await crypto.waitForForwardingToFinish()
        crypto.releaseInitialRead()
        await observing.value
        XCTAssertEqual(crypto.readCount, 2, "one initial read and one coalesced wakeup")
        XCTAssertEqual(observer.status, eligible)
        XCTAssertTrue(observer.enablesVerifyThisDevice)
    }

    private func makeSession() throws -> AuthenticatedSession {
        AuthenticatedSession(
            userID: "@alice:matrix.org",
            deviceID: "DEVICE",
            homeserverURL: try XCTUnwrap(URL(string: "https://matrix.org")),
            accessToken: "token"
        )
    }
}

private actor PathRecorder {
    struct Applied: Equatable {
        var previous: ConnectionPathSnapshot?
        var current: ConnectionPathSnapshot
    }

    private(set) var applied: [Applied] = []
    private(set) var maxConcurrent = 0
    private var running = 0

    func begin(previous: ConnectionPathSnapshot?, current: ConnectionPathSnapshot) {
        running += 1
        maxConcurrent = max(maxConcurrent, running)
        applied.append(Applied(previous: previous, current: current))
    }

    func end() {
        running -= 1
    }
}

@MainActor
private final class MockBackgroundTaskManager: SynaraBackgroundTaskManaging {
    let issuedIdentifier = UIBackgroundTaskIdentifier(rawValue: 42)
    private(set) var beginNames: [String] = []
    private(set) var endedIdentifiers: [UIBackgroundTaskIdentifier] = []
    private var expirationHandler: (@Sendable () -> Void)?

    func beginBackgroundTask(
        withName taskName: String?,
        expirationHandler handler: (@Sendable () -> Void)?
    ) -> UIBackgroundTaskIdentifier {
        beginNames.append(taskName ?? "")
        expirationHandler = handler
        return issuedIdentifier
    }

    func endBackgroundTask(_ identifier: UIBackgroundTaskIdentifier) {
        endedIdentifiers.append(identifier)
    }

    func expire() {
        expirationHandler?()
    }
}

@MainActor
private func waitUntil(
    timeoutNanoseconds: UInt64 = 1_000_000_000,
    condition: @escaping () -> Bool
) async throws {
    let started = DispatchTime.now().uptimeNanoseconds
    while condition() == false {
        if DispatchTime.now().uptimeNanoseconds - started >= timeoutNanoseconds {
            XCTFail("Timed out waiting for lifecycle transition")
            return
        }
        try await Task.sleep(nanoseconds: 5_000_000)
    }
}

private final class SequencingCryptoStatusService: CryptoStatusServicing {
    private let lock = NSLock()
    private var remaining: [SessionCryptoStatus]
    private let suspendInitialRead: Bool
    private var initialRead: CheckedContinuation<Void, Never>?
    private(set) var readCount = 0
    private var burstRemaining: Int?
    private var forwardingFinished = false

    init(statuses: [SessionCryptoStatus], suspendInitialRead: Bool = false) {
        remaining = statuses
        self.suspendInitialRead = suspendInitialRead
    }

    var initialReadIsSuspended: Bool {
        lock.lock()
        defer { lock.unlock() }
        return initialRead != nil
    }

    func releaseInitialRead() {
        lock.lock()
        let continuation = initialRead
        initialRead = nil
        lock.unlock()
        continuation?.resume()
    }

    func sendDeviceBurstAndFinish(count: Int) {
        lock.lock()
        burstRemaining = count
        lock.unlock()
    }

    func waitForForwardingToFinish() async {
        while true {
            lock.lock()
            let finished = forwardingFinished
            lock.unlock()
            if finished { return }
            await Task.yield()
        }
    }

    func roomStatus(roomID: String) async -> RoomCryptoStatus {
        .unknown
    }

    func sessionStatus() async -> SessionCryptoStatus {
        lock.lock()
        readCount += 1
        let shouldSuspend = suspendInitialRead && readCount == 1
        lock.unlock()
        if shouldSuspend {
            await withCheckedContinuation { continuation in
                lock.lock()
                initialRead = continuation
                lock.unlock()
            }
        }
        lock.lock()
        defer { lock.unlock() }
        if remaining.count > 1 {
            return remaining.removeFirst()
        }
        return remaining.first ?? .unknown
    }

    func verificationUpdates() -> AsyncStream<CryptoVerificationSnapshot?> {
        // No active or completed verification request exists in this fixture.
        AsyncStream { $0.finish() }
    }

    func sessionDeviceUpdates() -> AsyncStream<Void> {
        guard suspendInitialRead else {
            return AsyncStream { continuation in
                continuation.yield(())
                continuation.finish()
            }
        }
        let updates = AsyncStream<Void>(unfolding: { [self] in
            while true {
                lock.lock()
                if let remaining = burstRemaining {
                    if remaining == 0 {
                        // The production forwarder has already yielded the
                        // final signal before requesting this next element.
                        forwardingFinished = true
                        lock.unlock()
                        return nil
                    }
                    burstRemaining = remaining - 1
                    lock.unlock()
                    return ()
                }
                lock.unlock()
                await Task.yield()
            }
        })
        return SharedCoreSessionDeviceInvalidations.stream(updates)
    }

    func retryDecryption(roomID: String) async -> CryptoActionResult {
        .unavailable("unused")
    }

    func requestDeviceVerification(deviceId: String?) async -> CryptoActionResult {
        _ = deviceId
        return .unavailable("unused")
    }

    func acceptVerificationRequest() async -> CryptoActionResult {
        .unavailable("unused")
    }

    func startSasVerification() async -> CryptoActionResult {
        .unavailable("unused")
    }

    func approveVerification() async -> CryptoActionResult {
        .unavailable("unused")
    }

    func declineVerification() async -> CryptoActionResult {
        .unavailable("unused")
    }

    func cancelVerification() async -> CryptoActionResult {
        .unavailable("unused")
    }

    func recover(recoveryKey: String) async -> CryptoActionResult {
        .unavailable("unused")
    }
}

final class SignOutCopyTests: XCTestCase {
    func testOnlyAConfirmedLastDeviceShowsTheRecoveryKeyWarning() {
        XCTAssertTrue(SignOutCopy.message(isLastDevice: true).hasPrefix(SignOutCopy.lastDeviceWarning))
        XCTAssertTrue(SignOutCopy.message(isLastDevice: true).contains(SignOutCopy.standardMessage))
        XCTAssertEqual(SignOutCopy.message(isLastDevice: false), SignOutCopy.standardMessage)
        XCTAssertEqual(SignOutCopy.message(isLastDevice: nil), SignOutCopy.standardMessage)
        XCTAssertTrue(SignOutCopy.lastDeviceWarning.contains("recovery key"))
    }

    func testDehydratedDevicesAreNotOtherSignedInDevices() {
        XCTAssertTrue(SignOutCopy.isOtherSignedInDevice(isCurrent: false, trust: .unverified))
        XCTAssertTrue(SignOutCopy.isOtherSignedInDevice(isCurrent: false, trust: .verified))
        XCTAssertFalse(SignOutCopy.isOtherSignedInDevice(isCurrent: false, trust: .dehydrated))
        XCTAssertFalse(SignOutCopy.isOtherSignedInDevice(isCurrent: true, trust: .verified))
    }
}
