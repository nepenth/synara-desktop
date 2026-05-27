import SwiftUI

struct AppEnvironment {
    let session: SessionServicing
    let matrix: MatrixClientServicing
    let push: PushServicing
    let logger: LoggingServicing
    let settings: SettingsStoring
    let router: AppRouter
    let homeserverDiscovery: HomeserverDiscovering

    static func live() -> AppEnvironment {
        let logger = AppLogger()
        return AppEnvironment(
            session: PlaceholderSessionService(),
            matrix: PlaceholderMatrixClientService(),
            push: PlaceholderPushService(),
            logger: logger,
            settings: InMemorySettingsStore(),
            router: AppRouter(),
            homeserverDiscovery: PlaceholderHomeserverDiscoveryService()
        )
    }

    static func mock(
        router: AppRouter = AppRouter(),
        session: SessionServicing = MockSessionService(),
        homeserverDiscovery: HomeserverDiscovering = MockHomeserverDiscoveryService()
    ) -> AppEnvironment {
        AppEnvironment(
            session: session,
            matrix: MockMatrixClientService(),
            push: MockPushService(),
            logger: MockLoggingService(),
            settings: InMemorySettingsStore(),
            router: router,
            homeserverDiscovery: homeserverDiscovery
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
