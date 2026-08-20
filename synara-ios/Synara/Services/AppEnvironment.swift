import SwiftUI
import SynaraCore

enum SharedCoreLaunchReset {
    static func resetStoreRootIfRequested(
        _ storeRoot: URL,
        environment: [String: String]
    ) throws -> Bool {
        guard environment["SYNARA_RESET_SESSION_ON_LAUNCH"] == "1" else {
            return false
        }
        if FileManager.default.fileExists(atPath: storeRoot.path) {
            try FileManager.default.removeItem(at: storeRoot)
        }
        try FileManager.default.createDirectory(
            at: storeRoot,
            withIntermediateDirectories: true
        )
        return true
    }
}

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
    let agentApprovalReactions: AgentApprovalReactionServicing
    let readMarkers: RoomReadMarkerServicing
    let mediaLoader: MediaLoading
    let mediaUploader: MediaUploading
    let crypto: CryptoStatusServicing
    let roomManagement: RoomManagementServicing
    let sessionReadiness: SignedInSessionReadinessServicing
    let connectionStatus: ConnectionStatusStore
    let outgoingSends: OutgoingSendCoordinator

    @MainActor
    static func live() -> AppEnvironment {
        let logger = AppLogger()
        let secureStore = KeychainSecureSessionStore()
        let storeRoot = SharedCoreProductHost.liveStoreRoot()
        if (try? SharedCoreLaunchReset.resetStoreRootIfRequested(
            storeRoot,
            environment: ProcessInfo.processInfo.environment
        )) == true {
            try? secureStore.delete()
        }
        let core = SharedCore.newWithSecretStore(store: KeychainIosSecretVault())
        let session = AppSessionStore(
            secureStore: secureStore,
            restorePersistedSession: true
        )
        if let restoreFailure = session.restoreFailureLogDescription {
            logger.error("Session restore failed: \(restoreFailure)", category: .auth)
        } else if case .signedIn = session.currentState {
            logger.info("Session restore succeeded", category: .auth)
        }
        let host = SharedCoreProductHost(
            core: core,
            storeRoot: storeRoot,
            sessionStore: session
        )
        let connectionStatus = ConnectionStatusStore()
        let matrix = SharedCoreMatrixClientService(host: host, connectionStatus: connectionStatus)
        let pusherService = SharedCorePusherService(
            host: host,
            gatewayURL: resolvedPushGatewayURL(),
            logger: logger
        )
        let push = SynaraPushService(
            logger: logger,
            pusherService: pusherService,
            sparseRouteResolver: SharedCoreSparsePushRouteResolver()
        )
        let router = AppRouter()
        let drafts = DraftStore()
        let timeline = SharedCoreTimelineService(host: host)
        let roomList = SharedCoreRoomListService(host: host)
        let roomMembership = SharedCoreRoomMembershipService(host: host)
        let crypto = SharedCoreCryptoStatusService(host: host)
        let roomManagement = SharedCoreRoomManagementService(host: host)
        let sessionReadiness = SignedInSessionReadiness()
        let messageSender = SharedCoreMessageSendService(host: host)
        let outgoingSends = OutgoingSendCoordinator(
            messageSender: messageSender,
            connectionStatus: connectionStatus
        )
        return AppEnvironment(
            session: session,
            matrix: matrix,
            push: push,
            logger: logger,
            settings: UserDefaultsSettingsStore(
                defaults: SynaraSharedConstants.appGroupDefaults() ?? .standard
            ),
            router: router,
            homeserverDiscovery: makeLiveHomeserverDiscovery(),
            auth: SharedCoreAuthService(host: host),
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
                router: router,
                outgoingSends: outgoingSends
            ),
            timeline: timeline,
            later: SharedCoreLaterService(host: host),
            messageSender: messageSender,
            drafts: drafts,
            eventActions: SharedCoreEventActionService(host: host),
            agentApprovals: SharedCoreAgentApprovalService(host: host),
            agentApprovalReactions: SharedCoreAgentApprovalReactionService(host: host),
            readMarkers: SharedCoreRoomReadMarkerService(host: host),
            mediaLoader: SharedCoreMediaLoader(host: host),
            mediaUploader: SharedCoreMediaUploadService(host: host),
            crypto: crypto,
            roomManagement: roomManagement,
            sessionReadiness: sessionReadiness,
            connectionStatus: connectionStatus,
            outgoingSends: outgoingSends
        )
    }

    static func makeLiveHomeserverDiscovery() -> HomeserverDiscovering {
        CoreHomeserverDiscoveryService()
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
        agentApprovalReactions: AgentApprovalReactionServicing = MockAgentApprovalReactionService(),
        readMarkers: RoomReadMarkerServicing = MockRoomReadMarkerService(),
        mediaLoader: MediaLoading = MockMediaLoader(),
        mediaUploader: MediaUploading = MockMediaUploadService(),
        crypto: CryptoStatusServicing = MockCryptoStatusService(),
        roomManagement: RoomManagementServicing = MockRoomManagementService(),
        sessionReadiness: SignedInSessionReadinessServicing = ImmediateSignedInSessionReadiness(),
        settings: SettingsStoring = InMemorySettingsStore(),
        connectionStatus: ConnectionStatusStore = ConnectionStatusStore(),
        outgoingSends: OutgoingSendCoordinator? = nil
    ) -> AppEnvironment {
        let resolvedOutgoingSends = outgoingSends ?? OutgoingSendCoordinator(
            messageSender: messageSender,
            connectionStatus: connectionStatus
        )
        return AppEnvironment(
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
                router: router,
                outgoingSends: resolvedOutgoingSends
            ),
            timeline: timeline,
            later: later,
            messageSender: messageSender,
            drafts: drafts,
            eventActions: eventActions,
            agentApprovals: agentApprovals,
            agentApprovalReactions: agentApprovalReactions,
            readMarkers: readMarkers,
            mediaLoader: mediaLoader,
            mediaUploader: mediaUploader,
            crypto: crypto,
            roomManagement: roomManagement,
            sessionReadiness: sessionReadiness,
            connectionStatus: connectionStatus,
            outgoingSends: resolvedOutgoingSends
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
