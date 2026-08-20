import XCTest
@testable import Synara

final class SharedCoreSessionBootstrapTests: XCTestCase {
    func testLoginPathSkipsRestoreThenAttachAndStart() async throws {
        let engine = MockLiveSessionEngine(
            restoreResult: .failure(.failed(code: SharedCoreSessionBootstrap.alreadyRestoredCode)),
            attachResult: .success(()),
            startResults: [.success(.init(started: true, readiness: "running"))]
        )

        let outcome = await prepare(engine: engine)

        XCTAssertEqual(engine.calls, [.restore, .attach, .start])
        XCTAssertFalse(outcome.restored)
        XCTAssertTrue(outcome.skippedRestore)
        XCTAssertTrue(outcome.attached)
        XCTAssertTrue(outcome.started)
        XCTAssertEqual(outcome.readiness, "running")
        XCTAssertNil(outcome.failure)
        XCTAssertTrue(outcome.hasLiveClient)
        assertPrivacySafe(outcome)
    }

    func testRestorePathRestoreAttachStart() async throws {
        let engine = MockLiveSessionEngine(
            restoreResult: .success(()),
            attachResult: .success(()),
            startResults: [.success(.init(started: true, readiness: "running"))]
        )

        let outcome = await prepare(engine: engine)

        XCTAssertEqual(engine.calls, [.restore, .attach, .start])
        XCTAssertTrue(outcome.restored)
        XCTAssertFalse(outcome.skippedRestore)
        XCTAssertTrue(outcome.attached)
        XCTAssertTrue(outcome.started)
        XCTAssertEqual(outcome.readiness, "running")
        XCTAssertNil(outcome.failure)
        assertPrivacySafe(outcome)
    }

    func testIdleAfterStartIsNotLiveAndFailsClosed() async throws {
        let engine = MockLiveSessionEngine(
            restoreResult: .success(()),
            attachResult: .success(()),
            startResults: [
                .success(.init(started: false, readiness: "idle")),
                .success(.init(started: false, readiness: "idle"))
            ],
            observeResults: [
                .success(.init(started: false, readiness: "idle")),
                .success(.init(started: false, readiness: "idle"))
            ]
        )

        let outcome = await prepare(engine: engine, observeAttempts: 2)

        XCTAssertEqual(engine.calls, [.restore, .attach, .start, .start, .observe, .observe])
        XCTAssertTrue(outcome.restored)
        XCTAssertTrue(outcome.attached)
        XCTAssertFalse(outcome.started)
        XCTAssertEqual(outcome.readiness, "idle")
        XCTAssertEqual(outcome.failure, .startFailed)
        XCTAssertEqual(outcome.failure?.syncStatus, .disconnected)
        assertPrivacySafe(outcome)
    }

    func testIdleThenRunningViaObserveIsLive() async throws {
        let engine = MockLiveSessionEngine(
            restoreResult: .success(()),
            attachResult: .success(()),
            startResults: [.success(.init(started: false, readiness: "idle"))],
            observeResults: [.success(.init(started: false, readiness: "running"))]
        )

        let outcome = await prepare(engine: engine, observeAttempts: 2)

        XCTAssertEqual(engine.calls, [.restore, .attach, .start, .start, .observe])
        XCTAssertTrue(outcome.started)
        XCTAssertEqual(outcome.readiness, "running")
        XCTAssertNil(outcome.failure)
        assertPrivacySafe(outcome)
    }

    func testRestoreFailureDoesNotAttachOrStart() async throws {
        let engine = MockLiveSessionEngine(
            restoreResult: .failure(.failed(code: "p4-s3b-restore-failed")),
            attachResult: .success(()),
            startResults: [.success(.init(started: true, readiness: "running"))]
        )

        let outcome = await prepare(engine: engine)

        XCTAssertEqual(engine.calls, [.restore])
        XCTAssertFalse(outcome.restored)
        XCTAssertFalse(outcome.skippedRestore)
        XCTAssertFalse(outcome.attached)
        XCTAssertFalse(outcome.started)
        XCTAssertEqual(outcome.failure, .restoreFailed)
        XCTAssertEqual(outcome.failure?.syncStatus, .restoreFailed)
        XCTAssertFalse(ConnectionStatusCopy.showsRetryAction(.restoreFailed))
        XCTAssertTrue(ConnectionStatusCopy.showsSignOutAction(.restoreFailed))
        assertPrivacySafe(outcome)
    }

