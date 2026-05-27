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
    let wipe: LocalWiping
    let timeline: TimelineServicing

    static func live() -> AppEnvironment {
        let logger = AppLogger()
        let session = AppSessionStore(
            secureStore: KeychainSecureSessionStore(),
            restorePersistedSession: true
        )
        let matrix = PlaceholderMatrixClientService()
        let push = PlaceholderPushService()
        let roomList = PlaceholderRoomListService()
        return AppEnvironment(
            session: session,
            matrix: matrix,
            push: push,
            logger: logger,
            settings: InMemorySettingsStore(),
            router: AppRouter(),
            homeserverDiscovery: PlaceholderHomeserverDiscoveryService(),
            auth: PlaceholderAuthService(),
            roomList: roomList,
            wipe: AppLocalWipeService(
                session: session,
                matrix: matrix,
                roomList: roomList,
                push: push
            ),
            timeline: MockTimelineService()
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
        wipe: LocalWiping? = nil,
        timeline: TimelineServicing = MockTimelineService()
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
            wipe: wipe ?? AppLocalWipeService(
                session: session,
                matrix: matrix,
                roomList: roomList,
                push: push
            ),
            timeline: timeline
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
