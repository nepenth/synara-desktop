import Foundation
import SynaraCore

enum MatrixSessionRestoreError: LocalizedError, Equatable {
    case persistedStoreUnavailable
    case restorationFailed
    case serverDeviceKeysUnavailable
    case deviceIdentityMismatch

    /// A failed restore must never destroy the crypto identity and then reuse the
    /// persisted Matrix device ID with a newly-created store.
    var shouldDeletePersistedStore: Bool { false }

    var errorDescription: String? {
        switch self {
        case .persistedStoreUnavailable:
            return "Local encryption data is missing. Sign in again to create a new Matrix device; existing local data was preserved."
        case .restorationFailed:
            return "Local encryption data could not be restored. Retry first, or sign in again; existing local data was preserved."
        case .serverDeviceKeysUnavailable:
            return "The restored device identity could not be confirmed with the homeserver. Retry before signing in again; existing local data was preserved."
        case .deviceIdentityMismatch:
            return "The restored encryption identity does not match this Matrix device. Sign in again to create a new device; existing local data was preserved."
        }
    }
}

enum MatrixDeviceKeyContinuityResult: Equatable {
    case matches
    case unavailable
    case mismatch
}

enum MatrixDeviceKeyContinuityValidator {
    static func validate(
        responseData: Data,
        userID: String,
        deviceID: String,
        localCurve25519Key: String?,
        localEd25519Key: String?
    ) -> MatrixDeviceKeyContinuityResult {
        guard let localCurve25519Key, localCurve25519Key.isEmpty == false,
              let localEd25519Key, localEd25519Key.isEmpty == false,
              let root = try? JSONSerialization.jsonObject(with: responseData) as? [String: Any],
              let users = root["device_keys"] as? [String: Any],
              let devices = users[userID] as? [String: Any],
              let device = devices[deviceID] as? [String: Any],
              let keys = device["keys"] as? [String: Any],
              let serverCurve25519Key = keys["curve25519:\(deviceID)"] as? String,
              let serverEd25519Key = keys["ed25519:\(deviceID)"] as? String
        else {
            return .unavailable
        }

        guard serverCurve25519Key == localCurve25519Key,
              serverEd25519Key == localEd25519Key
        else {
            return .mismatch
        }
        return .matches
    }
}

enum MatrixVerificationEventSource: Equatable {
    case delegate
    case localRequest
}

enum CryptoVerificationPresentationPolicy {
    static func allowsInteractiveDismiss(_ state: CryptoVerificationState?) -> Bool {
        state?.isTerminal == true
    }

    /// Restore a still-active flow if local UI was cleared (swipe / nil) while
    /// the native inbox still has a non-terminal request.
    static func restoredStateIfCleared(
        presented: CryptoVerificationState?,
        latest: CryptoVerificationState?
    ) -> CryptoVerificationState? {
        if let presented {
            return presented
        }
        guard let latest, latest.isTerminal == false else {
            return nil
        }
        return latest
    }
}

enum SecuritySettingsVerificationPolicy {
    static func showsVerifyThisDevice(_ status: SessionCryptoStatus) -> Bool {
        status.verification != .verified && status.isLastDevice != true
    }
}

struct MatrixVerificationStateReducer {
    private(set) var state: CryptoVerificationState?
    private var activeFlowID: String?
    private var completedFlowIDs = Set<String>()
    private var completedFlowOrder: [String] = []
    private static let maximumCompletedFlows = 64

    @discardableResult
    mutating func reduce(
        _ candidate: CryptoVerificationState,
        source: MatrixVerificationEventSource
    ) -> CryptoVerificationState? {
        if case .finished = candidate, source != .delegate {
            return nil
        }

        if case .requestReceived(let request) = candidate {
            guard completedFlowIDs.contains(request.flowID) == false else {
                return nil
            }
            if let state, state.isTerminal == false {
                return nil
            }
            activeFlowID = request.flowID
            state = candidate
            return candidate
        }

        if case .requestSent = candidate {
            if let state, state.isTerminal == false {
                return nil
            }
            activeFlowID = nil
            state = candidate
            return candidate
        }

        guard let current = state, current.isTerminal == false else {
            return nil
        }

        if candidate.isTerminal {
            if let activeFlowID {
                rememberCompletedFlow(activeFlowID)
            }
            activeFlowID = nil
            state = candidate
            return candidate
        }

        let currentRank = phaseRank(current)
        let candidateRank = phaseRank(candidate)
        guard candidateRank > currentRank else {
            return nil
        }

        state = candidate
        return candidate
    }

