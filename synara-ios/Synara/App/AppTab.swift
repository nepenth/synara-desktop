import SwiftUI

enum AppTab: String, CaseIterable, Identifiable {
    case rooms
    case notifications
    case later
    case settings

    var id: String { rawValue }

    @ViewBuilder
    var content: some View {
        switch self {
        case .rooms:
            RoomListView()
        case .notifications:
            PlaceholderScreen(title: "Notifications", systemImage: "bell")
        case .later:
            LaterListView()
        case .settings:
            SettingsView()
        }
    }

    @ViewBuilder
    var label: some View {
        switch self {
        case .rooms:
            Label("Rooms", systemImage: "bubble.left.and.bubble.right")
                .accessibilityIdentifier("RoomsTab")
        case .notifications:
            Label("Notifications", systemImage: "bell")
                .accessibilityIdentifier("NotificationsTab")
        case .later:
            Label("Later", systemImage: "clock")
                .accessibilityIdentifier("LaterTab")
        case .settings:
            Label("Settings", systemImage: "gearshape")
                .accessibilityIdentifier("SettingsTab")
        }
    }
}
