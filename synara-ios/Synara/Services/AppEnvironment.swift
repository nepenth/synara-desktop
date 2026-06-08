import SwiftUI

struct AppEnvironment {
    let session: AppSessionStore
    let matrix: MatrixClientServicing
    let push: PushServicing
    let logger: LoggingServicing
    let settings: SettingsStoring
    let router: AppRouter
    let homeserverDiscovery: HomeserverDiscovering
    let auth: AuthServicing
    let roomList: RoomListServicing
    let roomMembership: RoomMembershipServicing
    let notificationPermission: NotificationPermissionServicing
    let wipe: LocalWiping
    let timeline: TimelineServicing
    let later: LaterServicing
    let messageSender: MessageSending
    let drafts: DraftStore
    let eventActions: EventActionServicing
    let agentApprovals: AgentApprovalServicing
    let readMarkers: RoomReadMarkerServicing
    let mediaLoader: MediaLoading
    let mediaUploader: MediaUploading
    let crypto: CryptoStatusServicing
    let roomManagement: RoomManagementServicing

    static func live() -> AppEnvironment {
        let logger = AppLogger()
        let secureStore = KeychainSecureSessionStore()
        if ProcessInfo.processInfo.environment["SYNARA_RESET_SESSION_ON_LAUNCH"] == "1" {
            try? secureStore.delete()
            try? MatrixRustSDKClientStore.deletePersistedStores()
        }
        let session = AppSessionStore(
            secureStore: secureStore,
            restorePersistedSession: true
        )
        clearLegacyRestoredSessionIfNeeded(session: session, logger: logger)
        if let restoreFailure = session.restoreFailureLogDescription {
            logger.error("Session restore failed: \(restoreFailure)", category: .auth)
        } else if case .signedIn = session.currentState {
            logger.info("Session restore succeeded", category: .auth)
        }
        if case .signedIn(let restoredSession) = session.currentState,
           MatrixRustSDKClientStore.persistedStoreExists(for: restoredSession) == false {
            logger.error("Clearing restored session because the Matrix SDK store is missing", category: .auth)
            try? session.signOut()
        }
        let matrixSDKClientStore = MatrixRustSDKClientStore()
        let matrix = MatrixRustSDKMatrixClientService(clientStore: matrixSDKClientStore)
        let pusherService = MatrixPusherService(
            clientStore: matrixSDKClientStore,
            gatewayURL: pushGatewayURL(),
            logger: logger
        )
        let push = SynaraPushService(logger: logger, pusherService: pusherService)
        let roomList = MatrixRustSDKRoomListService(sessionStore: session, clientStore: matrixSDKClientStore)
        let roomMembership = MatrixRustSDKRoomMembershipService(sessionStore: session, clientStore: matrixSDKClientStore)
        let crypto = MatrixRustSDKCryptoStatusService(sessionStore: session, clientStore: matrixSDKClientStore)
        let roomManagement = MatrixRustSDKRoomManagementService(sessionStore: session, clientStore: matrixSDKClientStore)
        return AppEnvironment(
            session: session,
            matrix: matrix,
            push: push,
            logger: logger,
            settings: InMemorySettingsStore(),
            router: AppRouter(),
            homeserverDiscovery: PlaceholderHomeserverDiscoveryService(),
            auth: MatrixRustSDKAuthService(clientStore: matrixSDKClientStore),
            roomList: roomList,
            roomMembership: roomMembership,
            notificationPermission: UserNotificationPermissionService(),
            wipe: AppLocalWipeService(
                session: session,
                matrix: matrix,
                roomList: roomList,
                push: push
            ),
            timeline: MatrixRustSDKTimelineService(sessionStore: session, clientStore: matrixSDKClientStore),
            later: MatrixRustSDKLaterService(sessionStore: session, clientStore: matrixSDKClientStore),
            messageSender: MatrixRustSDKMessageSendService(sessionStore: session, clientStore: matrixSDKClientStore),
            drafts: DraftStore(),
            eventActions: MatrixRustSDKEventActionService(sessionStore: session, clientStore: matrixSDKClientStore),
            agentApprovals: MatrixRustSDKAgentApprovalService(sessionStore: session, clientStore: matrixSDKClientStore),
            readMarkers: MatrixRoomReadMarkerService(sessionStore: session),
            mediaLoader: MatrixMediaLoader(sessionStore: session, clientStore: matrixSDKClientStore),
            mediaUploader: MatrixMediaUploadService(sessionStore: session, clientStore: matrixSDKClientStore),
            crypto: crypto,
            roomManagement: roomManagement
        )
    }