    mutating func reset() {
        state = nil
        activeFlowID = nil
        completedFlowIDs.removeAll()
        completedFlowOrder.removeAll()
    }

    private mutating func rememberCompletedFlow(_ flowID: String) {
        guard completedFlowIDs.insert(flowID).inserted else {
            return
        }
        completedFlowOrder.append(flowID)
        while completedFlowOrder.count > Self.maximumCompletedFlows {
            completedFlowIDs.remove(completedFlowOrder.removeFirst())
        }
    }

    private func phaseRank(_ state: CryptoVerificationState) -> Int {
        switch state {
        case .requestReceived, .requestSent:
            return 0
        case .accepted:
            return 1
        case .sasStarted:
            return 2
        case .keysExchanging:
            return 3
        case .emojis, .decimals:
            return 4
        case .confirmed:
            return 5
        case .finished, .cancelled, .failed, .mismatched:
            return 6
        }
    }
}

struct MatrixVerificationContinuationRegistrationTracker {
    private var registeredIDs = Set<UUID>()
    private var cancellationTombstones = Set<UUID>()
    private var tombstoneOrder: [UUID] = []
    private static let maximumTombstones = 128

    mutating func register(id: UUID, isTaskCancelled: Bool) -> Bool {
        guard isTaskCancelled == false else {
            return false
        }
        if cancellationTombstones.remove(id) != nil {
            compactTombstoneOrder()
            return false
        }
        registeredIDs.insert(id)
        return true
    }

    mutating func cancel(id: UUID) {
        if registeredIDs.remove(id) != nil {
            return
        }
        guard cancellationTombstones.insert(id).inserted else {
            return
        }
        tombstoneOrder.append(id)
        while cancellationTombstones.count > Self.maximumTombstones,
              let oldest = tombstoneOrder.first
        {
            tombstoneOrder.removeFirst()
            cancellationTombstones.remove(oldest)
        }
    }

    mutating func removeRegistered(id: UUID) {
        registeredIDs.remove(id)
    }

    func isRegistered(id: UUID) -> Bool {
        registeredIDs.contains(id)
    }

    private mutating func compactTombstoneOrder() {
        tombstoneOrder.removeAll { cancellationTombstones.contains($0) == false }
    }
}

enum MatrixVerificationLifecycleTransition: Equatable {
    case backgroundPause
    case foregroundResume
    case sessionReplaced
    case localStateReset
}

enum MatrixVerificationLifecyclePolicy {
    static func shouldReset(for transition: MatrixVerificationLifecycleTransition) -> Bool {
        switch transition {
        case .backgroundPause, .foregroundResume:
            return false
        case .sessionReplaced, .localStateReset:
            return true
        }
    }
}

enum MatrixInteractiveFreshnessPolicy {
    static func shouldPerformSync(
        hasActiveSyncService: Bool,
        lastSuccessfulSyncAt: Date?,
        now: Date,
        maximumAge: TimeInterval
    ) -> Bool {
        guard hasActiveSyncService == false else {
            return false
        }
        guard let lastSuccessfulSyncAt else {
            return true
        }
        return now.timeIntervalSince(lastSuccessfulSyncAt) >= maximumAge
    }

    static func ownsInstalledOperation(installedGeneration: UInt64, currentGeneration: UInt64) -> Bool {
        installedGeneration == currentGeneration
    }
}

