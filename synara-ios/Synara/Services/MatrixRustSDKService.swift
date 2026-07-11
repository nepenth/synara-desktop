import Foundation
@preconcurrency import MatrixRustSDK

private final class SynaraUnableToDecryptRecorder: UnableToDecryptDelegate, @unchecked Sendable {
    private let lock = NSLock()
    private var count = 0

    var unableToDecryptCount: Int {
        lock.lock()
        defer { lock.unlock() }
        return count
    }

    func onUtd(info: UnableToDecryptInfo) {
        lock.lock()
        defer { lock.unlock() }
        count += 1
    }

    func reset() {
        lock.lock()
        defer { lock.unlock() }
        count = 0
    }
}

private final class SynaraSessionVerificationDelegate: SessionVerificationControllerDelegate, @unchecked Sendable {
    private let clientStore: MatrixRustSDKClientStore

    init(clientStore: MatrixRustSDKClientStore) {
        self.clientStore = clientStore
    }

    func didReceiveVerificationRequest(details: SessionVerificationRequestDetails) {
        Task {
            await clientStore.handleVerificationRequest(details: details)
        }
    }

    func didAcceptVerificationRequest() {
        Task {
            await clientStore.handleVerificationAccepted()
        }
    }

    func didStartSasVerification() {
        Task {
            await clientStore.handleVerificationSasStarted()
        }
    }

    func didReceiveVerificationData(data: SessionVerificationData) {
        Task {
            await clientStore.handleVerificationData(data)
        }
    }

    func didFail() {
        Task {
            await clientStore.handleVerificationFailed()
        }
    }

    func didCancel() {
        Task {
            await clientStore.handleVerificationCancelled()
        }
    }

    func didFinish() {
        Task {
            await clientStore.handleVerificationFinished()
        }
    }
}

enum MatrixRustSDKTimelineMessageMapper {
    static func mapMessageLike(
        _ content: MsgLikeContent,
        eventID: String,
        eventTypeRaw: String?,
        isEncrypted: Bool
    ) -> TimelineItem.Kind {
        switch content.kind {
        case .message(let message):
            if let agentCard = SynaraAgentCardPayloadParser.parse(body: message.body) {
                return .agentCard(agentCard)
            }
            switch message.msgType {
            case .text(let content):
                return mapTextMessage(body: content.body, formatted: content.formatted)
            case .notice(let content):
                return mapTextMessage(body: content.body, formatted: content.formatted)
            case .emote(let content):
                return mapTextMessage(body: content.body, formatted: content.formatted)
            case .other(_, let body):
                return .text(body)
            case .image(let content):
                return mapMediaPlaceholder(
                    eventID: eventID,
                    filename: content.filename,
                    source: content.source,
                    mimeType: content.info?.mimetype,
                    byteSize: content.info?.size,
                    isEncrypted: isEncrypted
                )
            case .file(let content):
                return mapMediaPlaceholder(
                    eventID: eventID,
                    filename: content.filename,
                    source: content.source,
                    mimeType: content.info?.mimetype,
                    byteSize: content.info?.size,
                    isEncrypted: isEncrypted
                )
            case .audio, .video, .gallery, .location:
                return .text(message.body)
            }
        case .unableToDecrypt:
            return .encryptedPlaceholder
        case .redacted:
            return .redacted
        case .sticker(let body, _, _):
            return .text(body)
        case .other(let eventType):
            return eventTypeRaw == "m.room.encrypted" ? .encryptedPlaceholder : .unknown(type: "\(eventType)")
        case .poll, .liveLocation:
            return .unknown(type: eventTypeRaw ?? "m.room.message")
        }
    }

    private static func mapTextMessage(body: String, formatted: FormattedBody?) -> TimelineItem.Kind {
        guard let formatted,
              case .html = formatted.format,
              formatted.body.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false else {
            return .text(body)
        }

        return .formattedText(body: body, html: formatted.body)
    }

    private static func mapMediaPlaceholder(
        eventID: String,
        filename: String,
        source: MediaSource,
        mimeType: String?,
        byteSize: UInt64?,
        isEncrypted: Bool
    ) -> TimelineItem.Kind {
        .mediaPlaceholder(
            MediaResource(
                id: eventID,
                filename: filename,
                authenticatedURL: URL(string: source.url()),
                requiresAuthentication: true,
                isEncrypted: isEncrypted,
                mimeType: mimeType,
                byteSize: byteSize
            )
        )
    }
}

