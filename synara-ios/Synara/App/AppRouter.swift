import Foundation
import SwiftUI

final class AppRouter: ObservableObject {
    @Published var selectedTab: AppTab = .rooms
    @Published var authPath: [AppRoute] = []
    @Published var roomsPath: [AppRoute] = []
    @Published var notificationsPath: [AppRoute] = []
    @Published var laterPath: [AppRoute] = []
    @Published var settingsPath: [AppRoute] = []
    @Published var sheetDestination: SheetDestination?
    @Published private(set) var pendingDeepLink: AppRoute?

    func binding(for tab: AppTab) -> Binding<[AppRoute]> {
        switch tab {
        case .rooms:
            return Binding(get: { self.roomsPath }, set: { self.roomsPath = $0 })
        case .notifications:
            return Binding(get: { self.notificationsPath }, set: { self.notificationsPath = $0 })
        case .later:
            return Binding(get: { self.laterPath }, set: { self.laterPath = $0 })
        case .settings:
            return Binding(get: { self.settingsPath }, set: { self.settingsPath = $0 })
        }
    }

    func route(to route: AppRoute, sessionIsSignedIn: Bool = true) {
        guard sessionIsSignedIn || route.mayRouteWhileSignedOut else {
            pendingDeepLink = route
            return
        }

        switch route {
        case .login:
            authPath = [route]
        case .room(let id, let eventID, let title):
            let destination: AppRoute = .room(id: id, eventID: eventID, title: title)
            switch selectedTab {
            case .notifications:
                notificationsPath = [destination]
            case .later:
                laterPath = [destination]
            default:
                selectedTab = .rooms
                roomsPath = [destination]
            }
        case .thread:
            selectedTab = .rooms
            if roomsPath.isEmpty {
                roomsPath = [route]
            } else {
                roomsPath.append(route)
            }
        case .settings:
            selectedTab = .settings
            settingsPath = []
        case .notifications:
            selectedTab = .notifications
            notificationsPath = []
        case .later:
            selectedTab = .later
            laterPath = [route]
        }
    }

    func routeToNotificationFallback() {
        selectedTab = .notifications
    }

    @discardableResult
    func open(url: URL, sessionIsSignedIn: Bool) -> Bool {
        guard let deepLink = AppDeepLink(url: url) else {
            return false
        }

        switch deepLink {
        case .room(let id, let eventID):
            if let eventID {
                route(to: .room(id: id, eventID: eventID, title: nil), sessionIsSignedIn: sessionIsSignedIn)
            } else {
                route(to: .room(id: id, title: nil), sessionIsSignedIn: sessionIsSignedIn)
            }
        case .settings:
            route(to: .settings, sessionIsSignedIn: sessionIsSignedIn)
        case .notifications:
            route(to: .notifications, sessionIsSignedIn: sessionIsSignedIn)
        case .later:
            route(to: .later, sessionIsSignedIn: sessionIsSignedIn)
        }

        return true
    }

    func replayPendingDeepLinkIfNeeded(sessionIsSignedIn: Bool) {
        guard sessionIsSignedIn, let pendingDeepLink else {
            return
        }

        self.pendingDeepLink = nil
        route(to: pendingDeepLink, sessionIsSignedIn: true)
    }

    func present(_ destination: SheetDestination) {
        sheetDestination = destination
    }

    func dismissSheet() {
        sheetDestination = nil
    }

    func popSelectedTabToRoot() {
        switch selectedTab {
        case .rooms:
            roomsPath = []
        case .notifications:
            notificationsPath = []
        case .later:
            laterPath = []
        case .settings:
            settingsPath = []
        }
    }

    @MainActor
    func resetNavigationPathsForAccountChange() {
        selectedTab = .rooms
        authPath = []
        roomsPath = []
        notificationsPath = []
        laterPath = []
        settingsPath = []
        sheetDestination = nil
    }
}

private extension AppRoute {
    var mayRouteWhileSignedOut: Bool {
        switch self {
        case .login:
            return true
        case .room, .thread, .settings, .notifications, .later:
            return false
        }
    }
}
