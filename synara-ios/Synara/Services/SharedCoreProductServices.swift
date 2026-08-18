import Foundation
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
        let root = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("SynaraCore", isDirectory: true)
        try? FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        return root
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
    private(set) var syncStatus: MatrixSyncStatus = .stopped

    var syncStatusDescription: String {
        syncStatus.description
    }

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func start(session: AuthenticatedSession) async {
        await applyLiveSession(session)
    }

    func warmSync(session: AuthenticatedSession) async {
        _ = session
    }

    func revokeServerSession(_ session: AuthenticatedSession) async -> Bool {
        _ = session
        do {
            _ = try await SharedCoreLeftovers.logout(core: host.core)
            return true
        } catch {
            return false
        }
    }

    func stop() async {
        syncStatus = .stopped
    }

    func pauseForBackground() async {}

    func resumeFromForeground(session: AuthenticatedSession) async {
        await applyLiveSession(session)
    }

    private func applyLiveSession(_ session: AuthenticatedSession) async {
        syncStatus = .starting
        let outcome = await SharedCoreSessionBootstrap.prepareLiveSession(
            userID: session.userID,
            homeserverURL: session.homeserverURL.absoluteString,
            storeRoot: host.storeRoot,
            core: host.core
        )
        if outcome.started {
            syncStatus = .syncing
        } else if outcome.attached || outcome.restored {
            syncStatus = .starting
        } else {
            syncStatus = .stopped
        }
    }

    func syncForBackgroundNotification(session: AuthenticatedSession) async -> Bool {
        _ = session
        return false
    }

    func resetLocalState(for session: AuthenticatedSession?) async {
        _ = session
        _ = try? await SharedCoreLeftovers.wipePersistedStores(
            core: host.core,
            storeRoot: host.storeRoot.path
        )
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
                        lastMessagePreview: $0.lastMessagePreview
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
                hasHighlight: room.hasHighlight
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

final class SharedCoreTimelineService: TimelineServicing {
    private let host: SharedCoreProductHost
    private let streamLock = NSLock()
    private var streams: [String: String] = [:]

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
            storeStream(opened.streamId, for: roomID)
            return SharedCoreTimelineRows.outcome(from: opened.snapshot.rows)
        } catch {
            return .empty
        }
    }

    func loadOlderTimeline(roomID: String, before eventID: String) async -> TimelineLoadOutcome {
        _ = eventID
        guard let streamId = stream(for: roomID) else {
            return .empty
        }
        do {
            let snapshot = try await SharedCoreTimeline.timelinePaginate(
                core: host.core,
                streamId: streamId,
                direction: "backwards"
            )
            return SharedCoreTimelineRows.outcome(from: snapshot.rows)
        } catch {
            return .empty
        }
    }

    func timelineUpdates(roomID: String, focusedEventID: String?) -> AsyncStream<TimelineLoadOutcome> {
        // Register before the task begins so an SDK update cannot be lost
        // between the initial snapshot and the first live readback.
        let signals = host.livePoller.timelineSignals(roomId: roomID)
        return AsyncStream { continuation in
            let task = Task {
                if SharedCoreTimelineUpdateBootstrap.shouldRefreshOpenStream(
                    focusedEventID: focusedEventID
                ) {
                    continuation.yield(
                        await refreshOpenTimeline(roomID: roomID, focusedEventID: focusedEventID)
                    )
                }
                for await update in signals {
                    guard Task.isCancelled == false else {
                        break
                    }
                    let watchingStreamId = stream(for: roomID)
                    guard SharedCoreTimelineLiveRefresh.shouldRefresh(
                        watchingRoomID: roomID,
                        watchingStreamId: watchingStreamId,
                        updateRoomId: update.roomId,
                        updateStreamId: update.streamId
                    ) else {
                        continue
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
        if let streamId = stream(for: roomID) {
            do {
                let snapshot = try await SharedCoreTimeline.timelineSnapshot(
                    core: host.core,
                    streamId: streamId
                )
                return SharedCoreTimelineRows.outcome(from: snapshot.rows)
            } catch {
                return await loadInitialTimeline(roomID: roomID, focusedEventID: focusedEventID)
            }
        }
        return await loadInitialTimeline(roomID: roomID, focusedEventID: focusedEventID)
    }

    private func stream(for roomID: String) -> String? {
        streamLock.lock()
        defer { streamLock.unlock() }
        return streams[roomID]
    }

    private func storeStream(_ streamId: String, for roomID: String) {
        streamLock.lock()
        streams[roomID] = streamId
        streamLock.unlock()
    }

    private func takeStream(for roomID: String) -> String? {
        streamLock.lock()
        defer { streamLock.unlock() }
        return streams.removeValue(forKey: roomID)
    }

    private func takeAllStreams() -> [String] {
        streamLock.lock()
        defer { streamLock.unlock() }
        let values = Array(streams.values)
        streams.removeAll()
        return values
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
                    threadRoot: nil,
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
                isEdited: request.editEventID != nil,
                reactions: [:]
            )
        } catch {
            throw MessageSendError.failed
        }
    }

    func sendSticker(_ request: StickerSendRequest) async throws -> TimelineItem {
        let body = request.body.trimmingCharacters(in: .whitespacesAndNewlines)
        let mxc = request.mxc.trimmingCharacters(in: .whitespacesAndNewlines)
        guard body.isEmpty == false, mxc.hasPrefix("mxc://") else {
            throw MessageSendError.failed
        }
        do {
            _ = try await SharedCoreSendSticker.sendSticker(
                core: host.core,
                roomId: request.roomID,
                body: body,
                mxc: mxc,
                width: request.width,
                height: request.height,
                mimetype: request.mimetype,
                size: request.size,
                replyTo: request.replyToEventID,
                threadRoot: request.threadRoot
            )
            return TimelineItem(
                id: "$local-\(UUID().uuidString)",
                eventID: "$local-\(UUID().uuidString)",
                senderID: signedInUserID(),
                timestamp: Date(),
                kind: .unknown(type: "sticker"),
                replyToEventID: request.replyToEventID,
                isEdited: false,
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

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func availability(for item: TimelineItem, currentUserID: String) -> EventActionAvailability {
        MockEventActionService().availability(for: item, currentUserID: currentUserID)
    }

    func apply(
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
                _ = try await SharedCoreTimelineMutate.timelineRedact(
                    core: host.core,
                    roomId: roomID,
                    eventId: item.eventID,
                    reason: nil
                )
            } catch {
                throw EventActionError.failed
            }
            return item
        case .react(let reaction):
            do {
                _ = try await SharedCoreTimelineReactions.timelineReactionToggle(
                    core: host.core,
                    roomId: roomID,
                    eventId: item.eventID,
                    key: reaction
                )
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

final class SharedCoreAgentApprovalReactionService: AgentApprovalReactionServicing {
    private let host: SharedCoreProductHost

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func submitReaction(_ request: SynaraAgentApprovalReactionRequest) async throws {
        guard case .signedIn = host.sessionStore.currentState else {
            throw SynaraAgentApprovalError.signedOut
        }
        do {
            _ = try await SharedCoreTimelineReactions.reactionEnsure(
                core: host.core,
                roomId: request.roomID,
                eventId: request.sourceEventID,
                key: request.reactionKey
            )
        } catch let error as SynaraAgentApprovalError {
            throw error
        } catch {
            throw SynaraAgentApprovalError.failed
        }
    }
}

final class SharedCoreCryptoStatusService: CryptoStatusServicing {
    private let host: SharedCoreProductHost
    private let flowLock = NSLock()
    private var flowId: String?

    init(host: SharedCoreProductHost) {
        self.host = host
    }

    func roomStatus(roomID: String) async -> RoomCryptoStatus {
        let session = await sessionStatus()
        let listEncrypted = await listEncryption(roomID: roomID)
        let inviteEncrypted = await inviteEncryption(roomID: roomID)
        let isEncrypted = listEncrypted ?? inviteEncrypted
        guard session != .unknown || isEncrypted != nil else {
            return .unknown
        }
        return SharedCoreSessionCrypto.roomStatus(
            isEncrypted: isEncrypted,
            session: session
        )
    }

    func sessionStatus() async -> SessionCryptoStatus {
        let crypto = try? await SharedCoreLeftovers.cryptoStatus(core: host.core)
        let backup = try? await SharedCoreLeftovers.backupStatus(core: host.core)
        let secretStorage = try? await SharedCoreSessionStatus.secretStorageStatus(core: host.core)
        guard crypto != nil || backup != nil || secretStorage != nil else {
            return .unknown
        }
        return SharedCoreSessionCrypto.status(
            crossSigningState: crypto?.crossSigningState,
            backupEnabled: backup?.enabled,
            backupAvailability: backup?.availability,
            backupDeviceState: backup?.deviceState,
            recoveryState: backup?.recoveryState,
            secretStorageState: secretStorage?.state
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

    func requestDeviceVerification() async -> CryptoActionResult {
        await runVerification {
            let dto = try await SharedCoreVerificationSas.verificationStart(
                core: host.core,
                deviceId: nil
            )
            storeFlow(dto.flowId)
            return "Device verification request sent."
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
        do {
            _ = try await SharedCoreLeftovers.recover(core: host.core, recoveryKey: recoveryKey)
            return .completed("Recovery completed.")
        } catch {
            return .failed("Recovery is unavailable.")
        }
    }

    func sessionDevices() async -> [SharedCoreSessionDevice] {
        guard let snapshot = try? await SharedCoreDevices.deviceSnapshot(core: host.core) else {
            return []
        }
        return snapshot.devices.map {
            SharedCoreDevicesLive.devices(
                deviceId: $0.deviceId,
                displayName: $0.displayName,
                isCurrent: $0.isCurrent,
                trust: $0.trust
            )
        }
    }

    private func listEncryption(roomID: String) async -> Bool? {
        guard let rooms = try? await SharedCoreRoomList.roomListSnapshot(core: host.core) else {
            return nil
        }
        return rooms.rooms.first { $0.roomId == roomID }?.isEncrypted
    }

    private func inviteEncryption(roomID: String) async -> Bool? {
        guard let invites = try? await SharedCoreInvites.invitesSnapshot(core: host.core) else {
            return nil
        }
        return invites.invites.first { $0.roomId == roomID }?.isEncrypted
    }

    private func currentVerificationState() async -> CryptoVerificationState? {
        guard let inbox = try? await SharedCoreVerificationList.verificationList(core: host.core) else {
            return nil
        }
        if let first = inbox.requests.first {
            storeFlow(first.flowId)
        }
        return SharedCoreVerificationLive.state(from: inbox)
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
        let list = try? await SharedCoreRoomList.roomListSnapshot(core: host.core)
        let room = list?.rooms.first(where: { $0.roomId == roomID })
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
            isEncrypted: room?.isEncrypted ?? invite?.isEncrypted ?? false,
            notificationMode: SharedCoreRoomDetails.notificationMode(room?.notificationMode)
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
        if case .remove = request.avatar {
            _ = try await SharedCoreRoomProfile.setRoomAvatar(
                core: host.core,
                roomId: request.roomID,
                mxc: ""
            )
        }
    }

    func setNotificationMode(_ mode: SynaraRoomNotificationMode, roomID: String) async throws {
        do {
            _ = try await SharedCoreLeftovers.setNotificationMode(
                core: host.core,
                roomId: roomID,
                mode: mode.rawValue
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
        guard let url = resource.authenticatedURL else {
            return .failed("Media is unavailable.")
        }
        do {
            _ = try await SharedCoreLeftovers.mediaThumbnail(
                core: host.core,
                mxc: url.absoluteString,
                width: 640,
                height: 480
            )
            return .thumbnail(resource)
        } catch {
            return .failed("Media could not be loaded.")
        }
    }

    func loadThumbnailData(for resource: MediaResource, width: UInt64, height: UInt64) async -> Data? {
        _ = (width, height)
        return await loadMediaData(for: resource)
    }

    func loadMediaData(for resource: MediaResource) async -> Data? {
        guard resource.isEncrypted == false else {
            return nil
        }
        if let handle = SharedCoreTimelineMedia.handleId(from: resource.authenticatedURL) {
            return try? await SharedCoreTimelineMedia.mediaBytes(core: host.core, handleId: handle)
        }
        guard let url = resource.authenticatedURL else {
            return nil
        }
        return try? await SharedCoreLeftovers.mediaDownload(
            core: host.core,
            mxc: url.absoluteString
        ).payload
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
            _ = try await SharedCoreLeftovers.mediaUpload(
                core: host.core,
                payload: request.data,
                mimeType: request.mimeType,
                filename: request.displayName
            )
            return .failed("Media upload is unavailable.")
        } catch {
            return .failed("Media could not be uploaded.")
        }
    }
}

struct SharedCoreSparsePushRouteResolver: SparsePushRouteResolving {
    func resolveRoute(eventID: String) async -> AppRoute? {
        _ = eventID
        return nil
    }
}

final class SharedCorePusherService: MatrixPusherServicing {
    private let host: SharedCoreProductHost
    private let gatewayURL: URL?
    private let appID: String
    private let logger: LoggingServicing

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
        logger: LoggingServicing = AppLogger()
    ) {
        self.host = host
        self.appID = appID
        self.gatewayURL = gatewayURL
        self.logger = logger
    }

    func registerPusher(session: AuthenticatedSession, pushKey: String) async throws {
        guard let gatewayURL else {
            logger.info("Push gateway URL is not configured; skipping pusher registration", category: .push)
            return
        }
        _ = try await SharedCoreLeftovers.pusherSet(
            core: host.core,
            pushKey: pushKey,
            appId: appID,
            gatewayUrl: gatewayURL.absoluteString,
            appDisplayName: "Synara",
            deviceDisplayName: session.deviceID,
            lang: "en-US"
        )
    }

    func unregisterPusher(session: AuthenticatedSession, pushKey: String) async throws {
        _ = session
        _ = try await SharedCoreLeftovers.pusherDelete(
            core: host.core,
            pushKey: pushKey,
            appId: appID
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
                ownReadEventID: opened.snapshot.ownReadEventId,
                rowEventIDs: opened.snapshot.rows.map(\.eventId)
            )
        }
    }

    func markFullyRead(roomID: String, eventID: String) async -> Bool {
        guard MatrixServerEventIDPolicy.canAcknowledge(eventID) else {
            return false
        }
        return await markRoomAsRead(roomID: roomID) != nil
    }

    func markRoomAsRead(roomID: String) async -> String? {
        await withOpenLive(roomID: roomID) { opened in
            let readback = try? await SharedCoreTimelineReadState.timelineSetReadState(
                core: host.core,
                streamId: opened.streamId,
                action: "mark_read"
            )
            return SharedCoreReadMarkers.acknowledgedEventID(
                ownReadEventID: readback?.snapshot.ownReadEventId ?? opened.snapshot.ownReadEventId,
                rowEventIDs: (readback?.snapshot.rows ?? opened.snapshot.rows).map(\.eventId)
            )
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