    func testMaterialMissingRestoreFailsClosedWithoutStart() async throws {
        let engine = MockLiveSessionEngine(
            restoreResult: .failure(.failed(code: "p4-s3b-session-material-missing")),
            attachResult: .success(()),
            startResults: [.success(.init(started: true, readiness: "running"))]
        )

        let outcome = await prepare(engine: engine)

        XCTAssertEqual(engine.calls, [.restore])
        XCTAssertEqual(outcome.failure, .restoreFailed)
        XCTAssertFalse(outcome.started)
        assertPrivacySafe(outcome)
    }

    func testAttachFailureAfterRestoreDoesNotStartAndIsRetryable() async throws {
        let engine = MockLiveSessionEngine(
            restoreResult: .success(()),
            attachResult: .failure(.failed(code: "p4-s3d-attach-failed")),
            startResults: [.success(.init(started: true, readiness: "running"))]
        )

        let outcome = await prepare(engine: engine)

        XCTAssertEqual(engine.calls, [.restore, .attach])
        XCTAssertTrue(outcome.restored)
        XCTAssertFalse(outcome.attached)
        XCTAssertFalse(outcome.started)
        XCTAssertEqual(outcome.failure, .attachFailed)
        XCTAssertEqual(outcome.failure?.syncStatus, .disconnected)
        XCTAssertTrue(ConnectionStatusCopy.showsRetryAction(.disconnected))
        XCTAssertTrue(ConnectionStatusCopy.showsSignOutAction(.disconnected))
        assertPrivacySafe(outcome)
    }

    func testAlreadyAttachedIsSkippedThenStartRuns() async throws {
        let engine = MockLiveSessionEngine(
            restoreResult: .failure(.failed(code: SharedCoreSessionBootstrap.alreadyRestoredCode)),
            attachResult: .failure(.failed(code: SharedCoreSessionBootstrap.alreadyAttachedCode)),
            startResults: [.success(.init(started: true, readiness: "running"))]
        )

        let outcome = await prepare(engine: engine)

        XCTAssertEqual(engine.calls, [.restore, .attach, .start])
        XCTAssertTrue(outcome.skippedRestore)
        XCTAssertTrue(outcome.skippedAttach)
        XCTAssertTrue(outcome.started)
        XCTAssertNil(outcome.failure)
        assertPrivacySafe(outcome)
    }

    func testStartFailureAfterRestoreAndAttachFailsClosed() async throws {
        let engine = MockLiveSessionEngine(
            restoreResult: .success(()),
            attachResult: .success(()),
            startResults: [
                .failure(.failed(code: "p4-s12-sync-start-failed")),
                .failure(.failed(code: "p4-s12-sync-start-failed"))
            ]
        )

        let outcome = await prepare(engine: engine)

        XCTAssertEqual(engine.calls, [.restore, .attach, .start, .start])
        XCTAssertTrue(outcome.restored)
        XCTAssertTrue(outcome.attached)
        XCTAssertFalse(outcome.started)
        XCTAssertEqual(outcome.failure, .startFailed)
        XCTAssertEqual(outcome.failure?.syncStatus, .disconnected)
        assertPrivacySafe(outcome)
    }

    func testStartRetriesOnceWhenFirstAttemptIsNotLive() async throws {
        let engine = MockLiveSessionEngine(
            restoreResult: .success(()),
            attachResult: .success(()),
            startResults: [
                .success(.init(started: false, readiness: "failed")),
                .success(.init(started: true, readiness: "running"))
            ]
        )

        let outcome = await prepare(engine: engine)

        XCTAssertEqual(engine.calls, [.restore, .attach, .start, .start])
        XCTAssertTrue(outcome.started)
        XCTAssertEqual(outcome.readiness, "running")
        XCTAssertNil(outcome.failure)
        assertPrivacySafe(outcome)
    }

