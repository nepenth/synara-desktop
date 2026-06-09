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

enum MatrixRustSDKTimelineMessageMapper {
    static func mapMessageLike(_ content: MsgLikeContent, eventTypeRaw: String?) -> TimelineItem.Kind {
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
            case .image, .audio, .video, .file, .gallery, .location:
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
}

actor MatrixRustSDKClientStore {
    /// Bump when the on-disk Matrix SDK store layout or compatibility requirements change.
    static let persistedStoreSchemaVersion = 2
    private static let platformDeviceDisplayName = "Synara iOS"
    private var client: Client?
    private var activeSession: AuthenticatedSession?
    private var syncService: SyncService?
    private var roomListService: RoomListService?
    private var syncStatus: MatrixSyncStatus = .stopped
    private let unableToDecryptRecorder = SynaraUnableToDecryptRecorder()
    /// Serializes client creation, restoration, and teardown. Actors allow reentrancy
    /// across `await`, so concurrent ensure/reset calls could otherwise free the Rust
    /// client while another task still reads rooms from it.
    private var isMutatingClient = false
    private var clientMutationWaiters: [CheckedContinuation<Void, Never>] = []

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
            client = nil
            activeSession = nil
            syncService = nil
            roomListService = nil
            unableToDecryptRecorder.reset()
        }

