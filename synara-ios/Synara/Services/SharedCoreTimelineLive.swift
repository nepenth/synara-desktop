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

/// A live timeline needs one readback after its signal listener is registered.
/// This catches SDK updates that landed between the initial open and listener
/// attachment without replacing the native timeline stream.
enum SharedCoreTimelineUpdateBootstrap {
    static func shouldRefreshOpenStream(focusedEventID: String?) -> Bool {
        focusedEventID == nil
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
    private var ownerWaiters: [UUID: (families: Set<String>, continuation: AsyncStream<OwnerUpdateDto>.Continuation)] = [:]
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

    func ownerSignals(families: Set<String>) -> AsyncStream<OwnerUpdateDto> {
        AsyncStream { continuation in
            let id = UUID()
            lock.lock()
            ownerWaiters[id] = (families, continuation)
            startPollLocked()
            lock.unlock()
            continuation.onTermination = { [weak self] _ in
                self?.removeOwnerWaiter(id)
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

    private func removeOwnerWaiter(_ id: UUID) {
        lock.lock()
        ownerWaiters.removeValue(forKey: id)
        stopPollIfIdleLocked()
        lock.unlock()
    }

    private func stopPollIfIdleLocked() {
        if waiters.isEmpty && roomListWaiters.isEmpty && ownerWaiters.isEmpty {
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
            var emptyTimelineTicks = 0
            var emptyRoomTicks = 0
            while Task.isCancelled == false {
                try? await Task.sleep(nanoseconds: 250_000_000)
                guard Task.isCancelled == false else {
                    return
                }
                let wantsTimeline = self?.hasTimelineWaiters() ?? false
                let wantsRooms = self?.hasRoomListWaiters() ?? false
                let wantsOwners = self?.hasOwnerWaiters() ?? false
                let updates = wantsTimeline
                    ? ((try? await SharedCoreTimelineViewUpdates.poll(core: core)) ?? [])
                    : []
                let rooms = wantsRooms
                    ? ((try? await SharedCoreRoomListUpdates.poll(core: core)) ?? [])
                    : []
                let owners = wantsOwners
                    ? ((try? await SharedCoreOwnerUpdates.poll(core: core)) ?? [])
                    : []
                if updates.isEmpty == false {
                    self?.dispatch(updates)
                } else if wantsTimeline {
                    emptyTimelineTicks += 1
                    if emptyTimelineTicks >= 4 {
                        emptyTimelineTicks = 0
                        self?.dispatchSyntheticTimelineWakes()
                    }
                } else {
                    emptyTimelineTicks = 0
                }
                if rooms.isEmpty == false {
                    emptyRoomTicks = 0
                    self?.dispatchRoomList()
                } else if wantsRooms {
                    emptyRoomTicks += 1
                    if emptyRoomTicks >= 4 {
                        emptyRoomTicks = 0
                        self?.dispatchRoomList()
                    }
                } else {
                    emptyRoomTicks = 0
                }
                if owners.isEmpty == false {
                    self?.dispatchOwners(owners)
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

    private func hasOwnerWaiters() -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return ownerWaiters.isEmpty == false
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

    private func dispatchSyntheticTimelineWakes() {
        lock.lock()
        let waiters = self.waiters
        lock.unlock()
        for waiter in waiters.values {
            waiter.continuation.yield(
                TimelineViewUpdateDto(
                    schemaVersion: 1,
                    sessionGeneration: 0,
                    streamId: "",
                    roomId: waiter.roomId,
                    revision: 0,
                    opCount: 0
                )
            )
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

    private func dispatchOwners(_ updates: [OwnerUpdateDto]) {
        lock.lock()
        let waiters = ownerWaiters
        lock.unlock()
        for update in updates {
            for waiter in waiters.values where waiter.families.contains(update.family) {
                waiter.continuation.yield(update)
            }
        }
    }
}
