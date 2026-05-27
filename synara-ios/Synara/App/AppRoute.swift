import Foundation

enum AppRoute: Hashable {
    case login(homeserverURL: String)
    case room(id: String)
    case settings
}

enum AppDeepLink: Equatable {
    case room(id: String)
    case settings

    init?(url: URL) {
        let host = url.host?.lowercased()
        let pathComponents = url.pathComponents.filter { $0 != "/" }

        if host == "settings" || pathComponents.first?.lowercased() == "settings" {
            self = .settings
            return
        }

        if host == "room", let id = pathComponents.first {
            self = .room(id: id)
            return
        }

        if pathComponents.first?.lowercased() == "room", let id = pathComponents.dropFirst().first {
            self = .room(id: id)
            return
        }

        return nil
    }
}

enum SheetDestination: String, Identifiable {
    case accountSwitcher

    var id: String { rawValue }
}
