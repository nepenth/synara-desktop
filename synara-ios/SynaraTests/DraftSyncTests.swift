import SynaraCore
import XCTest
@testable import Synara

/// Records calls and answers `resolve` with a scripted decision.
private final class RecordingDraftSync: DraftSyncServicing, @unchecked Sendable {
    struct Save: Equatable {
        let roomID: String
        let slot: String
        let markdown: String
        let replyToEventID: String?
        let flush: Bool
    }

    private let lock = NSLock()
    private var _saves: [Save] = []
    private var _clears: [String] = []
    private var _resolves: [LocalDraftState] = []
    var decision: DraftSyncDecision = .keep
    var stamp: UInt64 = 100

    var saves: [Save] { lock.withLock { _saves } }
    var clears: [String] { lock.withLock { _clears } }
    var resolves: [LocalDraftState] { lock.withLock { _resolves } }

    func setEnabled(_ enabled: Bool) async {}

    func save(roomID: String, slot: String, markdown: String, replyToEventID: String?, flush: Bool) async -> UInt64? {
        lock.withLock {
            _saves.append(Save(roomID: roomID, slot: slot, markdown: markdown, replyToEventID: replyToEventID, flush: flush))
        }
        return stamp
    }

    func clear(roomID: String, slot: String) async -> UInt64? {
        lock.withLock { _clears.append(roomID) }
        return stamp
    }

    func resolve(roomID: String, slot: String, local: LocalDraftState, fetchFresh: Bool) async -> DraftSyncDecision {
        lock.withLock { _resolves.append(local) }
        return decision
    }

    func flush() async {}

    func updates() -> AsyncStream<String?> {
        AsyncStream { $0.finish() }
    }
}

private func remote(_ markdown: String, cleared: Bool = false) -> RemoteDraft {
    RemoteDraft(
        roomID: "!room:example.org",
        slot: DraftSyncSlot.main,
        updatedAt: 500,
        deviceID: "DESKTOP",
        cleared: cleared,
        markdown: markdown,
        replyToEventID: nil,
        truncated: false
    )
}

@MainActor
final class DraftSyncTests: XCTestCase {
    private func settle() async {
        for _ in 0..<5 { await Task.yield() }
        try? await Task.sleep(nanoseconds: 20_000_000)
    }

    func testSlotsMatchDesktop() {
        XCTAssertEqual(DraftSyncSlot.slot(threadRootEventID: nil), "main")
        XCTAssertEqual(DraftSyncSlot.slot(threadRootEventID: "$root"), "thread:$root")
    }

    func testTypingSavesAndErasingClearsEverywhere() async {
        let service = RecordingDraftSync()
        let sync = ComposerDraftSync(service: service)
        sync.reset(roomID: "!room:example.org")

        sync.noteChange("hello", replyToEventID: "$reply")
        await settle()
        XCTAssertEqual(service.saves, [.init(
            roomID: "!room:example.org", slot: "main", markdown: "hello", replyToEventID: "$reply", flush: false
        )])

        sync.noteChange("  ", replyToEventID: nil)
        await settle()
        XCTAssertEqual(service.clears, ["!room:example.org"])
    }

    func testRestoringTheOpeningDraftDoesNotOverwriteAPeersNewerOne() async {
        let service = RecordingDraftSync()
        let sync = ComposerDraftSync(service: service)
        sync.reset(roomID: "!room:example.org", openingText: "stale local text")

        // The view restores the stored draft; that is not typing.
        sync.noteChange("stale local text", replyToEventID: nil)
        await settle()
        XCTAssertTrue(service.saves.isEmpty)
        XCTAssertTrue(service.clears.isEmpty)

        sync.noteChange("stale local text, edited", replyToEventID: nil)
        await settle()
        XCTAssertEqual(service.saves.map(\.markdown), ["stale local text, edited"])
    }

    func testAnEmptyOpeningComposerDoesNotClearPeers() async {
        let service = RecordingDraftSync()
        let sync = ComposerDraftSync(service: service)
        sync.reset(roomID: "!room:example.org", openingText: "")
        sync.noteChange("", replyToEventID: nil)
        await settle()
        XCTAssertTrue(service.clears.isEmpty)
    }

