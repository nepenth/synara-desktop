import SwiftUI

private struct NotificationsTabView: View {
    var body: some View {
        SynaraEmptyState(
            title: "Notifications Open Rooms",
            systemImage: "bell",
            message: "Synara does not keep a separate notification inbox. Push alerts open the relevant room directly. Check Settings to manage notification permissions and push registration."
        )
        .navigationTitle("Notifications")
        .accessibilityIdentifier("NotificationsScreen")
    }
}

enum AppTab: String, CaseIterable, Identifiable {
    case rooms
    case later
    case notifications
    case settings

    var id: String { rawValue }

    @ViewBuilder
    var content: some View {
        switch self {
        case .rooms:
            RoomListView()
        case .notifications:
            NotificationsTabView()
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
