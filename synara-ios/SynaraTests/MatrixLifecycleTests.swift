import XCTest
@testable import Synara

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

    func testBackgroundNotificationSyncReportsResult() async throws {
        let matrix = MockMatrixClientService()
        matrix.backgroundSyncResult = true
        let session = try makeSession()

        let synced = await matrix.syncForBackgroundNotification(session: session)

        XCTAssertTrue(synced)
        XCTAssertEqual(matrix.backgroundSyncCallCount, 1)
    }

    func testVerificationStateSurvivesBackgroundPauseAndForegroundResume() throws {
        var reducer = MatrixVerificationStateReducer()
        let request = CryptoVerificationRequest(
            userID: "@alice:matrix.org",
            displayName: "Alice",
            deviceID: "OTHER",
            deviceDisplayName: "Other device",
            flowID: "flow"
        )
        reducer.reduce(.requestReceived(request), source: .delegate)
        reducer.reduce(.accepted, source: .delegate)

        if MatrixVerificationLifecyclePolicy.shouldReset(for: .backgroundPause) {
            reducer.reset()
        }
        if MatrixVerificationLifecyclePolicy.shouldReset(for: .foregroundResume) {
            reducer.reset()
        }

        XCTAssertEqual(reducer.state, .accepted)
        XCTAssertTrue(MatrixVerificationLifecyclePolicy.shouldReset(for: .sessionReplaced))
        XCTAssertTrue(MatrixVerificationLifecyclePolicy.shouldReset(for: .localStateReset))
    }

    func testVerificationReducerIgnoresDuplicateAndOutOfOrderCallbacks() {
        var reducer = MatrixVerificationStateReducer()
        let emojis = [CryptoVerificationEmoji(symbol: "🐶", description: "Dog")]
        let request = CryptoVerificationRequest(
            userID: "@alice:matrix.org",
            displayName: "Alice",
            deviceID: "OTHER",
            deviceDisplayName: "Other device",
            flowID: "flow"
        )

        XCTAssertEqual(reducer.reduce(.requestReceived(request), source: .delegate), .requestReceived(request))
        XCTAssertEqual(reducer.reduce(.accepted, source: .delegate), .accepted)
        XCTAssertEqual(reducer.reduce(.sasStarted, source: .delegate), .sasStarted)
        XCTAssertEqual(reducer.reduce(.emojis(emojis), source: .delegate), .emojis(emojis))

        XCTAssertNil(reducer.reduce(.requestReceived(request), source: .delegate))
        XCTAssertNil(reducer.reduce(.accepted, source: .delegate))
        XCTAssertNil(reducer.reduce(.sasStarted, source: .delegate))
        XCTAssertNil(reducer.reduce(.emojis(emojis), source: .delegate))
        XCTAssertEqual(reducer.state, .emojis(emojis))
    }

    func testOnlyDelegateDidFinishCanCompleteVerification() {
        var reducer = MatrixVerificationStateReducer()
        reducer.reduce(.requestSent, source: .localRequest)
        reducer.reduce(.sasStarted, source: .delegate)

        XCTAssertNil(reducer.reduce(.finished, source: .localRequest))
        XCTAssertEqual(reducer.state, .sasStarted)
        XCTAssertEqual(reducer.reduce(.finished, source: .delegate), .finished)
        XCTAssertNil(reducer.reduce(.failed, source: .delegate))
        XCTAssertEqual(reducer.state, .finished)
    }

    func testVerificationReducerKeepsOneActiveFlowBecauseCallbacksLackFlowIDs() {
        var reducer = MatrixVerificationStateReducer()
        let first = CryptoVerificationRequest(
            userID: "@alice:matrix.org",
            displayName: "Alice",
            deviceID: "FIRST",
            deviceDisplayName: "First device",
            flowID: "first-flow"
        )
        let second = CryptoVerificationRequest(
            userID: "@alice:matrix.org",
            displayName: "Alice",
            deviceID: "SECOND",
            deviceDisplayName: "Second device",
            flowID: "second-flow"
        )

        XCTAssertEqual(reducer.reduce(.requestReceived(first), source: .delegate), .requestReceived(first))
        XCTAssertNil(reducer.reduce(.requestReceived(second), source: .delegate))
        XCTAssertEqual(reducer.reduce(.accepted, source: .delegate), .accepted)
        XCTAssertEqual(reducer.state, .accepted)
    }

    func testVerificationReducerDeduplicatesCompletedFlowUntilLifecycleReset() {
        var reducer = MatrixVerificationStateReducer()
        let completed = CryptoVerificationRequest(
            userID: "@alice:matrix.org",
            displayName: "Alice",
            deviceID: "FIRST",
            deviceDisplayName: "First device",
            flowID: "completed-flow"
        )
        let next = CryptoVerificationRequest(
            userID: "@alice:matrix.org",
            displayName: "Alice",
            deviceID: "SECOND",
            deviceDisplayName: "Second device",
            flowID: "next-flow"
        )

        reducer.reduce(.requestReceived(completed), source: .delegate)
        XCTAssertEqual(reducer.reduce(.finished, source: .delegate), .finished)
        XCTAssertNil(reducer.reduce(.requestReceived(completed), source: .delegate))
        XCTAssertEqual(reducer.reduce(.requestReceived(next), source: .delegate), .requestReceived(next))

        reducer.reset()
        XCTAssertEqual(
            reducer.reduce(.requestReceived(completed), source: .delegate),
            .requestReceived(completed)
        )
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

    func testClearedNonTerminalVerificationStateIsRestoredFromInbox() {
        let emojis = CryptoVerificationState.emojis([
            CryptoVerificationEmoji(symbol: "🐶", description: "Dog")
        ])
        XCTAssertEqual(
            CryptoVerificationPresentationPolicy.restoredStateIfCleared(presented: nil, latest: emojis),
            emojis
        )
        XCTAssertEqual(
            CryptoVerificationPresentationPolicy.restoredStateIfCleared(presented: emojis, latest: .requestSent),
            emojis
        )
        XCTAssertNil(
            CryptoVerificationPresentationPolicy.restoredStateIfCleared(presented: nil, latest: .finished)
        )
        XCTAssertNil(
            CryptoVerificationPresentationPolicy.restoredStateIfCleared(presented: nil, latest: nil)
        )
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
        XCTAssertFalse(SecuritySettingsVerificationPolicy.showsVerifyThisDevice(verified))

        let crypto = SequencingCryptoStatusService(statuses: [unverified, verified])
        let observer = SessionCryptoStatusObserver()
        await observer.start(crypto: crypto)
        XCTAssertEqual(observer.status.verification, .verified)
        XCTAssertFalse(observer.showsVerifyThisDevice)
    }

    func testVerificationContinuationCancellationBeforeRegistrationUsesTombstone() {
        var tracker = MatrixVerificationContinuationRegistrationTracker()
        let id = UUID()

        tracker.cancel(id: id)

        XCTAssertFalse(tracker.register(id: id, isTaskCancelled: false))
        XCTAssertFalse(tracker.isRegistered(id: id))
    }

    func testVerificationContinuationCancellationAfterRegistrationRemovesIt() {
        var tracker = MatrixVerificationContinuationRegistrationTracker()
        let id = UUID()

        XCTAssertTrue(tracker.register(id: id, isTaskCancelled: false))
        XCTAssertTrue(tracker.isRegistered(id: id))
        tracker.cancel(id: id)

        XCTAssertFalse(tracker.isRegistered(id: id))
        XCTAssertFalse(tracker.register(id: UUID(), isTaskCancelled: true))
    }

    func testTimelineStreamInvalidatesWhenSyncGenerationChanges() {
        XCTAssertFalse(
            MatrixTimelineStreamLifecycle.shouldInvalidate(
                expectedGeneration: 4,
                currentGeneration: 4,
                isPaused: false
            )
        )
        XCTAssertTrue(
            MatrixTimelineStreamLifecycle.shouldInvalidate(
                expectedGeneration: 4,
                currentGeneration: 5,
                isPaused: false
            )
        )
    }

    func testTimelineStreamInvalidatesImmediatelyInBackground() {
        XCTAssertTrue(
            MatrixTimelineStreamLifecycle.shouldInvalidate(
                expectedGeneration: 4,
                currentGeneration: 4,
                isPaused: true
            )
        )
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

private final class SequencingCryptoStatusService: CryptoStatusServicing {
    private let lock = NSLock()
    private var remaining: [SessionCryptoStatus]

    init(statuses: [SessionCryptoStatus]) {
        remaining = statuses
    }

    func roomStatus(roomID: String) async -> RoomCryptoStatus {
        .unknown
    }

    func sessionStatus() async -> SessionCryptoStatus {
        lock.lock()
        defer { lock.unlock() }
        if remaining.count > 1 {
            return remaining.removeFirst()
        }
        return remaining.first ?? .unknown
    }

    func verificationUpdates() -> AsyncStream<CryptoVerificationState> {
        AsyncStream { continuation in
            continuation.yield(.finished)
            continuation.finish()
        }
    }

    func retryDecryption(roomID: String) async -> CryptoActionResult {
        .unavailable("unused")
    }

    func requestDeviceVerification() async -> CryptoActionResult {
        .unavailable("unused")
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
