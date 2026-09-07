import Foundation
import Network
import SynaraCore

/// Caller-owned SharedCore used by live product services after S10 leftover retirement.
/// UniFFI must not construct-and-drop this instance.
final class SharedCoreProductHost {
    let core: SharedCore
    let storeRoot: URL
    let sessionStore: AppSessionStore
    let livePoller: SharedCoreLivePoller

    init(
        core: SharedCore,
        storeRoot: URL,
        sessionStore: AppSessionStore
    ) {
        self.core = core
        self.storeRoot = storeRoot
        self.sessionStore = sessionStore
        self.livePoller = SharedCoreLivePoller(core: core)
    }

    static func liveStoreRoot() -> URL {
        let fileManager = FileManager.default
        let legacyRoot = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("SynaraCore", isDirectory: true)
        guard let sharedContainer = fileManager.containerURL(
            forSecurityApplicationGroupIdentifier: SynaraSharedConstants.appGroupIdentifier
        ) else {
            try? fileManager.createDirectory(at: legacyRoot, withIntermediateDirectories: true)
            return legacyRoot
        }

        let sharedRoot = sharedContainer.appendingPathComponent(
            SynaraSharedConstants.sharedCoreStoreDirectory,
            isDirectory: true
        )
        return resolvedLiveStoreRoot(
            legacyRoot: legacyRoot,
            sharedRoot: sharedRoot,
            fileManager: fileManager
        )
    }

    static func resolvedLiveStoreRoot(
        legacyRoot: URL,
        sharedRoot: URL,
        fileManager: FileManager
    ) -> URL {
        let readyMarker = sharedRoot.appendingPathComponent(
            SynaraSharedConstants.sharedCoreStoreReadyMarker
        )
        if fileManager.fileExists(atPath: readyMarker.path) {
            return sharedRoot
        }

        let legacyExists = fileManager.fileExists(atPath: legacyRoot.path)
        let sharedExists = fileManager.fileExists(atPath: sharedRoot.path)

        if legacyExists, sharedExists {
            let sharedContents = try? fileManager.contentsOfDirectory(
                at: sharedRoot,
                includingPropertiesForKeys: nil
            )
            guard sharedContents?.isEmpty == true else {
                // Never guess between two populated SDK stores.
                return legacyRoot
            }
            do {
                try fileManager.removeItem(at: sharedRoot)
            } catch {
                return legacyRoot
            }
        }

        if legacyExists {
            do {
                try fileManager.createDirectory(
                    at: sharedRoot.deletingLastPathComponent(),
                    withIntermediateDirectories: true
                )
                try fileManager.moveItem(at: legacyRoot, to: sharedRoot)
            } catch {
                return legacyRoot
            }
        }

        do {
            try fileManager.createDirectory(at: sharedRoot, withIntermediateDirectories: true)
        } catch {
            try? fileManager.createDirectory(at: legacyRoot, withIntermediateDirectories: true)
            return legacyRoot
        }

        return sharedRoot
    }

    /// Publish only after product session restore has successfully opened the
    /// shared SDK store and migrated its key into the shared Keychain group.
    @discardableResult
    static func publishNseStoreReady(
        at storeRoot: URL,
        fileManager: FileManager = .default,
        expectedSharedRoot: URL? = nil
    ) throws -> Bool {
        guard let expectedRoot = expectedSharedRoot
                ?? SynaraSharedConstants.sharedCoreStoreRoot(fileManager: fileManager),
              expectedRoot.standardizedFileURL == storeRoot.standardizedFileURL,
              fileManager.fileExists(atPath: storeRoot.path) else {
            return false
        }
        let readyMarker = storeRoot.appendingPathComponent(
            SynaraSharedConstants.sharedCoreStoreReadyMarker
        )
        try Data("ready-v1".utf8).write(to: readyMarker, options: .atomic)
        return true
    }
}

enum SharedCoreLoginErrorMapping {
    static func loginError(for error: Error) -> LoginError {
        guard case let SessionLoginError.Failed(code, _) = error else {
            return .invalidCredentials
        }
        switch code {
        case "p4-s3c-secret-vault-unavailable", "p4-s3c-store-root-invalid":
            return .sessionPersistenceFailed
        case "p4-s3c-identity-invalid":
            return .unsupported
        default:
            return .invalidCredentials
        }
    }
}

struct SharedCoreAuthService: AuthServicing {
    let host: SharedCoreProductHost

    func login(_ request: LoginRequest) async throws -> AuthenticatedSession {
        let username = request.username.trimmingCharacters(in: .whitespacesAndNewlines)
        guard username.isEmpty == false else {
            throw LoginError.missingUsername
        }
        guard request.password.isEmpty == false else {
            throw LoginError.missingPassword
        }
        let userID = username.hasPrefix("@")
            ? username
            : "@\(username):\(request.homeserverURL.host ?? "localhost")"
        do {
            let dto = try await SharedCoreSessionLogin.loginWithPassword(
                userID: userID,
                homeserverURL: request.homeserverURL.absoluteString,
                storeRoot: host.storeRoot,
                password: request.password,
                core: host.core
            )
            return AuthenticatedSession(
                userID: dto.userId,
                deviceID: dto.deviceId,
                homeserverURL: URL(string: dto.homeserverUrl) ?? request.homeserverURL,
                accessToken: ""
            )
        } catch {
            throw SharedCoreLoginErrorMapping.loginError(for: error)
        }
    }
}

final class SharedCoreMatrixClientService: MatrixClientServicing {
    private let host: SharedCoreProductHost
    private let connectionStatus: ConnectionStatusStore
    private let applyLock = NSLock()
    private var applyChain: Task<Void, Never> = Task {}
    /// Fail closed until the app delegate binds UIKit's actual state. This
    /// prevents the signed-in SwiftUI shell from starting live SQLite owners
    /// during a notification-driven background launch.
    private var foregroundActive = false
    private var lastSession: AuthenticatedSession?
    private var pathMonitor: NWPathMonitor?
    private let pathQueue = DispatchQueue(label: "com.whylandcreative.synara.connection-path")
    private var statusWatchTask: Task<Void, Never>?
    private(set) var syncStatus: MatrixSyncStatus = .stopped
    private static let syncNotAttachedCode = "p4-s12-sync-not-attached"

    var syncStatusDescription: String {
        syncStatus.description
    }

    init(host: SharedCoreProductHost, connectionStatus: ConnectionStatusStore = ConnectionStatusStore()) {
        self.host = host
        self.connectionStatus = connectionStatus
    }

    func start(session: AuthenticatedSession) async {
        await enqueueLiveSession(session)
    }

    func warmSync(session: AuthenticatedSession) async {
        _ = session
    }

    func revokeServerSession(_ session: AuthenticatedSession) async -> Bool {
        do {
            return try await host.core.revokeServerSession(
                userId: session.userID,
                deviceId: session.deviceID,
                homeserverUrl: session.homeserverURL.absoluteString
            )
        } catch {
            return false
        }
    }

    func forgetPersistedSession(_ session: AuthenticatedSession) async throws {
        await stop()
        _ = try await host.core.forgetSession(
            userId: session.userID,
            homeserverUrl: session.homeserverURL.absoluteString
        )
    }

    func stop() async {
        await enqueueBackgroundPause(clearLastSession: true)
    }

    func setForegroundActive(_ active: Bool) {
        applyLock.withLock {
            foregroundActive = active
        }
    }

    func pauseForBackground() async {
        setForegroundActive(false)
        await enqueueBackgroundPause(clearLastSession: false)
    }

    func resumeFromForeground(session: AuthenticatedSession) async {
        setForegroundActive(true)
        await enqueueLiveSession(session)
    }

    private func enqueueLiveSession(_ session: AuthenticatedSession) async {
        let queued: Task<Void, Never> = applyLock.withLock {
            lastSession = session
            let previous = applyChain
            let next = Task { [weak self] in
                await previous.value
                guard let self else {
                    return
                }
                let mayStart = self.applyLock.withLock { self.foregroundActive }
                guard mayStart else {
                    PerformanceTrace.event("MatrixStoreOpenSuppressed")
                    await self.publish(.stopped)
                    return
                }
                PerformanceTrace.event("MatrixStoreOpenAuthorized")
                await self.applyLiveSession(session)
            }
            applyChain = next
            return next
        }
        await queued.value
    }

    private func enqueueBackgroundPause(clearLastSession: Bool) async {
        stopStatusWatch()
        stopPathMonitor()
        let queued: Task<Void, Never> = applyLock.withLock {
            let previous = applyChain
            let next = Task { [weak self] in
                await previous.value
                await self?.applyBackgroundPause(clearLastSession: clearLastSession)
            }
            applyChain = next
            return next
        }
        await queued.value
    }

    private func applyBackgroundPause(clearLastSession: Bool) async {
        PerformanceTrace.event("MatrixStoreCloseBegin")
        defer { PerformanceTrace.event("MatrixStoreCloseComplete") }
        do {
            let stopped = try await SharedCoreSyncStop.stopSync(core: host.core)
            if clearLastSession {
                lastSession = nil
            }
            await publish(stopped.stopped ? .stopped : .failed("Native sync did not stop"))
        } catch let SyncStopError.Failed(code, _) where code == Self.syncNotAttachedCode {
            if clearLastSession {
                lastSession = nil
            }
            // A cold background launch has no native owners or open stores to
            // quiesce. Preserve the fail-closed state instead of inventing a
            // connection failure that could trigger recovery work.
            await publish(.stopped)
        } catch {
            if clearLastSession {
                lastSession = nil
            }
            await publish(.failed("Native sync stop failed"))
        }
    }

    private func applyLiveSession(_ session: AuthenticatedSession) async {
        lastSession = session
        await publish(.starting)
        let outcome = await SharedCoreSessionBootstrap.prepareLiveSession(
            userID: session.userID,
            homeserverURL: session.homeserverURL.absoluteString,
            storeRoot: host.storeRoot,
            core: host.core
        )
        if outcome.hasLiveClient {
            _ = try? SharedCoreProductHost.publishNseStoreReady(at: host.storeRoot)
        }
        if let failure = outcome.failure {
            await publish(failure.syncStatus)
            if failure == .restoreFailed || failure == .attachFailed {
                stopStatusWatch()
                stopPathMonitor()
            } else {
                startStatusWatch()
                startPathMonitor()
            }
            return
        }
        await publish(ConnectionStatusCopy.fromReadiness(outcome.readiness, previous: .starting))
        startStatusWatch()
        startPathMonitor()
    }

