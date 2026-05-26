import SwiftUI

struct PlaceholderScreen: View {
    let title: String
    let systemImage: String

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: systemImage)
                .font(.system(size: 42, weight: .semibold))
                .foregroundStyle(.secondary)
            Text(title)
                .font(.title2.weight(.semibold))
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
            .navigationTitle(title)
            .accessibilityIdentifier("\(title)Screen")
    }
}

struct RoutePlaceholderView: View {
    let route: AppRoute

    var body: some View {
        switch route {
        case .room(let id):
            PlaceholderScreen(title: "Room \(id)", systemImage: "number")
        case .settings:
            PlaceholderScreen(title: "Settings", systemImage: "gearshape")
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
    }
}
