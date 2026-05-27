import SwiftUI

struct RootShellView: View {
    let environment: AppEnvironment
    @ObservedObject private var router: AppRouter

    init(environment: AppEnvironment = .mock()) {
        self.environment = environment
        self.router = environment.router
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
        switch environment.session.currentState {
        case .signedOut:
            signedOutShell
        case .signedIn:
            signedInShell
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

    private var signedInShell: some View {
        TabView(selection: $router.selectedTab) {
            tab(.rooms)
            tab(.notifications)
            tab(.later)
            tab(.settings)
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
