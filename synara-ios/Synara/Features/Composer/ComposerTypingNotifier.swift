import Foundation

/// A cancellable one-shot or repeating timer.
protocol ComposerTypingTimer: AnyObject {
    func cancel()
}

protocol ComposerTypingScheduling {
    func schedule(after interval: TimeInterval, repeats: Bool, _ action: @escaping () -> Void) -> ComposerTypingTimer
}

extension Timer: ComposerTypingTimer {
    func cancel() {
        invalidate()
    }
}

/// Main run loop timers in common mode, so they also fire while the
/// timeline scrolls.
struct RunLoopComposerTypingScheduler: ComposerTypingScheduling {
    func schedule(after interval: TimeInterval, repeats: Bool, _ action: @escaping () -> Void) -> ComposerTypingTimer {
        let timer = Timer(timeInterval: interval, repeats: repeats) { _ in action() }
        RunLoop.main.add(timer, forMode: .common)
        return timer
    }
}

/// Keeps the outgoing typing notice alive while the user types, as desktop's
/// `useTypingStatusUpdater` does.
///
/// Timing contract with matrix-sdk 0.19 `Room::typing_notice` (Core
/// `typing_set`): typing=true goes out with a 4 s server timeout, and a
/// repeated true is only re-sent once 3 s have passed since the last send.
/// So while typing is active the SDK must hear from us inside that 3-4 s
/// window, or other members see the indicator lapse. A 500 ms refresh gives
/// two chances in each window, also through pauses between keystrokes; the
/// SDK decides which refreshes reach the server. Later keystrokes only push
/// back the idle stop. typing=false goes out once: after 5 s idle, on send,
/// when the text is cleared, and when the composer goes away.
///
/// Used from the main thread only.
final class ComposerTypingNotifier {
    static let refreshInterval: TimeInterval = 0.5
    static let idleStopInterval: TimeInterval = 5

    private let scheduler: ComposerTypingScheduling
    private let send: (Bool) -> Void
    private var refreshTimer: ComposerTypingTimer?
    private var idleTimer: ComposerTypingTimer?

    init(
        scheduler: ComposerTypingScheduling = RunLoopComposerTypingScheduler(),
        send: @escaping (Bool) -> Void
    ) {
        self.scheduler = scheduler
        self.send = send
    }

    deinit {
        idleTimer?.cancel()
        refreshTimer?.cancel()
    }

    var isTyping: Bool {
        refreshTimer != nil
    }

    /// Call on every keystroke or focus change with whether the user is
    /// composing (focused, non-empty text, activity sharing on).
    func update(isTyping typing: Bool) {
        guard typing else {
            stop()
            return
        }
        idleTimer?.cancel()
        idleTimer = scheduler.schedule(after: Self.idleStopInterval, repeats: false) { [weak self] in
            self?.stop()
        }
        guard refreshTimer == nil else {
            return
        }
        send(true)
        refreshTimer = scheduler.schedule(after: Self.refreshInterval, repeats: true) { [weak self] in
            self?.send(true)
        }
    }

    /// Sends typing=false if typing is active and clears both timers.
    func stop() {
        idleTimer?.cancel()
        idleTimer = nil
        guard let refreshTimer else {
            return
        }
        refreshTimer.cancel()
        self.refreshTimer = nil
        send(false)
    }

    /// Hands typing states to Core in order: a refresh queued just before a
    /// stop must not land after it. A true is dropped while another true is
    /// still waiting, so a slow request never builds a backlog.
    static func serialSender(_ deliver: @escaping (Bool) async -> Void) -> (Bool) -> Void {
        let queue = SerialTypingQueue(deliver: deliver)
        return { typing in queue.enqueue(typing) }
    }
}

private final class SerialTypingQueue {
    private let deliver: (Bool) async -> Void
    private var previous: Task<Void, Never>?
    private var hasPendingTrue = false

    init(deliver: @escaping (Bool) async -> Void) {
        self.deliver = deliver
    }

    func enqueue(_ typing: Bool) {
        if typing {
            guard hasPendingTrue == false else {
                return
            }
            hasPendingTrue = true
        }
        let prior = previous
        previous = Task { @MainActor [self] in
            await prior?.value
            if typing {
                hasPendingTrue = false
            }
            await deliver(typing)
        }
    }
}
