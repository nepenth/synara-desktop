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
    let wipe: LocalWiping
    let timeline: TimelineServicing
    let messageSender: MessageSending
    let drafts: DraftStore
    let eventActions: EventActionServicing
    let mediaLoader: MediaLoading
    let mediaUploader: MediaUploading

    static func live() -> AppEnvironment {
        let logger = AppLogger()
        let session = AppSessionStore(
            secureStore: KeychainSecureSessionStore(),
            restorePersistedSession: true
        )
        if let restoreFailure = session.restoreFailureLogDescription {
            logger.error("Session restore failed: \(restoreFailure)", category: .auth)
        } else if case .signedIn = session.currentState {
            logger.info("Session restore succeeded", category: .auth)
        }
        let matrix = PlaceholderMatrixClientService()
        let push = PlaceholderPushService()
        let roomList = MatrixRoomListService(sessionStore: session)
        let roomMembership = MatrixRoomMembershipService(sessionStore: session)
        return AppEnvironment(
            session: session,
            matrix: matrix,
            push: push,
            logger: logger,
            settings: InMemorySettingsStore(),
            router: AppRouter(),
            homeserverDiscovery: PlaceholderHomeserverDiscoveryService(),
            auth: MatrixPasswordAuthService(),
            roomList: roomList,
            roomMembership: roomMembership,
            wipe: AppLocalWipeService(
                session: session,
                matrix: matrix,
                roomList: roomList,
                push: push
            ),
            timeline: MatrixTimelineService(sessionStore: session),
            messageSender: MatrixMessageSendService(sessionStore: session),
            drafts: DraftStore(),
            eventActions: MatrixEventActionService(sessionStore: session),
            mediaLoader: MatrixMediaLoader(sessionStore: session),
            mediaUploader: MatrixMediaUploadService(sessionStore: session)
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
        wipe: LocalWiping? = nil,
        timeline: TimelineServicing = MockTimelineService(),
        messageSender: MessageSending = MockMessageSendService(),
        drafts: DraftStore = DraftStore(),
        eventActions: EventActionServicing = MockEventActionService(),
        mediaLoader: MediaLoading = MockMediaLoader(),
        mediaUploader: MediaUploading = MockMediaUploadService()
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
            wipe: wipe ?? AppLocalWipeService(
                session: session,
                matrix: matrix,
                roomList: roomList,
                push: push
            ),
            timeline: timeline,
            messageSender: messageSender,
            drafts: drafts,
            eventActions: eventActions,
            mediaLoader: mediaLoader,
            mediaUploader: mediaUploader
        )
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
