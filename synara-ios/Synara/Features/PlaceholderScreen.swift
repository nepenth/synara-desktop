import SwiftUI

struct PlaceholderScreen: View {
    let title: String
    let systemImage: String
    @Environment(\.appEnvironment) private var environment

    var body: some View {
        SynaraEmptyState(
            title: title,
            systemImage: systemImage,
            message: environment.matrix.syncStatusDescription
        )
        .navigationTitle(title)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                SynaraToolbarIconButton(systemImage: "person.crop.circle", accessibilityLabel: "Accounts") {
                    environment.router.present(.accountSwitcher)
                }
            }
        }
        .accessibilityIdentifier("\(title)Screen")
    }
}

struct RoutePlaceholderView: View {
    let route: AppRoute

    var body: some View {
        switch route {
        case .login(let homeserverURL):
            LoginView(homeserverURLString: homeserverURL)
        case .room(let id, let eventID, let title):
            RoomTimelineView(
                roomID: id,
                roomTitle: title,
                focusedEventID: eventID
            )
        case .thread(let roomID, let rootEventID, let roomTitle, let rootTitle):
            ThreadTimelineView(
                roomID: roomID,
                rootEventID: rootEventID,
                roomTitle: roomTitle,
                rootTitle: rootTitle
            )
        case .settings:
            PlaceholderScreen(title: "Settings", systemImage: "gearshape")
        case .notifications:
            PlaceholderScreen(title: "Notifications", systemImage: "bell")
        case .later:
            LaterListView()
        }
    }
}

struct SheetPlaceholderView: View {
    let destination: SheetDestination

    var body: some View {
        switch destination {
        case .accountSwitcher:
            PlaceholderScreen(title: "Accounts", systemImage: "person.crop.circle")
        }
    }
}

struct PlaceholderScreen_Previews: PreviewProvider {
    static var previews: some View {
        NavigationStack {
            PlaceholderScreen(title: "Rooms", systemImage: "bubble.left.and.bubble.right")
        }
        .environment(\.appEnvironment, AppEnvironment.mock())
    }
}
