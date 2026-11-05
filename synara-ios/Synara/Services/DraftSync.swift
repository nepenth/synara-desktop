import Foundation
import SynaraCore

/// Settings → Composer "Sync drafts across devices". Per device, on by default.
enum DraftSyncSettings {
    static let syncDraftsKey = "synara.settings.syncDrafts"
}

/// The composer's draft slot: the room, or one of its threads.
enum DraftSyncSlot {
    static let main = "main"

    static func slot(threadRootEventID: String?) -> String {
        threadRootEventID.map { "thread:\($0)" } ?? main
    }
}

/// A newer draft another of this account's Synara devices wrote.
struct RemoteDraft: Equatable {
    let roomID: String
    let slot: String
    let updatedAt: UInt64
    let deviceID: String
    let cleared: Bool
    let markdown: String
    let replyToEventID: String?
    let truncated: Bool
}

enum DraftSyncDecision: Equatable {
    /// Nothing to change.
    case keep
    /// The composer is idle: show this draft.
    case apply(RemoteDraft)
    /// The composer is being typed in: offer this draft.
    case prompt(RemoteDraft)
}

/// What the composer holds right now.
struct LocalDraftState: Equatable {
    var updatedAt: UInt64?
    var dirty: Bool
    var markdown: String
}

/// Encrypted cross-device composer drafts (`in.synara.draft`) through Core.
/// The local `DraftStore` stays the composer's source of truth; this mirrors
/// it to the account's other Synara devices. Failures are silent: drafts
/// stay on this device.
protocol DraftSyncServicing {
    func setEnabled(_ enabled: Bool) async
    /// Queue a draft (or with `flush`, write now). Returns its stamp.
    func save(roomID: String, slot: String, markdown: String, replyToEventID: String?, flush: Bool) async -> UInt64?
    /// The draft was sent or erased: clear it on every device now.
    func clear(roomID: String, slot: String) async -> UInt64?
    func resolve(roomID: String, slot: String, local: LocalDraftState, fetchFresh: Bool) async -> DraftSyncDecision
    func flush() async
    /// Room IDs whose synced drafts changed; `nil` means every room.
    func updates() -> AsyncStream<String?>
}

struct NoDraftSync: DraftSyncServicing {
    func setEnabled(_ enabled: Bool) async {}
    func save(roomID: String, slot: String, markdown: String, replyToEventID: String?, flush: Bool) async -> UInt64? { nil }
    func clear(roomID: String, slot: String) async -> UInt64? { nil }
    func resolve(roomID: String, slot: String, local: LocalDraftState, fetchFresh: Bool) async -> DraftSyncDecision { .keep }
    func flush() async {}
    func updates() -> AsyncStream<String?> {
        AsyncStream { $0.finish() }
    }
}

final class SharedCoreDraftSyncService: DraftSyncServicing {
    private let host: SharedCoreProductHost

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    private var signedIn: Bool {
        if case .signedIn = host.sessionStore.currentState {
            return true
        }
        return false
    }

    func setEnabled(_ enabled: Bool) async {
        guard signedIn else { return }
        _ = try? await host.core.draftsSetEnabled(enabled: enabled)
    }

    func save(roomID: String, slot: String, markdown: String, replyToEventID: String?, flush: Bool) async -> UInt64? {
        guard signedIn else { return nil }
        return try? await host.core.draftsSave(
            roomId: roomID,
            slot: slot,
            markdown: markdown,
            replyTo: replyToEventID,
            editOf: nil,
            flush: flush
        ).updatedAt
    }

    func clear(roomID: String, slot: String) async -> UInt64? {
        guard signedIn else { return nil }
        return try? await host.core.draftsClear(roomId: roomID, slot: slot).updatedAt
    }

    func resolve(roomID: String, slot: String, local: LocalDraftState, fetchFresh: Bool) async -> DraftSyncDecision {
        guard signedIn,
              let resolution = try? await host.core.draftsResolve(
                  roomId: roomID,
                  slot: slot,
                  localUpdatedAt: local.updatedAt,
                  localDirty: local.dirty,
                  localMarkdown: local.markdown,
                  fetchFresh: fetchFresh
              )
        else {
            return .keep
        }
        return Self.decision(from: resolution)
    }

    func flush() async {
        guard signedIn else { return }
        try? await host.core.draftsFlush()
    }

    func updates() -> AsyncStream<String?> {
        let signals = host.livePoller.ownerSignals(families: ["drafts"], bufferingPolicy: .bufferingNewest(8))
        return AsyncStream { continuation in
            let task = Task {
                for await signal in signals {
                    continuation.yield(signal.roomId)
                }
                continuation.finish()
            }
            continuation.onTermination = { _ in task.cancel() }
        }
    }

