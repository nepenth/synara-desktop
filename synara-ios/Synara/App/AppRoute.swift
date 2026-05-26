import Foundation

enum AppRoute: Hashable {
    case room(id: String)
    case settings
}

enum SheetDestination: String, Identifiable {
    case accountSwitcher

    var id: String { rawValue }
}
