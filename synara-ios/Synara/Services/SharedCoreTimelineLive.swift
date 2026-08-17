import Foundation
import SynaraCore

/// P4-S18 decide whether a drained S14 summary should refresh a watched room.
///
/// Matches room id. When the product already has a stream id, the update
/// must name that stream (or leave it empty). No tokens or presence user
/// ids. This is not `Platform.emit` and not P4 acceptance.
enum SharedCoreTimelineLiveRefresh {
    static func shouldRefresh(
        watchingRoomID: String,
        watchingStreamId: String?,
        updateRoomId: String,
        updateStreamId: String
    ) -> Bool {
        guard updateRoomId == watchingRoomID else {
            return false
        }
        guard let watchingStreamId, watchingStreamId.isEmpty == false else {
            return true
        }
        return updateStreamId.isEmpty || updateStreamId == watchingStreamId
    }
}

/// One SharedCore poller so two open rooms cannot steal each other's S14
/// summaries. Starts only while a timeline stream is listening. NSE still
/// cannot poll (the Core method fail-closes).
final class SharedCoreLivePoller: @unchecked Sendable {
    private let core: SharedCore
    private let lock = NSLock()
    private var waiters: [UUID: (roomId: String, continuation: AsyncStream<TimelineViewUpdateDto>.Continuation)] = [:]
    private var roomListWaiters: [UUID: AsyncStream<Void>.Continuation] = [:]
    private var pollTask: Task<Void, Never>?

    init(core: SharedCore) {
        self.core = core
    }

    func timelineSignals(roomId: String) -> AsyncStream<TimelineViewUpdateDto> {
        AsyncStream { continuation in
            let id = UUID()
            lock.lock()
            waiters[id] = (roomId, continuation)
            startPollLocked()
            lock.unlock()
            continuation.onTermination = { [weak self] _ in
                self?.removeWaiter(id)
            }
        }
    }

    func roomListSignals() -> AsyncStream<Void> {
        AsyncStream { continuation in
            let id = UUID()
            lock.lock()
            roomListWaiters[id] = continuation
            startPollLocked()
            lock.unlock()
            continuation.onTermination = { [weak self] _ in
                self?.removeRoomListWaiter(id)
            }
        }
    }

    private func removeWaiter(_ id: UUID) {
        lock.lock()
        waiters.removeValue(forKey: id)
        stopPollIfIdleLocked()
        lock.unlock()
    }

    private func removeRoomListWaiter(_ id: UUID) {
        lock.lock()
        roomListWaiters.removeValue(forKey: id)
        stopPollIfIdleLocked()
        lock.unlock()
    }

    private func stopPollIfIdleLocked() {
        if waiters.isEmpty && roomListWaiters.isEmpty {
            pollTask?.cancel()
            pollTask = nil
        }
    }

    private func startPollLocked() {
        guard pollTask == nil else {
            return
        }
        let core = self.core
        pollTask = Task { [weak self] in
            while Task.isCancelled == false {
                try? await Task.sleep(nanoseconds: 250_000_000)
                guard Task.isCancelled == false else {
                    return
                }
                let wantsTimeline = self?.hasTimelineWaiters() ?? false
                let wantsRooms = self?.hasRoomListWaiters() ?? false
                let updates = wantsTimeline
                    ? ((try? await SharedCoreTimelineViewUpdates.poll(core: core)) ?? [])
                    : []
                let rooms = wantsRooms
                    ? ((try? await SharedCoreRoomListUpdates.poll(core: core)) ?? [])
                    : []
                if updates.isEmpty == false {
                    self?.dispatch(updates)
                }
                if rooms.isEmpty == false {
                    self?.dispatchRoomList()
                }
            }
        }
    }

    private func hasTimelineWaiters() -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return waiters.isEmpty == false
    }

    private func hasRoomListWaiters() -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return roomListWaiters.isEmpty == false
    }

    private func dispatch(_ updates: [TimelineViewUpdateDto]) {
        lock.lock()
        let waiters = self.waiters
        lock.unlock()
        for update in updates {
            for waiter in waiters.values where waiter.roomId == update.roomId {
                waiter.continuation.yield(update)
            }
        }
    }

    private func dispatchRoomList() {
        lock.lock()
        let waiters = roomListWaiters
        lock.unlock()
        for continuation in waiters.values {
            continuation.yield(())
        }
    }
}
