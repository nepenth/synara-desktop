import Combine
import Foundation

/// The one owner of session crypto status for every screen.
///
/// Room, Account, Security and the verification sheet each used to run their
/// own `sessionDeviceUpdates()` loop and re-read `sessionStatus()` (backup,
/// secret storage and the device snapshot) on every wakeup. Those reads could
/// re-arm the wakeup, which kept an open room at about 8 reads a second. Core
/// now wakes only on real changes; this store keeps the Swift side bounded
/// even if that regresses:
/// - reads are single-flight, and `read()` reuses a result younger than
///   `freshness`;
/// - one device-update subscription exists while anyone iterates `updates()`;
/// - wakeups are coalesced: a wakeup that arrives during a read waits for it
///   and then reads once, and wakeup-driven reads are at least
///   `wakeupSpacing` apart;
/// - `status` and `updates()` publish only when the status changes.
///
/// Wakeups are never dropped, so verification or recovery completion that
/// races a read is still picked up by the follow-up read.
final class SessionCryptoStore: ObservableObject {
    @Published private(set) var status: SessionCryptoStatus = .unknown
    /// Bumps after every wakeup-driven read, changed or not, for screens that
    /// refresh other device data (the Account session list) on the same wakeup.
    @Published private(set) var wakeupRevision = 0

    private let crypto: CryptoStatusServicing
    private let freshness: TimeInterval
    private let wakeupSpacing: TimeInterval
    private var inFlight: Task<SessionCryptoStatus, Never>?
    /// Bumped by `reset()`; a read started before it is not published.
    private var readGeneration = 0
    private var lastReadAt: TimeInterval?
    private var lastWakeupReadAt: TimeInterval?
    private var observers: [UUID: AsyncStream<SessionCryptoStatus>.Continuation] = [:]
    private var signalTask: Task<Void, Never>?
    private var wakeupTask: Task<Void, Never>?
    private var wakeupPending = false
    private var signalsEnded = false

    init(
        crypto: CryptoStatusServicing,
        freshness: TimeInterval = 5,
        wakeupSpacing: TimeInterval = 1
    ) {
        self.crypto = crypto
        self.freshness = freshness
        self.wakeupSpacing = wakeupSpacing
    }

    /// The session status, read through Core only when the last read is older
    /// than `maxAge` (default `freshness`). Concurrent callers share one read.
    @MainActor
    @discardableResult
    func read(maxAge: TimeInterval? = nil) async -> SessionCryptoStatus {
        if let inFlight {
            return await inFlight.value
        }
        if let lastReadAt, Self.now - lastReadAt < (maxAge ?? freshness) {
            return status
        }
        let crypto = crypto
        let generation = readGeneration
        // The read task records its own completion, so every caller awaiting
        // it resumes with `inFlight` cleared and `status` already published.
        let task = Task { @MainActor [weak self] in
            let value = await crypto.sessionStatus()
            self?.completeRead(value, generation: generation)
            return value
        }
        inFlight = task
        return await task.value
    }

    /// Screen open and explicit Retry: a new read unless one is in flight.
    @MainActor
    @discardableResult
    func refresh() async -> SessionCryptoStatus {
        await read(maxAge: 0)
    }

    /// Each status change while iterated. Iterating holds the shared wakeup
    /// subscription; the stream ends when the session's wakeups end.
    @MainActor
    func updates() -> AsyncStream<SessionCryptoStatus> {
        var captured: AsyncStream<SessionCryptoStatus>.Continuation?
        let stream = AsyncStream<SessionCryptoStatus>(bufferingPolicy: .bufferingNewest(1)) {
            captured = $0
        }
        guard let continuation = captured else { return stream }
        let id = UUID()
        observers[id] = continuation
        continuation.onTermination = { [weak self] _ in
            Task { @MainActor in self?.removeObserver(id) }
        }
        startSignalsIfNeeded()
        return stream
    }

    /// Forget the signed-out account's status. The store lives as long as the
    /// app, so without this the next account's screens would start from the
    /// previous account's status and could reuse its last read.
    @MainActor
    func reset() {
        readGeneration &+= 1
        inFlight = nil
        lastReadAt = nil
        lastWakeupReadAt = nil
        publish(.unknown)
    }

    @MainActor
    private func completeRead(_ value: SessionCryptoStatus, generation: Int) {
        guard generation == readGeneration else { return }
        inFlight = nil
        // An all-failed read is not worth reusing; the next caller retries.
        lastReadAt = value == .unknown ? nil : Self.now
        publish(value)
    }

    @MainActor
    private func publish(_ value: SessionCryptoStatus) {
        guard value != status else { return }
        status = value
        for continuation in observers.values {
            continuation.yield(value)
        }
    }

    @MainActor
    private func removeObserver(_ id: UUID) {
        observers[id] = nil
        guard observers.isEmpty else { return }
        signalTask?.cancel()
        signalTask = nil
        wakeupTask?.cancel()
        wakeupTask = nil
        wakeupPending = false
    }

    @MainActor
    private func startSignalsIfNeeded() {
        guard signalTask == nil else { return }
        signalsEnded = false
        // Subscribe before any read so a change during that read still wakes.
        let signals = crypto.sessionDeviceUpdates()
        signalTask = Task { @MainActor [weak self] in
            for await _ in signals {
                guard Task.isCancelled == false else { return }
                self?.wakeupArrived()
            }
            guard Task.isCancelled == false else { return }
            self?.signalsFinished()
        }
    }

    @MainActor
    private func wakeupArrived() {
        wakeupPending = true
        guard wakeupTask == nil else { return }
        wakeupTask = Task { @MainActor [weak self] in
            await self?.drainWakeups()
        }
    }

    @MainActor
    private func drainWakeups() async {
        while wakeupPending, Task.isCancelled == false {
            // A read already in flight may have started before the wakeup.
            if let inFlight {
                _ = await inFlight.value
                continue
            }
            if let lastWakeupReadAt {
                let remaining = wakeupSpacing - (Self.now - lastWakeupReadAt)
                if remaining > 0 {
                    try? await Task.sleep(nanoseconds: UInt64(remaining * 1_000_000_000))
                    continue
                }
            }
            // Every wakeup so far is covered by the read that starts now.
            wakeupPending = false
            lastWakeupReadAt = Self.now
            await read(maxAge: 0)
            wakeupRevision &+= 1
        }
        // A cancelled drain belongs to observers that are already gone.
        guard Task.isCancelled == false else { return }
        wakeupTask = nil
        finishObserversIfDone()
    }

    @MainActor
    private func signalsFinished() {
        signalTask = nil
        signalsEnded = true
        finishObserversIfDone()
    }

    @MainActor
    private func finishObserversIfDone() {
        guard signalsEnded, wakeupTask == nil else { return }
        signalsEnded = false
        let finished = observers.values
        observers.removeAll()
        for continuation in finished {
            continuation.finish()
        }
    }

    private static var now: TimeInterval {
        ProcessInfo.processInfo.systemUptime
    }
}