actor MatrixRustSDKClientStore {
    /// Bump when the on-disk Matrix SDK store layout or compatibility requirements change.
    static let persistedStoreSchemaVersion = 2
    private static let platformDeviceDisplayName = "Synara iOS"
    private let logger: LoggingServicing
    private var client: Client?
    private var activeSession: AuthenticatedSession?
    private var syncService: SyncService?
    private var roomListService: RoomListService?
    private var syncStatus: MatrixSyncStatus = .stopped
    private let unableToDecryptRecorder = SynaraUnableToDecryptRecorder()
    private var verificationController: SessionVerificationController?
    private var verificationDelegate: SynaraSessionVerificationDelegate?
    private var verificationContinuations: [UUID: AsyncStream<CryptoVerificationState>.Continuation] = [:]
    private var lastVerificationState: CryptoVerificationState?
    private var retainedClientHandles: [Client] = []
    private var retainedRoomHandlesByID: [String: Room] = [:]
    /// Serializes client creation, restoration, and teardown. Actors allow reentrancy
    /// across `await`, so concurrent ensure/reset calls could otherwise free the Rust
    /// client while another task still reads rooms from it.
    private var isMutatingClient = false
    private var clientMutationWaiters: [CheckedContinuation<Void, Never>] = []
    private var isClientPaused = false
    private var roomListSyncGeneration: UInt64 = 0

    init(logger: LoggingServicing = AppLogger()) {
        self.logger = logger
    }

    var syncStatusDescription: String {
        syncStatus.description
    }

    func currentSyncStatus() -> MatrixSyncStatus {
        syncStatus
    }

    func login(_ request: LoginRequest) async throws -> AuthenticatedSession {
        let username = request.username.trimmingCharacters(in: .whitespacesAndNewlines)
        guard username.isEmpty == false else {
            throw LoginError.missingUsername
        }

        guard request.password.isEmpty == false else {
            throw LoginError.missingPassword
        }

        await acquireClientMutationLock()
        defer { releaseClientMutationLock() }

        if activeSession != nil {
            await detachSyncServices()
            retainClientHandle(client)
            client = nil
            activeSession = nil
            syncService = nil
            roomListService = nil
            unableToDecryptRecorder.reset()
        }

        let storeID = Self.storeID(for: username, homeserverURL: request.homeserverURL)
        try? Self.deletePersistedStore(storeID: storeID)
        let client = try await buildClient(homeserverURL: request.homeserverURL, storeID: storeID)

        do {
            unableToDecryptRecorder.reset()
            try await installUnableToDecryptDelegate(on: client)
            try await client.login(
                username: username,
                password: request.password,
                initialDeviceName: "Synara iOS",
                deviceId: nil
            )
            await client.encryption().waitForE2eeInitializationTasks()

            let sdkSession = try client.session()
            let availableSlidingSyncVersions = await client.availableSlidingSyncVersions()
            let slidingSyncVersion = MatrixSlidingSyncCompatibility.storedRawValue(
                reported: sdkSession.slidingSyncVersion,
                available: availableSlidingSyncVersions
            )
            let session = AuthenticatedSession(
                userID: sdkSession.userId,
                deviceID: sdkSession.deviceId,
                homeserverURL: URL(string: sdkSession.homeserverUrl) ?? request.homeserverURL,
                accessToken: sdkSession.accessToken,
                refreshToken: sdkSession.refreshToken,
                slidingSyncVersion: slidingSyncVersion,
                sdkStoreID: storeID
            )
            self.client = client
            self.activeSession = session
            self.syncService = nil
            self.roomListService = nil
            await ensurePlatformDeviceDisplayName(session: session)
            self.syncStatus = .syncing
            return session
        } catch let error as ClientError {
            retainClientHandle(client)
            self.syncStatus = .failed("Could not sign in.")
            logger.error("Password login SDK client error: \(String(describing: error))", category: .auth)
            throw Self.mapLoginError(error)
        } catch {
            retainClientHandle(client)
            self.syncStatus = .failed("Could not sign in.")
            logger.error("Password login SDK error: \(String(describing: error))", category: .auth)
            // TODO: Map additional non-ClientError login failures when the SDK exposes stable types.
            throw LoginError.networkFailure
        }
    }

    func warmSync(session: AuthenticatedSession) async throws {
        do {
            if try await supportsNativeSlidingSyncService(session: session) {
                _ = try await startSyncService(session: session)
            } else {
                try await syncOnce(session: session, fullState: false)
            }
        } catch {
            logger.info("Warm Matrix sync service failed; falling back to classic sync: \(String(describing: error))", category: .sync)
            try await syncOnce(session: session, fullState: false)
        }
    }

    func start(session: AuthenticatedSession) async {
        do {
            _ = try await ensureClient(for: session)
            syncStatus = .syncing
        } catch {
            logger.error("Matrix session start failed: \(String(describing: error))", category: .sync)
            syncStatus = .failed("Could not start sync.")
        }
    }

    func stop() async {
        if let syncService {
            await syncService.stop()
        }
        syncService = nil
        roomListService = nil
        syncStatus = .stopped
    }

    func pauseForBackground() async {
        await acquireClientMutationLock()
        defer { releaseClientMutationLock() }

        guard isClientPaused == false else {
            return
        }

        await detachSyncServices()

        if let client {
            do {
                try await client.pause()
            } catch {
                // Pausing is best-effort before suspension; sync is already stopped.
            }
        }

        isClientPaused = true
        syncStatus = .stopped
    }

    func resumeFromForeground(session: AuthenticatedSession) async {
        await acquireClientMutationLock()

        if isClientPaused, let client {
            do {
                try await client.resume()
            } catch {
                releaseClientMutationLock()
                syncStatus = .failed("Could not resume sync.")
                return
            }
            isClientPaused = false
        }

        let shouldStartSync = activeSession == session && client != nil
        releaseClientMutationLock()

        guard shouldStartSync else {
            return
        }

        do {
            if try await supportsNativeSlidingSyncService(session: session) {
                _ = try await startSyncService(session: session)
            } else {
                try await syncOnce(session: session, fullState: false)
            }
        } catch {
            logger.info(
                "Foreground Matrix sync service failed; falling back to classic sync: \(String(describing: error))",
                category: .sync
            )
            do {
                try await syncOnce(session: session, fullState: false)
            } catch {
                logger.error("Foreground Matrix resume failed: \(String(describing: error))", category: .sync)
                syncStatus = .failed("Could not resume sync.")
            }
        }
    }

    func syncForBackgroundNotification(session: AuthenticatedSession) async -> Bool {
        await acquireClientMutationLock()

        guard activeSession == session, let client else {
            releaseClientMutationLock()
            return false
        }

        let wasPaused = isClientPaused
        if wasPaused {
            do {
                try await client.resume()
                isClientPaused = false
            } catch {
                releaseClientMutationLock()
                return false
            }
        }

        releaseClientMutationLock()

        do {
            _ = try await client.syncOnceV2(settings: SyncSettingsV2(timeoutMs: 5_000, fullState: false))
            syncStatus = .syncing
        } catch {
            if wasPaused {
                await acquireClientMutationLock()
                try? await client.pause()
                isClientPaused = true
                syncStatus = .stopped
                releaseClientMutationLock()
            }
            return false
        }

        if wasPaused {
            await acquireClientMutationLock()
            await detachSyncServices()
            do {
                try await client.pause()
            } catch {
                releaseClientMutationLock()
                return true
            }
            isClientPaused = true
            syncStatus = .stopped
            releaseClientMutationLock()
        }

        return true
    }

    func resetLocalState(for session: AuthenticatedSession? = nil) async {
        await acquireClientMutationLock()
        defer { releaseClientMutationLock() }

        if let syncService {
            await syncService.stop()
        }
        retainClientHandle(client)
        client = nil
        activeSession = nil
        syncService = nil
        roomListService = nil
        isClientPaused = false
        syncStatus = .stopped
        unableToDecryptRecorder.reset()
        if let session {
            try? Self.deletePersistedStore(for: session)
        } else {
            try? Self.deletePersistedStores()
        }
    }

    func resetPersistedStore(for session: AuthenticatedSession) async {
        await acquireClientMutationLock()
        defer { releaseClientMutationLock() }

        if activeSession == session {
            await detachSyncServices()
            retainClientHandle(client)
            client = nil
            activeSession = nil
        }
        syncStatus = .stopped
        unableToDecryptRecorder.reset()
        try? Self.deletePersistedStore(for: session)
    }

    func loadInteractiveRoomListState(
        session: AuthenticatedSession,
        fallbackCache: [RoomSummary],
        allowsStoreRepair: Bool
    ) async -> RoomListState {
        do {
            let activeClient = try await ensureClient(for: session, allowsStoreRepair: allowsStoreRepair)
            let cachedState = await buildRoomListState(
                from: activeClient,
                fallbackCache: fallbackCache
            )
            if case .loaded(let rooms) = cachedState, rooms.isEmpty == false {
                do {
                    try await syncOnceForInteractiveOpen(session: session)
                } catch {
                    logger.info(
                        "Room list fast sync failed; using SDK cached rooms count=\(rooms.count): \(String(describing: error))",
                        category: .sync
                    )
                    return cachedState
                }
                return await buildRoomListState(
                    from: activeClient,
                    fallbackCache: fallbackCache
                )
            }

            do {
                try await syncOnceForInitialRoomList(session: session)
            } catch {
                if fallbackCache.isEmpty == false {
                    logger.info(
                        "Room list initial sync failed; using fallback rooms count=\(fallbackCache.count): \(String(describing: error))",
                        category: .sync
                    )
                    return .loaded(fallbackCache)
                }

                let localState = await buildRoomListState(
                    from: activeClient,
                    fallbackCache: fallbackCache
                )
                if case .loaded(let rooms) = localState, rooms.isEmpty == false {
                    logger.info(
                        "Room list initial sync failed; using local SDK rooms count=\(rooms.count): \(String(describing: error))",
                        category: .sync
                    )
                    return localState
                }

                logger.info(
                    "Room list initial sync failed with no local rooms; starting room-list stream: \(String(describing: error))",
                    category: .sync
                )
                return .empty
            }

            return await buildRoomListState(
                from: activeClient,
                fallbackCache: fallbackCache
            )
        } catch {
            logger.error(
                "Room list client restore failed repair_available=\(allowsStoreRepair): \(String(describing: error))",
                category: .sync
            )
            if fallbackCache.isEmpty == false {
                return .loaded(fallbackCache)
            }
            if allowsStoreRepair {
                return await repairInteractiveRoomListState(
                    session: session,
                    fallbackCache: fallbackCache
                )
            }
            return .failed("The local Matrix session could not be restored. Retry, or sign in again to rebuild local data.")
        }
    }

    private func repairInteractiveRoomListState(
        session: AuthenticatedSession,
        fallbackCache: [RoomSummary]
    ) async -> RoomListState {
        await resetPersistedStore(for: session)
        return await loadInteractiveRoomListState(
            session: session,
            fallbackCache: fallbackCache,
            allowsStoreRepair: false
        )
    }

    func syncOnce(session: AuthenticatedSession, fullState: Bool = false) async throws {
        let client = try await ensureClient(for: session)
        _ = try await client.syncOnceV2(settings: SyncSettingsV2(timeoutMs: 5_000, fullState: fullState))
        syncStatus = .syncing
    }

    func syncOnceForInitialRoomList(session: AuthenticatedSession) async throws {
        let client = try await ensureClient(for: session)
        _ = try await client.syncOnceV2(settings: SyncSettingsV2(timeoutMs: 8_000, fullState: false))
        syncStatus = .syncing
    }

    func syncOnceForInteractiveOpen(session: AuthenticatedSession) async throws {
        let client = try await ensureClient(for: session)
        _ = try await client.syncOnceV2(settings: SyncSettingsV2(timeoutMs: 1_500, fullState: false))
        syncStatus = .syncing
    }

    @discardableResult
    func startSyncService(session: AuthenticatedSession) async throws -> SyncService {
        if let syncService, activeSession == session, client != nil {
            syncStatus = .syncing
            return syncService
        }

        guard try await supportsNativeSlidingSyncService(session: session) else {
            logger.info("Skipping Matrix sync service because native sliding sync is unavailable", category: .sync)
            throw MatrixSyncServiceUnavailableError()
        }

        let service = try await prepareSyncService(session: session)
        await service.start()
        return service
    }

    private func supportsNativeSlidingSyncService(session: AuthenticatedSession) async throws -> Bool {
        guard session.slidingSyncVersion != SlidingSyncVersion.none.synaraRawValue else {
            return false
        }

        let client = try await ensureClient(for: session)
        let versions = await client.availableSlidingSyncVersions()
        let supportsNative = versions.contains(.native)
        if supportsNative == false {
            logger.info("Native sliding sync is unavailable for this homeserver/session", category: .sync)
        }
        return supportsNative
    }

    private func prepareSyncService(session: AuthenticatedSession) async throws -> SyncService {
        await acquireClientMutationLock()
        defer { releaseClientMutationLock() }

        if let syncService, activeSession == session, client != nil {
            return syncService
        }

        await detachSyncServices()

        let client = try await prepareClient(for: session, allowsStoreRepair: true)
        let builtService = try await client.syncService().finish()
        syncService = builtService
        roomListService = builtService.roomListService()
        roomListSyncGeneration &+= 1
        syncStatus = .syncing
        return builtService
    }

    func currentRoomListSyncGeneration() -> UInt64 {
        roomListSyncGeneration
    }

    func isPausedForBackground() -> Bool {
        isClientPaused
    }

    func streamingRoomListService(session: AuthenticatedSession) async throws -> RoomListService {
        let service = try await startSyncService(session: session)
        if let roomListService {
            return roomListService
        }

        let listService = service.roomListService()
        roomListService = listService
        return listService
    }

    func rooms(session: AuthenticatedSession) async throws -> [Room] {
        let client = try await ensureClient(for: session)
        let rooms = client.rooms()
        retainRoomHandles(rooms)
        return rooms
    }

    func room(roomID: String, session: AuthenticatedSession) async throws -> Room? {
        let client = try await ensureClient(for: session)
        if let room = try client.getRoom(roomId: roomID) {
            retainRoomHandles([room])
            return room
        }
        let rooms = client.rooms()
        retainRoomHandles(rooms)
        return rooms.first { $0.id() == roomID }
    }

    func userProfile(userID: String, session: AuthenticatedSession) async throws -> UserProfile {
        let client = try await ensureClient(for: session)
        return try await client.getProfile(userId: userID)
    }

    func roomCryptoStatus(roomID: String, session: AuthenticatedSession) async throws -> RoomCryptoStatus {
        let client = try await ensureClient(for: session)
        let sessionStatus = try await cryptoSessionStatus(client: client)

        guard let room = try client.getRoom(roomId: roomID) ?? client.rooms().first(where: { $0.id() == roomID }) else {
            return RoomCryptoStatus(
                encryption: .unavailable,
                verification: sessionStatus.verification,
                recovery: sessionStatus.recovery,
                backup: sessionStatus.backup,
                unableToDecryptCount: sessionStatus.unableToDecryptCount
            )
        }

        let latestEncryptionState = try? await room.latestEncryptionState()
        let currentEncryptionState = room.encryptionState()
        let isEncrypted = await room.isEncrypted()
        let encryption: SynaraRoomEncryptionStatus
        if latestEncryptionState == .encrypted || currentEncryptionState == .encrypted || isEncrypted {
            encryption = .encrypted
        } else {
            encryption = .notEncrypted
        }

        return RoomCryptoStatus(
            encryption: encryption,
            verification: sessionStatus.verification,
            recovery: sessionStatus.recovery,
            backup: sessionStatus.backup,
            unableToDecryptCount: sessionStatus.unableToDecryptCount
        )
    }

    func sessionCryptoStatus(session: AuthenticatedSession) async throws -> SessionCryptoStatus {
        let client = try await ensureClient(for: session)
        return try await cryptoSessionStatus(client: client)
    }

    func retryDecryption(roomID: String, session: AuthenticatedSession) async throws {
        let client = try await ensureClient(for: session)
        await client.encryption().waitForE2eeInitializationTasks()
        _ = try await client.syncOnceV2(settings: SyncSettingsV2(timeoutMs: 5_000, fullState: false))
        if let room = try client.getRoom(roomId: roomID) ?? client.rooms().first(where: { $0.id() == roomID }) {
            let timeline = try await room.timeline()
            _ = try? await timeline.paginateBackwards(numEvents: 20)
        }
    }

    func requestDeviceVerification(session: AuthenticatedSession) async throws {
        let client = try await ensureClient(for: session)
        if verificationController == nil {
            try await installSessionVerificationDelegate(on: client)
        }
        guard let controller = verificationController else {
            throw MessageSendError.failed
        }
        try await controller.requestDeviceVerification()
        broadcastVerificationState(.requestSent)
    }

    func acceptVerificationRequest(session: AuthenticatedSession) async throws {
        _ = try await ensureClient(for: session)
        let controller = try requireVerificationController()
        try await controller.acceptVerificationRequest()
        broadcastVerificationState(.accepted)
    }

    func startSasVerification(session: AuthenticatedSession) async throws {
        _ = try await ensureClient(for: session)
        let controller = try requireVerificationController()
        try await controller.startSasVerification()
        broadcastVerificationState(.sasStarted)
    }

    func approveVerification(session: AuthenticatedSession) async throws {
        _ = try await ensureClient(for: session)
        let controller = try requireVerificationController()
        try await controller.approveVerification()
    }

    func declineVerification(session: AuthenticatedSession) async throws {
        _ = try await ensureClient(for: session)
        let controller = try requireVerificationController()
        try await controller.declineVerification()
    }

    func cancelVerification(session: AuthenticatedSession) async throws {
        _ = try await ensureClient(for: session)
        let controller = try requireVerificationController()
        try await controller.cancelVerification()
    }

    private func requireVerificationController() throws -> SessionVerificationController {
        guard let verificationController else {
            throw MessageSendError.failed
        }
        return verificationController
    }

    func recover(recoveryKey: String, session: AuthenticatedSession) async throws {
        let client = try await ensureClient(for: session)
        try await client.encryption().recoverAndFixBackup(recoveryKey: recoveryKey)
    }

    func createRoom(_ request: RoomCreateRequest, session: AuthenticatedSession) async throws -> RoomOperationResult {
        let client = try await ensureClient(for: session)
        let name = request.name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard name.isEmpty == false else {
            throw RoomManagementError.missingRoomName
        }

        let roomID = try await client.createRoom(
            request: CreateRoomParameters(
                name: name,
                topic: request.topic.trimmingCharacters(in: .whitespacesAndNewlines).nilIfEmpty,
                isEncrypted: request.isEncrypted,
                visibility: request.visibility == .public ? .public : .private,
                preset: request.visibility == .public ? .publicChat : .privateChat
            )
        )
        _ = try await client.syncOnceV2(settings: SyncSettingsV2(timeoutMs: 5_000, fullState: true))
        return RoomOperationResult(roomID: roomID, name: name)
    }

    func createDirectMessage(_ request: DirectMessageCreateRequest, session: AuthenticatedSession) async throws -> RoomOperationResult {
        let client = try await ensureClient(for: session)
        let userID = request.userID.trimmingCharacters(in: .whitespacesAndNewlines)
        guard Self.isValidMatrixID(userID) else {
            throw RoomManagementError.invalidMatrixID
        }

        let roomID = try await client.createRoom(
            request: CreateRoomParameters(
                name: nil,
                topic: nil,
                isEncrypted: request.isEncrypted,
                isDirect: true,
                visibility: .private,
                preset: .trustedPrivateChat,
                invite: [userID]
            )
        )
        _ = try await client.syncOnceV2(settings: SyncSettingsV2(timeoutMs: 5_000, fullState: true))
        return RoomOperationResult(roomID: roomID, name: userID)
    }

    func joinRoom(_ request: RoomJoinRequest, session: AuthenticatedSession) async throws -> RoomOperationResult {
        let client = try await ensureClient(for: session)
        let reference = request.reference.trimmingCharacters(in: .whitespacesAndNewlines)
        guard reference.isEmpty == false else {
            throw RoomManagementError.missingRoomReference
        }

        let room = reference.hasPrefix("!")
            ? try await client.joinRoomById(roomId: reference)
            : try await client.joinRoomByIdOrAlias(roomIdOrAlias: reference, serverNames: [])
        _ = try await client.syncOnceV2(settings: SyncSettingsV2(timeoutMs: 5_000, fullState: true))
        return RoomOperationResult(roomID: room.id(), name: room.displayName() ?? room.canonicalAlias())
    }

    func leaveRoom(roomID: String, session: AuthenticatedSession) async throws {
        guard let room = try await room(roomID: roomID, session: session) else {
            throw RoomManagementError.failed
        }
        try await room.leave()
        try await syncOnce(session: session, fullState: true)
    }

    func inviteUser(roomID: String, userID: String, session: AuthenticatedSession) async throws {
        let trimmedUserID = userID.trimmingCharacters(in: .whitespacesAndNewlines)
        guard Self.isValidMatrixID(trimmedUserID) else {
            throw RoomManagementError.invalidMatrixID
        }
        guard let room = try await room(roomID: roomID, session: session) else {
            throw RoomManagementError.failed
        }
        try await room.inviteUserById(userId: trimmedUserID)
    }

    func roomDetails(roomID: String, session: AuthenticatedSession) async throws -> RoomDetails? {
        let client = try await ensureClient(for: session)
        guard let room = try client.getRoom(roomId: roomID) ?? client.rooms().first(where: { $0.id() == roomID }) else {
            return nil
        }

        let isEncrypted = await room.isEncrypted()
        let isDirect = await room.isDirect()
        let notificationSettings = try? await client.getNotificationSettings()
            .getRoomNotificationSettings(roomId: roomID, isEncrypted: isEncrypted, isOneToOne: isDirect)
        let powerLevels = try? await room.getPowerLevels()
        let powerLevelValues = powerLevels?.values()
        let ownUserID = room.ownUserId()
        let ownUserLevel = powerLevels?.userPowerLevels()[ownUserID] ?? powerLevelValues?.usersDefault ?? 0

        return RoomDetails(
            roomID: room.id(),
            name: room.displayName() ?? room.canonicalAlias() ?? room.id(),
            topic: room.topic(),
            aliases: [room.canonicalAlias()].compactMap(\.self) + room.alternativeAliases(),
            isEncrypted: isEncrypted,
            isPublic: room.isPublic(),
            memberCount: Int(room.joinedMembersCount() + room.invitedMembersCount()),
            canInvite: powerLevels?.canOwnUserInvite() ?? false,
            canEditName: powerLevels?.canOwnUserSendState(stateEvent: .roomName) ?? false,
            canEditTopic: powerLevels?.canOwnUserSendState(stateEvent: .roomTopic) ?? false,
            canEditAvatar: powerLevels?.canOwnUserSendState(stateEvent: .roomAvatar) ?? false,
            canEditAliases: powerLevels?.canOwnUserSendState(stateEvent: .roomCanonicalAlias) ?? false,
            powerLevels: powerLevelSummary(
                values: powerLevelValues,
                ownUserLevel: ownUserLevel,
                powerLevels: powerLevels
            ),
            notificationMode: Self.mapNotificationMode(notificationSettings?.mode),
            avatarURL: room.avatarUrl()
        )
    }

    func updateRoomProfile(_ request: RoomProfileUpdateRequest, session: AuthenticatedSession) async throws {
        guard let room = try await room(roomID: request.roomID, session: session) else {
            throw RoomManagementError.failed
        }

        let name = request.name?.trimmingCharacters(in: .whitespacesAndNewlines)
        let topic = request.topic?.trimmingCharacters(in: .whitespacesAndNewlines)
        let canonicalAlias = request.canonicalAlias?.trimmingCharacters(in: .whitespacesAndNewlines)
        let alternativeAliases = request.alternativeAliases?
            .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
            .filter { $0.isEmpty == false }
        guard name != nil || topic != nil || canonicalAlias != nil || alternativeAliases != nil || request.avatar != nil else {
            throw RoomManagementError.noProfileChanges
        }
        if let name, name.isEmpty {
            throw RoomManagementError.missingRoomName
        }
        if let canonicalAlias, canonicalAlias.isEmpty == false, Self.isValidRoomAlias(canonicalAlias) == false {
            throw RoomManagementError.invalidRoomAlias
        }
        if let alternativeAliases, alternativeAliases.contains(where: { Self.isValidRoomAlias($0) == false }) {
            throw RoomManagementError.invalidRoomAlias
        }

        let powerLevels = try? await room.getPowerLevels()
        if let name {
            guard powerLevels?.canOwnUserSendState(stateEvent: .roomName) ?? false else {
                throw RoomManagementError.failed
            }
            try await room.setName(name: name)
        }
        if let topic {
            guard powerLevels?.canOwnUserSendState(stateEvent: .roomTopic) ?? false else {
                throw RoomManagementError.failed
            }
            try await room.setTopic(topic: topic)
        }
        if canonicalAlias != nil || alternativeAliases != nil {
            guard powerLevels?.canOwnUserSendState(stateEvent: .roomCanonicalAlias) ?? false else {
                throw RoomManagementError.failed
            }
            try await room.updateCanonicalAlias(alias: canonicalAlias?.nilIfEmpty, altAliases: alternativeAliases ?? room.alternativeAliases())
        }
        if let avatar = request.avatar {
            guard powerLevels?.canOwnUserSendState(stateEvent: .roomAvatar) ?? false else {
                throw RoomManagementError.failed
            }
            switch avatar {
            case .upload(let data, let mimeType):
                try await room.uploadAvatar(mimeType: mimeType, data: data, mediaInfo: nil)
            case .remove:
                try await room.removeAvatar()
            }
        }
        try await syncOnce(session: session, fullState: true)
    }

    func searchPublicRooms(query: String, session: AuthenticatedSession) async throws -> [PublicRoomSummary] {
        let trimmedQuery = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmedQuery.isEmpty == false else {
            return []
        }

        let client = try await ensureClient(for: session)
        let search = client.roomDirectorySearch()
        let collector = MatrixRustSDKRoomDirectoryCollector()
        let handle = await search.results(listener: collector)
        defer { handle.cancel() }
        try await search.search(filter: trimmedQuery, batchSize: 20, viaServerName: nil)
        let rooms = await collector.waitForRooms(timeoutNanoseconds: 1_500_000_000)
        return rooms.map(Self.mapPublicRoom)
    }

    func setNotificationMode(_ mode: SynaraRoomNotificationMode, roomID: String, session: AuthenticatedSession) async throws {
        let client = try await ensureClient(for: session)
        let notificationSettings = await client.getNotificationSettings()
        try await notificationSettings.setRoomNotificationMode(roomId: roomID, mode: Self.mapNotificationMode(mode))
    }

    func accountData(eventType: String, session: AuthenticatedSession) async throws -> String? {
        let client = try await ensureClient(for: session)
        return try await client.accountData(eventType: eventType)
    }

    func setAccountData(eventType: String, content: String, session: AuthenticatedSession) async throws {
        let client = try await ensureClient(for: session)
        try await client.setAccountData(eventType: eventType, content: content)
    }

    func mediaThumbnailData(mxcURL: URL, width: UInt64 = 640, height: UInt64 = 480, session: AuthenticatedSession) async throws -> Data {
        let client = try await ensureClient(for: session)
        let source = try MediaSource.fromUrl(url: mxcURL.absoluteString)
        return try await client.getMediaThumbnail(mediaSource: source, width: width, height: height)
    }

    func mediaContentData(mxcURL: URL, session: AuthenticatedSession) async throws -> Data {
        let client = try await ensureClient(for: session)
        let source = try MediaSource.fromUrl(url: mxcURL.absoluteString)
        return try await client.getMediaContent(mediaSource: source)
    }

    func uploadMedia(data: Data, mimeType: String, session: AuthenticatedSession) async throws -> String {
        let client = try await ensureClient(for: session)
        return try await client.uploadMedia(mimeType: mimeType, data: data, progressWatcher: nil)
    }

    func sendMediaMessage(
        roomID: String,
        filename: String,
        contentURI: String,
        mimeType: String,
        size: UInt64?,
        session: AuthenticatedSession
    ) async throws {
        guard let room = try await room(roomID: roomID, session: session) else {
            throw MessageSendError.failed
        }
        let source = try MediaSource.fromUrl(url: contentURI)
        let msgtype: MessageType
        if mimeType.hasPrefix("image/") {
            msgtype = .image(
                content: ImageMessageContent(
                    filename: filename,
                    caption: nil,
                    formattedCaption: nil,
                    source: source,
                    info: ImageInfo(
                        height: nil,
                        width: nil,
                        mimetype: mimeType,
                        size: size,
                        thumbnailInfo: nil,
                        thumbnailSource: nil,
                        blurhash: nil,
                        isAnimated: nil
                    )
                )
            )
        } else {
            msgtype = .file(
                content: FileMessageContent(
                    filename: filename,
                    caption: nil,
                    formattedCaption: nil,
                    source: source,
                    info: FileInfo(
                        mimetype: mimeType,
                        size: size,
                        thumbnailInfo: nil,
                        thumbnailSource: nil
                    )
                )
            )
        }
        let content = try messageEventContentNew(msgtype: msgtype)
        let timeline = try await room.timeline()
        _ = try await timeline.send(msg: content)
    }

    func setPusher(
        pushKey: String,
        appID: String,
        gatewayURL: URL,
        appDisplayName: String,
        deviceDisplayName: String,
        lang: String,
        session: AuthenticatedSession
    ) async throws {
        let client = try await ensureClient(for: session)
        try await client.setPusher(
            identifiers: PusherIdentifiers(pushkey: pushKey, appId: appID),
            kind: .http(
                data: HttpPusherData(
                    url: gatewayURL.absoluteString,
                    format: .eventIdOnly,
                    defaultPayload: nil
                )
            ),
            appDisplayName: appDisplayName,
            deviceDisplayName: deviceDisplayName,
            profileTag: nil,
            lang: lang
        )
    }

    func deletePusher(pushKey: String, appID: String, session: AuthenticatedSession) async throws {
        let client = try await ensureClient(for: session)
        try await client.deletePusher(
            identifiers: PusherIdentifiers(pushkey: pushKey, appId: appID)
        )
    }

    func latestEventID(roomID: String, session: AuthenticatedSession) async throws -> String? {
        guard let room = try await room(roomID: roomID, session: session) else {
            return nil
        }

        let timeline = try await room.timeline()
        return await timeline.latestEventId()
    }

    func markRoomRead(roomID: String, session: AuthenticatedSession) async throws {
        guard let room = try await room(roomID: roomID, session: session) else {
            throw MessageSendError.failed
        }

        let timeline = try await room.timeline()
        var firstMarkerError: Error?

        do {
            try await timeline.markAsRead(receiptType: .read)
        } catch {
            firstMarkerError = error
            logger.error("Could not send Matrix read receipt", category: .sync)
        }

        do {
            try await timeline.markAsRead(receiptType: .fullyRead)
        } catch {
            firstMarkerError = firstMarkerError ?? error
            logger.error("Could not update Matrix fully-read marker", category: .sync)
        }

        do {
            try await room.setUnreadFlag(newValue: false)
        } catch {
            logger.info("Could not clear the explicit Matrix unread flag", category: .sync)
        }

        if let firstMarkerError {
            throw firstMarkerError
        }
    }

    func resolvePushRoute(eventID: String, session: AuthenticatedSession) async -> AppRoute? {
        do {
            let client = try await ensureClient(for: session)
            let syncService = try await startSyncService(session: session)
            let notificationClient = try await client.notificationClient(
                processSetup: .singleProcess(syncService: syncService)
            )

            let rooms = try await rooms(session: session)
            guard rooms.isEmpty == false else {
                return nil
            }

            return try await resolvePushRoute(
                eventID: eventID,
                rooms: prioritizedRoomsForNotificationLookup(rooms),
                notificationClient: notificationClient
            )
        } catch {
            return nil
        }
    }

    private func prioritizedRoomsForNotificationLookup(_ rooms: [Room]) async -> [Room] {
        var unreadCountsByRoomID: [String: UInt64] = [:]
        unreadCountsByRoomID.reserveCapacity(rooms.count)
        for room in rooms {
            let roomInfo = try? await room.roomInfo()
            unreadCountsByRoomID[room.id()] = roomInfo?.numUnreadNotifications ?? 0
        }

        return rooms.sorted { lhs, rhs in
            let lhsUnread = unreadCountsByRoomID[lhs.id()] ?? 0
            let rhsUnread = unreadCountsByRoomID[rhs.id()] ?? 0
            if lhsUnread != rhsUnread {
                return lhsUnread > rhsUnread
            }
            return lhs.id() < rhs.id()
        }
    }

    private func resolvePushRoute(
        eventID: String,
        rooms: [Room],
        notificationClient: NotificationClient
    ) async throws -> AppRoute? {
        let batchSize = 32
        var offset = 0

        while offset < rooms.count {
            let end = min(offset + batchSize, rooms.count)
            let slice = rooms[offset..<end]
            let requests = slice.map { NotificationItemsRequest(roomId: $0.id(), eventIds: [eventID]) }
            let results = try await notificationClient.getNotifications(requests: requests)

            for request in requests {
                guard case .ok = results[request.roomId] else {
                    continue
                }
                return .room(id: request.roomId, eventID: eventID)
            }

            offset = end
        }

        return nil
    }

    private func detachSyncServices() async {
        let invalidatesRoomList = syncService != nil || roomListService != nil
        if let syncService {
            await syncService.stop()
        }
        syncService = nil
        roomListService = nil
        if invalidatesRoomList {
            roomListSyncGeneration &+= 1
        }
        verificationController?.setDelegate(delegate: nil)
        verificationController = nil
        verificationDelegate = nil
        lastVerificationState = nil
    }

    func sendRawRoomEvent(roomID: String, eventType: String, content: String, session: AuthenticatedSession) async throws {
        guard let room = try await room(roomID: roomID, session: session) else {
            throw MessageSendError.failed
        }
        try await room.sendRaw(eventType: eventType, content: content)
    }

    fileprivate func ensureClient(for session: AuthenticatedSession, allowsStoreRepair: Bool = true) async throws -> Client {
        if let client, activeSession == session {
            return client
        }

        await acquireClientMutationLock()
        defer { releaseClientMutationLock() }

        if let client, activeSession == session {
            return client
        }

        return try await prepareClient(for: session, allowsStoreRepair: allowsStoreRepair)
    }

    private func acquireClientMutationLock() async {
        if isMutatingClient == false {
            isMutatingClient = true
            return
        }

        await withCheckedContinuation { continuation in
            clientMutationWaiters.append(continuation)
        }
    }

    private func releaseClientMutationLock() {
        if let next = clientMutationWaiters.first {
            clientMutationWaiters.removeFirst()
            next.resume()
        } else {
            isMutatingClient = false
        }
    }

    private func prepareClient(
        for session: AuthenticatedSession,
        allowsStoreRepair: Bool
    ) async throws -> Client {
        if let client, activeSession == session {
            return client
        }

        var allowRepair = allowsStoreRepair

        while true {
            if let client, activeSession == session {
                return client
            }

            if let activeSession, activeSession != session {
                await detachSyncServices()
                retainClientHandle(client)
                client = nil
                self.activeSession = nil
                unableToDecryptRecorder.reset()
            }

            let storeID = session.sdkStoreID ?? Self.storeID(for: session.userID, homeserverURL: session.homeserverURL)
            let newClient = try await buildClient(homeserverURL: session.homeserverURL, storeID: storeID)

            do {
                try await installUnableToDecryptDelegate(on: newClient)
                let availableSlidingSyncVersions = await newClient.availableSlidingSyncVersions()
                let restoreSession = session.sdkSession(availableSlidingSyncVersions: availableSlidingSyncVersions)
                if session.slidingSyncVersion == "native", availableSlidingSyncVersions.contains(.native) == false {
                    logger.info(
                        "Restoring Matrix session without native sliding sync because homeserver does not advertise it",
                        category: .sync
                    )
                }
                try await newClient.restoreSession(session: restoreSession)
                await newClient.encryption().waitForE2eeInitializationTasks()
                try await installSessionVerificationDelegate(on: newClient)
                await ensurePlatformDeviceDisplayName(session: session)

                self.client = newClient
                self.activeSession = session
                return newClient
            } catch {
                logger.error(
                    "Matrix session restore failed repair_available=\(allowRepair): \(String(describing: error))",
                    category: .auth
                )
                retainClientHandle(newClient)
                if allowRepair {
                    await detachSyncServices()
                    retainClientHandle(client)
                    self.client = nil
                    activeSession = nil
                    syncService = nil
                    roomListService = nil
                    try? Self.deletePersistedStore(for: session)
                    allowRepair = false
                    continue
                }
                throw error
            }
        }
    }

    private func retainClientHandle(_ client: Client?) {
        guard let client else {
            return
        }
        retainedClientHandles.append(client)
    }

    func retainRoomHandles(_ rooms: [Room]) {
        for room in rooms {
            retainedRoomHandlesByID[room.id()] = room
        }
    }

    private func buildRoomListState(
        from client: Client,
        fallbackCache: [RoomSummary]
    ) async -> RoomListState {
        let spaceService = await client.spaceService()
        let sdkRooms = client.rooms()
        retainRoomHandles(sdkRooms)
        return await MatrixRoomListStateBuilder.build(
            from: sdkRooms,
            spaceService: spaceService,
            fallbackCache: fallbackCache
        )
    }

    private static func mapLoginError(_ error: ClientError) -> LoginError {
        switch error {
        case .MatrixApi(let kind, let code, _, _):
            if kind == .forbidden || kind == .unauthorized || code == "M_FORBIDDEN" || code == "M_UNAUTHORIZED" {
                return .invalidCredentials
            }
            // TODO: Map additional Matrix API login failures (e.g. M_USER_DEACTIVATED) when the UI handles them.
            return .networkFailure
        case .Generic:
            // TODO: Inspect Generic ClientError messages for credential vs connectivity failures.
            return .networkFailure
        }
    }

    private func ensurePlatformDeviceDisplayName(session: AuthenticatedSession) async {
        guard session.accessToken.isEmpty == false,
              session.deviceID.isEmpty == false else {
            return
        }

        var url = session.homeserverURL
        url.appendPathComponent("_matrix")
        url.appendPathComponent("client")
        url.appendPathComponent("v3")
        url.appendPathComponent("devices")
        url.appendPathComponent(session.deviceID)

        do {
            var request = URLRequest(url: url)
            request.httpMethod = "PUT"
            request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
            request.httpBody = try JSONSerialization.data(withJSONObject: [
                "display_name": Self.platformDeviceDisplayName
            ])
            _ = try await URLSession.shared.data(for: request)
        } catch {
            // Device naming should not block login, restore, or room loading.
        }
    }

    private func installUnableToDecryptDelegate(on client: Client) async throws {
        try await client.setUtdDelegate(utdDelegate: unableToDecryptRecorder)
    }

    private func installSessionVerificationDelegate(on client: Client) async throws {
        let controller = try await client.getSessionVerificationController()
        let delegate = SynaraSessionVerificationDelegate(clientStore: self)
        controller.setDelegate(delegate: delegate)
        verificationController = controller
        verificationDelegate = delegate
    }

    nonisolated func verificationUpdates(session: AuthenticatedSession) -> AsyncStream<CryptoVerificationState> {
        AsyncStream { continuation in
            let id = UUID()
            let registrationTask = Task {
                await addVerificationContinuation(id: id, continuation: continuation, session: session)
            }
            continuation.onTermination = { _ in
                registrationTask.cancel()
                Task {
                    await self.removeVerificationContinuation(id: id)
                }
            }
        }
    }

    private func addVerificationContinuation(
        id: UUID,
        continuation: AsyncStream<CryptoVerificationState>.Continuation,
        session: AuthenticatedSession
    ) async {
        verificationContinuations[id] = continuation
        do {
            let client = try await ensureClient(for: session)
            if verificationController == nil {
                try await installSessionVerificationDelegate(on: client)
            }
        } catch {
            continuation.yield(.failed)
        }
        if let lastVerificationState {
            continuation.yield(lastVerificationState)
        }
    }

    private func removeVerificationContinuation(id: UUID) {
        verificationContinuations.removeValue(forKey: id)
    }

    private func broadcastVerificationState(_ state: CryptoVerificationState) {
        logger.info("Matrix session verification state: \(state.logLabel)", category: .auth)
        lastVerificationState = state.isTerminal ? nil : state
        for continuation in verificationContinuations.values {
            continuation.yield(state)
        }
    }

    func handleVerificationRequest(details: SessionVerificationRequestDetails) async {
        do {
            try await verificationController?.acknowledgeVerificationRequest(
                senderId: details.senderProfile.userId,
                flowId: details.flowId
            )
            broadcastVerificationState(.requestReceived(
                CryptoVerificationRequest(
                    userID: details.senderProfile.userId,
                    displayName: details.senderProfile.displayName,
                    deviceID: details.deviceId,
                    deviceDisplayName: details.deviceDisplayName,
                    flowID: details.flowId
                )
            ))
        } catch {
            broadcastVerificationState(.failed)
        }
    }

    func handleVerificationAccepted() {
        broadcastVerificationState(.accepted)
    }

    func handleVerificationSasStarted() {
        broadcastVerificationState(.sasStarted)
    }

    func handleVerificationData(_ data: SessionVerificationData) {
        switch data {
        case .emojis(let emojis, _):
            broadcastVerificationState(.emojis(emojis.map {
                CryptoVerificationEmoji(symbol: $0.symbol(), description: $0.description())
            }))
        case .decimals(let values):
            broadcastVerificationState(.decimals(values))
        }
    }

    func handleVerificationFailed() {
        broadcastVerificationState(.failed)
    }

    func handleVerificationCancelled() {
        broadcastVerificationState(.cancelled)
    }

    func handleVerificationFinished() {
        broadcastVerificationState(.finished)
    }

    private func cryptoSessionStatus(client: Client) async throws -> SessionCryptoStatus {
        let encryption = client.encryption()
        await encryption.waitForE2eeInitializationTasks()

        let backupExists = try? await encryption.backupExistsOnServer()
        let hasDevicesToVerifyAgainst = try? await encryption.hasDevicesToVerifyAgainst()
        let isLastDevice = try? await encryption.isLastDevice()

        return SessionCryptoStatus(
            verification: Self.mapVerificationState(encryption.verificationState()),
            recovery: Self.mapRecoveryState(encryption.recoveryState()),
            backup: Self.mapBackupState(encryption.backupState(), backupExists: backupExists),
            hasDevicesToVerifyAgainst: hasDevicesToVerifyAgainst,
            isLastDevice: isLastDevice,
            unableToDecryptCount: unableToDecryptRecorder.unableToDecryptCount
        )
    }

    private static func mapVerificationState(_ state: VerificationState) -> SynaraCryptoVerificationStatus {
        switch state {
        case .verified:
            return .verified
        case .unverified:
            return .unverified
        case .unknown:
            return .unknown
        }
    }

    private static func mapRecoveryState(_ state: RecoveryState) -> SynaraCryptoRecoveryStatus {
        switch state {
        case .enabled:
            return .enabled
        case .disabled:
            return .disabled
        case .incomplete:
            return .incomplete
        case .unknown:
            return .unknown
        }
    }

    private static func mapBackupState(_ state: BackupState, backupExists: Bool?) -> SynaraCryptoBackupStatus {
        if backupExists == false {
            return .unavailable
        }

        switch state {
        case .enabled:
            return .enabled
        case .creating, .enabling, .resuming, .downloading, .disabling:
            return .syncing
        case .unknown:
            return .unknown
        }
    }

    private static func mapNotificationMode(_ mode: MatrixRustSDK.RoomNotificationMode?) -> SynaraRoomNotificationMode {
        switch mode {
        case .allMessages:
            return .allMessages
        case .mentionsAndKeywordsOnly:
            return .mentionsOnly
        case .mute:
            return .mute
        case nil:
            return .allMessages
        }
    }

    private static func mapNotificationMode(_ mode: SynaraRoomNotificationMode) -> MatrixRustSDK.RoomNotificationMode {
        switch mode {
        case .allMessages:
            return .allMessages
        case .mentionsOnly:
            return .mentionsAndKeywordsOnly
        case .mute:
            return .mute
        }
    }

    private static func mapPublicRoom(_ room: RoomDescription) -> PublicRoomSummary {
        PublicRoomSummary(
            id: room.roomId,
            name: room.name ?? room.alias ?? room.roomId,
            topic: room.topic,
            alias: room.alias,
            memberCount: Int(room.joinedMembers),
            isWorldReadable: room.isWorldReadable
        )
    }

    private func powerLevelSummary(
        values: RoomPowerLevelsValues?,
        ownUserLevel: Int64,
        powerLevels: RoomPowerLevels?
    ) -> RoomPowerLevelSummary? {
        guard let values, let powerLevels else {
            return nil
        }

        return RoomPowerLevelSummary(
            ownUserLevel: ownUserLevel,
            usersDefault: values.usersDefault,
            eventsDefault: values.eventsDefault,
            stateDefault: values.stateDefault,
            invite: values.invite,
            kick: values.kick,
            ban: values.ban,
            redact: values.redact,
            roomName: values.roomName,
            roomTopic: values.roomTopic,
            roomAvatar: values.roomAvatar,
            canInvite: powerLevels.canOwnUserInvite(),
            canKick: powerLevels.canOwnUserKick(),
            canBan: powerLevels.canOwnUserBan(),
            canRedactOther: powerLevels.canOwnUserRedactOther(),
            canEditName: powerLevels.canOwnUserSendState(stateEvent: .roomName),
            canEditTopic: powerLevels.canOwnUserSendState(stateEvent: .roomTopic),
            canEditAvatar: powerLevels.canOwnUserSendState(stateEvent: .roomAvatar)
        )
    }

    private static func isValidMatrixID(_ value: String) -> Bool {
        value.hasPrefix("@") && value.contains(":") && value.count > 3
    }

    private static func isValidRoomAlias(_ value: String) -> Bool {
        value.hasPrefix("#") && value.contains(":") && value.count > 3
    }

    private func buildClient(homeserverURL: URL, storeID: String) async throws -> Client {
        try Self.ensurePlatformInitialized()
        let paths = try Self.sessionPaths(storeID: storeID)
        return try await ClientBuilder()
            .homeserverUrl(url: homeserverURL.absoluteString)
            .slidingSyncVersionBuilder(versionBuilder: .discoverNative)
            .sessionPaths(dataPath: paths.data.path, cachePath: paths.cache.path)
            .build()
    }

    private static func sessionPaths(storeID: String) throws -> (data: URL, cache: URL) {
        let base = try versionedStoreRoot(storeID: storeID, create: true)

        let data = base.appendingPathComponent("data", isDirectory: true)
        let cache = base.appendingPathComponent("cache", isDirectory: true)
        try FileManager.default.createDirectory(at: data, withIntermediateDirectories: true)
        try FileManager.default.createDirectory(at: cache, withIntermediateDirectories: true)
        return (data, cache)
    }

    static func deletePersistedStores() throws {
        let root = try storeRootURL(create: false)
        guard FileManager.default.fileExists(atPath: root.path) else {
            return
        }
        try FileManager.default.removeItem(at: root)
    }

    static func deletePersistedStore(for session: AuthenticatedSession) throws {
        let storeID = session.sdkStoreID ?? storeID(for: session.userID, homeserverURL: session.homeserverURL)
        try deletePersistedStore(storeID: storeID)
    }

    private static func deletePersistedStore(storeID: String) throws {
        try deleteStoreDirectoryIfPresent(at: try versionedStoreRoot(storeID: storeID, create: false))
        try deleteStoreDirectoryIfPresent(at: try legacyStoreRoot(storeID: storeID, create: false))
    }

    static func persistedStoreExists(for session: AuthenticatedSession) -> Bool {
        let storeID = session.sdkStoreID ?? storeID(for: session.userID, homeserverURL: session.homeserverURL)
        guard let root = try? versionedStoreRoot(storeID: storeID, create: false) else {
            return false
        }
        return FileManager.default.fileExists(atPath: root.path)
    }

    static func materializePersistedStore(for session: AuthenticatedSession) throws {
        let storeID = session.sdkStoreID ?? storeID(for: session.userID, homeserverURL: session.homeserverURL)
        _ = try sessionPaths(storeID: storeID)
    }

    static func pruneLegacyPersistedStores() throws {
        try pruneLegacyPersistedStores(in: storeRootURL(create: false))
    }

    static func pruneLegacyPersistedStores(in root: URL) throws {
        guard FileManager.default.fileExists(atPath: root.path) else {
            return
        }

        let versionDirectoryPrefix = "v"
        for child in try FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: [.isDirectoryKey]) {
            let isDirectory = (try? child.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true
            guard isDirectory else {
                continue
            }

            let name = child.lastPathComponent
            if name.hasPrefix(versionDirectoryPrefix) {
                continue
            }

            try? FileManager.default.removeItem(at: child)
        }
    }

    private static func versionedStoreRoot(storeID: String, create: Bool) throws -> URL {
        let root = try storeRootURL(create: create)
            .appendingPathComponent("v\(persistedStoreSchemaVersion)", isDirectory: true)
            .appendingPathComponent(storeID, isDirectory: true)
        if create {
            try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        }
        return root
    }

    private static func legacyStoreRoot(storeID: String, create: Bool) throws -> URL {
        let root = try storeRootURL(create: create)
            .appendingPathComponent(storeID, isDirectory: true)
        if create {
            try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        }
        return root
    }

    private static func deleteStoreDirectoryIfPresent(at url: URL) throws {
        guard FileManager.default.fileExists(atPath: url.path) else {
            return
        }
        try FileManager.default.removeItem(at: url)
    }

    private static func storeRootURL(create: Bool = true) throws -> URL {
        let base = try FileManager.default.url(
            for: .applicationSupportDirectory,
            in: .userDomainMask,
            appropriateFor: nil,
            create: create
        )
        .appendingPathComponent("Synara", isDirectory: true)
        .appendingPathComponent("MatrixRustSDK", isDirectory: true)
        if create {
            try FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
        }
        return base
    }

    private static func storeID(for subject: String, homeserverURL: URL) -> String {
        let raw = "\(homeserverURL.host ?? "homeserver")-\(subject)"
        let allowed = CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "-_"))
        let sanitized = raw.unicodeScalars.map { scalar in
            allowed.contains(scalar) ? Character(scalar) : "-"
        }
        return String(sanitized).lowercased()
    }

    private static let platformBootstrapLock = NSLock()
    private static var isPlatformInitialized = false

    private static func ensurePlatformInitialized() throws {
        platformBootstrapLock.lock()
        defer { platformBootstrapLock.unlock() }

        guard isPlatformInitialized == false else {
            return
        }

        try MatrixRustSDK.initPlatform(
            config: TracingConfiguration(
                logLevel: .warn,
                traceLogPacks: [],
                extraTargets: [],
                writeToStdoutOrSystem: false,
                writeToFiles: nil,
                sentryConfig: nil
            ),
            useLightweightTokioRuntime: false
        )
        isPlatformInitialized = true
    }
}