    private func startStatusWatch() {
        if statusWatchTask != nil {
            return
        }
        statusWatchTask = Task { [weak self] in
            while Task.isCancelled == false {
                try? await Task.sleep(nanoseconds: 1_000_000_000)
                guard Task.isCancelled == false else {
                    return
                }
                await self?.refreshLiveSyncStatus()
            }
        }
    }

    private func stopStatusWatch() {
        statusWatchTask?.cancel()
        statusWatchTask = nil
    }

    private func refreshLiveSyncStatus() async {
        switch syncStatus {
        case .restoreFailed, .stopped:
            return
        default:
            break
        }
        guard let dto = try? await SharedCoreSessionStatus.syncStatus(core: host.core) else {
            return
        }
        await publish(ConnectionStatusCopy.fromReadiness(dto.readiness, previous: syncStatus))
    }

    private func publish(_ status: MatrixSyncStatus) async {
        syncStatus = status
        await MainActor.run {
            connectionStatus.update(status)
        }
    }

    private func startPathMonitor() {
        if pathMonitor != nil {
            return
        }
        let monitor = NWPathMonitor()
        monitor.pathUpdateHandler = { [weak self] path in
            Task { await self?.handlePath(path) }
        }
        monitor.start(queue: pathQueue)
        pathMonitor = monitor
    }

    private func stopPathMonitor() {
        pathMonitor?.cancel()
        pathMonitor = nil
    }

    private func handlePath(_ path: NWPath) async {
        switch syncStatus {
        case .restoreFailed, .stopped:
            return
        default:
            break
        }
        switch path.status {
        case .unsatisfied, .requiresConnection:
            switch syncStatus {
            case .connected, .syncing, .starting:
                await publish(.reconnecting)
            default:
                break
            }
        case .satisfied:
            if syncStatus == .reconnecting, let lastSession {
                await enqueueLiveSession(lastSession)
            }
        @unknown default:
            break
        }
    }

    func syncForBackgroundNotification(session: AuthenticatedSession) async -> Bool {
        _ = session
        return false
    }

    func resetLocalState(for session: AuthenticatedSession?) async {
        _ = session
        // Keep the per-account crypto store so the next password login can
        // reuse this device id. Wiping here minted a new Matrix session on
        // every sign-in.
        _ = try? await SharedCoreLeftovers.logout(core: host.core)
    }

    func coreSessionIdentity() async -> CoreSessionIdentity? {
        guard let snapshot = try? await SharedCoreSessionStatus.sessionSnapshot(core: host.core),
              snapshot.status == "logged_in",
              let userID = snapshot.userId,
              let deviceID = snapshot.deviceId,
              let homeserver = snapshot.homeserverUrl
        else {
            return nil
        }
        return CoreSessionIdentity(
            userID: userID,
            deviceID: deviceID,
            homeserverURL: homeserver
        )
    }

    func presence(userID: String) async -> SharedCorePresence? {
        guard let snapshot = try? await SharedCoreTypingPresence.presenceSnapshot(
            core: host.core,
            userId: userID
        ) else {
            return nil
        }
        return SharedCorePresenceLive.presence(
            userId: snapshot.userId,
            state: snapshot.state,
            currentlyActive: snapshot.currentlyActive,
            statusMsg: snapshot.statusMsg
        )
    }

    func setOwnPresence(_ state: String) async -> Bool {
        do {
            _ = try await SharedCoreTypingPresence.presenceSet(
                core: host.core,
                state: state,
                statusMsg: nil
            )
            return true
        } catch {
            return false
        }
    }

    func ownProfile() async -> SharedCoreOwnProfileInfo? {
        guard let dto = try? await SharedCoreOwnProfile.getOwnProfile(core: host.core) else {
            return nil
        }
        return SharedCoreOwnProfileInfo(
            userID: dto.userId,
            displayName: dto.displayName,
            avatarURL: dto.avatarUrl
        )
    }

    func setOwnDisplayName(_ displayName: String) async -> Bool {
        do {
            _ = try await SharedCoreOwnProfile.setOwnDisplayName(
                core: host.core,
                displayName: displayName
            )
            return true
        } catch {
            return false
        }
    }

    func uploadOwnAvatar(payload: Data, mimeType: String) async -> Bool {
        do {
            let uploaded = try await SharedCoreOwnProfile.uploadAvatar(
                core: host.core,
                payload: payload,
                mimeType: mimeType
            )
            _ = try await SharedCoreOwnProfile.setOwnAvatar(core: host.core, mxc: uploaded.mxc)
            return true
        } catch {
            return false
        }
    }

    func setOutgoingTyping(roomID: String, typing: Bool) async {
        _ = try? await SharedCoreTypingPresence.typingSet(
            core: host.core,
            roomId: roomID,
            typing: typing
        )
    }

    func ignoredUserIDs() async -> [String] {
        (try? await SharedCoreAccountSettings.ignoredUsersSnapshot(core: host.core))?.userIds ?? []
    }

    func ignoreUser(_ userID: String) async -> Bool {
        do {
            try await SharedCoreAccountSettings.ignoredUsersIgnore(core: host.core, userId: userID)
            return true
        } catch {
            return false
        }
    }

    func unignoreUser(_ userID: String) async -> Bool {
        do {
            try await SharedCoreAccountSettings.ignoredUsersUnignore(core: host.core, userId: userID)
            return true
        } catch {
            return false
        }
    }

    func pushRulesSnapshot() async -> SynaraPushRulesSnapshot? {
        guard let snapshot = try? await SharedCoreAccountSettings.pushRulesSnapshot(core: host.core) else {
            return nil
        }
        return SynaraPushRulesSnapshot(
            dm: snapshot.dm,
            dmEncrypted: snapshot.dmEncrypted,
            group: snapshot.group,
            groupEncrypted: snapshot.groupEncrypted,
            mentions: SynaraPushRuleMentions(
                userMention: snapshot.mentions.userMention,
                displayName: snapshot.mentions.displayName,
                userName: snapshot.mentions.userName,
                roomMention: snapshot.mentions.roomMention,
                atRoom: snapshot.mentions.atRoom
            ),
            keywords: snapshot.keywords
        )
    }

    func setPushRuleDefault(encrypted: Bool, oneToOne: Bool, mode: String) async -> Bool {
        do {
            try await SharedCoreAccountSettings.pushRulesSetDefault(
                core: host.core,
                encrypted: encrypted,
                oneToOne: oneToOne,
                mode: mode
            )
            return true
        } catch {
            return false
        }
    }

    func setPushRuleMention(ruleID: String, enabled: Bool) async -> Bool {
        do {
            try await SharedCoreAccountSettings.pushRulesSetMention(
                core: host.core,
                ruleId: ruleID,
                enabled: enabled
            )
            return true
        } catch {
            return false
        }
    }

    func addPushKeyword(_ keyword: String) async -> Bool {
        do {
            try await SharedCoreAccountSettings.pushRulesAddKeyword(core: host.core, keyword: keyword)
            return true
        } catch {
            return false
        }
    }

    func removePushKeyword(_ keyword: String) async -> Bool {
        do {
            try await SharedCoreAccountSettings.pushRulesRemoveKeyword(core: host.core, keyword: keyword)
            return true
        } catch {
            return false
        }
    }

    func threepidEmails() async -> [String] {
        (try? await SharedCoreAccountSettings.threepidSnapshot(core: host.core))?.emails.map(\.address) ?? []
    }

    func deleteThreepidEmail(_ address: String) async -> Bool {
        do {
            try await SharedCoreAccountSettings.threepidDelete(core: host.core, address: address)
            return true
        } catch {
            return false
        }
    }

    func requestThreepidEmailToken(_ email: String) async -> Bool {
        do {
            _ = try await SharedCoreAccountSettings.threepidRequestEmailToken(core: host.core, email: email)
            return true
        } catch {
            return false
        }
    }

    func addThreepidEmail() async -> String? {
        try? await SharedCoreAccountSettings.threepidAddEmail(core: host.core).status
    }

    func addThreepidEmailPassword(_ password: String) async -> String? {
        try? await SharedCoreAccountSettings.threepidAddEmailPassword(core: host.core, password: password).status
    }
}

final class SharedCoreRoomListService: RoomListServicing {
    private let host: SharedCoreProductHost
    private let stateLock = NSLock()
    private var cachedNames: [String: String] = [:]
    private var cachedRooms: [String: RoomSummary] = [:]
    private var latestState: RoomListState?
    private var updateContinuations: [UUID: AsyncStream<RoomListState>.Continuation] = [:]
    private var updatesTask: Task<Void, Never>?

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func loadRooms() async -> RoomListState {
        do {
            let snapshot = try await SharedCoreRoomList.roomListSnapshot(core: host.core)
            let invites = (try? await SharedCoreInvites.invitesSnapshot(core: host.core))?.invites ?? []
            let spaceParents = (try? await SharedCoreSpaces.spaceParentsSnapshot(core: host.core))?.entries ?? []
            let rooms = SharedCoreRoomListRows.rooms(
                rooms: snapshot.rooms.map {
                    SharedCoreRoomListRows.RoomRow(
                        roomId: $0.roomId,
                        name: $0.name,
                        avatarUrl: $0.avatarUrl,
                        membership: $0.membership,
                        isDirect: $0.isDirect,
                        unreadCount: Int($0.unreadCount),
                        highlightCount: Int($0.highlightCount),
                        markedUnread: $0.markedUnread,
                        lastActivityTs: $0.lastActivityTs,
                        lastMessagePreview: $0.lastMessagePreview,
                        isFavorite: $0.isFavorite,
                        encryptionStatus: SharedCoreRoomListRows.encryptionStatus($0.encryptionStatus)
                    )
                },
                invites: invites.map {
                    SharedCoreRoomListRows.InviteRow(
                        roomId: $0.roomId,
                        roomName: $0.roomName,
                        roomTopic: $0.roomTopic,
                        senderName: $0.senderName,
                        reason: $0.reason
                    )
                },
                spaceParents: spaceParents.map {
                    SharedCoreRoomListRows.SpaceParentRow(
                        roomId: $0.roomId,
                        parentIds: $0.parentIds
                    )
                }
            )
            let state: RoomListState = rooms.isEmpty ? .empty : .loaded(rooms)
            stateLock.withLock {
                cachedNames = Dictionary(uniqueKeysWithValues: rooms.map { ($0.id, $0.name) })
                cachedRooms = Dictionary(uniqueKeysWithValues: rooms.map { ($0.id, $0) })
                latestState = state
            }
            return state
        } catch {
            return .empty
        }
    }

