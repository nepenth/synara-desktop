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

actor MatrixRustSDKClientStore {
    private static let platformDeviceDisplayName = "Synara iOS"
    private var client: Client?
    private var activeSession: AuthenticatedSession?
    private var syncService: SyncService?
    private var roomListService: RoomListService?
    private var syncStatus: MatrixSyncStatus = .stopped
    private let unableToDecryptRecorder = SynaraUnableToDecryptRecorder()

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
        } catch {
            self.syncStatus = .failed("Could not sign in.")
            throw LoginError.networkFailure
        }
    }

    func start(session: AuthenticatedSession) async {
        syncStatus = .syncing
    }

    func stop() {
        if let syncService {
            Task {
                await syncService.stop()
            }
        }
        syncService = nil
        roomListService = nil
        syncStatus = .stopped
    }

    func resetLocalState() {
        client = nil
        activeSession = nil
        syncService = nil
        roomListService = nil
        syncStatus = .stopped
        unableToDecryptRecorder.reset()
        try? Self.deletePersistedStores()
    }

    func resetPersistedStore(for session: AuthenticatedSession) async {
        if activeSession == session {
            client = nil
            activeSession = nil
            syncService = nil
            roomListService = nil
        }
        syncStatus = .stopped
        unableToDecryptRecorder.reset()
        try? Self.deletePersistedStore(for: session)
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
        _ = try await ensureClient(for: session)
        if let syncService {
            syncStatus = .syncing
            return syncService
        }

        let client = try await ensureClient(for: session)
        let service = try await client.syncService().finish()
        syncService = service
        roomListService = service.roomListService()
        syncStatus = .syncing
        Task {
            await service.start()
        }
        return service
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

    func roomCryptoStatus(roomID: String, session: AuthenticatedSession) async throws -> RoomCryptoStatus {
        let client = try await ensureClient(for: session)
        _ = try await client.syncOnceV2(settings: SyncSettingsV2(timeoutMs: 5_000, fullState: false))
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

    fileprivate func ensureClient(for session: AuthenticatedSession) async throws -> Client {
        if let client, activeSession == session {
            return client
        }

        let client = try await buildClient(
            homeserverURL: session.homeserverURL,
            storeID: session.sdkStoreID ?? Self.storeID(for: session.userID, homeserverURL: session.homeserverURL)
        )
        try await installUnableToDecryptDelegate(on: client)
        try await client.restoreSession(session: session.sdkSession)
        await client.encryption().waitForE2eeInitializationTasks()
        await ensurePlatformDeviceDisplayName(session: session)

        self.client = client
        self.activeSession = session
        return client
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
        let paths = try Self.sessionPaths(storeID: storeID)
        return try await ClientBuilder()
            .homeserverUrl(url: homeserverURL.absoluteString)
            .sessionPaths(dataPath: paths.data.path, cachePath: paths.cache.path)
            .build()
    }

    private static func sessionPaths(storeID: String) throws -> (data: URL, cache: URL) {
        let base = try storeRootURL()
            .appendingPathComponent(storeID, isDirectory: true)

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
        let root = try storeRootURL(create: false).appendingPathComponent(storeID, isDirectory: true)
        guard FileManager.default.fileExists(atPath: root.path) else {
            return
        }
        try FileManager.default.removeItem(at: root)
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

    func resetLocalState() {
        setSyncStatus(.stopped)
        Task {
            await clientStore.resetLocalState()
        }
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

final class MatrixRustSDKRoomListService: RoomListServicing {
    private let sessionStore: AppSessionStore
    private let clientStore: MatrixRustSDKClientStore
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
                do {
                    let client = try await clientStore.ensureClient(for: session)
                    let roomListService = try await clientStore.streamingRoomListService(session: session)
                    let allRooms = try await roomListService.allRooms()
                    let spaceService = await client.spaceService()
                    let listener = MatrixRustSDKRoomListEntriesCollector { [weak self] rooms in
                        guard let self else {
                            return
                        }
                        Task {
                            let summaries = await self.roomSummaries(from: rooms, spaceService: spaceService)
                            let state: RoomListState = summaries.isEmpty ? .empty : .loaded(summaries)
                            continuation.yield(state)
                        }
                    }
                    let result = allRooms.entriesWithDynamicAdapters(pageSize: 100, listener: listener)
                    _ = result.controller().setFilter(kind: .all(filters: []))
                    let handle = result.entriesStream()
                    let subscription = MatrixRustSDKRoomListSubscription(
                        listener: listener,
                        result: result,
                        streamHandle: handle
                    )

                    if cachedRooms.isEmpty == false {
                        continuation.yield(.loaded(cachedRooms))
                    }

                    await subscription.waitUntilCancelled()
                } catch {
                    if cachedRooms.isEmpty == false {
                        continuation.yield(.loaded(cachedRooms))
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

    private func loadRooms(session: AuthenticatedSession, allowsStoreRepair: Bool) async -> RoomListState {
        do {
            let client = try await clientStore.ensureClient(for: session)
            let cachedState = await roomListState(from: client)
            if case .loaded(let rooms) = cachedState, rooms.isEmpty == false {
                cachedRooms = rooms
                Task {
                    try? await self.clientStore.syncOnce(session: session, fullState: false)
                }
                return cachedState
            }

            do {
                try await clientStore.syncOnce(session: session, fullState: false)
            } catch {
                if cachedRooms.isEmpty == false {
                    return .loaded(cachedRooms)
                }
                if allowsStoreRepair {
                    return await repairStoreAndReloadRooms(session: session)
                }
                return .failed("Could not load rooms. Try again.")
            }

            return await roomListState(from: client)
        } catch {
            if cachedRooms.isEmpty == false {
                return .loaded(cachedRooms)
            }
            if allowsStoreRepair {
                return await repairStoreAndReloadRooms(session: session)
            }
            return .failed("Could not load rooms. Try again.")
        }
    }

    private func roomSummaries(from rooms: [Room], spaceService: SpaceService?) async -> [RoomSummary] {
        let shouldMapSpaces = rooms.count <= 80
        var summaries: [RoomSummary] = []
        summaries.reserveCapacity(rooms.count)
        for room in rooms {
            if let summary = await Self.mapRoom(
                room,
                spaceService: shouldMapSpaces ? spaceService : nil
            ) {
                summaries.append(summary)
            }
        }
        let sorted = RoomListFixtures.sorted(summaries)
        if sorted.isEmpty == false {
            cachedRooms = sorted
        }
        return sorted
    }

    private func roomListState(from client: Client) async -> RoomListState {
        let spaceService = await client.spaceService()
        let sdkRooms = client.rooms()
        let sorted = await roomSummaries(from: sdkRooms, spaceService: spaceService)
        if sorted.isEmpty == false {
            cachedRooms = sorted
        }
        if sorted.isEmpty {
            if cachedRooms.isEmpty == false {
                return .loaded(cachedRooms)
            }
            return .empty
        }
        return .loaded(sorted)
    }

    private func repairStoreAndReloadRooms(session: AuthenticatedSession) async -> RoomListState {
        await clientStore.resetPersistedStore(for: session)
        return await loadRooms(session: session, allowsStoreRepair: false)
    }

    func clearCache() {
        cachedRooms = []
    }

    private static func mapRoom(_ room: Room, spaceService: SpaceService?) async -> RoomSummary? {
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
    private let sessionStore: AppSessionStore
    private let clientStore: MatrixRustSDKClientStore
    private let rawTimelineFallback: TimelineServicing?
    private let httpClient: AuthHTTPClient
    private let jsonDecoder: JSONDecoder
    private var profileCacheByUserID: [String: MatrixRustSDKProfileResponse?] = [:]

    init(
        sessionStore: AppSessionStore,
        clientStore: MatrixRustSDKClientStore,
        rawTimelineFallback: TimelineServicing? = nil,
        httpClient: AuthHTTPClient = URLSession.shared,
        jsonDecoder: JSONDecoder = JSONDecoder()
    ) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
        self.rawTimelineFallback = rawTimelineFallback ?? MatrixTimelineService(sessionStore: sessionStore)
        self.httpClient = httpClient
        self.jsonDecoder = jsonDecoder
    }

    func loadInitialTimeline(roomID: String) async -> [TimelineItem] {
        await loadInitialTimeline(roomID: roomID, focusedEventID: nil)
    }

    func loadInitialTimeline(roomID: String, focusedEventID: String?) async -> [TimelineItem] {
        await loadTimeline(roomID: roomID, focusedEventID: focusedEventID, pageSize: 20, enrichProfiles: false)
    }

    func loadOlderTimeline(roomID: String, before eventID: String) async -> [TimelineItem] {
        await loadTimeline(roomID: roomID, focusedEventID: nil, pageSize: 50, enrichProfiles: true)
    }

    func timelineUpdates(roomID: String, focusedEventID: String?) -> AsyncStream<[TimelineItem]> {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return AsyncStream { continuation in
                continuation.yield([])
                continuation.finish()
            }
        }

        return AsyncStream(bufferingPolicy: .bufferingNewest(1)) { continuation in
            let task = Task {
                do {
                    _ = try await clientStore.startSyncService(session: session)
                    let room: Room
                    if let restoredRoom = try await clientStore.room(roomID: roomID, session: session) {
                        room = restoredRoom
                    } else {
                        try? await clientStore.syncOnceForInteractiveOpen(session: session)
                        guard let syncedRoom = try await clientStore.room(roomID: roomID, session: session) else {
                            continuation.yield(await rawAgentFallbackItems(roomID: roomID))
                            continuation.finish()
                            return
                        }
                        room = syncedRoom
                    }

                    let timeline: Timeline
                    if let focusedEventID, focusedEventID.isEmpty == false {
                        timeline = try await room.timelineWithConfiguration(
                            configuration: TimelineConfiguration(
                                focus: .event(
                                    eventId: focusedEventID,
                                    numContextEvents: 20,
                                    threadMode: .automatic(hideThreadedEvents: true)
                                ),
                                filter: .all,
                                internalIdPrefix: nil,
                                dateDividerMode: .daily,
                                trackReadReceipts: .disabled,
                                reportUtds: true
                            )
                        )
                    } else {
                        timeline = try await room.timeline()
                    }

                    let listener = MatrixRustSDKStreamingTimelineCollector { events in
                        Task {
                            let sdkItems = events.compactMap(Self.mapTimelineItem)
                                .sorted { $0.timestamp < $1.timestamp }
                            continuation.yield(sdkItems)
                        }
                    }
                    let handle = await timeline.addListener(listener: listener)
                    let subscription = MatrixRustSDKTimelineSubscription(
                        timeline: timeline,
                        listener: listener,
                        handle: handle
                    )

                    _ = try? await timeline.paginateBackwards(numEvents: 20)
                    if focusedEventID != nil {
                        for _ in 0..<3 {
                            _ = try? await timeline.paginateForwards(numEvents: 20)
                        }
                    }

                    await subscription.waitUntilCancelled()
                } catch {
                    continuation.yield(await rawAgentFallbackItems(roomID: roomID))
                    continuation.finish()
                }
            }

            continuation.onTermination = { _ in
                task.cancel()
            }
        }
    }

    private func loadTimeline(
        roomID: String,
        focusedEventID: String?,
        pageSize: UInt16,
        enrichProfiles: Bool
    ) async -> [TimelineItem] {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return []
        }

        do {
            let room: Room
            if let restoredRoom = try await clientStore.room(roomID: roomID, session: session) {
                room = restoredRoom
            } else {
                try await clientStore.syncOnce(session: session, fullState: false)
                guard let syncedRoom = try await clientStore.room(roomID: roomID, session: session) else {
                    return []
                }
                room = syncedRoom
            }

            try? await clientStore.syncOnceForInteractiveOpen(session: session)

            let timeline: Timeline
            if let focusedEventID, focusedEventID.isEmpty == false {
                timeline = try await room.timelineWithConfiguration(
                    configuration: TimelineConfiguration(
                        focus: .event(
                            eventId: focusedEventID,
                            numContextEvents: pageSize,
                            threadMode: .automatic(hideThreadedEvents: true)
                        ),
                        filter: .all,
                        internalIdPrefix: nil,
                        dateDividerMode: .daily,
                        trackReadReceipts: .disabled,
                        reportUtds: true
                    )
                )
            } else {
                timeline = try await room.timeline()
            }
            let collector = MatrixRustSDKTimelineCollector()
            let handle = await timeline.addListener(listener: collector)
            defer { handle.cancel() }

            _ = try await timeline.paginateBackwards(numEvents: pageSize)
            if focusedEventID != nil {
                for _ in 0..<3 {
                    _ = try? await timeline.paginateForwards(numEvents: pageSize)
                }
            }
            let items = await collector.waitForItems(timeoutNanoseconds: 1_000_000_000)
            let sdkItems = items.compactMap(Self.mapTimelineItem)
                .sorted { $0.timestamp < $1.timestamp }
            guard sdkItems.isEmpty == false else {
                try? await clientStore.syncOnce(session: session, fullState: false)
                return await rawAgentFallbackItems(roomID: roomID)
            }
            let enrichedSDKItems = enrichProfiles ? await enrichWithProfiles(sdkItems, session: session) : sdkItems
            let rawAgentItems = enrichProfiles ? await rawAgentFallbackItems(roomID: roomID) : []
            return Self.mergedTimelineItems(sdkItems: enrichedSDKItems, rawAgentItems: rawAgentItems)
        } catch {
            return await rawAgentFallbackItems(roomID: roomID)
        }
    }

    private func enrichWithProfiles(_ items: [TimelineItem], session: AuthenticatedSession) async -> [TimelineItem] {
        var profilesByUserID: [String: MatrixRustSDKProfileResponse?] = [:]

        for senderID in Set(items.map(\.senderID)) {
            profilesByUserID[senderID] = await profile(for: senderID, session: session)
        }

        return items.map { item in
            guard let profile = profilesByUserID[item.senderID] ?? nil,
                  let avatarURL = profile.avatarURL.flatMap(URL.init(string:)) else {
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

    private func profile(for userID: String, session: AuthenticatedSession) async -> MatrixRustSDKProfileResponse? {
        if let cached = profileCacheByUserID[userID] {
            return cached
        }

        var url = session.homeserverURL
        url.appendPathComponent("_matrix")
        url.appendPathComponent("client")
        url.appendPathComponent("v3")
        url.appendPathComponent("profile")
        url.appendPathComponent(userID)

        do {
            var request = URLRequest(url: url)
            request.httpMethod = "GET"
            request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")

            let (data, response) = try await httpClient.data(for: request)
            guard let http = response as? HTTPURLResponse,
                  http.statusCode == 200 else {
                profileCacheByUserID[userID] = nil
                return nil
            }

            let profile = try jsonDecoder.decode(MatrixRustSDKProfileResponse.self, from: data)
            profileCacheByUserID[userID] = profile
            return profile
        } catch {
            profileCacheByUserID[userID] = nil
            return nil
        }
    }

    private func rawAgentFallbackItems(roomID: String) async -> [TimelineItem] {
        guard let rawTimelineFallback else {
            return []
        }

        return await rawTimelineFallback.loadInitialTimeline(roomID: roomID)
            .filter { item in
                if case .agentCard = item.kind {
                    return true
                }
                return false
            }
    }

    static func mergedTimelineItems(sdkItems: [TimelineItem], rawAgentItems: [TimelineItem]) -> [TimelineItem] {
        var itemsByID = Dictionary(uniqueKeysWithValues: sdkItems.map { ($0.id, $0) })
        for item in rawAgentItems {
            itemsByID[item.id] = item
        }
        return itemsByID.values.sorted { $0.timestamp < $1.timestamp }
    }

    private static func mapTimelineItem(_ item: EventTimelineItem) -> TimelineItem? {
        let eventID = item.eventOrTransactionId.synaraID
        let kind: TimelineItem.Kind

        switch item.content {
        case .msgLike(let content):
            kind = mapMessageLike(content, eventTypeRaw: item.eventTypeRaw)
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
            timestamp: Date(timeIntervalSince1970: TimeInterval(item.timestamp) / 1_000),
            kind: kind,
            replyToEventID: nil,
            isEdited: false,
            reactions: Dictionary(uniqueKeysWithValues: item.content.synaraReactions.map { ($0.key, $0.senders.count) }),
            isEncrypted: item.eventTypeRaw == "m.room.encrypted" || kind == .encryptedPlaceholder
        )
    }

    private static func mapMessageLike(_ content: MsgLikeContent, eventTypeRaw: String?) -> TimelineItem.Kind {
        switch content.kind {
        case .message(let message):
            if let agentCard = SynaraAgentCardPayloadParser.parse(body: message.body) {
                return .agentCard(agentCard)
            }
            return .text(message.body)
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
}

private struct MatrixRustSDKProfileResponse: Decodable {
    let avatarURL: String?

    enum CodingKeys: String, CodingKey {
        case avatarURL = "avatar_url"
    }
}

final class MatrixRustSDKMessageSendService: MessageSending {
    private let sessionStore: AppSessionStore
    private let clientStore: MatrixRustSDKClientStore
    private let httpClient: AuthHTTPClient
    private let jsonDecoder: JSONDecoder

    init(
        sessionStore: AppSessionStore,
        clientStore: MatrixRustSDKClientStore,
        httpClient: AuthHTTPClient = URLSession.shared,
        jsonDecoder: JSONDecoder = JSONDecoder()
    ) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
        self.httpClient = httpClient
        self.jsonDecoder = jsonDecoder
    }

    func send(_ request: MessageSendRequest) async throws -> TimelineItem {
        let body = request.body.trimmingCharacters(in: .whitespacesAndNewlines)
        guard body.isEmpty == false else {
            throw MessageSendError.emptyMessage
        }
        guard request.editEventID == nil else {
            throw MessageSendError.failed
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

            if let replyToEventID = request.replyToEventID {
                try await timeline.sendReply(msg: content, eventId: replyToEventID)
            } else {
                _ = try await timeline.send(msg: content)
            }

            try await clientStore.syncOnce(session: session, fullState: false)
            let eventID = "$local-\(UUID().uuidString)"
            let senderAvatarURL = await profileAvatarURL(for: session.userID, session: session)
            return TimelineItem(
                id: eventID,
                eventID: eventID,
                senderID: session.userID,
                senderAvatarURL: senderAvatarURL,
                timestamp: Date(),
                kind: .text(body),
                replyToEventID: request.replyToEventID,
                isEdited: false,
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
        var url = session.homeserverURL
        url.appendPathComponent("_matrix")
        url.appendPathComponent("client")
        url.appendPathComponent("v3")
        url.appendPathComponent("profile")
        url.appendPathComponent(userID)

        do {
            var request = URLRequest(url: url)
            request.httpMethod = "GET"
            request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")

            let (data, response) = try await httpClient.data(for: request)
            guard let http = response as? HTTPURLResponse,
                  http.statusCode == 200 else {
                return nil
            }

            let profile = try jsonDecoder.decode(MatrixRustSDKProfileResponse.self, from: data)
            return profile.avatarURL.flatMap(URL.init(string:))
        } catch {
            return nil
        }
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
    private var collected: [EventTimelineItem] = []

    func onUpdate(diff: [TimelineDiff]) {
        lock.lock()
        defer { lock.unlock() }

        for timelineItem in diff.flatMap(Self.items(from:)) {
            if let event = timelineItem.asEvent() {
                collected.append(event)
            }
        }
    }

    func items() -> [EventTimelineItem] {
        lock.lock()
        defer { lock.unlock() }

        var seen = Set<String>()
        return collected.filter { item in
            let id = item.eventOrTransactionId.synaraID
            guard seen.contains(id) == false else {
                return false
            }
            seen.insert(id)
            return true
        }
    }

    func waitForItems(timeoutNanoseconds: UInt64) async -> [EventTimelineItem] {
        let deadline = DispatchTime.now().uptimeNanoseconds + timeoutNanoseconds

        while DispatchTime.now().uptimeNanoseconds < deadline {
            let currentItems = items()
            if currentItems.isEmpty == false {
                return currentItems
            }
            try? await Task.sleep(nanoseconds: 100_000_000)
        }

        return items()
    }

    private static func items(from diff: TimelineDiff) -> [MatrixRustSDK.TimelineItem] {
        switch diff {
        case let .append(values), let .reset(values):
            return values
        case let .pushFront(value), let .pushBack(value), let .insert(_, value), let .set(_, value):
            return [value]
        case .clear, .popFront, .popBack, .remove, .truncate:
            return []
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