private struct MatrixSyncServiceUnavailableError: Error {}

final class MatrixRustSDKMatrixClientService: MatrixClientServicing {
    private let clientStore: MatrixRustSDKClientStore
    private let lock = NSLock()
    private var cachedSyncStatus: MatrixSyncStatus = .stopped

    init(clientStore: MatrixRustSDKClientStore) {
        self.clientStore = clientStore
    }

    var syncStatusDescription: String {
        syncStatus.description
    }

    var syncStatus: MatrixSyncStatus {
        lock.lock()
        defer { lock.unlock() }
        return cachedSyncStatus
    }

    func start(session: AuthenticatedSession) async {
        setSyncStatus(.starting)
        await clientStore.start(session: session)
        setSyncStatus(await clientStore.currentSyncStatus())
    }

    func stop() async {
        await clientStore.stop()
        setSyncStatus(.stopped)
    }

    func warmSync(session: AuthenticatedSession) async {
        try? await clientStore.warmSync(session: session)
        setSyncStatus(await clientStore.currentSyncStatus())
    }

    func resetLocalState(for session: AuthenticatedSession? = nil) async {
        setSyncStatus(.stopped)
        await clientStore.resetLocalState(for: session)
    }

    func pauseForBackground() async {
        await clientStore.pauseForBackground()
        setSyncStatus(await clientStore.currentSyncStatus())
    }

