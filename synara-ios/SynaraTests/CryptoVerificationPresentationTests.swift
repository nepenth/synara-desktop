import XCTest
@testable import Synara

@MainActor
final class CryptoVerificationPresentationTests: XCTestCase {
    private func snapshot(_ flow: String = "first", state: CryptoVerificationState = .finished, generation: UInt64 = 1) -> CryptoVerificationSnapshot {
        .init(id: .init(sessionGeneration: generation, flowID: flow), state: state)
    }

    func testDoneAcknowledgesExactFlowAndRejectsBufferedTerminalReplay() async {
        let presentation = CryptoVerificationPresentation()
        let crypto = DismissalCryptoFixture()
        let done = snapshot()
        presentation.receive(done)
        await presentation.dismiss(using: crypto)
        XCTAssertEqual(crypto.dismissed, ["first"])
        XCTAssertNil(presentation.snapshot)
        for _ in 0..<5 { presentation.receive(done) }
        XCTAssertNil(presentation.snapshot, "Already-buffered done updates must not reopen the sheet")
        presentation.receive(snapshot("next"))
        XCTAssertEqual(presentation.snapshot?.id.flowID, "next", "A new flow with the same phase still presents")
    }

    func testDismissalFailureKeepsResultAndAllowsExplicitRetry() async {
        let presentation = CryptoVerificationPresentation()
        let crypto = DismissalCryptoFixture()
        crypto.result = .failed("Could not close")
        presentation.receive(snapshot())
        await presentation.dismiss(using: crypto)
        XCTAssertEqual(presentation.snapshot, snapshot())
        XCTAssertEqual(presentation.error, "Could not close")
        crypto.result = .completed("Closed")
        await presentation.dismiss(using: crypto)
        XCTAssertNil(presentation.snapshot)
        XCTAssertNil(presentation.error)
    }

    func testInFlightDismissalIgnoresReplayAndDoesNotClearNextFlow() async {
        let presentation = CryptoVerificationPresentation()
        let crypto = DismissalCryptoFixture()
        crypto.suspend = true
        presentation.receive(snapshot())
        let closing = Task { await presentation.dismiss(using: crypto) }
        while crypto.completion == nil { await Task.yield() }
        presentation.receive(nil)
        presentation.receive(snapshot())
        XCTAssertEqual(presentation.snapshot, snapshot())
        await presentation.dismiss(using: crypto)
        XCTAssertEqual(crypto.dismissed.count, 1, "Repeated Done/swipe must not dispatch another dismissal")
        presentation.receive(snapshot("next", state: .requestSent))
        crypto.completion?.resume()
        await closing.value
        XCTAssertEqual(presentation.snapshot?.id.flowID, "next")
        presentation.receive(snapshot())
        XCTAssertEqual(presentation.snapshot?.id.flowID, "next")
    }

    func testSessionGenerationScopesAcknowledgementAndRejectsOldSessionUpdates() async {
        let presentation = CryptoVerificationPresentation()
        presentation.receive(snapshot())
        await presentation.dismiss(using: DismissalCryptoFixture())
        presentation.receive(snapshot(generation: 2))
        XCTAssertEqual(presentation.snapshot, snapshot(generation: 2))
        presentation.receive(snapshot("old", generation: 1))
        XCTAssertEqual(presentation.snapshot, snapshot(generation: 2))
    }

    func testActiveComparisonCannotBeAcknowledged() async {
        let presentation = CryptoVerificationPresentation()
        let crypto = DismissalCryptoFixture()
        presentation.receive(snapshot(state: .confirmed))
        await presentation.dismiss(using: crypto)
        XCTAssertTrue(crypto.dismissed.isEmpty)
        XCTAssertEqual(presentation.snapshot?.state, .confirmed)
    }

    func testResetRejectsCompletionFromPreviousPresentationLifecycle() async {
        let presentation = CryptoVerificationPresentation()
        let crypto = DismissalCryptoFixture()
        crypto.suspend = true
        crypto.result = .failed("Old failure")
        presentation.receive(snapshot())
        let closing = Task { await presentation.dismiss(using: crypto) }
        while crypto.completion == nil { await Task.yield() }
        presentation.reset()
        presentation.receive(snapshot("new", state: .requestSent, generation: 2))
        crypto.completion?.resume()
        await closing.value
        XCTAssertEqual(presentation.snapshot?.id.flowID, "new")
        XCTAssertNil(presentation.error)
    }
}

private final class DismissalCryptoFixture: CryptoStatusServicing {
    var dismissed: [String] = []
    var result: CryptoActionResult = .completed("Closed")
    var suspend = false
    var completion: CheckedContinuation<Void, Never>?

    func dismissVerification(flowID: String) async -> CryptoActionResult {
        dismissed.append(flowID)
        if suspend { await withCheckedContinuation { completion = $0 } }
        return result
    }
    func verificationUpdates() -> AsyncStream<CryptoVerificationSnapshot?> { AsyncStream { $0.finish() } }
    func roomStatus(roomID: String) async -> RoomCryptoStatus { .unknown }
    func sessionStatus() async -> SessionCryptoStatus { .unknown }
    func retryDecryption(roomID: String) async -> CryptoActionResult { result }
    func requestDeviceVerification(deviceId: String?) async -> CryptoActionResult { result }
    func acceptVerificationRequest() async -> CryptoActionResult { result }
    func startSasVerification() async -> CryptoActionResult { result }
    func approveVerification() async -> CryptoActionResult { result }
    func declineVerification() async -> CryptoActionResult { result }
    func cancelVerification() async -> CryptoActionResult { result }
    func recover(recoveryKey: String) async -> CryptoActionResult { result }
}
