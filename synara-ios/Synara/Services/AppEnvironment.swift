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
        if let restoreFailure = session.restoreFailureLogDescription {
            logger.error("Session restore failed: \(restoreFailure)", category: .auth)
        } else if case .signedIn = session.currentState {
            logger.info("Session restore succeeded", category: .auth)
        }
        let matrixSDKClientStore = MatrixRustSDKClientStore()
        let matrix = MatrixRustSDKMatrixClientService(clientStore: matrixSDKClientStore)
        let pusherService = MatrixPusherService(
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
            later: MatrixAccountDataLaterService(sessionStore: session),
            messageSender: MatrixRustSDKMessageSendService(sessionStore: session, clientStore: matrixSDKClientStore),
            drafts: DraftStore(),
            eventActions: MatrixEventActionService(sessionStore: session),
            agentApprovals: MatrixAgentApprovalService(sessionStore: session),
            mediaLoader: MatrixMediaLoader(sessionStore: session),
            mediaUploader: MatrixMediaUploadService(sessionStore: session),
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