    func resumeFromForeground(session: AuthenticatedSession) async {
        await clientStore.resumeFromForeground(session: session)
        setSyncStatus(await clientStore.currentSyncStatus())
    }

    func syncForBackgroundNotification(session: AuthenticatedSession) async -> Bool {
        let synced = await clientStore.syncForBackgroundNotification(session: session)
        setSyncStatus(await clientStore.currentSyncStatus())
        return synced
    }

    private func setSyncStatus(_ status: MatrixSyncStatus) {
        lock.lock()
        defer { lock.unlock() }
        cachedSyncStatus = status
    }
}

struct MatrixRustSDKAuthService: AuthServicing {
    let clientStore: MatrixRustSDKClientStore

    func login(_ request: LoginRequest) async throws -> AuthenticatedSession {
        try await clientStore.login(request)
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
        if membership == .invited {
            return (1, true)
        }

        let messages = Int(numUnreadMessages)
        let notifications = Int(numUnreadNotifications)
        let mentions = Int(numUnreadMentions)

        var unreadCount = max(messages, notifications)
        if isMarkedUnread, unreadCount == 0 {
            unreadCount = 1
        }

        let hasHighlight = mentions > 0
        return (unreadCount, hasHighlight)
    }
}

private enum MatrixRoomListStateBuilder {
    static func build(
        from sdkRooms: [Room],
        spaceService: SpaceService?,
        fallbackCache: [RoomSummary]
    ) async -> RoomListState {
        let sorted = await roomSummaries(from: sdkRooms, spaceService: spaceService)
        if sorted.isEmpty == false {
            return .loaded(sorted)
        }
        if fallbackCache.isEmpty == false {
            return .loaded(fallbackCache)
        }
        return .empty
    }

