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

    static func live() -> AppEnvironment {
        let logger = AppLogger()
        return AppEnvironment(
            session: AppSessionStore(),
            matrix: PlaceholderMatrixClientService(),
            push: PlaceholderPushService(),
            logger: logger,
            settings: InMemorySettingsStore(),
            router: AppRouter(),
            homeserverDiscovery: PlaceholderHomeserverDiscoveryService(),
            auth: PlaceholderAuthService()
        )
    }

    static func mock(
        router: AppRouter = AppRouter(),
        session: AppSessionStore = AppSessionStore(),
        homeserverDiscovery: HomeserverDiscovering = MockHomeserverDiscoveryService(),
        auth: AuthServicing = MockAuthService()
    ) -> AppEnvironment {
        AppEnvironment(
            session: session,
            matrix: MockMatrixClientService(),
            push: MockPushService(),
            logger: MockLoggingService(),
            settings: InMemorySettingsStore(),
            router: router,
            homeserverDiscovery: homeserverDiscovery,
            auth: auth
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