enum RoomUnreadPresentation {
    static func make(
        membership: RoomSummary.Membership,
        numUnreadMessages: UInt64 = 0,
        numUnreadNotifications: UInt64 = 0,
        numUnreadMentions: UInt64 = 0,
        isMarkedUnread: Bool = false
    ) -> (unreadCount: Int, hasHighlight: Bool) {
        let coreMembership: SynaraCore.RoomUnreadMembership
        switch membership {
        case .joined:
            coreMembership = .joined
        case .invited:
            coreMembership = .invited
        }

        let projection = SynaraCore.roomUnreadPresentation(
            membership: coreMembership,
            numUnreadMessages: numUnreadMessages,
            numUnreadNotifications: numUnreadNotifications,
            numUnreadMentions: numUnreadMentions,
            isMarkedUnread: isMarkedUnread
        )
        return (Int(projection.unreadCount), projection.hasHighlight)
    }
}

enum RoomListDynamicPagingPolicy {
    static func nextRequestedPageCount(
        snapshotCount: Int,
        requestedPageCount: Int,
        pageSize: Int
    ) -> Int? {
        guard pageSize > 0,
              requestedPageCount > 0,
              snapshotCount >= requestedPageCount * pageSize
        else {
            return nil
        }
        return requestedPageCount + 1
    }
}

enum RoomListCacheRetentionPolicy {
    static func retainedPreviousIDs(
        previousIDs: Set<String>,
        explicitlyRemovedIDs: Set<String>
    ) -> Set<String> {
        previousIDs.subtracting(explicitlyRemovedIDs)
    }
}

enum RoomListAuthoritativePruningPolicy {
    static let reconciliationInterval: TimeInterval = 30

    static func provenRemovedIDs(
        knownRoomIDs: Set<String>,
        joinedOrInvitedRoomIDs: Set<String>
    ) -> Set<String> {
        knownRoomIDs.subtracting(joinedOrInvitedRoomIDs)
    }

    static func shouldReconcile(
        cachedRoomIDs: Set<String>,
        dynamicSnapshotRoomIDs: Set<String>,
        requiresFullRemap: Bool,
        currentCatchUpPageCount: Int = 1,
        lastAttemptedCatchUpPageCount: Int? = nil,
        lastAttemptedAt: Date? = nil,
        now: Date = Date(),
        minimumInterval: TimeInterval = reconciliationInterval
    ) -> Bool {
        guard cachedRoomIDs.isEmpty == false else {
            return false
        }
        if requiresFullRemap {
            return true
        }
        guard cachedRoomIDs.isSubset(of: dynamicSnapshotRoomIDs) == false else {
            return false
        }
        if lastAttemptedCatchUpPageCount != currentCatchUpPageCount {
            return true
        }
        guard let lastAttemptedAt else {
            return true
        }
        return now.timeIntervalSince(lastAttemptedAt) >= minimumInterval
    }
}

enum RoomListAuthoritativeFallbackPolicy {
    static func shouldUseCachedFallback(
        authoritativeRoomCount: Int,
        cachedRoomCount: Int
    ) -> Bool {
        authoritativeRoomCount == 0 && cachedRoomCount > 0
    }
}

enum RoomListReconciliationHeartbeatPolicy {
    static func shouldContinue(
        isCancelled: Bool,
        isCurrentSession: Bool,
        currentGeneration: UInt64,
        expectedGeneration: UInt64
    ) -> Bool {
        isCancelled == false
            && isCurrentSession
            && currentGeneration == expectedGeneration
    }

    static func shouldEmit(
        cachedRoomIDs: Set<String>,
        dynamicSnapshotRoomIDs: Set<String>
    ) -> Bool {
        cachedRoomIDs.isEmpty == false
            && cachedRoomIDs.isSubset(of: dynamicSnapshotRoomIDs) == false
    }
}

enum MatrixTimelineReadReceiptPolicy {
    static func hasCurrentUserReceipt(
        readReceiptUserIDs: [String],
        currentUserID: String?
    ) -> Bool {
        guard let currentUserID, currentUserID.isEmpty == false else {
            return false
        }
        return readReceiptUserIDs.contains(currentUserID)
    }
}