    static func roomSummaries(
        from rooms: [Room],
        spaceService: SpaceService?,
        previous: [RoomSummary] = []
    ) async -> [RoomSummary] {
        let shouldMapSpaces = rooms.count <= 80
        let previousByID = Dictionary(uniqueKeysWithValues: previous.map { ($0.id, $0) })
        var summaries: [RoomSummary] = []
        summaries.reserveCapacity(rooms.count)
        for room in rooms {
            if let summary = await MatrixRustSDKRoomListService.mapRoom(
                room,
                spaceService: shouldMapSpaces ? spaceService : nil,
                previous: previousByID[room.id()]
            ) {
                summaries.append(summary)
            }
        }
        return RoomListFixtures.sorted(summaries)
    }
}

final class MatrixRustSDKRoomListService: RoomListServicing {
    private let sessionStore: AppSessionStore
    private let clientStore: MatrixRustSDKClientStore
    private let logger: LoggingServicing
    private let cacheLock = NSLock()
    private var cachedRooms: [RoomSummary] = []

    init(
        sessionStore: AppSessionStore,
        clientStore: MatrixRustSDKClientStore,
        logger: LoggingServicing = AppLogger()
    ) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
        self.logger = logger
    }

    func loadRooms() async -> RoomListState {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .empty
        }

        return await loadRooms(session: session, allowsStoreRepair: true)
    }

    func roomUpdates() -> AsyncStream<RoomListState> {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return AsyncStream { continuation in
                continuation.yield(.empty)
                continuation.finish()
            }
        }

        return AsyncStream(bufferingPolicy: .bufferingNewest(1)) { continuation in
            let task = Task {
                while Task.isCancelled == false, isCurrentSignedInUser(session) {
                    if await clientStore.isPausedForBackground() {
                        do {
                            try await Task.sleep(nanoseconds: 500_000_000)
                        } catch {
                            break
                        }
                        continue
                    }

                    do {
                        try await streamNativeRoomListUpdates(
                            session: session,
                            continuation: continuation
                        )
                        if Task.isCancelled == false, isCurrentSignedInUser(session) {
                            logger.info("Room list sync service changed; reconnecting stream", category: .sync)
                        }
                    } catch {
                        guard isCurrentSignedInUser(session) else {
                            break
                        }

                        let cachedSnapshot = cachedRoomsSnapshot()
                        if cachedSnapshot.isEmpty == false {
                            logger.info(
                                "Room list stream failed; using cached rooms count=\(cachedSnapshot.count): \(String(describing: error))",
                                category: .sync
                            )
                            continuation.yield(.loaded(cachedSnapshot))
                        } else {
                            logger.info(
                                "Room list stream failed with no cached rooms: \(String(describing: error))",
                                category: .sync
                            )
                        }
                        await runClassicRoomListUpdates(session: session, continuation: continuation)
                    }
                }

                continuation.finish()
            }

            continuation.onTermination = { _ in
                task.cancel()
            }
        }
    }

    private func streamNativeRoomListUpdates(
        session: AuthenticatedSession,
        continuation: AsyncStream<RoomListState>.Continuation
    ) async throws {
        let client = try await clientStore.ensureClient(for: session)
        let roomListService = try await clientStore.streamingRoomListService(session: session)
        let generation = await clientStore.currentRoomListSyncGeneration()
        let allRooms = try await roomListService.allRooms()
        let spaceService = await client.spaceService()
        let roomUpdates = AsyncStream<[Room]>.makeStream(bufferingPolicy: .bufferingNewest(1))

        let listener = MatrixRustSDKRoomListEntriesCollector { rooms in
            roomUpdates.continuation.yield(rooms)
        }
        let result = allRooms.entriesWithDynamicAdapters(pageSize: 100, listener: listener)
        _ = result.controller().setFilter(kind: .all(filters: []))
        let handle = result.entriesStream()
        let subscription = MatrixRustSDKRoomListSubscription(
            listener: listener,
            result: result,
            streamHandle: handle
        )

        let cachedSnapshot = cachedRoomsSnapshot()
        if cachedSnapshot.isEmpty == false, isCurrentSignedInUser(session) {
            continuation.yield(.loaded(cachedSnapshot))
        }

        let mappingTask = Task { [weak self] in
            guard let self else {
                return
            }

            for await rooms in roomUpdates.stream {
                guard Task.isCancelled == false else {
                    return
                }
                guard self.isCurrentSignedInUser(session) else {
                    return
                }

                await self.clientStore.retainRoomHandles(rooms)
                let summaries = await MatrixRoomListStateBuilder.roomSummaries(
                    from: rooms,
                    spaceService: spaceService,
                    previous: self.cachedRoomsSnapshot()
                )
                if summaries.isEmpty == false {
                    self.setCachedRooms(summaries)
                    self.logger.info(
                        "Room list stream mapped rooms count=\(summaries.count)",
                        category: .sync
                    )
                }
                let state: RoomListState = summaries.isEmpty ? .empty : .loaded(summaries)
                continuation.yield(state)
            }
        }

        await subscription.waitUntilInvalidated(
            clientStore: clientStore,
            generation: generation
        )
        roomUpdates.continuation.finish()
        mappingTask.cancel()
        _ = await mappingTask.result
    }

    private func runClassicRoomListUpdates(
        session: AuthenticatedSession,
        continuation: AsyncStream<RoomListState>.Continuation
    ) async {
        logger.info("Starting classic room-list sync fallback", category: .sync)
        var didYieldEmptyState = false
        let generation = await clientStore.currentRoomListSyncGeneration()

        while Task.isCancelled == false, isCurrentSignedInUser(session) {
            let isPaused = await clientStore.isPausedForBackground()
            let currentGeneration = await clientStore.currentRoomListSyncGeneration()
            if isPaused || currentGeneration != generation {
                return
            }

            do {
                try await clientStore.syncOnce(session: session, fullState: false)
            } catch {
                logger.info(
                    "Classic room-list sync attempt failed: \(String(describing: error))",
                    category: .sync
                )
            }

            do {
                let client = try await clientStore.ensureClient(for: session)
                let rooms = client.rooms()
                await clientStore.retainRoomHandles(rooms)
                let summaries = await MatrixRoomListStateBuilder.roomSummaries(
                    from: rooms,
                    spaceService: await client.spaceService(),
                    previous: cachedRoomsSnapshot()
                )

                if summaries.isEmpty == false {
                    setCachedRooms(summaries)
                    logger.info(
                        "Classic room-list fallback mapped rooms count=\(summaries.count)",
                        category: .sync
                    )
                    continuation.yield(.loaded(summaries))
                } else if cachedRoomsSnapshot().isEmpty == false {
                    let cachedSnapshot = cachedRoomsSnapshot()
                    logger.info(
                        "Classic room-list fallback using cached rooms count=\(cachedSnapshot.count)",
                        category: .sync
                    )
                    continuation.yield(.loaded(cachedSnapshot))
                } else if didYieldEmptyState == false {
                    didYieldEmptyState = true
                    continuation.yield(.empty)
                }
            } catch {
                let cachedSnapshot = cachedRoomsSnapshot()
                if cachedSnapshot.isEmpty == false {
                    logger.info(
                        "Classic room-list fallback failed; using cached rooms count=\(cachedSnapshot.count): \(String(describing: error))",
                        category: .sync
                    )
                    continuation.yield(.loaded(cachedSnapshot))
                } else {
                    logger.info(
                        "Classic room-list fallback failed with no cached rooms: \(String(describing: error))",
                        category: .sync
                    )
                }
            }

            try? await Task.sleep(nanoseconds: 8_000_000_000)
        }
    }

    private func isCurrentSignedInUser(_ session: AuthenticatedSession) -> Bool {
        guard case .signedIn(let currentSession) = sessionStore.currentState else {
            return false
        }
        return currentSession.userID == session.userID
    }

    private func cachedRoomsSnapshot() -> [RoomSummary] {
        cacheLock.lock()
        defer { cacheLock.unlock() }
        return cachedRooms
    }

    private func setCachedRooms(_ rooms: [RoomSummary]) {
        cacheLock.lock()
        defer { cacheLock.unlock() }
        cachedRooms = rooms
    }

    private func loadRooms(session: AuthenticatedSession, allowsStoreRepair: Bool) async -> RoomListState {
        let state = await clientStore.loadInteractiveRoomListState(
            session: session,
            fallbackCache: cachedRoomsSnapshot(),
            allowsStoreRepair: allowsStoreRepair
        )
        if case .loaded(let rooms) = state, rooms.isEmpty == false {
            setCachedRooms(rooms)
            logger.info("Room list initial load mapped rooms count=\(rooms.count)", category: .sync)
        } else if case .empty = state {
            logger.info("Room list initial load returned empty; waiting for stream", category: .sync)
        } else if case .failed(let message) = state {
            logger.error("Room list initial load failed: \(message)", category: .sync)
        }
        return state
    }

    func roomDisplayName(roomID: String) -> String? {
        cachedRoomsSnapshot().first { $0.id == roomID }?.name
    }

    func isAgentRoom(roomID: String) -> Bool {
        cachedRoomsSnapshot().first { $0.id == roomID }?.isAgentRoom ?? false
    }

    func clearCache() {
        cacheLock.lock()
        defer { cacheLock.unlock() }
        cachedRooms = []
    }

    fileprivate static func mapRoom(
        _ room: Room,
        spaceService: SpaceService?,
        previous: RoomSummary? = nil
    ) async -> RoomSummary? {
        let membership: RoomSummary.Membership
        switch room.membership() {
        case .joined:
            membership = .joined
        case .invited:
            membership = .invited
        default:
            return nil
        }

        let roomInfo = try? await room.roomInfo()
        let unread = RoomUnreadPresentation.make(
            membership: membership,
            numUnreadMessages: roomInfo?.numUnreadMessages ?? 0,
            numUnreadNotifications: roomInfo?.numUnreadNotifications ?? 0,
            numUnreadMentions: roomInfo?.numUnreadMentions ?? 0,
            isMarkedUnread: roomInfo?.isMarkedUnread ?? false
        )
        let latestPreview = await latestPreview(for: room)
        let latestAgentCardEventID = latestPreview.agentCard == nil
            ? nil
            : await latestAgentEventID(for: room)
        let shouldScanPendingApprovals = latestPreview.hasAgentActivity || previous?.hasAgentActivity == true
        let pendingAgentApprovals = shouldScanPendingApprovals
            ? await MatrixRustSDKTimelineService.pendingAgentApprovalCards(in: room)
            : []
        let hasAgentActivity = latestPreview.hasAgentActivity
            || previous?.hasAgentActivity == true
            || pendingAgentApprovals.isEmpty == false
        let name = room.displayName() ?? room.canonicalAlias() ?? room.id()
        let preview = latestPreview.text ?? defaultPreview(for: room, membership: membership)
        let parentSpaces = (try? await spaceService?.joinedParentsOfChild(childId: room.id()).map {
            SpaceSummary(id: $0.roomId, name: $0.displayName)
        }) ?? []
        return RoomSummary(
            id: room.id(),
            name: name,
            lastMessagePreview: preview,
            unreadCount: unread.unreadCount,
            hasHighlight: unread.hasHighlight,
            kind: (await room.isDirect()) ? .directMessage : .room,
            membership: membership,
            lastActivityAt: latestPreview.timestamp ?? .distantPast,
            parentSpaces: parentSpaces,
            avatarURL: room.avatarUrl().flatMap(URL.init(string:)),
            hasAgentActivity: hasAgentActivity,
            latestAgentCard: latestPreview.agentCard,
            latestAgentCardEventID: latestAgentCardEventID,
            pendingAgentApprovals: pendingAgentApprovals
        )
    }

    private static func latestAgentEventID(for room: Room) async -> String? {
        guard let timeline = try? await room.timeline() else {
            return nil
        }
        return await timeline.latestEventId()
    }

    private static func defaultPreview(for room: Room, membership: RoomSummary.Membership) -> String {
        if membership == .invited {
            return "Invited to room"
        }
        if room.encryptionState() == .encrypted {
            return "Encrypted room"
        }
        return "Tap to open room"
    }

    private static func latestPreview(for room: Room) async -> LatestRoomEventPreview {
        switch await room.latestEvent() {
        case .none:
            return LatestRoomEventPreview(text: nil, timestamp: nil, hasAgentActivity: false, agentCard: nil)
        case .remote(let timestamp, let sender, let isOwn, _, let content):
            let preview = previewDetails(content: content, sender: sender, isOwn: isOwn)
            return LatestRoomEventPreview(
                text: preview.text,
                timestamp: Date(timeIntervalSince1970: TimeInterval(timestamp) / 1_000),
                hasAgentActivity: preview.hasAgentActivity,
                agentCard: preview.agentCard
            )
        case .local(let timestamp, let sender, _, let content, _):
            let preview = previewDetails(content: content, sender: sender, isOwn: true)
            return LatestRoomEventPreview(
                text: preview.text,
                timestamp: Date(timeIntervalSince1970: TimeInterval(timestamp) / 1_000),
                hasAgentActivity: preview.hasAgentActivity,
                agentCard: preview.agentCard
            )
        case .remoteInvite(let timestamp, let inviter, _):
            let inviterName = inviter.map { senderDisplayName($0, isOwn: false) } ?? "Someone"
            return LatestRoomEventPreview(
                text: "\(inviterName) invited you",
                timestamp: Date(timeIntervalSince1970: TimeInterval(timestamp) / 1_000),
                hasAgentActivity: false,
                agentCard: nil
            )
        }
    }

    private struct RoomPreviewDetails {
        let text: String?
        let hasAgentActivity: Bool
        let agentCard: SynaraAgentCard?
    }

    private static func previewDetails(content: TimelineItemContent, sender: String, isOwn: Bool) -> RoomPreviewDetails {
        let agentCard = agentCard(from: content)
        return RoomPreviewDetails(
            text: previewText(content: content, sender: sender, isOwn: isOwn),
            hasAgentActivity: agentCard != nil,
            agentCard: agentCard
        )
    }

    private static func agentCard(from content: TimelineItemContent) -> SynaraAgentCard? {
        switch content {
        case .msgLike(let content):
            switch content.kind {
            case .message(let message):
                return SynaraAgentCardPayloadParser.parse(body: message.body)
            default:
                return nil
            }
        default:
            return nil
        }
    }

    private static func previewText(content: TimelineItemContent, sender: String, isOwn: Bool) -> String? {
        let prefix = senderDisplayName(sender, isOwn: isOwn)
        switch content {
        case .msgLike(let content):
            switch content.kind {
            case .message(let message):
                return "\(prefix): \(message.body)"
            case .sticker(let body, _, _):
                return "\(prefix): Sticker: \(body)"
            case .poll(let question, _, _, _, _, _, _):
                return "\(prefix): Poll: \(question)"
            case .redacted:
                return "\(prefix): Message deleted"
            case .unableToDecrypt:
                return "\(prefix): Encrypted message"
            case .other:
                return "\(prefix): Message"
            case .liveLocation:
                return "\(prefix): Live location"
            }
        case .roomMembership(let userID, let displayName, let change, _):
            return membershipPreview(userID: userID, displayName: displayName, change: change)
        case .profileChange:
            return nil
        case .state:
            return nil
        case .callInvite:
            return "\(prefix): Call started"
        case .rtcNotification:
            return nil
        case .failedToParseMessageLike(let eventType, _):
            return eventType == "m.room.encrypted" ? "\(prefix): Encrypted message" : nil
        case .failedToParseState:
            return nil
        }
    }

    private static func senderDisplayName(_ sender: String, isOwn: Bool) -> String {
        if isOwn {
            return "You"
        }

        let trimmed = sender.trimmingCharacters(in: CharacterSet(charactersIn: "@"))
        guard let localpart = trimmed.split(separator: ":").first, localpart.isEmpty == false else {
            return sender
        }
        return String(localpart.prefix(1)).uppercased() + localpart.dropFirst()
    }

    private static func membershipPreview(
        userID: String,
        displayName: String?,
        change: MembershipChange?
    ) -> String? {
        let name = displayName ?? senderDisplayName(userID, isOwn: false)
        switch change {
        case .joined:
            return "\(name) joined"
        case .left, .kicked, .banned:
            return "\(name) left"
        case .invited:
            return "\(name) was invited"
        case nil:
            return nil
        default:
            return nil
        }
    }
}

