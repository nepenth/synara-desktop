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

    func route(to route: AppRoute) {
        switch route {
        case .login:
            authPath = [route]
        case .room:
            selectedTab = .rooms
            roomsPath = [route]
        case .settings:
            selectedTab = .settings
            settingsPath = [route]
        }
    }

    @discardableResult
    func open(url: URL) -> Bool {
        guard let deepLink = AppDeepLink(url: url) else {
            return false
        }

        switch deepLink {
        case .room(let id):
            route(to: .room(id: id, title: nil))
        case .settings:
            route(to: .settings)
        }

        return true
    }

    func present(_ destination: SheetDestination) {
        sheetDestination = destination
    }

    func dismissSheet() {
        sheetDestination = nil
    }

    func resetForAccountChange() {
        selectedTab = .rooms
        authPath = []
        roomsPath = []
        notificationsPath = []
        laterPath = []
        settingsPath = []
        sheetDestination = nil
    }
}