    static func mock(
        router: AppRouter = AppRouter(),
        session: AppSessionStore = AppSessionStore(),
        homeserverDiscovery: HomeserverDiscovering = MockHomeserverDiscoveryService(),
        auth: AuthServicing = MockAuthService(),
        matrix: MatrixClientServicing = MockMatrixClientService(),
        push: PushServicing = MockPushService(),
        roomList: RoomListServicing = MockRoomListService(),
        roomMembership: RoomMembershipServicing = MockRoomMembershipService(),
        notificationPermission: NotificationPermissionServicing = MockNotificationPermissionService(),
        wipe: LocalWiping? = nil,
        timeline: TimelineServicing = MockTimelineService(),
        later: LaterServicing = MockLaterService(),
        messageSender: MessageSending = MockMessageSendService(),
        drafts: DraftStore = DraftStore(),
        eventActions: EventActionServicing = MockEventActionService(),
        agentApprovals: AgentApprovalServicing = MockAgentApprovalService(),
        readMarkers: RoomReadMarkerServicing = MockRoomReadMarkerService(),
        mediaLoader: MediaLoading = MockMediaLoader(),
        mediaUploader: MediaUploading = MockMediaUploadService(),
        crypto: CryptoStatusServicing = MockCryptoStatusService(),
        roomManagement: RoomManagementServicing = MockRoomManagementService()
    ) -> AppEnvironment {
        AppEnvironment(
            session: session,
            matrix: matrix,
            push: push,
            logger: MockLoggingService(),
            settings: InMemorySettingsStore(),
            router: router,
            homeserverDiscovery: homeserverDiscovery,
            auth: auth,
            roomList: roomList,
            roomMembership: roomMembership,
            notificationPermission: notificationPermission,
            wipe: wipe ?? AppLocalWipeService(
                session: session,
                matrix: matrix,
                roomList: roomList,
                push: push
            ),
            timeline: timeline,
            later: later,
            messageSender: messageSender,
            drafts: drafts,
            eventActions: eventActions,
            agentApprovals: agentApprovals,
            readMarkers: readMarkers,
            mediaLoader: mediaLoader,
            mediaUploader: mediaUploader,
            crypto: crypto,
            roomManagement: roomManagement
        )
    }

    private static func pushGatewayURL() -> URL? {
        guard
            let value = ProcessInfo.processInfo.environment["SYNARA_PUSH_GATEWAY_URL"]?.trimmingCharacters(in: .whitespacesAndNewlines),
            value.isEmpty == false,
            let url = URL(string: value),
            url.scheme?.lowercased() == "https",
            url.host?.isEmpty == false
        else {
            return nil
        }

        return url
    }

    private static func clearLegacyRestoredSessionIfNeeded(session: AppSessionStore, logger: LoggingServicing) {
        let defaults = UserDefaults.standard
        let migrationKey = "SynaraClearedLegacyMatrixStartupStores_20260607"
        guard defaults.bool(forKey: migrationKey) == false else {
            return
        }

        defer {
            defaults.set(true, forKey: migrationKey)
        }

        guard case .signedIn = session.currentState else {
            return
        }

        logger.error("Clearing legacy restored Matrix session during startup migration", category: .auth)
        try? MatrixRustSDKClientStore.deletePersistedStores()
        try? session.signOut()
    }
}

private struct AppEnvironmentKey: EnvironmentKey {
    static let defaultValue = AppEnvironment.mock()
}

extension EnvironmentValues {
    var appEnvironment: AppEnvironment {
        get { self[AppEnvironmentKey.self] }
        set { self[AppEnvironmentKey.self] = newValue }
    }
}