private struct LatestRoomEventPreview {
    let text: String?
    let timestamp: Date?
    let hasAgentActivity: Bool
    let agentCard: SynaraAgentCard?
}

final class MatrixRustSDKRoomMembershipService: RoomMembershipServicing {
    private let sessionStore: AppSessionStore
    private let clientStore: MatrixRustSDKClientStore

    init(sessionStore: AppSessionStore, clientStore: MatrixRustSDKClientStore) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
    }

    func acceptInvite(roomID: String) async throws {
        guard case .signedIn(let session) = sessionStore.currentState else {
            throw RoomMembershipError.signedOut
        }
        guard let room = try await clientStore.room(roomID: roomID, session: session) else {
            throw RoomMembershipError.failed
        }
        try await room.join()
        try await clientStore.syncOnce(session: session, fullState: true)
    }

    func rejectInvite(roomID: String) async throws {
        guard case .signedIn(let session) = sessionStore.currentState else {
            throw RoomMembershipError.signedOut
        }
        guard let room = try await clientStore.room(roomID: roomID, session: session) else {
            throw RoomMembershipError.failed
        }
        try await room.leave()
        try await clientStore.syncOnce(session: session, fullState: true)
    }
}

final class MatrixRustSDKTimelineService: TimelineServicing {
    private enum TimelineCacheFocus: Hashable {
        case live
        case event(String)
        case thread(String)
    }

    private let sessionStore: AppSessionStore
    private let clientStore: MatrixRustSDKClientStore
    private var profileAvatarCacheByUserID: [String: URL?] = [:]
    private let timelineCacheLock = NSLock()
    private var cachedTimelines: [String: Timeline] = [:]

