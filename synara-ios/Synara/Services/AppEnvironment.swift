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
    let sessionReadiness: SignedInSessionReadinessServicing

    @MainActor
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
            do {
                try MatrixRustSDKClientStore.pruneLegacyPersistedStores()
                logger.info("Pruned legacy Matrix SDK stores after session restore", category: .auth)
            } catch {
                logger.error("Could not prune legacy Matrix SDK stores", category: .auth)
            }
        }
        let matrixSDKClientStore = MatrixRustSDKClientStore(logger: logger)
        let matrix = MatrixRustSDKMatrixClientService(clientStore: matrixSDKClientStore)
        let pusherService = MatrixPusherService(
            clientStore: matrixSDKClientStore,
            gatewayURL: resolvedPushGatewayURL(),
            logger: logger
        )
        let sparsePushRouteResolver = MatrixSparsePushRouteResolver(
            sessionStore: session,
            clientStore: matrixSDKClientStore
        )
        let push = SynaraPushService(
            logger: logger,
            pusherService: pusherService,
            sparseRouteResolver: sparsePushRouteResolver
        )
        let router = AppRouter()
        let drafts = DraftStore()
        let timeline = MatrixRustSDKTimelineService(sessionStore: session, clientStore: matrixSDKClientStore)
        let roomList = MatrixRustSDKRoomListService(
            sessionStore: session,
            clientStore: matrixSDKClientStore,
            logger: logger
        )
        let roomMembership = MatrixRustSDKRoomMembershipService(sessionStore: session, clientStore: matrixSDKClientStore)
        let crypto = MatrixRustSDKCryptoStatusService(sessionStore: session, clientStore: matrixSDKClientStore)
        let roomManagement = MatrixRustSDKRoomManagementService(sessionStore: session, clientStore: matrixSDKClientStore)
        let sessionReadiness = SignedInSessionReadiness()
        return AppEnvironment(
            session: session,
            matrix: matrix,
            push: push,
            logger: logger,
            settings: UserDefaultsSettingsStore(),
            router: router,
            homeserverDiscovery: PlaceholderHomeserverDiscoveryService(),
            auth: MatrixRustSDKAuthService(clientStore: matrixSDKClientStore),
            roomList: roomList,
            roomMembership: roomMembership,
            notificationPermission: UserNotificationPermissionService(),
            wipe: AppLocalWipeService(
                session: session,
                matrix: matrix,
                roomList: roomList,
                timeline: timeline,
                drafts: drafts,
                push: push,
                router: router
            ),
            timeline: timeline,
            later: MatrixRustSDKLaterService(sessionStore: session, clientStore: matrixSDKClientStore),
            messageSender: MatrixRustSDKMessageSendService(sessionStore: session, clientStore: matrixSDKClientStore),
            drafts: drafts,
            eventActions: MatrixRustSDKEventActionService(sessionStore: session, clientStore: matrixSDKClientStore),
            agentApprovals: MatrixRustSDKAgentApprovalService(sessionStore: session, clientStore: matrixSDKClientStore),
            readMarkers: MatrixRoomReadMarkerService(sessionStore: session, clientStore: matrixSDKClientStore),
            mediaLoader: MatrixMediaLoader(sessionStore: session, clientStore: matrixSDKClientStore),
            mediaUploader: MatrixMediaUploadService(sessionStore: session, clientStore: matrixSDKClientStore),
            crypto: crypto,
            roomManagement: roomManagement,
            sessionReadiness: sessionReadiness
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
        roomManagement: RoomManagementServicing = MockRoomManagementService(),
        sessionReadiness: SignedInSessionReadinessServicing = ImmediateSignedInSessionReadiness(),
        settings: SettingsStoring = InMemorySettingsStore()
    ) -> AppEnvironment {
        AppEnvironment(
            session: session,
            matrix: matrix,
            push: push,
            logger: MockLoggingService(),
            settings: settings,
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
                timeline: timeline,
                drafts: drafts,
                push: push,
                router: router
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
            roomManagement: roomManagement,
            sessionReadiness: sessionReadiness
        )
    }

    static func configuredPushGatewayURL(environmentValue: String?, bundleValue: String?) -> URL? {
        parsePushGatewayURL(environmentValue) ?? parsePushGatewayURL(bundleValue)
    }

    private static func resolvedPushGatewayURL() -> URL? {
        configuredPushGatewayURL(
            environmentValue: ProcessInfo.processInfo.environment["SYNARA_PUSH_GATEWAY_URL"],
            bundleValue: Bundle.main.object(forInfoDictionaryKey: "SynaraPushGatewayURL") as? String
        )
    }

    private static func parsePushGatewayURL(_ value: String?) -> URL? {
        guard
            let value = value?.trimmingCharacters(in: .whitespacesAndNewlines),
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