    static func decision(from resolution: DraftResolutionDto) -> DraftSyncDecision {
        guard let draft = resolution.draft else {
            return .keep
        }
        let remote = RemoteDraft(
            roomID: draft.roomId,
            slot: draft.slot,
            updatedAt: draft.updatedAt,
            deviceID: draft.deviceId,
            cleared: draft.cleared,
            markdown: draft.cleared ? "" : (draft.markdown ?? ""),
            replyToEventID: draft.replyTo,
            truncated: draft.truncated
        )
        switch resolution.decision {
        case .keep: return .keep
        case .apply: return .apply(remote)
        case .prompt: return .prompt(remote)
        }
    }
}

/// One composer's sync state: what it last saved or applied, and whether the
/// user has typed since. Owned by the room view; all calls on the main actor.
@MainActor
final class ComposerDraftSync: ObservableObject {
    /// Typing this recently counts as "being typed in" even without focus.
    static let activeTypingInterval: TimeInterval = 3

    @Published private(set) var prompt: RemoteDraft?

    private var service: DraftSyncServicing
    private(set) var roomID: String = ""
    private(set) var slot: String = DraftSyncSlot.main
    private var stamp: UInt64?
    private var dirty = false
    private var unsynced = false
    private var lastTypedAt: Date = .distantPast
    /// Text just applied from a peer: its `onChange` is not a local edit.
    private var applied: String?
    /// Until the first real edit: the text the composer opened with.
    /// Restoring it is not an edit; saving it with a fresh stamp would bury
    /// a newer draft from another device.
    private var baseline: String?

    init(service: DraftSyncServicing) {
        self.service = service
    }

    /// Swap the service, e.g. to `NoDraftSync` when the setting is off.
    func attach(_ service: DraftSyncServicing) {
        self.service = service
    }

    func reset(roomID: String, threadRootEventID: String? = nil, openingText: String = "") {
        self.roomID = roomID
        slot = DraftSyncSlot.slot(threadRootEventID: threadRootEventID)
        stamp = nil
        dirty = false
        unsynced = false
        applied = nil
        baseline = openingText
        prompt = nil
    }

    /// Call from the composer's text change. Returns nothing; the local
    /// store is written by the caller.
    func noteChange(_ text: String, replyToEventID: String?) {
        guard roomID.isEmpty == false else { return }
        if let appliedText = applied {
            applied = nil
            if appliedText == text { return }
        }
        if let opening = baseline {
            if opening == text { return }
            baseline = nil
        }
        lastTypedAt = Date()
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        let roomID = roomID
        let slot = slot
        if trimmed.isEmpty {
            dirty = false
            unsynced = false
            Task { [service] in
                let updatedAt = await service.clear(roomID: roomID, slot: slot)
                await MainActor.run { if let updatedAt { self.stamp = updatedAt } }
            }
            return
        }
        dirty = true
        unsynced = true
        Task { [service] in
            let updatedAt = await service.save(
                roomID: roomID, slot: slot, markdown: text, replyToEventID: replyToEventID, flush: false
            )
            await MainActor.run { if let updatedAt { self.stamp = updatedAt } }
        }
    }

    /// Compare with the synced draft. Returns text to put in the composer
    /// when the remote draft applies silently.
    func resolve(currentText: String, isFocused: Bool, fetchFresh: Bool) async -> RemoteDraft? {
        guard roomID.isEmpty == false else { return nil }
        let typing = dirty && (isFocused || Date().timeIntervalSince(lastTypedAt) < Self.activeTypingInterval)
        let decision = await service.resolve(
            roomID: roomID,
            slot: slot,
            local: LocalDraftState(updatedAt: stamp, dirty: typing, markdown: currentText),
            fetchFresh: fetchFresh
        )
        switch decision {
        case .keep:
            return nil
        case let .prompt(remote):
            prompt = remote
            return nil
        case let .apply(remote):
            prompt = nil
            markApplied(remote)
            return remote
        }
    }

    /// The user chose the other device's draft. Returns it to show.
    func takeRemote() -> RemoteDraft? {
        guard let remote = prompt else { return nil }
        prompt = nil
        markApplied(remote)
        return remote
    }

    /// The user kept theirs: write it now so it becomes the newest.
    func keepLocal(_ text: String, replyToEventID: String?) {
        prompt = nil
        let roomID = roomID
        let slot = slot
        Task { [service] in
            let updatedAt = await service.save(
                roomID: roomID, slot: slot, markdown: text, replyToEventID: replyToEventID, flush: true
            )
            await MainActor.run { if let updatedAt { self.stamp = updatedAt } }
        }
    }

    /// Leaving the room or going to the background.
    func flushIfNeeded() {
        guard unsynced else { return }
        unsynced = false
        Task { [service] in await service.flush() }
    }

    private func markApplied(_ remote: RemoteDraft) {
        applied = remote.markdown
        baseline = nil
        stamp = remote.updatedAt
        dirty = false
        unsynced = false
    }
}