    init(
        sessionStore: AppSessionStore,
        clientStore: MatrixRustSDKClientStore
    ) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
    }

    func loadInitialTimeline(roomID: String) async -> TimelineLoadOutcome {
        await loadInitialTimeline(roomID: roomID, focusedEventID: nil)
    }

    func loadInitialTimeline(roomID: String, focusedEventID: String?) async -> TimelineLoadOutcome {
        if let focusedEventID, focusedEventID.isEmpty == false {
            invalidateTimelineCache(roomID: roomID, focus: .event(focusedEventID))
        } else {
            // A cached live Timeline retains every page previously loaded into it. Reusing
            // it on each room open makes the initial snapshot grow without bound.
            invalidateTimelineCache(roomID: roomID, focus: .live)
        }
        return await loadTimeline(roomID: roomID, focusedEventID: focusedEventID, pageSize: 20, enrichProfiles: false)
    }

    func loadLatestTimeline(roomID: String) async -> TimelineLoadOutcome {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .empty
        }

        try? await clientStore.syncOnce(session: session, fullState: false)
        invalidateTimelineCache(roomID: roomID, focus: .live)
        return await loadTimelinePage(
            roomID: roomID,
            focus: .live,
            pageSize: 50,
            enrichProfiles: false,
            paginateForwardWhenFocused: false
        )
    }

    func loadThreadTimeline(roomID: String, rootEventID: String) async -> TimelineLoadOutcome {
        await loadThreadTimeline(roomID: roomID, rootEventID: rootEventID, pageSize: 20)
    }

    func loadOlderTimeline(roomID: String, before eventID: String) async -> TimelineLoadOutcome {
        await loadOlderTimeline(roomID: roomID, beforeEventID: eventID, pageSize: 50)
    }

    func clearSessionCaches() {
        profileAvatarCacheByUserID.removeAll()
        withTimelineCacheLock {
            cachedTimelines.removeAll()
        }
    }

    func threadTimelineUpdates(roomID: String, rootEventID: String) -> AsyncStream<TimelineLoadOutcome> {
        timelineUpdates(roomID: roomID, focusedEventID: rootEventID, focus: .thread(rootEventID))
    }

    func timelineUpdates(roomID: String, focusedEventID: String?) -> AsyncStream<TimelineLoadOutcome> {
        let focus: TimelineCacheFocus
        if let focusedEventID, focusedEventID.isEmpty == false {
            focus = .event(focusedEventID)
        } else {
            focus = .live
        }
        return timelineUpdates(roomID: roomID, focusedEventID: focusedEventID, focus: focus)
    }

    private func timelineUpdates(
        roomID: String,
        focusedEventID: String?,
        focus: TimelineCacheFocus
    ) -> AsyncStream<TimelineLoadOutcome> {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return AsyncStream { continuation in
                continuation.yield(.empty)
                continuation.finish()
            }
        }

        return AsyncStream(bufferingPolicy: .bufferingNewest(1)) { continuation in
            let task = Task { [weak self] in
                guard let self else {
                    continuation.finish()
                    return
                }

                var mappingTask: Task<Void, Never>?
                var eventUpdatesContinuation: AsyncStream<[EventTimelineItem]>.Continuation?

                defer {
                    eventUpdatesContinuation?.finish()
                    mappingTask?.cancel()
                }

                do {
                    _ = try await clientStore.startSyncService(session: session)
                    let room = try await self.resolveRoom(roomID: roomID, session: session)
                    guard let room else {
                        if self.isCurrentSignedInUser(session) {
                            continuation.yield(.empty)
                        }
                        continuation.finish()
                        return
                    }

                    let timeline = try await self.resolveCachedTimeline(
                        room: room,
                        focus: focus,
                        pageSize: 20
                    )

                    let eventUpdates = AsyncStream<[EventTimelineItem]>.makeStream(bufferingPolicy: .bufferingNewest(1))
                    eventUpdatesContinuation = eventUpdates.continuation

                    let listener = MatrixRustSDKStreamingTimelineCollector { events in
                        eventUpdates.continuation.yield(events)
                    }
                    let handle = await timeline.addListener(listener: listener)
                    let subscription = MatrixRustSDKTimelineSubscription(
                        timeline: timeline,
                        listener: listener,
                        handle: handle
                    )

                    mappingTask = Task { [weak self] in
                        guard let self else {
                            return
                        }

                        for await events in eventUpdates.stream {
                            guard Task.isCancelled == false else {
                                return
                            }
                            guard self.isCurrentSignedInUser(session) else {
                                return
                            }

                            let sdkItems = events.compactMap(Self.mapTimelineItem)
                                .sorted { $0.timestamp < $1.timestamp }
                            guard sdkItems.isEmpty == false else {
                                continue
                            }
                            continuation.yield(.loaded(sdkItems))
                        }
                    }

                    if focusedEventID != nil || focus != .live {
                        await Self.paginateFocusedTimelineForwardToLiveEnd(timeline)
                    }

                    await subscription.waitUntilCancelled()
                } catch {
                    if self.isCurrentSignedInUser(session) {
                        continuation.yield(.failed("Could not load messages. Try again."))
                    }
                    continuation.finish()
                }
            }

            continuation.onTermination = { _ in
                task.cancel()
            }
        }
    }

    private func isCurrentSignedInUser(_ session: AuthenticatedSession) -> Bool {
        guard case .signedIn(let currentSession) = sessionStore.currentState else {
            return false
        }
        return currentSession.userID == session.userID
    }

    private func loadThreadTimeline(
        roomID: String,
        rootEventID: String,
        pageSize: UInt16
    ) async -> TimelineLoadOutcome {
        await loadTimelinePage(
            roomID: roomID,
            focus: .thread(rootEventID),
            pageSize: pageSize,
            enrichProfiles: false,
            paginateForwardWhenFocused: true
        )
    }

    private func loadTimeline(
        roomID: String,
        focusedEventID: String?,
        pageSize: UInt16,
        enrichProfiles: Bool
    ) async -> TimelineLoadOutcome {
        let focus: TimelineCacheFocus
        if let focusedEventID, focusedEventID.isEmpty == false {
            focus = .event(focusedEventID)
        } else {
            focus = .live
        }

        return await loadTimelinePage(
            roomID: roomID,
            focus: focus,
            pageSize: pageSize,
            enrichProfiles: enrichProfiles,
            paginateForwardWhenFocused: focusedEventID != nil
        )
    }

    private func loadTimelinePage(
        roomID: String,
        focus: TimelineCacheFocus,
        pageSize: UInt16,
        enrichProfiles: Bool,
        paginateForwardWhenFocused: Bool
    ) async -> TimelineLoadOutcome {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .empty
        }

        do {
            try? await clientStore.syncOnceForInteractiveOpen(session: session)
            guard let room = try await resolveRoom(roomID: roomID, session: session) else {
                return .empty
            }

            let timeline = try await resolveCachedTimeline(
                room: room,
                focus: focus,
                pageSize: pageSize
            )
            let collector = MatrixRustSDKTimelineCollector()
            let handle = await timeline.addListener(listener: collector)
            defer { handle.cancel() }

            var reachedTimelineStart = try await timeline.paginateBackwards(numEvents: pageSize)
            if paginateForwardWhenFocused {
                await Self.paginateFocusedTimelineForwardToLiveEnd(timeline)
            }
            var sdkItems = await Self.waitForMappedTimelineItems(
                collector: collector,
                timeoutNanoseconds: 1_500_000_000
            )

            while sdkItems.isEmpty && reachedTimelineStart == false {
                reachedTimelineStart = (try? await timeline.paginateBackwards(numEvents: pageSize)) ?? true
                sdkItems = await Self.waitForMappedTimelineItems(
                    collector: collector,
                    timeoutNanoseconds: 750_000_000
                )
            }

            if sdkItems.isEmpty {
                try? await clientStore.syncOnce(session: session, fullState: false)
                _ = try? await timeline.paginateBackwards(numEvents: pageSize)
                sdkItems = await Self.waitForMappedTimelineItems(
                    collector: collector,
                    timeoutNanoseconds: 1_500_000_000
                )
            }

            guard sdkItems.isEmpty == false else {
                return .empty
            }

            let enrichedSDKItems = enrichProfiles ? await enrichWithProfiles(sdkItems, session: session) : sdkItems
            return .loaded(enrichedSDKItems)
        } catch {
            return .failed("Could not load messages. Try again.")
        }
    }

    private func resolveRoom(roomID: String, session: AuthenticatedSession) async throws -> Room? {
        if let restoredRoom = try await clientStore.room(roomID: roomID, session: session) {
            return restoredRoom
        }

        try await clientStore.syncOnce(session: session, fullState: false)
        return try await clientStore.room(roomID: roomID, session: session)
    }

    private func invalidateTimelineCache(roomID: String, focus: TimelineCacheFocus) {
        let cacheKey = timelineCacheKey(roomID: roomID, focus: focus)
        withTimelineCacheLock {
            _ = cachedTimelines.removeValue(forKey: cacheKey)
        }
    }

    private func timelineCacheKey(roomID: String, focus: TimelineCacheFocus) -> String {
        switch focus {
        case .live:
            return "\(roomID)|live"
        case .event(let eventID):
            return "\(roomID)|event|\(eventID)"
        case .thread(let rootEventID):
            return "\(roomID)|thread|\(rootEventID)"
        }
    }

    private func resolveCachedTimeline(
        room: Room,
        focus: TimelineCacheFocus,
        pageSize: UInt16
    ) async throws -> Timeline {
        let cacheKey = timelineCacheKey(roomID: room.id(), focus: focus)

        if let cachedTimeline = withTimelineCacheLock({ cachedTimelines[cacheKey] }) {
            return cachedTimeline
        }

        let timeline: Timeline
        switch focus {
        case .live:
            timeline = try await room.timeline()
        case .event(let eventID):
            timeline = try await room.timelineWithConfiguration(
                configuration: Self.timelineConfiguration(
                    focus: .event(
                        eventId: eventID,
                        numContextEvents: pageSize,
                        threadMode: .automatic(hideThreadedEvents: true)
                    )
                )
            )
        case .thread(let rootEventID):
            timeline = try await room.timelineWithConfiguration(
                configuration: Self.timelineConfiguration(focus: .thread(rootEventId: rootEventID))
            )
        }

        withTimelineCacheLock {
            cachedTimelines[cacheKey] = timeline
        }
        return timeline
    }

    private func withTimelineCacheLock<T>(_ body: () throws -> T) rethrows -> T {
        timelineCacheLock.lock()
        defer { timelineCacheLock.unlock() }
        return try body()
    }

    private static func timelineConfiguration(focus: TimelineFocus) -> TimelineConfiguration {
        TimelineConfiguration(
            focus: focus,
            filter: .all,
            internalIdPrefix: nil,
            dateDividerMode: .daily,
            trackReadReceipts: .messageLikeEvents,
            reportUtds: true
        )
    }

    static func pendingAgentApprovalCards(in room: Room, pageSize: UInt16 = 25) async -> [PendingAgentCardRef] {
        guard let timeline = try? await room.timeline() else {
            return []
        }

        let collector = MatrixRustSDKTimelineCollector()
        let handle = await timeline.addListener(listener: collector)
        defer { handle.cancel() }

        _ = try? await timeline.paginateBackwards(numEvents: pageSize)
        let items = await waitForMappedTimelineItems(
            collector: collector,
            timeoutNanoseconds: 1_000_000_000
        )

        var refs: [PendingAgentCardRef] = []
        refs.reserveCapacity(items.count)
        for item in items {
            guard case .agentCard(let card) = item.kind else {
                continue
            }
            refs.append(
                PendingAgentCardRef(
                    eventID: item.eventID,
                    card: card,
                    timestamp: item.timestamp
                )
            )
        }

        var seenEventIDs = Set<String>()
        return refs
            .filter { ref in
                guard ref.card.requiresUserApproval else {
                    return false
                }
                return seenEventIDs.insert(ref.eventID).inserted
            }
            .sorted { $0.timestamp > $1.timestamp }
    }

    private static func waitForMappedTimelineItems(
        collector: MatrixRustSDKTimelineCollector,
        timeoutNanoseconds: UInt64
    ) async -> [TimelineItem] {
        let deadline = DispatchTime.now().uptimeNanoseconds + timeoutNanoseconds

        while DispatchTime.now().uptimeNanoseconds < deadline {
            let mappedItems = collector.items()
                .compactMap(Self.mapTimelineItem)
                .sorted { $0.timestamp < $1.timestamp }
            if mappedItems.isEmpty == false {
                return mappedItems
            }
            try? await Task.sleep(nanoseconds: 100_000_000)
        }

        return collector.items()
            .compactMap(Self.mapTimelineItem)
            .sorted { $0.timestamp < $1.timestamp }
    }

    private static func paginateFocusedTimelineForwardToLiveEnd(_ timeline: Timeline) async {
        var iterations = 0
        let maxIterations = 200

        while iterations < maxIterations {
            iterations += 1
            do {
                let hitEnd = try await timeline.paginateForwards(numEvents: 50)
                if hitEnd {
                    return
                }
            } catch {
                return
            }
        }
    }

    private func loadOlderTimeline(roomID: String, beforeEventID: String, pageSize: UInt16) async -> TimelineLoadOutcome {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .empty
        }

        do {
            let room: Room
            if let restoredRoom = try await clientStore.room(roomID: roomID, session: session) {
                room = restoredRoom
            } else {
                try await clientStore.syncOnce(session: session, fullState: false)
                guard let syncedRoom = try await clientStore.room(roomID: roomID, session: session) else {
                    return .empty
                }
                room = syncedRoom
            }

            try? await clientStore.syncOnceForInteractiveOpen(session: session)

            let timeline = try await room.timelineWithConfiguration(
                configuration: TimelineConfiguration(
                    focus: .event(
                        eventId: beforeEventID,
                        numContextEvents: 0,
                        threadMode: .automatic(hideThreadedEvents: true)
                    ),
                    filter: .all,
                    internalIdPrefix: nil,
                    dateDividerMode: .daily,
                    trackReadReceipts: .disabled,
                    reportUtds: true
                )
            )
            let collector = MatrixRustSDKTimelineCollector()
            let handle = await timeline.addListener(listener: collector)
            defer { handle.cancel() }

            _ = try? await timeline.paginateBackwards(numEvents: pageSize)
            let sdkItems = await Self.waitForMappedTimelineItems(
                collector: collector,
                timeoutNanoseconds: 1_500_000_000
            )
            let olderItems = sdkItems.filter { item in
                item.eventID != beforeEventID && item.id != beforeEventID
            }

            guard olderItems.isEmpty == false else {
                return .empty
            }

            return .loaded(await enrichWithProfiles(olderItems, session: session))
        } catch {
            return .failed("Could not load older messages. Try again.")
        }
    }

    private func enrichWithProfiles(_ items: [TimelineItem], session: AuthenticatedSession) async -> [TimelineItem] {
        var avatarURLsByUserID: [String: URL?] = [:]

        for senderID in Set(items.map(\.senderID)) {
            avatarURLsByUserID[senderID] = await profileAvatarURL(for: senderID, session: session)
        }

        return items.map { item in
            guard let avatarURL = avatarURLsByUserID[item.senderID] ?? nil else {
                return item
            }

            return TimelineItem(
                id: item.id,
                eventID: item.eventID,
                senderID: item.senderID,
                senderAvatarURL: avatarURL,
                timestamp: item.timestamp,
                kind: item.kind,
                replyToEventID: item.replyToEventID,
                isEdited: item.isEdited,
                reactions: item.reactions,
                isEncrypted: item.isEncrypted
            )
        }
    }

    private func profileAvatarURL(for userID: String, session: AuthenticatedSession) async -> URL? {
        if let cached = profileAvatarCacheByUserID[userID] {
            return cached
        }

        do {
            let profile = try await clientStore.userProfile(userID: userID, session: session)
            let avatarURL = profile.avatarUrl.flatMap(URL.init(string:))
            profileAvatarCacheByUserID[userID] = avatarURL
            return avatarURL
        } catch {
            profileAvatarCacheByUserID[userID] = nil
            return nil
        }
    }

    private static func mapTimelineItem(_ item: EventTimelineItem) -> TimelineItem? {
        let eventID = item.eventOrTransactionId.synaraID
        let kind: TimelineItem.Kind

        switch item.content {
        case .msgLike(let content):
            kind = MatrixRustSDKTimelineMessageMapper.mapMessageLike(
                content,
                eventID: eventID,
                eventTypeRaw: item.eventTypeRaw,
                isEncrypted: item.eventTypeRaw == "m.room.encrypted"
            )
        case .state:
            return nil
        case .failedToParseMessageLike(let eventType, _):
            kind = eventType == "m.room.encrypted" ? .encryptedPlaceholder : .unknown(type: eventType)
        case .failedToParseState:
            return nil
        case .callInvite, .rtcNotification, .roomMembership, .profileChange:
            return nil
        }

        return TimelineItem(
            id: eventID,
            eventID: eventID,
            senderID: item.sender,
            senderAvatarURL: avatarURL(from: item.senderProfile),
            timestamp: Date(timeIntervalSince1970: TimeInterval(item.timestamp) / 1_000),
            kind: kind,
            replyToEventID: nil,
            isEdited: false,
            reactions: Dictionary(uniqueKeysWithValues: item.content.synaraReactions.map { ($0.key, $0.senders.count) }),
            isEncrypted: item.eventTypeRaw == "m.room.encrypted" || kind == .encryptedPlaceholder
        )
    }

    private static func avatarURL(from profile: ProfileDetails) -> URL? {
        switch profile {
        case .ready(_, _, let avatarUrl):
            return avatarUrl.flatMap(URL.init(string:))
        case .unavailable, .pending, .error:
            return nil
        }
    }

}

final class MatrixRustSDKMessageSendService: MessageSending {
    private let sessionStore: AppSessionStore
    private let clientStore: MatrixRustSDKClientStore

    init(
        sessionStore: AppSessionStore,
        clientStore: MatrixRustSDKClientStore
    ) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
    }

    func send(_ request: MessageSendRequest) async throws -> TimelineItem {
        let body = request.body.trimmingCharacters(in: .whitespacesAndNewlines)
        guard body.isEmpty == false else {
            throw MessageSendError.emptyMessage
        }
        guard case .signedIn(let session) = sessionStore.currentState else {
            throw MessageSendError.failed
        }

        do {
            guard let room = try await clientStore.room(roomID: request.roomID, session: session) else {
                throw MessageSendError.failed
            }
            let timeline = try await room.timeline()
            let content = messageEventContentFromMarkdown(md: body)

            if let editEventID = request.editEventID {
                try await timeline.edit(
                    eventOrTransactionId: .eventId(eventId: editEventID),
                    newContent: .roomMessage(content: content)
                )
            } else if let replyToEventID = request.replyToEventID {
                try await timeline.sendReply(msg: content, eventId: replyToEventID)
            } else {
                _ = try await timeline.send(msg: content)
            }

            try await clientStore.syncOnce(session: session, fullState: false)
            let eventID = request.editEventID ?? "$local-\(UUID().uuidString)"
            let senderAvatarURL = await profileAvatarURL(for: session.userID, session: session)
            return TimelineItem(
                id: eventID,
                eventID: eventID,
                senderID: session.userID,
                senderAvatarURL: senderAvatarURL,
                timestamp: Date(),
                kind: .text(body),
                replyToEventID: request.editEventID == nil ? request.replyToEventID : nil,
                isEdited: request.editEventID != nil,
                reactions: [:],
                isEncrypted: await room.isEncrypted()
            )
        } catch let error as MessageSendError {
            throw error
        } catch {
            throw MessageSendError.failed
        }
    }

    private func profileAvatarURL(for userID: String, session: AuthenticatedSession) async -> URL? {
        let profile = try? await clientStore.userProfile(userID: userID, session: session)
        return profile?.avatarUrl.flatMap(URL.init(string:))
    }
}

final class MatrixRustSDKCryptoStatusService: CryptoStatusServicing {
    private let sessionStore: AppSessionStore
    private let clientStore: MatrixRustSDKClientStore

    init(sessionStore: AppSessionStore, clientStore: MatrixRustSDKClientStore) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
    }

    func roomStatus(roomID: String) async -> RoomCryptoStatus {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .unknown
        }

        do {
            return try await clientStore.roomCryptoStatus(roomID: roomID, session: session)
        } catch {
            return .unknown
        }
    }

    func sessionStatus() async -> SessionCryptoStatus {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .unknown
        }

        do {
            return try await clientStore.sessionCryptoStatus(session: session)
        } catch {
            return .unknown
        }
    }

    func verificationUpdates() -> AsyncStream<CryptoVerificationState> {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return AsyncStream { continuation in
                continuation.finish()
            }
        }
        return clientStore.verificationUpdates(session: session)
    }

    func retryDecryption(roomID: String) async -> CryptoActionResult {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .unavailable("Sign in before retrying encrypted message decryption.")
        }

        do {
            try await clientStore.retryDecryption(roomID: roomID, session: session)
            return .completed("Decryption retry started.")
        } catch {
            return .failed("Could not retry decryption. Try again after sync completes.")
        }
    }

    func requestDeviceVerification() async -> CryptoActionResult {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .unavailable("Sign in before requesting device verification.")
        }

        do {
            try await clientStore.requestDeviceVerification(session: session)
            return .completed("Device verification request sent to your other sessions.")
        } catch {
            return .failed("Could not start device verification from this device.")
        }
    }

    func acceptVerificationRequest() async -> CryptoActionResult {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .unavailable("Sign in before accepting verification.")
        }

        do {
            try await clientStore.acceptVerificationRequest(session: session)
            return .completed("Verification request accepted.")
        } catch {
            return .failed("Could not accept verification.")
        }
    }

    func startSasVerification() async -> CryptoActionResult {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .unavailable("Sign in before starting verification.")
        }

        do {
            try await clientStore.startSasVerification(session: session)
            return .completed("Secure comparison started.")
        } catch {
            return .failed("Could not start secure comparison.")
        }
    }

    func approveVerification() async -> CryptoActionResult {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .unavailable("Sign in before approving verification.")
        }

        do {
            try await clientStore.approveVerification(session: session)
            return .completed("Device verified.")
        } catch {
            return .failed("Could not approve verification.")
        }
    }

    func declineVerification() async -> CryptoActionResult {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .unavailable("Sign in before declining verification.")
        }

        do {
            try await clientStore.declineVerification(session: session)
            return .completed("Verification declined.")
        } catch {
            return .failed("Could not decline verification.")
        }
    }

    func cancelVerification() async -> CryptoActionResult {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .unavailable("Sign in before cancelling verification.")
        }

        do {
            try await clientStore.cancelVerification(session: session)
            return .completed("Verification cancelled.")
        } catch {
            return .failed("Could not cancel verification.")
        }
    }

    func recover(recoveryKey: String) async -> CryptoActionResult {
        let trimmed = recoveryKey.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false else {
            return .failed("Enter a recovery key before recovering keys.")
        }
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .unavailable("Sign in before recovering keys.")
        }

        do {
            try await clientStore.recover(recoveryKey: trimmed, session: session)
            return .completed("Recovery completed. Encrypted history will decrypt as keys arrive.")
        } catch {
            return .failed("Could not recover keys with that recovery key.")
        }
    }
}

