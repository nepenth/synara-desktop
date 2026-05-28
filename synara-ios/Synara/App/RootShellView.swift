import SwiftUI

struct RootShellView: View {
    let environment: AppEnvironment
    @ObservedObject private var router: AppRouter
    @ObservedObject private var session: AppSessionStore
    @State private var startedMatrixSessionID: String?

    init(environment: AppEnvironment = .mock()) {
        self.environment = environment
        self.router = environment.router
        self.session = environment.session
    }

    var body: some View {
        content
            .environment(\.appEnvironment, environment)
            .sheet(item: $router.sheetDestination) { destination in
                SheetPlaceholderView(destination: destination)
            }
            .onOpenURL { url in
                environment.logger.info("Opening deep link \(url.absoluteString)", category: .routing)
                _ = router.open(url: url)
            }
    }

    @ViewBuilder
    private var content: some View {
        switch session.currentState {
        case .signedOut:
            signedOutShell
        case .signedIn(let authenticatedSession):
            signedInShell(session: authenticatedSession)
        }
    }

    private var signedOutShell: some View {
        NavigationStack(path: $router.authPath) {
            HomeserverSelectionView()
                .navigationDestination(for: AppRoute.self) { route in
                    RoutePlaceholderView(route: route)
                }
        }
    }

    private func signedInShell(session authenticatedSession: AuthenticatedSession) -> some View {
        TabView(selection: $router.selectedTab) {
            tab(.rooms)
            tab(.later)
            tab(.notifications)
            tab(.settings)
        }
        .task(id: authenticatedSession.userID + authenticatedSession.deviceID) {
            let sessionID = authenticatedSession.userID + authenticatedSession.deviceID
            guard startedMatrixSessionID != sessionID else {
                return
            }
            startedMatrixSessionID = sessionID
            let signpostID = PerformanceTrace.begin("SignedInSessionStart")
            await environment.matrix.start(session: authenticatedSession)
            environment.push.configure(with: authenticatedSession)
            PerformanceTrace.end("SignedInSessionStart", id: signpostID)
        }
    }

    private func tab(_ tab: AppTab) -> some View {
        NavigationStack(path: router.binding(for: tab)) {
            tab.content
                .navigationDestination(for: AppRoute.self) { route in
                    RoutePlaceholderView(route: route)
                }
        }
        .tabItem { tab.label }
        .tag(tab)
    }
}

struct RootShellView_Previews: PreviewProvider {
    static var previews: some View {
        RootShellView(environment: .mock())
    }
}