    func roomDisplayName(roomID: String) -> String? {
        stateLock.withLock {
            cachedNames[roomID]
        }
    }

    func isAgentRoom(roomID: String) -> Bool {
        stateLock.withLock {
            cachedRooms[roomID]?.isAgentRoom ?? false
        }
    }

    func hasUnreadMessages(roomID: String) -> Bool {
        stateLock.withLock {
            guard let room = cachedRooms[roomID] else {
                return false
            }
            return SharedCoreRoomListRows.hasUnreadMessages(
                unreadCount: room.unreadCount,
                hasHighlight: room.hasHighlight,
                isMarkedUnread: room.isMarkedUnread
            )
        }
    }

    func clearCache() {
        let reset = stateLock.withLock {
            cachedNames.removeAll()
            cachedRooms.removeAll()
            latestState = nil
            let task = updatesTask
            let continuations = Array(updateContinuations.values)
            updatesTask = nil
            updateContinuations.removeAll()
            return (task, continuations)
        }
        reset.0?.cancel()
        for continuation in reset.1 {
            continuation.finish()
        }
    }

    func roomUpdates() -> AsyncStream<RoomListState> {
        AsyncStream { continuation in
            let id = UUID()
            let initialState: RoomListState? = stateLock.withLock {
                updateContinuations[id] = continuation
                if updatesTask == nil {
                    updatesTask = Task { [weak self] in
                        await self?.publishRoomUpdates()
                    }
                }
                return latestState
            }
            if let initialState {
                continuation.yield(initialState)
            }
            continuation.onTermination = { [weak self] _ in
                self?.removeRoomUpdateContinuation(id)
            }
        }
    }

    private func publishRoomUpdates() async {
        await refreshAndPublishRoomList()
        for await _ in host.livePoller.roomListSignals() {
            guard Task.isCancelled == false else {
                break
            }
            await refreshAndPublishRoomList()
        }
    }

    private func refreshAndPublishRoomList() async {
        let state = await loadRooms()
        guard Task.isCancelled == false else {
            return
        }
        let continuations = stateLock.withLock {
            Array(updateContinuations.values)
        }
        for continuation in continuations {
            continuation.yield(state)
        }
    }

    private func removeRoomUpdateContinuation(_ id: UUID) {
        stateLock.withLock {
            updateContinuations.removeValue(forKey: id)
            if updateContinuations.isEmpty {
                updatesTask?.cancel()
                updatesTask = nil
            }
        }
    }
}

final class SharedCoreRoomMembershipService: RoomMembershipServicing {
    private let host: SharedCoreProductHost

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func acceptInvite(roomID: String) async throws {
        do {
            _ = try await SharedCoreInviteActions.invitesAccept(core: host.core, roomId: roomID)
        } catch {
            throw RoomMembershipError.failed
        }
    }

    func rejectInvite(roomID: String) async throws {
        do {
            _ = try await SharedCoreInviteActions.invitesDecline(core: host.core, roomId: roomID)
        } catch {
            throw RoomMembershipError.failed
        }
    }
}

struct SharedCoreTimelinePaginationProgress {
    static let maximumPageRequests = 8
    static let maximumConsecutiveNoProgressPages = 2

    private(set) var pageRequestCount = 0
    private(set) var consecutiveNoProgressPages = 0
    private var lastRowIDs: Set<String>?

    mutating func observeInitial(rowIDs: Set<String>) {
        lastRowIDs = rowIDs
    }

    var canRequestPage: Bool {
        pageRequestCount < Self.maximumPageRequests
            && consecutiveNoProgressPages < Self.maximumConsecutiveNoProgressPages
    }

    mutating func recordPage(rowIDs: Set<String>) {
        pageRequestCount += 1
        if let lastRowIDs, lastRowIDs == rowIDs {
            consecutiveNoProgressPages += 1
        } else {
            consecutiveNoProgressPages = 0
        }
        lastRowIDs = rowIDs
    }
}

actor SharedCoreTimelineRecoveryGate {
    private struct Entry {
        let id: UUID
        let task: Task<TimelineLoadOutcome, Never>
    }

    private var entries: [String: Entry] = [:]

    func run(
        roomID: String,
        operation: @escaping () async -> TimelineLoadOutcome
    ) async -> TimelineLoadOutcome {
        if let existing = entries[roomID] {
            return await existing.task.value
        }

        let id = UUID()
        let task = Task { await operation() }
        entries[roomID] = Entry(id: id, task: task)
        let outcome = await task.value
        if entries[roomID]?.id == id {
            entries.removeValue(forKey: roomID)
        }
        return outcome
    }
}

final class SharedCoreTimelineService: TimelineServicing {
    private struct OpenStreamState {
        let streamID: String
        var visibleItemIDs: Set<String>
    }

    private let host: SharedCoreProductHost
    private let streamLock = NSLock()
    private let recoveryGate = SharedCoreTimelineRecoveryGate()
    private var streams: [String: OpenStreamState] = [:]

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func loadInitialTimeline(roomID: String, focusedEventID: String?) async -> TimelineLoadOutcome {
        do {
            if let previous = takeStream(for: roomID) {
                _ = try? await SharedCoreTimeline.timelineClose(core: host.core, streamId: previous)
            }
            let position = TimelineOpenPositionDto(
                kind: focusedEventID == nil ? "live" : "focused",
                atBottom: focusedEventID == nil,
                restoredAnchorEventId: nil,
                liveTailEventId: nil,
                updatedAtMs: nil,
                eventId: focusedEventID
            )
            let opened = try await SharedCoreTimeline.timelineOpen(
                core: host.core,
                roomId: roomID,
                position: position
            )
            storeStream(opened.streamId, visibleItemIDs: [], for: roomID)
            return await resolveInitialSnapshot(
                opened.snapshot,
                roomID: roomID,
                streamID: opened.streamId
            )
        } catch {
            return .failed(Self.failure(from: error))
        }
    }

    func loadOlderTimeline(roomID: String, before eventID: String) async -> TimelineLoadOutcome {
        _ = eventID
        guard let initialState = stream(for: roomID) else {
            return .failed(Self.viewUnavailableFailure)
        }
        var knownItemIDs = initialState.visibleItemIDs
        var paginationProgress = SharedCoreTimelinePaginationProgress()
        while Task.isCancelled == false {
            guard paginationProgress.canRequestPage else {
                return .failed(Self.temporarilyUnavailableFailure)
            }
            do {
                let snapshot = try await SharedCoreTimeline.timelinePaginate(
                    core: host.core,
                    streamId: initialState.streamID,
                    direction: "backwards"
                )
                paginationProgress.recordPage(rowIDs: Self.nativeRowIDs(snapshot.rows))
                let items = SharedCoreTimelineRows.items(from: snapshot.rows, visibleTailEventID: snapshot.visibleTailEventId, receiptTailEventID: snapshot.receiptTailEventId)
                let itemIDs = Self.stableItemIDs(items)
                let containsNewItems = itemIDs.isSubset(of: knownItemIDs) == false
                knownItemIDs.formUnion(itemIDs)
                updateVisibleItemIDs(
                    knownItemIDs,
                    roomID: roomID,
                    streamID: initialState.streamID
                )
                if containsNewItems {
                    return .loaded(items)
                }
                if snapshot.paginationBackward == "exhausted" {
                    return .empty
                }
                guard snapshot.paginationBackward == "available" else {
                    return .failed(Self.temporarilyUnavailableFailure)
                }
            } catch {
                return .failed(Self.failure(from: error))
            }
        }
        return .failed(Self.temporarilyUnavailableFailure)
    }

    func timelineUpdates(roomID: String, focusedEventID: String?) -> AsyncStream<TimelineLoadOutcome> {
        // Register before the task begins so an SDK update cannot be lost
        // between the initial snapshot and the first live readback.
        let signals = host.livePoller.timelineSignals(roomId: roomID)
        return AsyncStream(bufferingPolicy: .bufferingNewest(1)) { continuation in
            let task = Task {
                if SharedCoreTimelineUpdateBootstrap.shouldRefreshOpenStream(
                    focusedEventID: focusedEventID
                ) {
                    continuation.yield(
                        await refreshOpenTimeline(roomID: roomID, focusedEventID: focusedEventID)
                    )
                }
                for await _ in signals {
                    guard Task.isCancelled == false else {
                        break
                    }
                    continuation.yield(
                        await refreshOpenTimeline(roomID: roomID, focusedEventID: focusedEventID)
                    )
                }
                continuation.finish()
            }
            continuation.onTermination = { _ in
                task.cancel()
            }
        }
    }

    func typingUsers(roomID: String) -> AsyncStream<[String]> {
        AsyncStream { continuation in
            let task = Task {
                continuation.yield(await typingUsersInRoom(roomID))
                for await update in host.livePoller.ownerSignals(families: ["typing"]) {
                    guard Task.isCancelled == false else {
                        break
                    }
                    guard SharedCoreTypingLive.shouldRefresh(
                        watchingRoomID: roomID,
                        updateRoomId: update.roomId
                    ) else {
                        continue
                    }
                    continuation.yield(await typingUsersInRoom(roomID))
                }
                continuation.finish()
            }
            continuation.onTermination = { _ in
                task.cancel()
            }
        }
    }

    func clearSessionCaches() {
        let streamIds = takeAllStreams()
        Task {
            for streamId in streamIds {
                _ = try? await SharedCoreTimeline.timelineClose(core: host.core, streamId: streamId)
            }
        }
    }