        let storeID = Self.storeID(for: username, homeserverURL: request.homeserverURL)
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
            let session = AuthenticatedSession(
                userID: sdkSession.userId,
                deviceID: sdkSession.deviceId,
                homeserverURL: URL(string: sdkSession.homeserverUrl) ?? request.homeserverURL,
                accessToken: sdkSession.accessToken,
                refreshToken: sdkSession.refreshToken,
                slidingSyncVersion: sdkSession.slidingSyncVersion.synaraRawValue,
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
            self.syncStatus = .failed("Could not sign in.")
            throw Self.mapLoginError(error)
        } catch {
            self.syncStatus = .failed("Could not sign in.")
            // TODO: Map additional non-ClientError login failures when the SDK exposes stable types.
            throw LoginError.networkFailure
        }
    }

    func warmSync(session: AuthenticatedSession) async throws {
        _ = try await startSyncService(session: session)
    }

    func start(session: AuthenticatedSession) async {
        do {
            _ = try await ensureClient(for: session)
            syncStatus = .syncing
        } catch {
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

    func resetLocalState(for session: AuthenticatedSession? = nil) async {
        await acquireClientMutationLock()
        defer { releaseClientMutationLock() }

        if let syncService {
            await syncService.stop()
        }
        client = nil
        activeSession = nil
        syncService = nil
        roomListService = nil
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
            let cachedState = await MatrixRoomListStateBuilder.build(
                from: activeClient,
                fallbackCache: fallbackCache
            )
            if case .loaded(let rooms) = cachedState, rooms.isEmpty == false {
                do {
                    try await syncOnceForInteractiveOpen(session: session)
                } catch {
                    if allowsStoreRepair {
                        return await repairInteractiveRoomListState(
                            session: session,
                            fallbackCache: fallbackCache
                        )
                    }
                    return cachedState
                }
                return await MatrixRoomListStateBuilder.build(
                    from: activeClient,
                    fallbackCache: fallbackCache
                )
            }

            do {
                try await syncOnce(session: session, fullState: false)
            } catch {
                if fallbackCache.isEmpty == false {
                    return .loaded(fallbackCache)
                }
                if allowsStoreRepair {
                    return await repairInteractiveRoomListState(
                        session: session,
                        fallbackCache: fallbackCache
                    )
                }
                return .failed("Could not load rooms. Try again.")
            }

            return await MatrixRoomListStateBuilder.build(
                from: activeClient,
                fallbackCache: fallbackCache
            )
        } catch {
            if fallbackCache.isEmpty == false {
                return .loaded(fallbackCache)
            }
            if allowsStoreRepair {
                return await repairInteractiveRoomListState(
                    session: session,
                    fallbackCache: fallbackCache
                )
            }
            return .failed("Could not load rooms. Try again.")
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

        let service = try await prepareSyncService(session: session)
        await service.start()
        return service
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
        syncStatus = .syncing
        return builtService
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
        return client.rooms()
    }

    func room(roomID: String, session: AuthenticatedSession) async throws -> Room? {
        let client = try await ensureClient(for: session)
        if let room = try client.getRoom(roomId: roomID) {
            return room
        }
        return client.rooms().first { $0.id() == roomID }
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
        let controller = try await client.getSessionVerificationController()
        try await controller.requestDeviceVerification()
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

    func markRoomReadUpTo(roomID: String, eventID: String, session: AuthenticatedSession) async throws {
        guard let room = try await room(roomID: roomID, session: session) else {
            throw MessageSendError.failed
        }

        try await room.markAsFullyReadUnchecked(eventId: eventID)
        let timeline = try await room.timeline()
        try await timeline.sendReadReceipt(receiptType: .read, eventId: eventID)
        try await timeline.markAsRead(receiptType: .read)
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
        if let syncService {
            await syncService.stop()
        }
        syncService = nil
        roomListService = nil
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
                client = nil
                self.activeSession = nil
                unableToDecryptRecorder.reset()
            }

            let storeID = session.sdkStoreID ?? Self.storeID(for: session.userID, homeserverURL: session.homeserverURL)
            let newClient = try await buildClient(homeserverURL: session.homeserverURL, storeID: storeID)

            do {
                try await installUnableToDecryptDelegate(on: newClient)
                try await newClient.restoreSession(session: session.sdkSession)
                await newClient.encryption().waitForE2eeInitializationTasks()
                await ensurePlatformDeviceDisplayName(session: session)

                self.client = newClient
                self.activeSession = session
                return newClient
            } catch {
                if allowRepair {
                    await detachSyncServices()
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

private enum MatrixRoomListStateBuilder {
    static func build(from client: Client?, fallbackCache: [RoomSummary]) async -> RoomListState {
        guard let client else {
            if fallbackCache.isEmpty == false {
                return .loaded(fallbackCache)
            }
            return .failed("Could not load rooms. Try again.")
        }

        let spaceService = await client.spaceService()
        let sdkRooms = client.rooms()
        let sorted = await roomSummaries(from: sdkRooms, spaceService: spaceService)
        if sorted.isEmpty == false {
            return .loaded(sorted)
        }
        if fallbackCache.isEmpty == false {
            return .loaded(fallbackCache)
        }
        return .empty
    }

    static func roomSummaries(from rooms: [Room], spaceService: SpaceService?) async -> [RoomSummary] {
        let shouldMapSpaces = rooms.count <= 80
        var summaries: [RoomSummary] = []
        summaries.reserveCapacity(rooms.count)
        for room in rooms {
            if let summary = await MatrixRustSDKRoomListService.mapRoom(
                room,
                spaceService: shouldMapSpaces ? spaceService : nil
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
    private let cacheLock = NSLock()
    private var cachedRooms: [RoomSummary] = []

    init(sessionStore: AppSessionStore, clientStore: MatrixRustSDKClientStore) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
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
                var mappingTask: Task<Void, Never>?
                var roomUpdatesContinuation: AsyncStream<[Room]>.Continuation?

                defer {
                    roomUpdatesContinuation?.finish()
                    mappingTask?.cancel()
                }

                do {
                    let client = try await clientStore.ensureClient(for: session)
                    let roomListService = try await clientStore.streamingRoomListService(session: session)
                    let allRooms = try await roomListService.allRooms()
                    let spaceService = await client.spaceService()
                    let roomUpdates = AsyncStream<[Room]>.makeStream(bufferingPolicy: .bufferingNewest(1))
                    roomUpdatesContinuation = roomUpdates.continuation

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

                    mappingTask = Task { [weak self] in
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

                            let summaries = await MatrixRoomListStateBuilder.roomSummaries(
                                from: rooms,
                                spaceService: spaceService
                            )
                            if summaries.isEmpty == false {
                                self.setCachedRooms(summaries)
                            }
                            let state: RoomListState = summaries.isEmpty ? .empty : .loaded(summaries)
                            continuation.yield(state)
                        }
                    }

                    await subscription.waitUntilCancelled()
                } catch {
                    guard isCurrentSignedInUser(session) else {
                        continuation.finish()
                        return
                    }

                    let cachedSnapshot = cachedRoomsSnapshot()
                    if cachedSnapshot.isEmpty == false {
                        continuation.yield(.loaded(cachedSnapshot))
                    } else {
                        continuation.yield(.failed("Could not load rooms. Try again."))
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
        }
        return state
    }

    func clearCache() {
        cacheLock.lock()
        defer { cacheLock.unlock() }
        cachedRooms = []
    }

    fileprivate static func mapRoom(_ room: Room, spaceService: SpaceService?) async -> RoomSummary? {
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
        let latestPreview = await latestPreview(for: room)
        let name = room.displayName() ?? room.canonicalAlias() ?? room.id()
        let preview = latestPreview.text ?? defaultPreview(for: room, membership: membership)
        let parentSpaces = (try? await spaceService?.joinedParentsOfChild(childId: room.id()).map {
            SpaceSummary(id: $0.roomId, name: $0.displayName)
        }) ?? []
        return RoomSummary(
            id: room.id(),
            name: name,
            lastMessagePreview: preview,
            unreadCount: membership == .invited ? 1 : Int(roomInfo?.numUnreadNotifications ?? 0),
            hasHighlight: membership == .invited || (roomInfo?.numUnreadMentions ?? 0) > 0 || (roomInfo?.highlightCount ?? 0) > 0,
            kind: (await room.isDirect()) ? .directMessage : .room,
            membership: membership,
            lastActivityAt: latestPreview.timestamp ?? .distantPast,
            parentSpaces: parentSpaces
        )
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
            return LatestRoomEventPreview(text: nil, timestamp: nil)
        case .remote(let timestamp, let sender, let isOwn, _, let content):
            return LatestRoomEventPreview(
                text: previewText(content: content, sender: sender, isOwn: isOwn),
                timestamp: Date(timeIntervalSince1970: TimeInterval(timestamp) / 1_000)
            )
        case .local(let timestamp, let sender, _, let content, _):
            return LatestRoomEventPreview(
                text: previewText(content: content, sender: sender, isOwn: true),
                timestamp: Date(timeIntervalSince1970: TimeInterval(timestamp) / 1_000)
            )
        case .remoteInvite(let timestamp, let inviter, _):
            let inviterName = inviter.map { senderDisplayName($0, isOwn: false) } ?? "Someone"
            return LatestRoomEventPreview(
                text: "\(inviterName) invited you",
                timestamp: Date(timeIntervalSince1970: TimeInterval(timestamp) / 1_000)
            )
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
        await loadTimeline(roomID: roomID, focusedEventID: focusedEventID, pageSize: 20, enrichProfiles: false)
    }

    func loadThreadTimeline(roomID: String, rootEventID: String) async -> TimelineLoadOutcome {
        await loadThreadTimeline(roomID: roomID, rootEventID: rootEventID, pageSize: 20)
    }

    func loadOlderTimeline(roomID: String, before eventID: String) async -> TimelineLoadOutcome {
        await loadOlderTimeline(roomID: roomID, beforeEventID: eventID, pageSize: 50)
    }

    func clearSessionCaches() {
        profileAvatarCacheByUserID.removeAll()
        timelineCacheLock.lock()
        cachedTimelines.removeAll()
        timelineCacheLock.unlock()
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

                    _ = try? await timeline.paginateBackwards(numEvents: 20)
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

        timelineCacheLock.lock()
        if let cachedTimeline = cachedTimelines[cacheKey] {
            timelineCacheLock.unlock()
            return cachedTimeline
        }
        timelineCacheLock.unlock()

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

        timelineCacheLock.lock()
        cachedTimelines[cacheKey] = timeline
        timelineCacheLock.unlock()
        return timeline
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
            kind = MatrixRustSDKTimelineMessageMapper.mapMessageLike(content, eventTypeRaw: item.eventTypeRaw)
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

    func waitUntilCancelled() async {
        while Task.isCancelled == false {
            try? await Task.sleep(nanoseconds: 1_000_000_000)
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

private extension AuthenticatedSession {
    var sdkSession: MatrixRustSDK.Session {
        MatrixRustSDK.Session(
            accessToken: accessToken,
            refreshToken: refreshToken,
            userId: userID,
            deviceId: deviceID,
            homeserverUrl: homeserverURL.absoluteString,
            oauthData: nil,
            slidingSyncVersion: slidingSyncVersion == "none" ? .none : .native
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