struct RoomListCoalescingSnapshot<RoomValue>: @unchecked Sendable {
    let rooms: [RoomValue]
    let changedRoomIDs: Set<String>
    let requiresFullRemap: Bool
    let explicitlyRemovedRoomIDs: Set<String>
    let isReconciliationHeartbeat: Bool

    init(
        rooms: [RoomValue],
        changedRoomIDs: Set<String>,
        requiresFullRemap: Bool,
        explicitlyRemovedRoomIDs: Set<String> = [],
        isReconciliationHeartbeat: Bool = false
    ) {
        self.rooms = rooms
        self.changedRoomIDs = changedRoomIDs
        self.requiresFullRemap = requiresFullRemap
        self.explicitlyRemovedRoomIDs = explicitlyRemovedRoomIDs
        self.isReconciliationHeartbeat = isReconciliationHeartbeat
    }
}

final class RoomListLatestSnapshotAccumulator<RoomValue>: @unchecked Sendable {
    private let lock = NSLock()
    private var pendingSnapshot: RoomListCoalescingSnapshot<RoomValue>?
    private let signalContinuation: AsyncStream<Void>.Continuation
    let signals: AsyncStream<Void>

    init() {
        let stream = AsyncStream<Void>.makeStream(bufferingPolicy: .bufferingNewest(1))
        signals = stream.stream
        signalContinuation = stream.continuation
    }

    func yield(_ snapshot: RoomListCoalescingSnapshot<RoomValue>) {
        lock.lock()
        if let pendingSnapshot {
            self.pendingSnapshot = RoomListCoalescingSnapshot(
                rooms: snapshot.rooms,
                changedRoomIDs: pendingSnapshot.changedRoomIDs.union(snapshot.changedRoomIDs),
                requiresFullRemap: pendingSnapshot.requiresFullRemap || snapshot.requiresFullRemap,
                explicitlyRemovedRoomIDs: pendingSnapshot.explicitlyRemovedRoomIDs
                    .union(snapshot.explicitlyRemovedRoomIDs),
                isReconciliationHeartbeat: pendingSnapshot.isReconciliationHeartbeat
                    && snapshot.isReconciliationHeartbeat
            )
        } else {
            pendingSnapshot = snapshot
        }
        lock.unlock()
        signalContinuation.yield(())
    }

    func takePendingSnapshot() -> RoomListCoalescingSnapshot<RoomValue>? {
        lock.lock()
        defer { lock.unlock() }
        let snapshot = pendingSnapshot
        pendingSnapshot = nil
        return snapshot
    }

    func finish() {
        signalContinuation.finish()
    }
}

enum MatrixTimelineStreamLifecycle {
    static func shouldInvalidate(
        expectedGeneration: UInt64,
        currentGeneration: UInt64,
        isPaused: Bool
    ) -> Bool {
        isPaused || currentGeneration != expectedGeneration
    }
}

enum MatrixTimelineCollectorPolicy {
    static func retainedSuffixCount(itemCount: Int, limit: Int) -> Int {
        max(0, min(itemCount, limit))
    }

    static func droppedPrefixCount(itemCount: Int, limit: Int) -> Int {
        max(0, itemCount - max(0, limit))
    }

    static func droppedPrefixCountAfterPopBack(retainedCount: Int, droppedPrefixCount: Int) -> Int {
        guard retainedCount == 0, droppedPrefixCount > 0 else {
            return droppedPrefixCount
        }
        return droppedPrefixCount - 1
    }
}

enum MatrixSlidingSyncCompatibility {
    static func storedRawValue(reported: String, available: [String]) -> String {
        guard reported == "native", available.contains("native") == false else {
            return reported
        }
        return "none"
    }

    static func sdkVersion(storedRawValue: String, available: [String]?) -> String {
        guard storedRawValue != "none" else {
            return "none"
        }
        if let available, available.contains("native") == false {
            return "none"
        }
        return "native"
    }
}