    private func refreshOpenTimeline(roomID: String, focusedEventID: String?) async -> TimelineLoadOutcome {
        let traceID = PerformanceTrace.begin("TimelineSnapshotRefresh")
        defer { PerformanceTrace.end("TimelineSnapshotRefresh", id: traceID) }
        if let streamId = stream(for: roomID)?.streamID {
            do {
                let snapshot = try await SharedCoreTimeline.timelineSnapshot(
                    core: host.core,
                    streamId: streamId
                )
                return await resolveInitialSnapshot(
                    snapshot,
                    roomID: roomID,
                    streamID: streamId
                )
            } catch {
                let failure = Self.failure(from: error)
                if failure.kind == .viewUnavailable {
                    return await recoveryGate.run(roomID: roomID) { [weak self] in
                        guard let self else {
                            return .failed(Self.temporarilyUnavailableFailure)
                        }
                        guard let staleStreamID = self.takeStream(
                            for: roomID,
                            ifMatching: streamId
                        ) else {
                            return await self.refreshCurrentTimelineAfterRecoveryRace(roomID: roomID)
                        }
                        _ = try? await SharedCoreTimeline.timelineClose(
                            core: self.host.core,
                            streamId: staleStreamID
                        )
                        return await self.loadInitialTimeline(
                            roomID: roomID,
                            focusedEventID: focusedEventID
                        )
                    }
                }
                return .failed(failure)
            }
        }
        return await loadInitialTimeline(roomID: roomID, focusedEventID: focusedEventID)
    }

    private func resolveInitialSnapshot(
        _ initialSnapshot: TimelineSnapshotDto,
        roomID: String,
        streamID: String
    ) async -> TimelineLoadOutcome {
        var snapshot = initialSnapshot
        var paginationProgress = SharedCoreTimelinePaginationProgress()
        paginationProgress.observeInitial(rowIDs: Self.nativeRowIDs(snapshot.rows))
        while Task.isCancelled == false {
            let items = SharedCoreTimelineRows.items(from: snapshot.rows, visibleTailEventID: snapshot.visibleTailEventId, receiptTailEventID: snapshot.receiptTailEventId)
            updateVisibleItemIDs(
                Self.stableItemIDs(items),
                roomID: roomID,
                streamID: streamID
            )
            if let outcome = SharedCoreTimelineRows.authoritativeOutcome(
                from: snapshot.rows,
                paginationBackward: snapshot.paginationBackward,
                visibleTailEventID: snapshot.visibleTailEventId,
                receiptTailEventID: snapshot.receiptTailEventId
            ) {
                return outcome
            }
            guard snapshot.paginationBackward == "available" else {
                return .failed(Self.temporarilyUnavailableFailure)
            }
            guard paginationProgress.canRequestPage else {
                return .failed(Self.temporarilyUnavailableFailure)
            }
            do {
                snapshot = try await SharedCoreTimeline.timelinePaginate(
                    core: host.core,
                    streamId: streamID,
                    direction: "backwards"
                )
                paginationProgress.recordPage(rowIDs: Self.nativeRowIDs(snapshot.rows))
            } catch {
                return .failed(Self.failure(from: error))
            }
        }
        return .failed(Self.temporarilyUnavailableFailure)
    }

    private func stream(for roomID: String) -> OpenStreamState? {
        streamLock.lock()
        defer { streamLock.unlock() }
        return streams[roomID]
    }

    private func storeStream(
        _ streamID: String,
        visibleItemIDs: Set<String>,
        for roomID: String
    ) {
        streamLock.lock()
        streams[roomID] = OpenStreamState(
            streamID: streamID,
            visibleItemIDs: visibleItemIDs
        )
        streamLock.unlock()
    }

    private func updateVisibleItemIDs(
        _ itemIDs: Set<String>,
        roomID: String,
        streamID: String
    ) {
        streamLock.lock()
        defer { streamLock.unlock() }
        guard var state = streams[roomID], state.streamID == streamID else {
            return
        }
        state.visibleItemIDs.formUnion(itemIDs)
        streams[roomID] = state
    }

    private func takeStream(for roomID: String) -> String? {
        streamLock.lock()
        defer { streamLock.unlock() }
        return streams.removeValue(forKey: roomID)?.streamID
    }

    private func takeStream(for roomID: String, ifMatching expectedStreamID: String) -> String? {
        streamLock.lock()
        defer { streamLock.unlock() }
        guard streams[roomID]?.streamID == expectedStreamID else {
            return nil
        }
        return streams.removeValue(forKey: roomID)?.streamID
    }

    private func refreshCurrentTimelineAfterRecoveryRace(roomID: String) async -> TimelineLoadOutcome {
        guard let current = stream(for: roomID) else {
            return .failed(Self.temporarilyUnavailableFailure)
        }
        do {
            let snapshot = try await SharedCoreTimeline.timelineSnapshot(
                core: host.core,
                streamId: current.streamID
            )
            return await resolveInitialSnapshot(
                snapshot,
                roomID: roomID,
                streamID: current.streamID
            )
        } catch {
            return .failed(Self.failure(from: error))
        }
    }

    private func takeAllStreams() -> [String] {
        streamLock.lock()
        defer { streamLock.unlock() }
        let values = streams.values.map(\.streamID)
        streams.removeAll()
        return values
    }

    private static func stableItemIDs(_ items: [TimelineItem]) -> Set<String> {
        Set(items.map { $0.eventID.isEmpty ? $0.id : $0.eventID })
    }

    private static func nativeRowIDs(_ rows: [TimelineViewRowDto]) -> Set<String> {
        Set(rows.map(\.itemId))
    }

    private static let viewUnavailableFailure = TimelineLoadFailure(
        kind: .viewUnavailable,
        diagnosticCode: "timeline-view-unavailable"
    )

    private static let temporarilyUnavailableFailure = TimelineLoadFailure(
        kind: .temporarilyUnavailable,
        diagnosticCode: "timeline-temporarily-unavailable"
    )

    static func failure(from error: Error) -> TimelineLoadFailure {
        guard let timelineError = error as? TimelineError else {
            return temporarilyUnavailableFailure
        }
        switch timelineError {
        case let .Failed(code, _):
            let kind: TimelineLoadFailure.Kind
            switch code {
            case "p2-timeline-open-no-session", "p2-timeline-snapshot-no-session",
                 "p2-timeline-paginate-no-session":
                kind = .sessionUnavailable
            case "v-timeline-normal-room-not-found", "d0.3-timeline-room-not-found",
                 "d0.3-timeline-invalid-room-id":
                kind = .roomUnavailable
            case "v-timeline-view-not-open":
                kind = .viewUnavailable
            default:
                kind = .temporarilyUnavailable
            }
            return TimelineLoadFailure(kind: kind, diagnosticCode: code)
        }
    }

    private func typingUsersInRoom(_ roomID: String) async -> [String] {
        guard let snapshot = try? await SharedCoreTypingPresence.typingSnapshot(core: host.core) else {
            return []
        }
        return SharedCoreTypingLive.users(roomID: roomID, from: snapshot)
    }
}

final class SharedCoreLaterService: LaterServicing {
    private let host: SharedCoreProductHost

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func loadItems() async -> Result<([SynaraLaterListItem], LaterInboxError?), Never> {
        guard case .signedIn = host.sessionStore.currentState else {
            return .success(([], .noSession))
        }
        do {
            let snapshot = try await SharedCoreLater.laterSnapshot(core: host.core)
            let items = snapshot.items.map { item in
                SynaraLaterListItem(
                    id: item.id,
                    roomID: item.roomId,
                    eventID: item.eventId,
                    kind: item.kind == "reminder" ? .reminder : .saved,
                    dueTs: item.dueTs.map { Int($0) },
                    completedAt: item.completedAt.map { Int($0) },
                    createdAt: Int(item.createdAt),
                    isCompleted: item.completedAt != nil
                )
            }
            return .success((items, nil))
        } catch {
            return .success(([], .networkFailure))
        }
    }

    func completeItem(id: String) async -> Result<Bool, LaterInboxError> {
        do {
            _ = try await SharedCoreLater.laterComplete(
                core: host.core,
                itemId: id,
                completedAt: Date().timeIntervalSince1970 * 1000
            )
            return .success(true)
        } catch {
            return .failure(.networkFailure)
        }
    }
}

final class SharedCoreMessageSendService: MessageSending {
    private let host: SharedCoreProductHost

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func send(_ request: MessageSendRequest) async throws -> TimelineItem {
        let body = request.body.trimmingCharacters(in: .whitespacesAndNewlines)
        guard body.isEmpty == false else {
            throw MessageSendError.emptyMessage
        }
        do {
            if let editEventID = request.editEventID {
                _ = try await SharedCoreEditMessage.editMessage(
                    core: host.core,
                    roomId: request.roomID,
                    eventId: editEventID,
                    body: body,
                    msgType: nil,
                    formattedBody: request.formattedBody,
                    mentionUserIds: nil,
                    mentionRoom: nil,
                    txnId: nil
                )
            } else {
                _ = try await SharedCoreSendText.sendText(
                    core: host.core,
                    roomId: request.roomID,
                    body: body,
                    msgType: nil,
                    formattedBody: request.formattedBody,
                    mentionUserIds: nil,
                    mentionRoom: nil,
                    replyTo: request.replyToEventID,
                    threadRoot: request.threadRootEventID,
                    txnId: nil
                )
            }
            return TimelineItem(
                id: request.editEventID ?? "$local-\(UUID().uuidString)",
                eventID: request.editEventID ?? "$local-\(UUID().uuidString)",
                senderID: signedInUserID(),
                timestamp: Date(),
                kind: request.formattedBody.map { .formattedText(body: body, html: $0) } ?? .text(body),
                replyToEventID: request.replyToEventID,
                threadRootEventID: request.threadRootEventID,
                isEdited: request.editEventID != nil,
                reactions: [:]
            )
        } catch {
            throw MessageSendError.failed
        }
    }

    private func signedInUserID() -> String {
        if case .signedIn(let session) = host.sessionStore.currentState {
            return session.userID
        }
        return "@local:matrix.org"
    }
}

