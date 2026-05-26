import SwiftUI

struct RootShellView: View {
    @State private var selectedTab: AppTab = .rooms
    @State private var roomsPath: [AppRoute] = []
    @State private var notificationsPath: [AppRoute] = []
    @State private var laterPath: [AppRoute] = []
    @State private var settingsPath: [AppRoute] = []
    @State private var sheetDestination: SheetDestination?

    var body: some View {
        TabView(selection: $selectedTab) {
            tab(.rooms, path: $roomsPath)
            tab(.notifications, path: $notificationsPath)
            tab(.later, path: $laterPath)
            tab(.settings, path: $settingsPath)
        }
        .sheet(item: $sheetDestination) { destination in
            SheetPlaceholderView(destination: destination)
        }
    }

    private func tab(_ tab: AppTab, path: Binding<[AppRoute]>) -> some View {
        NavigationStack(path: path) {
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
        RootShellView()
    }
}