    func testGenericRestoreFailureIsNotTreatedAsAlreadyLive() async throws {
        let engine = MockLiveSessionEngine(
            restoreResult: .failure(.failed(code: "p4-s3b-restore-failed")),
            attachResult: .success(()),
            startResults: [.success(.init(started: true, readiness: "running"))]
        )

        let outcome = await SharedCoreSessionBootstrap.prepareLiveSession(
            userID: "@alice:example.org",
            homeserverURL: "https://user:secret@evil.example/?password=hunter2",
            storeRoot: FileManager.default.temporaryDirectory,
            engine: engine,
            observeAttempts: 0,
            observeDelayNanoseconds: 0
        )

        XCTAssertEqual(engine.calls, [.restore])
        XCTAssertEqual(outcome.failure, .restoreFailed)
        let publicError = String(describing: outcome)
        for forbidden in ["password", "syt_", "token", "secret", "hunter2", "@alice:example.org"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    private func prepare(
        engine: MockLiveSessionEngine,
        observeAttempts: Int = 0
    ) async -> SharedCoreSessionBootstrap.Outcome {
        await SharedCoreSessionBootstrap.prepareLiveSession(
            userID: "@alice:example.org",
            homeserverURL: "https://matrix.example.org",
            storeRoot: FileManager.default.temporaryDirectory,
            engine: engine,
            observeAttempts: observeAttempts,
            observeDelayNanoseconds: 0
        )
    }

    private func assertPrivacySafe(_ outcome: SharedCoreSessionBootstrap.Outcome) {
        let publicError = String(describing: outcome)
        for forbidden in ["password", "syt_", "token", "https://"] {
            XCTAssertFalse(publicError.contains(forbidden), "leaked \(forbidden)")
        }
    }
}

private final class MockLiveSessionEngine: LiveSessionEngine, @unchecked Sendable {
    enum Call: Equatable {
        case restore
        case attach
        case start
        case observe
    }

    var restoreResult: Result<Void, SharedCoreSessionBootstrap.StepError>
    var attachResult: Result<Void, SharedCoreSessionBootstrap.StepError>
    var startResults: [Result<SharedCoreSessionBootstrap.StartResult, SharedCoreSessionBootstrap.StepError>]
    var observeResults: [Result<SharedCoreSessionBootstrap.StartResult, SharedCoreSessionBootstrap.StepError>]
    private(set) var calls: [Call] = []

    init(
        restoreResult: Result<Void, SharedCoreSessionBootstrap.StepError>,
        attachResult: Result<Void, SharedCoreSessionBootstrap.StepError>,
        startResults: [Result<SharedCoreSessionBootstrap.StartResult, SharedCoreSessionBootstrap.StepError>],
        observeResults: [Result<SharedCoreSessionBootstrap.StartResult, SharedCoreSessionBootstrap.StepError>] = []
    ) {
        self.restoreResult = restoreResult
        self.attachResult = attachResult
        self.startResults = startResults
        self.observeResults = observeResults
    }

    func restorePersistedSession(
        userID: String,
        homeserverURL: String,
        storeRoot: URL
    ) async throws {
        _ = userID
        _ = homeserverURL
        _ = storeRoot
        calls.append(.restore)
        try restoreResult.get()
    }

    func attachSessionOwners() async throws {
        calls.append(.attach)
        try attachResult.get()
    }

    func startSync() async throws -> SharedCoreSessionBootstrap.StartResult {
        calls.append(.start)
        guard startResults.isEmpty == false else {
            throw SharedCoreSessionBootstrap.StepError.failed(code: "p4-s12-sync-start-failed")
        }
        return try startResults.removeFirst().get()
    }

    func observeSync() async throws -> SharedCoreSessionBootstrap.StartResult {
        calls.append(.observe)
        guard observeResults.isEmpty == false else {
            return SharedCoreSessionBootstrap.StartResult(started: false, readiness: "idle")
        }
        return try observeResults.removeFirst().get()
    }
}