final class SharedCoreEventActionService: EventActionServicing {
    private let host: SharedCoreProductHost
    private let inFlight = TimelineActionInFlightCoordinator()

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func availability(for item: TimelineItem, currentUserID: String) -> EventActionAvailability {
        MockEventActionService().availability(for: item, currentUserID: currentUserID)
    }

    func availability(
        for item: TimelineItem,
        currentUserID: String,
        roomID: String
    ) -> EventActionAvailability {
        let sessionEpoch = host.sessionStore.sessionEpoch
        inFlight.bindSession(epoch: sessionEpoch)
        let pollKey = actionKey(
            sessionEpoch: sessionEpoch,
            roomID: roomID,
            eventID: item.eventID,
            actionKey: "poll-vote"
        )
        if let poll = item.poll {
            inFlight.observePollProjection(
                pollKey,
                ownAnswerIDs: poll.answers.filter(\.isOwn).map(\.id)
            )
        }
        let reactionPrefix = actionKey(
            sessionEpoch: sessionEpoch,
            roomID: roomID,
            eventID: item.eventID,
            actionKey: "react:"
        )
        inFlight.observeReactionProjection(
            reactionPrefix,
            ownership: item.reactionOwnership
        )
        let projected = availability(for: item, currentUserID: currentUserID)
        let ownsReactionState: Bool
        if case .known = item.reactionOwnership {
            ownsReactionState = true
        } else {
            ownsReactionState = false
        }
        let pollPending = inFlight.contains(pollKey) && projected.canVote
        let reactionPending = inFlight.contains(prefix: reactionPrefix) && projected.canReact
        guard pollPending || reactionPending || (projected.canReact && !ownsReactionState) else {
            return projected
        }
        return EventActionAvailability(
            canReply: projected.canReply,
            canEdit: projected.canEdit,
            canRedact: projected.canRedact,
            canReact: projected.canReact && ownsReactionState && !reactionPending,
            canReport: projected.canReport,
            canForward: projected.canForward,
            canVote: projected.canVote && !pollPending,
            canDeclineCall: projected.canDeclineCall
        )
    }

    func apply(
        _ action: EventActionType,
        to item: TimelineItem,
        currentUserID: String,
        roomID: String
    ) async throws -> TimelineItem {
        let sessionEpoch = host.sessionStore.sessionEpoch
        inFlight.bindSession(epoch: sessionEpoch)
        let key = actionKey(
            sessionEpoch: sessionEpoch,
            roomID: roomID,
            eventID: item.eventID,
            actionKey: action.inFlightKey
        )
        let claimed: Bool
        if case .pollVote(let answerIDs) = action {
            claimed = inFlight.beginPoll(key, answerIDs: answerIDs)
        } else if case .react(let reaction) = action {
            guard case let .known(ownKeys) = item.reactionOwnership else {
                throw EventActionError.failed
            }
            claimed = inFlight.beginReaction(
                key,
                reactionKey: reaction,
                expectedOwn: !ownKeys.contains(reaction)
            )
        } else {
            claimed = inFlight.begin(key)
        }
        guard claimed else {
            throw EventActionError.alreadyInProgress
        }
        do {
            let updated = try await applyUnlocked(
                action,
                to: item,
                currentUserID: currentUserID,
                roomID: roomID
            )
            if case .pollVote = action {
                inFlight.settlePollDispatch(key)
            } else if case .react = action {
                inFlight.settleReactionDispatch(key)
            } else {
                inFlight.end(key)
            }
            return updated
        } catch {
            inFlight.end(key)
            throw error
        }
    }

    private func actionKey(
        sessionEpoch: Int,
        roomID: String,
        eventID: String,
        actionKey: String
    ) -> String {
        "\(sessionEpoch)\u{0}\(roomID)\u{0}\(eventID)\u{0}\(actionKey)"
    }

    private func applyUnlocked(
        _ action: EventActionType,
        to item: TimelineItem,
        currentUserID: String,
        roomID: String
    ) async throws -> TimelineItem {
        _ = currentUserID
        switch action {
        case .reply, .edit:
            return item
        case .redact:
            do {
                let readback = try await SharedCoreTimelineMutate.timelineRedact(
                    core: host.core,
                    roomId: roomID,
                    eventId: item.eventID,
                    reason: nil
                )
                guard TimelineActionReadbackPolicy.accepts(
                    schemaVersion: readback.schemaVersion,
                    action: readback.action,
                    roomID: readback.roomId,
                    eventID: readback.eventId,
                    status: readback.status,
                    expectedAction: "redact",
                    expectedRoomID: roomID,
                    expectedStatus: "redacted",
                    expectedEventID: item.eventID
                ) else {
                    throw EventActionError.failed
                }
            } catch {
                throw EventActionError.failed
            }
            return item
        case .react(let reaction):
            do {
                guard case let .known(ownKeys) = item.reactionOwnership else {
                    throw EventActionError.failed
                }
                let expectedOwn = !ownKeys.contains(reaction)
                let readback = try await SharedCoreTimelineReactions.timelineReactionToggle(
                    core: host.core,
                    roomId: roomID,
                    eventId: item.eventID,
                    key: reaction
                )
                guard TimelineReactionReadbackPolicy.acceptsToggle(
                          roomID: readback.roomId,
                          targetEventID: readback.targetEventId,
                          key: readback.key,
                          mutation: readback.mutation,
                          readbackKey: readback.readback?.key,
                          readbackOwnsReaction: readback.readback?.me,
                          expectedRoomID: roomID,
                          expectedTargetEventID: item.eventID,
                          expectedKey: reaction,
                          expectedOwn: expectedOwn
                      )
                else {
                    throw EventActionError.failed
                }
            } catch {
                throw EventActionError.failed
            }
            return item
        case .report(let reason):
            do {
                let readback = try await SharedCoreTimelineMutate.timelineReport(
                    core: host.core,
                    roomId: roomID,
                    eventId: item.eventID,
                    reason: reason
                )
                guard TimelineActionReadbackPolicy.accepts(
                    schemaVersion: readback.schemaVersion,
                    action: readback.action,
                    roomID: readback.roomId,
                    eventID: readback.eventId,
                    status: readback.status,
                    expectedAction: "report",
                    expectedRoomID: roomID,
                    expectedStatus: "reported",
                    expectedEventID: item.eventID
                ) else {
                    throw EventActionError.failed
                }
            } catch {
                throw EventActionError.failed
            }
            return item
        case let .forward(targetRoomID, asQuote, confirmedEncryptionDowngrade):
            do {
                switch item.forwardTransport {
                case .text:
                    let readback = try await SharedCoreTimelineForward.timelineForwardText(
                        core: host.core,
                        sourceRoomId: roomID,
                        eventId: item.eventID,
                        targetRoomId: targetRoomID,
                        asQuote: asQuote,
                        confirmedEncryptionDowngrade: confirmedEncryptionDowngrade
                    )
                    guard TimelineActionReadbackPolicy.accepts(
                        schemaVersion: readback.schemaVersion,
                        action: readback.action,
                        roomID: readback.roomId,
                        eventID: readback.eventId,
                        status: readback.status,
                        expectedAction: "forward_text",
                        expectedRoomID: targetRoomID,
                        expectedStatus: "sent"
                    ) else {
                        throw EventActionError.failed
                    }
                case .media:
                    let readback = try await SharedCoreTimelineForward.timelineForwardMedia(
                        core: host.core,
                        sourceRoomId: roomID,
                        eventId: item.eventID,
                        targetRoomId: targetRoomID,
                        confirmedEncryptionDowngrade: confirmedEncryptionDowngrade
                    )
                    guard TimelineActionReadbackPolicy.accepts(
                        schemaVersion: readback.schemaVersion,
                        action: readback.action,
                        roomID: readback.roomId,
                        eventID: readback.eventId,
                        status: readback.status,
                        expectedAction: "forward_media",
                        expectedRoomID: targetRoomID,
                        expectedStatus: "sent"
                    ) else {
                        throw EventActionError.failed
                    }
                case .unavailable:
                    throw EventActionError.failed
                }
            } catch let error as EventActionError {
                throw error
            } catch let error as TimelineForwardError {
                if case let .Failed(code, _) = error {
                    throw TimelineForwardErrorPolicy.map(coreCode: code)
                }
                throw EventActionError.failed
            } catch {
                throw EventActionError.failed
            }
            return item
        case .pollVote(let answerIDs):
            do {
                let readback = try await SharedCoreTimelineVoteDecline.timelinePollVote(
                    core: host.core,
                    roomId: roomID,
                    eventId: item.eventID,
                    answerIds: answerIDs
                )
                guard TimelineActionReadbackPolicy.accepts(
                    schemaVersion: readback.schemaVersion,
                    action: readback.action,
                    roomID: readback.roomId,
                    eventID: readback.eventId,
                    status: readback.status,
                    expectedAction: "poll_vote",
                    expectedRoomID: roomID,
                    expectedStatus: "voted",
                    expectedEventID: item.eventID
                ) else {
                    throw EventActionError.failed
                }
            } catch {
                throw EventActionError.failed
            }
            return item
        case .declineCall:
            do {
                let readback = try await SharedCoreTimelineVoteDecline.timelineCallDecline(
                    core: host.core,
                    roomId: roomID,
                    eventId: item.eventID
                )
                guard TimelineActionReadbackPolicy.accepts(
                    schemaVersion: readback.schemaVersion,
                    action: readback.action,
                    roomID: readback.roomId,
                    eventID: readback.eventId,
                    status: readback.status,
                    expectedAction: "call_decline",
                    expectedRoomID: roomID,
                    expectedStatus: "declined",
                    expectedEventID: item.eventID
                ) else {
                    throw EventActionError.failed
                }
            } catch {
                throw EventActionError.failed
            }
            return item
        }
    }
}

final class SharedCoreAgentApprovalService: AgentApprovalServicing {
    private let host: SharedCoreProductHost

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func submit(_ request: SynaraAgentApprovalRequest) async throws {
        guard case .signedIn = host.sessionStore.currentState else {
            throw SynaraAgentApprovalError.signedOut
        }
        guard request.action.id.isEmpty == false else {
            throw SynaraAgentApprovalError.unsupportedAction
        }
        do {
            _ = try await SharedCoreAgentApprovals.send(
                core: host.core,
                roomId: request.roomID,
                actionId: request.action.id,
                actionTitle: request.action.title,
                decision: request.decision.rawValue,
                sourceEventId: request.sourceEventID,
                createdAt: UInt64(max(0, Date().timeIntervalSince1970 * 1000))
            )
        } catch let error as SynaraAgentApprovalError {
            throw error
        } catch {
            throw SynaraAgentApprovalError.failed
        }
    }
}

