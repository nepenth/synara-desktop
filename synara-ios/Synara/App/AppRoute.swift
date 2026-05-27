import Foundation

enum AppRoute: Hashable {
    case login(homeserverURL: String)
    case room(id: String, eventID: String? = nil, title: String? = nil)
    case settings
    case notifications
    case later
}

enum AppDeepLink: Equatable {
    case room(id: String, eventID: String? = nil)
    case settings
    case notifications
    case later

    init?(url: URL) {
        let host = url.host?.lowercased()
        let pathComponents = url.pathComponents.filter { $0 != "/" }

        if host == "settings" || pathComponents.first?.lowercased() == "settings" {
            self = .settings
            return
        }

        if host == "notifications" || host == "notification" || pathComponents.first?.lowercased() == "notifications" {
            self = .notifications
            return
        }

        if host == "later" || pathComponents.first?.lowercased() == "later" {
            self = .later
            return
        }

        if host == "route" {
            let encodedPath = String(url.path.dropFirst())
            let decodedPath = encodedPath.removingPercentEncoding ?? encodedPath
            let normalizedPath = decodedPath.hasPrefix("/") ? decodedPath : "/\(decodedPath)"
            if let routeURL = URL(string: "synara://host\(normalizedPath)"),
               let parsed = AppDeepLink(url: routeURL) {
                self = parsed
                return
            }
        }

        if host == "inbox" || pathComponents.first?.lowercased() == "inbox" {
            let normalized = pathComponents.map { $0.lowercased() }
            if normalized.contains("later") {
                self = .later
            } else if normalized.contains("notifications") || normalized.contains("invites") {
                self = .notifications
            } else {
                self = .notifications
            }
            return
        }

        if host == "room", let id = pathComponents.first {
            let eventID = pathComponents.dropFirst().first
            self = .room(id: id, eventID: eventID)
            return
        }

        if pathComponents.first?.lowercased() == "room", let id = pathComponents.dropFirst().first {
            let eventID = pathComponents.dropFirst().dropFirst().first
            self = .room(id: id, eventID: eventID)
            return
        }

        return nil
    }
}

enum SheetDestination: String, Identifiable {
    case accountSwitcher

    var id: String { rawValue }
}
