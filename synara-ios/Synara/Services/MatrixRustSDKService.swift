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
    private var client: Client?
    private var activeSession: AuthenticatedSession?
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
            self.syncStatus = .syncing
            return session
        } catch {
            self.syncStatus = .failed("Could not sign in.")
            throw LoginError.networkFailure
        }
    }

    func start(session: AuthenticatedSession) async {
        syncStatus = .starting

        do {
            let client = try await ensureClient(for: session)
            _ = try await client.syncOnceV2(settings: SyncSettingsV2(timeoutMs: 5_000, fullState: true))
            syncStatus = .syncing
        } catch {
            syncStatus = .failed("Could not start sync.")
        }
    }

    func stop() {
        syncStatus = .stopped
    }

    func resetLocalState() {
        client = nil
        activeSession = nil
        syncStatus = .stopped
        unableToDecryptRecorder.reset()
        try? Self.deletePersistedStores()
    }

    func syncOnce(session: AuthenticatedSession, fullState: Bool = false) async throws {
        let client = try await ensureClient(for: session)
        _ = try await client.syncOnceV2(settings: SyncSettingsV2(timeoutMs: 5_000, fullState: fullState))
        syncStatus = .syncing
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

    private func ensureClient(for session: AuthenticatedSession) async throws -> Client {
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

        self.client = client
        self.activeSession = session
        return client
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

    init(sessionStore: AppSessionStore, clientStore: MatrixRustSDKClientStore) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
    }

    func loadRooms() async -> RoomListState {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .empty
        }

        do {
            try await clientStore.syncOnce(session: session, fullState: true)
            let rooms = try await clientStore.rooms(session: session)
                .compactMap(Self.mapRoom)
            let sorted = RoomListFixtures.sorted(rooms)
            return sorted.isEmpty ? .empty : .loaded(sorted)
        } catch {
            return .failed("Could not load rooms. Try again.")
        }
    }

    func clearCache() {}

    private static func mapRoom(_ room: Room) -> RoomSummary? {
        let membership: RoomSummary.Membership
        switch room.membership() {
        case .joined:
            membership = .joined
        case .invited:
            membership = .invited
        default:
            return nil
        }

        let name = room.displayName() ?? room.canonicalAlias() ?? room.id()
        let encryptedPreview = room.encryptionState() == .encrypted ? "Encrypted room" : "No recent messages"
        return RoomSummary(
            id: room.id(),
            name: name,
            lastMessagePreview: membership == .invited ? "Invited to room" : encryptedPreview,
            unreadCount: membership == .invited ? 1 : 0,
            hasHighlight: membership == .invited,
            kind: .room,
            membership: membership,
            lastActivityAt: Date()
        )
    }
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

    init(
        sessionStore: AppSessionStore,
        clientStore: MatrixRustSDKClientStore,
        rawTimelineFallback: TimelineServicing? = nil
    ) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
        self.rawTimelineFallback = rawTimelineFallback ?? MatrixTimelineService(sessionStore: sessionStore)
    }

    func loadInitialTimeline(roomID: String) async -> [TimelineItem] {
        await loadTimeline(roomID: roomID, pageSize: 50)
    }

    func loadOlderTimeline(roomID: String, before eventID: String) async -> [TimelineItem] {
        await loadTimeline(roomID: roomID, pageSize: 50)
    }

    private func loadTimeline(roomID: String, pageSize: UInt16) async -> [TimelineItem] {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return []
        }

        do {
            try await clientStore.syncOnce(session: session, fullState: false)
            guard let room = try await clientStore.room(roomID: roomID, session: session) else {
                return []
            }

            let timeline = try await room.timeline()
            let collector = MatrixRustSDKTimelineCollector()
            let handle = await timeline.addListener(listener: collector)
            defer { handle.cancel() }

            _ = try await timeline.paginateBackwards(numEvents: pageSize)
            let items = await collector.waitForItems(timeoutNanoseconds: 1_500_000_000)
            let sdkItems = items.compactMap(Self.mapTimelineItem)
                .sorted { $0.timestamp < $1.timestamp }
            let rawAgentItems = await rawAgentFallbackItems(roomID: roomID)
            return Self.mergedTimelineItems(sdkItems: sdkItems, rawAgentItems: rawAgentItems)
        } catch {
            return await rawAgentFallbackItems(roomID: roomID)
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

final class MatrixRustSDKMessageSendService: MessageSending {
    private let sessionStore: AppSessionStore
    private let clientStore: MatrixRustSDKClientStore

    init(sessionStore: AppSessionStore, clientStore: MatrixRustSDKClientStore) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
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
            return TimelineItem(
                id: eventID,
                eventID: eventID,
                senderID: session.userID,
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