final class SharedCoreAgentApprovalDecisionService: AgentApprovalDecisionServicing {
    private let host: SharedCoreProductHost

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func submitDecision(
        _ request: SynaraAgentApprovalPromptDecisionRequest
    ) async throws -> SynaraAgentApprovalPromptDecisionOutcome {
        guard case .signedIn = host.sessionStore.currentState else {
            throw SynaraAgentApprovalError.signedOut
        }
        do {
            let result = try await host.core.agentApprovalDecide(
                roomId: request.roomID,
                eventId: request.sourceEventID,
                actionId: request.actionIdentifier
            )
            switch result.status {
            case "applied":
                return .applied
            case "already_decided":
                return .alreadyDecided
            default:
                throw SynaraAgentApprovalError.failed
            }
        } catch {
            throw SynaraAgentApprovalError.failed
        }
    }
}

final class SharedCoreCryptoStatusService: CryptoStatusServicing {
    struct JoinedRoomEncryptionRow: Equatable {
        let roomID: String
        let membership: String
        let encryption: SynaraRoomEncryptionStatus
    }

    private let host: SharedCoreProductHost
    private let flowLock = NSLock()
    private var flowId: String?

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func roomStatus(roomID: String) async -> RoomCryptoStatus {
        let session = await sessionStatus()
        // Timeline actions address joined rooms. Their encryption authority is
        // the joined-room Core snapshot only; an invite Bool is not a valid
        // fallback when that snapshot is missing or failed.
        let encryption = await listEncryption(roomID: roomID)
        guard session != .unknown || encryption != .unknown else {
            return .unknown
        }
        return SharedCoreSessionCrypto.roomStatus(
            encryption: encryption,
            session: session
        )
    }

    func sessionStatus() async -> SessionCryptoStatus {
        let crypto = try? await SharedCoreLeftovers.cryptoStatus(core: host.core)
        let backup = try? await SharedCoreLeftovers.backupStatus(core: host.core)
        let secretStorage = try? await SharedCoreSessionStatus.secretStorageStatus(core: host.core)
        let deviceSnapshot = try? await SharedCoreDevices.deviceSnapshot(core: host.core)
        let devices = deviceSnapshot?.devices ?? []
        guard crypto != nil || backup != nil || secretStorage != nil || deviceSnapshot != nil else {
            return .unknown
        }
        let mapped = SharedCoreSessionCrypto.status(
            crossSigningState: crypto?.crossSigningState,
            backupEnabled: backup?.enabled,
            backupAvailability: backup?.availability,
            backupDeviceState: backup?.deviceState,
            recoveryState: backup?.recoveryState,
            secretStorageState: secretStorage?.state
        )
        let hasOtherDevices = devices.contains { $0.isCurrent == false }
        let verification: SynaraCryptoVerificationStatus
        switch deviceSnapshot?.ownVerification {
        case "verified":
            verification = .verified
        case "unverified":
            verification = .unverified
        default:
            verification = .unknown
        }
        return SessionCryptoStatus(
            verification: verification,
            recovery: mapped.recovery,
            backup: mapped.backup,
            hasDevicesToVerifyAgainst: deviceSnapshot?.hasDevicesToVerifyAgainst,
            isLastDevice: devices.isEmpty ? nil : hasOtherDevices == false,
            unableToDecryptCount: mapped.unableToDecryptCount
        )
    }

    func verificationUpdates() -> AsyncStream<CryptoVerificationState> {
        AsyncStream { continuation in
            let task = Task {
                if let state = await currentVerificationState() {
                    continuation.yield(state)
                }
                for await _ in host.livePoller.ownerSignals(families: ["verification", "devices"]) {
                    guard Task.isCancelled == false else {
                        break
                    }
                    if let state = await currentVerificationState() {
                        continuation.yield(state)
                    }
                }
                continuation.finish()
            }
            continuation.onTermination = { _ in
                task.cancel()
            }
        }
    }

    func retryDecryption(roomID: String) async -> CryptoActionResult {
        _ = roomID
        return .unavailable("Crypto recovery is unavailable.")
    }

    func requestDeviceVerification(deviceId: String?) async -> CryptoActionResult {
        await runVerification {
            let dto = try await SharedCoreVerificationSas.verificationStart(
                core: host.core,
                // nil is semantically meaningful: it requests verification of
                // this device through the account's own cross-signing identity.
                // Explicit session-row IDs remain direct peer verification.
                deviceId: deviceId
            )
            storeFlow(dto.flowId)
            return "Device verification request sent."
        }
    }

    func dismissVerification() async -> CryptoActionResult {
        guard let flowId = resolvedFlowId() else {
            return .completed("Verification closed.")
        }
        do {
            try await SharedCoreVerificationSas.verificationDismiss(
                core: host.core,
                flowId: flowId
            )
            clearFlow()
            return .completed("Verification closed.")
        } catch {
            clearFlow()
            return .completed("Verification closed.")
        }
    }

    func acceptVerificationRequest() async -> CryptoActionResult {
        await runVerification(requiresFlow: true) { flowId in
            _ = try await SharedCoreVerificationSas.verificationAccept(
                core: host.core,
                flowId: flowId
            )
            return "Verification request accepted."
        }
    }

    func startSasVerification() async -> CryptoActionResult {
        await runVerification(requiresFlow: true) { flowId in
            _ = try await SharedCoreVerificationSas.verificationBeginSas(
                core: host.core,
                flowId: flowId
            )
            return "Verification comparison started."
        }
    }

    func approveVerification() async -> CryptoActionResult {
        await runVerification(requiresFlow: true) { flowId in
            _ = try await SharedCoreVerificationSas.verificationConfirm(
                core: host.core,
                flowId: flowId
            )
            return "Device verified."
        }
    }

    func declineVerification() async -> CryptoActionResult {
        await runVerification(requiresFlow: true) { flowId in
            _ = try await SharedCoreVerificationSas.verificationMismatch(
                core: host.core,
                flowId: flowId
            )
            return "Verification declined."
        }
    }

    func cancelVerification() async -> CryptoActionResult {
        await runVerification(requiresFlow: true) { flowId in
            _ = try await SharedCoreVerificationSas.verificationCancel(
                core: host.core,
                flowId: flowId
            )
            return "Verification cancelled."
        }
    }

    func recover(recoveryKey: String) async -> CryptoActionResult {
        let trimmed = recoveryKey.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false else {
            return .failed("Enter your recovery key or passphrase.")
        }
        do {
            _ = try await SharedCoreBackupRestore.restoreBackup(
                core: host.core,
                recoverySecret: trimmed
            )
            return .completed("Recovery completed.")
        } catch {
            return Self.restoreBackupResult(for: error)
        }
    }

    private static func restoreBackupResult(for error: Error) -> CryptoActionResult {
        guard case let RestoreBackupError.Failed(code, _) = error else {
            return .failed("Could not restore encryption backup.")
        }
        switch code {
        case "v-crypto.3-restore-rejected":
            return .failed("The recovery key or passphrase was rejected. Check it and try again.")
        case "v-crypto.3-recovery-secret-empty":
            return .failed("Enter your recovery key or passphrase.")
        case "p2-restore-backup-no-session":
            return .failed("Sign in to restore encryption backup.")
        case "v-crypto.3-restore-incomplete":
            return .failed("Native encryption backup could not be activated.")
        default:
            return .failed("Could not restore encryption backup.")
        }
    }

    func sessionDevices() async -> [SharedCoreSessionDevice] {
        let snapshot: DeviceSnapshotDto
        do {
            snapshot = try await SharedCoreDevices.deviceSnapshot(core: host.core)
        } catch {
            AppLogger().error(
                "device snapshot failed: \(String(describing: error))",
                category: .matrix
            )
            return []
        }
        return snapshot.devices.map {
            SharedCoreDevicesLive.devices(
                deviceId: $0.deviceId,
                displayName: $0.displayName,
                isCurrent: $0.isCurrent,
                trust: $0.trust,
                lastSeenTs: $0.lastSeenTs
            )
        }
    }

    func sessionDeviceUpdates() -> AsyncStream<Void> {
        AsyncStream { continuation in
            let task = Task {
                for await _ in host.livePoller.ownerSignals(families: ["devices"]) {
                    guard Task.isCancelled == false else {
                        break
                    }
                    continuation.yield(())
                }
                continuation.finish()
            }
            continuation.onTermination = { _ in
                task.cancel()
            }
        }
    }

    func signOutSession(deviceId: String, password: String) async -> CryptoActionResult {
        let trimmedPassword = password.trimmingCharacters(in: .whitespacesAndNewlines)
        guard deviceId.isEmpty == false, trimmedPassword.isEmpty == false else {
            return .failed("Enter your account password to sign out that session.")
        }
        do {
            let started = try await SharedCoreDevices.deviceDeleteStart(
                core: host.core,
                deviceIds: [deviceId]
            )
            if started.outcome == "complete" {
                return .completed("Session signed out.")
            }
            guard started.outcome == "authentication_required",
                  let challenge = started.challenge
            else {
                return .failed("Could not sign out that session.")
            }
            let finished = try await SharedCoreDevices.deviceDeletePassword(
                core: host.core,
                operationId: challenge.operationId,
                sessionGeneration: challenge.sessionGeneration,
                password: trimmedPassword
            )
            if finished.outcome == "complete" {
                return .completed("Session signed out.")
            }
            if finished.challenge?.authenticationFailed == true {
                return .failed("Check your password and try again.")
            }
            return .failed("Could not sign out that session.")
        } catch {
            return .failed("Could not sign out that session.")
        }
    }

    static func joinedRoomEncryption(
        roomID: String,
        rows: [JoinedRoomEncryptionRow]?
    ) -> SynaraRoomEncryptionStatus {
        guard let row = rows?.first(where: {
            $0.roomID == roomID && $0.membership == "join"
        }) else {
            return .unknown
        }
        return row.encryption
    }