final class MatrixRustSDKRoomManagementService: RoomManagementServicing {
    private let sessionStore: AppSessionStore
    private let clientStore: MatrixRustSDKClientStore

    init(sessionStore: AppSessionStore, clientStore: MatrixRustSDKClientStore) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
    }

    func createRoom(_ request: RoomCreateRequest) async throws -> RoomOperationResult {
        guard case .signedIn(let session) = sessionStore.currentState else {
            throw RoomManagementError.signedOut
        }
        return try await clientStore.createRoom(request, session: session)
    }

    func createDirectMessage(_ request: DirectMessageCreateRequest) async throws -> RoomOperationResult {
        guard case .signedIn(let session) = sessionStore.currentState else {
            throw RoomManagementError.signedOut
        }
        return try await clientStore.createDirectMessage(request, session: session)
    }

    func joinRoom(_ request: RoomJoinRequest) async throws -> RoomOperationResult {
        guard case .signedIn(let session) = sessionStore.currentState else {
            throw RoomManagementError.signedOut
        }
        return try await clientStore.joinRoom(request, session: session)
    }

    func leaveRoom(roomID: String) async throws {
        guard case .signedIn(let session) = sessionStore.currentState else {
            throw RoomManagementError.signedOut
        }
        try await clientStore.leaveRoom(roomID: roomID, session: session)
    }

    func inviteUser(roomID: String, userID: String) async throws {
        guard case .signedIn(let session) = sessionStore.currentState else {
            throw RoomManagementError.signedOut
        }
        try await clientStore.inviteUser(roomID: roomID, userID: userID, session: session)
    }

    func searchPublicRooms(query: String) async throws -> [PublicRoomSummary] {
        guard case .signedIn(let session) = sessionStore.currentState else {
            throw RoomManagementError.signedOut
        }
        return try await clientStore.searchPublicRooms(query: query, session: session)
    }

    func roomDetails(roomID: String) async -> RoomDetails? {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return nil
        }
        return try? await clientStore.roomDetails(roomID: roomID, session: session)
    }

    func updateRoomProfile(_ request: RoomProfileUpdateRequest) async throws {
        guard case .signedIn(let session) = sessionStore.currentState else {
            throw RoomManagementError.signedOut
        }
        try await clientStore.updateRoomProfile(request, session: session)
    }

    func setNotificationMode(_ mode: SynaraRoomNotificationMode, roomID: String) async throws {
        guard case .signedIn(let session) = sessionStore.currentState else {
            throw RoomManagementError.signedOut
        }
        try await clientStore.setNotificationMode(mode, roomID: roomID, session: session)
    }
}

private final class MatrixRustSDKRoomDirectoryCollector: RoomDirectorySearchEntriesListener, @unchecked Sendable {
    private let lock = NSLock()
    private var rooms: [RoomDescription] = []

    func onUpdate(roomEntriesUpdate: [RoomDirectorySearchEntryUpdate]) {
        lock.lock()
        defer { lock.unlock() }

        for update in roomEntriesUpdate {
            switch update {
            case .append(let values), .reset(let values):
                rooms = values
            case .clear:
                rooms = []
            case .pushFront(let value):
                rooms.insert(value, at: 0)
            case .pushBack(let value):
                rooms.append(value)
            case .popFront:
                if rooms.isEmpty == false {
                    rooms.removeFirst()
                }
            case .popBack:
                _ = rooms.popLast()
            case .insert(let index, let value):
                let boundedIndex = min(Int(index), rooms.count)
                rooms.insert(value, at: boundedIndex)
            case .set(let index, let value):
                let intIndex = Int(index)
                if rooms.indices.contains(intIndex) {
                    rooms[intIndex] = value
                }
            case .remove(let index):
                let intIndex = Int(index)
                if rooms.indices.contains(intIndex) {
                    rooms.remove(at: intIndex)
                }
            case .truncate(let length):
                rooms = Array(rooms.prefix(Int(length)))
            }
        }
    }

    func waitForRooms(timeoutNanoseconds: UInt64) async -> [RoomDescription] {
        let deadline = DispatchTime.now().uptimeNanoseconds + timeoutNanoseconds

        while DispatchTime.now().uptimeNanoseconds < deadline {
            let currentRooms = snapshot()
            if currentRooms.isEmpty == false {
                return currentRooms
            }
            try? await Task.sleep(nanoseconds: 100_000_000)
        }

        return snapshot()
    }

    private func snapshot() -> [RoomDescription] {
        lock.lock()
        defer { lock.unlock() }
        return rooms
    }
}

private final class MatrixRustSDKRoomListSubscription: @unchecked Sendable {
    private let listener: MatrixRustSDKRoomListEntriesCollector
    private let result: RoomListEntriesWithDynamicAdaptersResult
    private let streamHandle: TaskHandle

    init(
        listener: MatrixRustSDKRoomListEntriesCollector,
        result: RoomListEntriesWithDynamicAdaptersResult,
        streamHandle: TaskHandle
    ) {
        self.listener = listener
        self.result = result
        self.streamHandle = streamHandle
    }

    func waitUntilInvalidated(
        clientStore: MatrixRustSDKClientStore,
        generation: UInt64
    ) async {
        while Task.isCancelled == false {
            if await clientStore.currentRoomListSyncGeneration() != generation {
                break
            }
            do {
                try await Task.sleep(nanoseconds: 250_000_000)
            } catch {
                break
            }
        }
        streamHandle.cancel()
        _ = listener
        _ = result
    }

    deinit {
        streamHandle.cancel()
    }
}

private final class MatrixRustSDKRoomListEntriesCollector: RoomListEntriesListener, @unchecked Sendable {
    private let lock = NSLock()
    private var rooms: [Room] = []
    private let onRooms: @Sendable ([Room]) -> Void

    init(onRooms: @escaping @Sendable ([Room]) -> Void) {
        self.onRooms = onRooms
    }

    func onUpdate(roomEntriesUpdate: [RoomListEntriesUpdate]) {
        lock.lock()
        for update in roomEntriesUpdate {
            apply(update)
        }
        let snapshot = rooms
        lock.unlock()
        onRooms(snapshot)
    }

    private func apply(_ update: RoomListEntriesUpdate) {
        switch update {
        case .append(let values):
            rooms.append(contentsOf: values)
        case .clear:
            rooms.removeAll()
        case .pushFront(let room):
            rooms.insert(room, at: 0)
        case .pushBack(let room):
            rooms.append(room)
        case .popFront:
            if rooms.isEmpty == false {
                rooms.removeFirst()
            }
        case .popBack:
            _ = rooms.popLast()
        case .insert(let index, let room):
            let boundedIndex = min(Int(index), rooms.count)
            rooms.insert(room, at: boundedIndex)
        case .set(let index, let room):
            let intIndex = Int(index)
            if rooms.indices.contains(intIndex) {
                rooms[intIndex] = room
            }
        case .remove(let index):
            let intIndex = Int(index)
            if rooms.indices.contains(intIndex) {
                rooms.remove(at: intIndex)
            }
        case .truncate(let length):
            rooms = Array(rooms.prefix(Int(length)))
        case .reset(let values):
            rooms = values
        }
    }
}

private final class MatrixRustSDKTimelineCollector: TimelineListener, @unchecked Sendable {
    private let lock = NSLock()
    private var timelineItems: [MatrixRustSDK.TimelineItem] = []

    func onUpdate(diff: [TimelineDiff]) {
        lock.lock()
        for update in diff {
            apply(update)
        }
        lock.unlock()
    }

    func items() -> [EventTimelineItem] {
        lock.lock()
        defer { lock.unlock() }

        var seen = Set<String>()
        return timelineItems.compactMap { item in
            guard let event = item.asEvent() else {
                return nil
            }
            let id = event.eventOrTransactionId.synaraID
            guard seen.contains(id) == false else {
                return nil
            }
            seen.insert(id)
            return event
        }
    }

    private func apply(_ update: TimelineDiff) {
        switch update {
        case .append(let values):
            timelineItems.append(contentsOf: values)
        case .clear:
            timelineItems.removeAll()
        case .pushFront(let item):
            timelineItems.insert(item, at: 0)
        case .pushBack(let item):
            timelineItems.append(item)
        case .popFront:
            if timelineItems.isEmpty == false {
                timelineItems.removeFirst()
            }
        case .popBack:
            _ = timelineItems.popLast()
        case .insert(let index, let item):
            let boundedIndex = min(Int(index), timelineItems.count)
            timelineItems.insert(item, at: boundedIndex)
        case .set(let index, let item):
            let intIndex = Int(index)
            if timelineItems.indices.contains(intIndex) {
                timelineItems[intIndex] = item
            }
        case .remove(let index):
            let intIndex = Int(index)
            if timelineItems.indices.contains(intIndex) {
                timelineItems.remove(at: intIndex)
            }
        case .truncate(let length):
            timelineItems = Array(timelineItems.prefix(Int(length)))
        case .reset(let values):
            timelineItems = values
        }
    }
}

private final class MatrixRustSDKTimelineSubscription: @unchecked Sendable {
    private let timeline: Timeline
    private let listener: MatrixRustSDKStreamingTimelineCollector
    private let handle: TaskHandle

    init(timeline: Timeline, listener: MatrixRustSDKStreamingTimelineCollector, handle: TaskHandle) {
        self.timeline = timeline
        self.listener = listener
        self.handle = handle
    }

    func waitUntilCancelled() async {
        while Task.isCancelled == false {
            try? await Task.sleep(nanoseconds: 1_000_000_000)
        }
        handle.cancel()
        _ = timeline
        _ = listener
    }

    deinit {
        handle.cancel()
    }
}

private final class MatrixRustSDKStreamingTimelineCollector: TimelineListener, @unchecked Sendable {
    private let lock = NSLock()
    private var timelineItems: [MatrixRustSDK.TimelineItem] = []
    private let onItems: @Sendable ([EventTimelineItem]) -> Void

    init(onItems: @escaping @Sendable ([EventTimelineItem]) -> Void) {
        self.onItems = onItems
    }

    func onUpdate(diff: [TimelineDiff]) {
        lock.lock()
        for update in diff {
            apply(update)
        }
        let events = timelineItems.compactMap { $0.asEvent() }
        lock.unlock()
        onItems(events)
    }

    private func apply(_ update: TimelineDiff) {
        switch update {
        case .append(let values):
            timelineItems.append(contentsOf: values)
        case .clear:
            timelineItems.removeAll()
        case .pushFront(let item):
            timelineItems.insert(item, at: 0)
        case .pushBack(let item):
            timelineItems.append(item)
        case .popFront:
            if timelineItems.isEmpty == false {
                timelineItems.removeFirst()
            }
        case .popBack:
            _ = timelineItems.popLast()
        case .insert(let index, let item):
            let boundedIndex = min(Int(index), timelineItems.count)
            timelineItems.insert(item, at: boundedIndex)
        case .set(let index, let item):
            let intIndex = Int(index)
            if timelineItems.indices.contains(intIndex) {
                timelineItems[intIndex] = item
            }
        case .remove(let index):
            let intIndex = Int(index)
            if timelineItems.indices.contains(intIndex) {
                timelineItems.remove(at: intIndex)
            }
        case .truncate(let length):
            timelineItems = Array(timelineItems.prefix(Int(length)))
        case .reset(let values):
            timelineItems = values
        }
    }
}

enum MatrixSlidingSyncCompatibility {
    static func storedRawValue(reported: SlidingSyncVersion, available: [SlidingSyncVersion]) -> String {
        guard reported == .native, available.contains(.native) == false else {
            return reported.synaraRawValue
        }
        return SlidingSyncVersion.none.synaraRawValue
    }

    static func sdkVersion(storedRawValue: String, available: [SlidingSyncVersion]?) -> SlidingSyncVersion {
        guard storedRawValue != SlidingSyncVersion.none.synaraRawValue else {
            return .none
        }
        if let available, available.contains(.native) == false {
            return .none
        }
        return .native
    }
}

private extension AuthenticatedSession {
    func sdkSession(availableSlidingSyncVersions available: [SlidingSyncVersion]? = nil) -> MatrixRustSDK.Session {
        MatrixRustSDK.Session(
            accessToken: accessToken,
            refreshToken: refreshToken,
            userId: userID,
            deviceId: deviceID,
            homeserverUrl: homeserverURL.absoluteString,
            oauthData: nil,
            slidingSyncVersion: MatrixSlidingSyncCompatibility.sdkVersion(
                storedRawValue: slidingSyncVersion,
                available: available
            )
        )
    }
}

private extension SlidingSyncVersion {
    var synaraRawValue: String {
        switch self {
        case .none:
            return "none"
        case .native:
            return "native"
        }
    }
}

private extension EventOrTransactionId {
    var synaraID: String {
        switch self {
        case .eventId(let eventId):
            return eventId
        case .transactionId(let transactionId):
            return transactionId
        }
    }
}

private extension TimelineItemContent {
    var synaraReactions: [Reaction] {
        guard case .msgLike(let content) = self else {
            return []
        }
        return content.reactions
    }
}

private extension String {
    var nilIfEmpty: String? {
        isEmpty ? nil : self
    }
}