    func testAppliedRemoteTextIsNotEchoedBack() async {
        let service = RecordingDraftSync()
        service.decision = .apply(remote("from desktop"))
        let sync = ComposerDraftSync(service: service)
        sync.reset(roomID: "!room:example.org")

        let applied = await sync.resolve(currentText: "", isFocused: false, fetchFresh: true)
        XCTAssertEqual(applied?.markdown, "from desktop")
        XCTAssertNil(sync.prompt)

        // The view puts the text in the composer; its change is not an edit.
        sync.noteChange("from desktop", replyToEventID: nil)
        await settle()
        XCTAssertTrue(service.saves.isEmpty)

        // The next resolve compares against the applied draft's stamp.
        service.decision = .keep
        _ = await sync.resolve(currentText: "from desktop", isFocused: false, fetchFresh: false)
        XCTAssertEqual(service.resolves.last?.updatedAt, 500)
        XCTAssertEqual(service.resolves.last?.dirty, false)
    }

    func testTypingComposerIsPromptedAndKeepMineWritesNow() async {
        let service = RecordingDraftSync()
        let sync = ComposerDraftSync(service: service)
        sync.reset(roomID: "!room:example.org")
        sync.noteChange("mine", replyToEventID: nil)
        await settle()

        service.decision = .prompt(remote("theirs"))
        let applied = await sync.resolve(currentText: "mine", isFocused: false, fetchFresh: false)
        XCTAssertNil(applied, "a typing composer is never overwritten")
        XCTAssertEqual(service.resolves.last?.dirty, true)
        XCTAssertEqual(sync.prompt?.markdown, "theirs")

        sync.keepLocal("mine", replyToEventID: nil)
        await settle()
        XCTAssertNil(sync.prompt)
        XCTAssertEqual(service.saves.last?.flush, true)
        XCTAssertEqual(service.saves.last?.markdown, "mine")
    }

    func testTakingTheRemoteDraftClearsThePrompt() async {
        let service = RecordingDraftSync()
        service.decision = .prompt(remote("theirs"))
        let sync = ComposerDraftSync(service: service)
        sync.reset(roomID: "!room:example.org")
        _ = await sync.resolve(currentText: "mine", isFocused: true, fetchFresh: false)

        XCTAssertEqual(sync.takeRemote()?.markdown, "theirs")
        XCTAssertNil(sync.prompt)
        XCTAssertNil(sync.takeRemote())
    }

    func testResetForgetsTheLastRoom() async {
        let service = RecordingDraftSync()
        service.decision = .prompt(remote("theirs"))
        let sync = ComposerDraftSync(service: service)
        sync.reset(roomID: "!room:example.org")
        _ = await sync.resolve(currentText: "mine", isFocused: true, fetchFresh: false)

        sync.reset(roomID: "!other:example.org")
        XCTAssertNil(sync.prompt)
        XCTAssertEqual(sync.roomID, "!other:example.org")
    }

    func testCoreResolutionMapsToDecisions() {
        let dto = RemoteDraftDto(
            roomId: "!room:example.org",
            slot: "main",
            updatedAt: 7,
            deviceId: "DESKTOP",
            cleared: true,
            markdown: "ignored for a cleared draft",
            replyTo: nil,
            editOf: nil,
            truncated: false
        )
        let apply = SharedCoreDraftSyncService.decision(
            from: DraftResolutionDto(sessionGeneration: 1, decision: .apply, draft: dto)
        )
        guard case let .apply(draft) = apply else {
            return XCTFail("expected apply")
        }
        XCTAssertTrue(draft.cleared)
        XCTAssertEqual(draft.markdown, "")
        XCTAssertEqual(
            SharedCoreDraftSyncService.decision(
                from: DraftResolutionDto(sessionGeneration: 1, decision: .prompt, draft: nil)
            ),
            .keep
        )
    }
}