    private func listEncryption(roomID: String) async -> SynaraRoomEncryptionStatus {
        guard let rooms = try? await SharedCoreRoomList.roomListSnapshot(core: host.core) else {
            // A failed joined-room read is not evidence that the room is clear;
            // preserve Unknown so callers cannot fall through to a Bool path.
            return .unknown
        }
        return Self.joinedRoomEncryption(
            roomID: roomID,
            rows: rooms.rooms.map {
                JoinedRoomEncryptionRow(
                    roomID: $0.roomId,
                    membership: $0.membership,
                    encryption: SharedCoreRoomListRows.encryptionStatus($0.encryptionStatus)
                )
            }
        )
    }

    func currentVerificationState() async -> CryptoVerificationState? {
        guard let inbox = try? await SharedCoreVerificationList.verificationList(core: host.core) else {
            return nil
        }
        guard let request = SharedCoreVerificationLive.selectRequest(
            from: inbox,
            preferring: resolvedFlowId()
        ) else {
            clearFlow()
            return nil
        }
        storeFlow(request.flowId)
        return SharedCoreVerificationLive.state(from: request)
    }

    private func runVerification(
        message: () async throws -> String
    ) async -> CryptoActionResult {
        do {
            return .completed(try await message())
        } catch {
            return .failed("Device verification is unavailable.")
        }
    }

    private func runVerification(
        requiresFlow: Bool,
        message: (String) async throws -> String
    ) async -> CryptoActionResult {
        _ = requiresFlow
        guard let flowId = resolvedFlowId() else {
            return .unavailable("Device verification is unavailable.")
        }
        do {
            return .completed(try await message(flowId))
        } catch {
            return .failed("Device verification is unavailable.")
        }
    }

    private func storeFlow(_ flowId: String) {
        flowLock.lock()
        self.flowId = flowId
        flowLock.unlock()
    }

    private func clearFlow() {
        flowLock.lock()
        flowId = nil
        flowLock.unlock()
    }

    private func resolvedFlowId() -> String? {
        flowLock.lock()
        defer { flowLock.unlock() }
        return flowId
    }
}

final class SharedCoreRoomManagementService: RoomManagementServicing {
    private let host: SharedCoreProductHost

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func createRoom(_ request: RoomCreateRequest) async throws -> RoomOperationResult {
        do {
            let dto = try await SharedCoreRoomCreate.roomCreate(
                core: host.core,
                request: RoomCreateRequestDto(
                    name: request.name,
                    topic: request.topic.isEmpty ? nil : request.topic,
                    roomAliasName: nil,
                    visibility: request.visibility == .public ? "public" : "private",
                    preset: nil,
                    isDirect: false,
                    encryption: request.isEncrypted,
                    invite: [],
                    roomVersion: nil,
                    joinRule: nil,
                    knock: false,
                    parentRoomId: nil
                )
            )
            return RoomOperationResult(roomID: dto.roomId, name: request.name)
        } catch {
            throw RoomManagementError.failed
        }
    }

    func createDirectMessage(_ request: DirectMessageCreateRequest) async throws -> RoomOperationResult {
        do {
            let dto = try await SharedCoreRoomCreate.roomCreate(
                core: host.core,
                request: RoomCreateRequestDto(
                    name: nil,
                    topic: nil,
                    roomAliasName: nil,
                    visibility: "private",
                    preset: "trusted_private_chat",
                    isDirect: true,
                    encryption: request.isEncrypted,
                    invite: [request.userID],
                    roomVersion: nil,
                    joinRule: nil,
                    knock: false,
                    parentRoomId: nil
                )
            )
            _ = try? await SharedCoreMDirect.mdirectAdd(
                core: host.core,
                roomId: dto.roomId,
                userId: request.userID
            )
            return RoomOperationResult(roomID: dto.roomId, name: request.userID)
        } catch {
            throw RoomManagementError.failed
        }
    }

    func joinRoom(_ request: RoomJoinRequest) async throws -> RoomOperationResult {
        do {
            _ = try await SharedCoreRoomLeaveJoin.roomJoin(
                core: host.core,
                roomIdOrAlias: request.reference,
                viaServers: nil
            )
            return RoomOperationResult(roomID: request.reference, name: nil)
        } catch {
            throw RoomManagementError.failed
        }
    }

    func leaveRoom(roomID: String) async throws {
        do {
            _ = try await SharedCoreRoomLeaveJoin.roomLeave(core: host.core, roomId: roomID)
        } catch {
            throw RoomManagementError.failed
        }
    }

    func setRoomFavorite(_ favorite: Bool, roomID: String) async throws {
        do {
            _ = try await SharedCoreRoomLeaveJoin.roomSetFavorite(
                core: host.core,
                roomId: roomID,
                favorite: favorite
            )
        } catch {
            throw RoomManagementError.failed
        }
    }

    func inviteUser(roomID: String, userID: String) async throws {
        do {
            _ = try await SharedCoreRoomModeration.roomInvite(
                core: host.core,
                roomId: roomID,
                userId: userID,
                reason: nil
            )
        } catch {
            throw RoomManagementError.failed
        }
    }

    func searchPublicRooms(query: String) async throws -> [PublicRoomSummary] {
        do {
            let page = try await SharedCoreDirectorySearch.roomDirectorySearch(
                core: host.core,
                sessionGeneration: 0,
                requestId: 1,
                serverName: nil,
                term: query,
                roomType: nil,
                thirdPartyInstanceId: nil,
                limit: 20,
                since: nil
            )
            return (page.page?.chunk ?? []).map { room in
                PublicRoomSummary(
                    id: room.roomId,
                    name: room.name ?? room.roomId,
                    topic: room.topic,
                    alias: room.canonicalAlias,
                    memberCount: Int(room.memberCount),
                    isWorldReadable: room.worldReadable
                )
            }
        } catch {
            throw RoomManagementError.failed
        }
    }

    func roomDetails(roomID: String) async -> RoomDetails? {
        let ownUserID = await coreSessionUserID()
        let list: RoomListSnapshotDto?
        let roomListReadFailed: Bool
        do {
            list = try await SharedCoreRoomList.roomListSnapshot(core: host.core)
            roomListReadFailed = false
        } catch {
            list = nil
            roomListReadFailed = true
        }
        let room = list?.rooms.first(where: {
            $0.roomId == roomID && $0.membership == "join"
        })
        let members = try? await SharedCoreRoomMembersSnapshots.roomMembersSnapshot(
            core: host.core,
            roomId: roomID
        )
        let power = try? await SharedCoreRoomMembersSnapshots.roomPowerLevelsSnapshot(
            core: host.core,
            roomId: roomID
        )
        let generation = list?.sessionGeneration ?? members?.sessionGeneration ?? 0
        let join = try? await SharedCoreJoinRules.roomJoinRuleSnapshot(
            core: host.core,
            roomId: roomID,
            sessionGeneration: generation
        )
        let invite = (try? await SharedCoreInvites.invitesSnapshot(core: host.core))?
            .invites
            .first(where: { $0.roomId == roomID })
        let notification = try? await SharedCoreRoomNotification.snapshot(
            core: host.core,
            roomId: roomID
        )
        let encryptionStatus: SynaraRoomEncryptionStatus
        if let room {
            encryptionStatus = SharedCoreRoomListRows.encryptionStatus(room.encryptionStatus)
        } else if roomListReadFailed {
            encryptionStatus = .unavailable
        } else if let invite {
            encryptionStatus = SharedCoreSessionCrypto.encryption(invite.isEncrypted)
        } else {
            encryptionStatus = .unknown
        }
        return SharedCoreRoomDetails.details(
            roomID: roomID,
            ownUserID: ownUserID,
            room: room.map {
                SharedCoreRoomDetails.RoomRow(
                    roomId: $0.roomId,
                    name: $0.name,
                    canonicalAlias: $0.canonicalAlias,
                    avatarUrl: $0.avatarUrl
                )
            },
            members: members?.members.map {
                SharedCoreRoomDetails.MemberRow(
                    userId: $0.userId,
                    membership: $0.membership,
                    powerLevel: Int($0.powerLevel)
                )
            } ?? [],
            powerLevelsJSON: power?.contentJson,
            joinRule: join?.joinRule,
            topic: invite?.roomTopic,
            encryptionStatus: encryptionStatus,
            notificationMode: SharedCoreRoomDetails.notificationMode(
                notification?.mode ?? room?.notificationMode
            )
        )
    }

    func stickers(roomID: String) async -> [SharedCoreSticker] {
        var rows: [SharedCoreSticker] = []
        if let user = try? await SharedCoreImagePacks.getUserImagePack(core: host.core),
           let pack = user.pack
        {
            rows.append(contentsOf: SharedCoreImagePackRows.stickers(
                packId: pack.id,
                packName: nil,
                contentJSON: pack.contentJson
            ))
        }
        if let room = try? await SharedCoreImagePacks.getRoomImagePacks(core: host.core, roomId: roomID) {
            for pack in room.packs {
                rows.append(contentsOf: SharedCoreImagePackRows.stickers(
                    packId: pack.id,
                    packName: nil,
                    contentJSON: pack.contentJson
                ))
            }
        }
        if let global = try? await SharedCoreImagePacks.getGlobalImagePacks(core: host.core) {
            for pack in global.packs {
                rows.append(contentsOf: SharedCoreImagePackRows.stickers(
                    packId: pack.id,
                    packName: nil,
                    contentJSON: pack.contentJson
                ))
            }
        }
        return rows
    }

    private func coreSessionUserID() async -> String? {
        guard let snapshot = try? await SharedCoreSessionStatus.sessionSnapshot(core: host.core) else {
            return nil
        }
        return snapshot.userId
    }

