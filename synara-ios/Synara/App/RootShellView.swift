import SwiftUI

struct RootShellView: View {
    let environment: AppEnvironment
    @ObservedObject private var router: AppRouter
    @ObservedObject private var session: AppSessionStore
    @State private var tabBadgeCounts = TabBadgeCounts()
    @State private var tabBadgeUpdatesTask: Task<Void, Never>?

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
                let sessionIsSignedIn: Bool
                if case .signedIn = session.currentState {
                    sessionIsSignedIn = true
                } else {
                    sessionIsSignedIn = false
                }
                _ = router.open(url: url, sessionIsSignedIn: sessionIsSignedIn)
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
        .task(id: "\(authenticatedSession.userID)-\(authenticatedSession.deviceID)-\(session.sessionEpoch)") {
            let signpostID = PerformanceTrace.begin("SignedInSessionStart")
            await SessionCoordinator.startSignedInSession(environment: environment, session: authenticatedSession)
            PerformanceTrace.end("SignedInSessionStart", id: signpostID)
            environment.router.replayPendingDeepLinkIfNeeded(sessionIsSignedIn: true)
            startTabBadgeUpdates()
        }
        .onDisappear {
            tabBadgeUpdatesTask?.cancel()
            tabBadgeUpdatesTask = nil
        }
    }

    private func tab(_ tab: AppTab) -> some View {
        NavigationStack(path: router.binding(for: tab)) {
            tab.content
                .navigationDestination(for: AppRoute.self) { route in
                    RoutePlaceholderView(route: route)
                }
        }
        .tabItem {
            tab.label(badgeCounts: tabBadgeCounts)
        }
        .tag(tab)
    }

    private func startTabBadgeUpdates() {
        tabBadgeUpdatesTask?.cancel()
        tabBadgeUpdatesTask = Task {
            for await update in environment.roomList.roomUpdates() {
                guard Task.isCancelled == false else {
                    return
                }

                let agentPendingCount = await environment.agentApprovals.pendingApprovalCount()
                let rooms = rooms(from: update)

                await MainActor.run {
                    tabBadgeCounts = TabBadgeCounts.make(from: rooms, agentApprovalCount: agentPendingCount)
                }
            }
        }
    }

    private func rooms(from state: RoomListState) -> [RoomSummary] {
        guard case .loaded(let rooms) = state else {
            return []
        }
        return rooms
    }
}

struct RootShellView_Previews: PreviewProvider {
    static var previews: some View {
        RootShellView(environment: .mock())
    }
}