    func updateRoomProfile(_ request: RoomProfileUpdateRequest) async throws {
        if let name = request.name {
            _ = try await SharedCoreRoomProfile.setRoomName(
                core: host.core,
                roomId: request.roomID,
                name: name
            )
        }
        if let topic = request.topic {
            _ = try await SharedCoreRoomProfile.setRoomTopic(
                core: host.core,
                roomId: request.roomID,
                topic: topic
            )
        }
        switch request.avatar {
        case let .upload(data, mimeType):
            let uploaded = try await SharedCoreMediaSend.uploadContent(
                core: host.core,
                payload: data,
                mimeType: mimeType,
                filename: nil
            )
            _ = try await SharedCoreRoomProfile.setRoomAvatar(
                core: host.core,
                roomId: request.roomID,
                mxc: uploaded.mxc
            )
        case .remove:
            _ = try await SharedCoreRoomProfile.setRoomAvatar(
                core: host.core,
                roomId: request.roomID,
                mxc: ""
            )
        case nil:
            break
        }
    }

    func setNotificationMode(_ mode: SynaraRoomNotificationMode, roomID: String) async throws {
        do {
            try await SharedCoreRoomNotification.set(
                core: host.core,
                roomId: roomID,
                mode: SharedCoreRoomDetails.wireNotificationMode(mode)
            )
        } catch {
            throw RoomManagementError.failed
        }
    }
}

final class SharedCoreMediaLoader: MediaLoading {
    private let host: SharedCoreProductHost

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func loadThumbnail(for resource: MediaResource) async -> MediaLoadState {
        guard resource.isEncrypted == false else {
            return .failed("Encrypted media requires recovered keys before it can be opened.")
        }
        if SharedCoreTimelineMedia.handleId(from: resource.authenticatedURL) != nil {
            return .thumbnail(resource)
        }
        guard let url = resource.authenticatedURL, url.scheme == "mxc" else {
            return .failed("Media is unavailable.")
        }
        do {
            let bytes = try await SharedCorePlainMedia.thumbnail(
                core: host.core,
                contentUri: url.absoluteString,
                width: 640,
                height: 480
            )
            guard bytes.isEmpty == false else {
                return .failed("Media could not be loaded.")
            }
            return .thumbnail(resource)
        } catch {
            return .failed("Media could not be loaded.")
        }
    }

    func loadThumbnailData(for resource: MediaResource, width: UInt64, height: UInt64) async -> Data? {
        guard resource.isEncrypted == false else {
            return nil
        }
        if let handle = SharedCoreTimelineMedia.handleId(from: resource.authenticatedURL) {
            return try? await SharedCoreTimelineMedia.mediaBytes(core: host.core, handleId: handle)
        }
        guard let url = resource.authenticatedURL, url.scheme == "mxc" else {
            return nil
        }
        return try? await SharedCorePlainMedia.thumbnail(
            core: host.core,
            contentUri: url.absoluteString,
            width: width,
            height: height
        )
    }

    func loadMediaData(for resource: MediaResource) async -> Data? {
        guard resource.isEncrypted == false else {
            return nil
        }
        if let handle = SharedCoreTimelineMedia.handleId(from: resource.authenticatedURL) {
            return try? await SharedCoreTimelineMedia.mediaBytes(core: host.core, handleId: handle)
        }
        guard let url = resource.authenticatedURL, url.scheme == "mxc" else {
            return nil
        }
        return try? await SharedCorePlainMedia.download(
            core: host.core,
            contentUri: url.absoluteString
        )
    }
}

final class SharedCoreMediaUploadService: MediaUploading {
    private let host: SharedCoreProductHost

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func upload(_ request: MediaUploadRequest) async -> MediaUploadState {
        guard case .signedIn = host.sessionStore.currentState else {
            return .failed("Sign in before uploading media.")
        }
        guard request.data.isEmpty == false else {
            return .failed("Attachment is empty.")
        }
        do {
            let result = try await SharedCoreMediaSend.sendRoomAttachment(
                core: host.core,
                roomId: request.roomID,
                filename: request.displayName,
                mimeType: request.mimeType,
                payload: request.data,
                caption: request.caption,
                formattedCaption: request.formattedCaption,
                replyTo: request.replyToEventID,
                threadRoot: request.threadRootEventID,
                transactionId: request.transactionID,
                mentionUserIds: request.mentionUserIDs,
                mentionRoom: request.mentionRoom
            )
            let safeName = URL(fileURLWithPath: request.displayName).lastPathComponent
            let resource = MediaResource(
                id: result.eventId,
                filename: safeName.isEmpty ? "Attachment" : safeName,
                caption: request.caption,
                formattedCaption: request.formattedCaption,
                authenticatedURL: URL(string: "mxc://local/upload"),
                requiresAuthentication: true,
                mimeType: request.mimeType,
                byteSize: UInt64(request.data.count)
            )
            let senderID = await coreSessionUserID() ?? ""
            let item = TimelineItem(
                id: result.eventId,
                eventID: result.eventId,
                serverEventID: result.eventId,
                senderID: senderID,
                timestamp: Date(),
                kind: .mediaPlaceholder(resource),
                replyToEventID: request.replyToEventID,
                threadRootEventID: request.threadRootEventID,
                isEdited: false,
                reactions: [:]
            )
            return .uploaded(item)
        } catch {
            return .failed("Media could not be uploaded.")
        }
    }

    private func coreSessionUserID() async -> String? {
        guard let snapshot = try? await SharedCoreSessionStatus.sessionSnapshot(core: host.core) else {
            return nil
        }
        return snapshot.userId
    }
}

struct SharedCoreSparsePushRouteResolver: SparsePushRouteResolving {
    func resolveRoute(eventID: String) async -> AppRoute? {
        _ = eventID
        return nil
    }
}

final class SharedCorePusherService: MatrixPusherServicing {
    typealias OwnerBinder = (AuthenticatedSession) throws -> SharedCoreHttpPusherOwning

    private let gatewayURL: URL?
    private let appID: String
    private let logger: LoggingServicing
    private let ownerBinder: OwnerBinder

    var isGatewayConfigured: Bool {
        gatewayURL != nil
    }

    var configuredGatewayURL: URL? {
        gatewayURL
    }

    init(
        host: SharedCoreProductHost,
        appID: String = "com.whylandcreative.synara",
        gatewayURL: URL? = nil,
        logger: LoggingServicing = AppLogger(),
        ownerBinder: OwnerBinder? = nil
    ) {
        self.appID = appID
        self.gatewayURL = gatewayURL
        self.logger = logger
        self.ownerBinder = ownerBinder ?? { session in
            try SharedCoreHttpPusher.bind(
                core: host.core,
                userID: session.userID,
                deviceID: session.deviceID,
                homeserverURL: session.homeserverURL.absoluteString
            )
        }
    }

    func bindPusher(to session: AuthenticatedSession) throws -> MatrixPusherAccountServicing {
        let owner = try ownerBinder(session)
        return SharedCoreBoundPusherService(
            owner: owner,
            appID: appID,
            gatewayURL: gatewayURL,
            logger: logger
        )
    }
}

private final class SharedCoreBoundPusherService: MatrixPusherAccountServicing {
    private let owner: SharedCoreHttpPusherOwning
    private let appID: String
    private let gatewayURL: URL?
    private let logger: LoggingServicing

    init(
        owner: SharedCoreHttpPusherOwning,
        appID: String,
        gatewayURL: URL?,
        logger: LoggingServicing
    ) {
        self.owner = owner
        self.appID = appID
        self.gatewayURL = gatewayURL
        self.logger = logger
    }

    func registerPusher(pushKey: String) async throws {
        guard let gatewayURL else {
            logger.info("Push gateway URL is not configured; skipping pusher registration", category: .push)
            return
        }
        _ = try await SharedCoreHttpPusher.register(
            owner: owner,
            pushKey: pushKey,
            appId: appID,
            gatewayUrl: gatewayURL.absoluteString,
            appDisplayName: "Synara",
            lang: "en-US"
        )
    }

    func unregisterPusher(pushKey: String) async throws {
        _ = try await SharedCoreHttpPusher.delete(
            owner: owner,
            pushKey: pushKey,
            appId: appID
        )
    }

    func unregisterAllPushersForDevice(lastPushKey: String?) async throws {
        _ = try await SharedCoreHttpPusher.deleteForDevice(
            owner: owner,
            appId: appID,
            lastPushKey: lastPushKey
        )
    }
}

final class SharedCoreRoomReadMarkerService: RoomReadMarkerServicing {
    private let host: SharedCoreProductHost

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func fullyReadEventID(roomID: String) async -> String? {
        await withOpenLive(roomID: roomID) { opened in
            SharedCoreReadMarkers.acknowledgedEventID(
                ownReadEventID: opened.snapshot.unreadAnchorEventId ?? opened.snapshot.ownReadEventId,
                rowEventIDs: opened.snapshot.rows.map(\.eventId)
            )
        }
    }

    func markFullyRead(roomID: String, eventID: String) async -> Bool {
        guard MatrixServerEventIDPolicy.canAcknowledge(eventID) else {
            return false
        }
        return await withOpenLive(roomID: roomID) { opened in
            let readback = try? await SharedCoreTimelineReadState.timelineSetReadState(
                core: self.host.core,
                streamId: opened.streamId,
                action: "mark_read",
                intent: "automatic_visibility",
                observedLiveTailEventId: eventID
            )
            return readback?.receiptSent == true
                && readback?.acknowledgedEventId == eventID
        } ?? false
    }

    func markRoomAsRead(roomID: String) async -> String? {
        return await withOpenLive(roomID: roomID) { opened in
            let readback = try? await SharedCoreTimelineReadState.timelineSetReadState(
                core: host.core,
                streamId: opened.streamId,
                action: "mark_read",
                intent: "explicit_user"
            )
            return readback?.acknowledgedEventId
        }
    }

    private func withOpenLive<T>(
        roomID: String,
        body: (TimelineOpenDto) async -> T?
    ) async -> T? {
        let position = TimelineOpenPositionDto(
            kind: "live",
            atBottom: true,
            restoredAnchorEventId: nil,
            liveTailEventId: nil,
            updatedAtMs: nil,
            eventId: nil
        )
        guard let opened = try? await SharedCoreTimeline.timelineOpen(
            core: host.core,
            roomId: roomID,
            position: position
        ) else {
            return nil
        }
        let result = await body(opened)
        _ = try? await SharedCoreTimeline.timelineClose(core: host.core, streamId: opened.streamId)
        return result
    }
}
