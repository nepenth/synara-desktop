import SwiftUI

struct RoomListView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: RoomListState = .idle

    var body: some View {
        Group {
            switch state {
            case .idle, .loading:
                SynaraLoadingState(title: environment.matrix.syncStatusDescription)
            case .empty:
                SynaraEmptyState(
                    title: "No Rooms",
                    systemImage: "bubble.left.and.bubble.right",
                    message: environment.matrix.syncStatusDescription
                )
            case .failed(let message):
                SynaraErrorState(title: "Could Not Load Rooms", message: message) {
                    loadRooms()
                }
            case .loaded(let rooms):
                List(rooms) { room in
                    Button {
                        environment.router.route(to: .room(id: room.id))
                    } label: {
                        RoomListRow(room: room)
                    }
                    .accessibilityIdentifier("RoomRow-\(room.id)")
                }
                .accessibilityIdentifier("RoomList")
            }
        }
        .navigationTitle("Rooms")
        .task {
            loadRooms()
        }
    }

    private func loadRooms() {
        state = .loading
        Task {
            let loadedState = await environment.roomList.loadRooms()
            await MainActor.run {
                state = loadedState
            }
        }
    }
}

private struct RoomListRow: View {
    let room: RoomSummary

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            Image(systemName: room.kind == .directMessage ? "person.crop.circle" : "number")
                .foregroundStyle(room.hasHighlight ? SynaraColor.accent : SynaraColor.secondaryText)
                .accessibilityHidden(true)

            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                Text(room.name)
                    .font(SynaraTypography.body)
                    .foregroundStyle(SynaraColor.primaryText)
                    .lineLimit(1)
                Text(room.lastMessagePreview)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(2)
            }

            Spacer()

            if room.unreadCount > 0 {
                Text("\(room.unreadCount)")
                    .font(.caption.weight(.semibold))
                    .padding(.horizontal, SynaraSpacing.small)
                    .padding(.vertical, SynaraSpacing.xSmall)
                    .background(room.hasHighlight ? SynaraColor.accent : SynaraColor.secondarySurface)
                    .foregroundStyle(room.hasHighlight ? Color.white : SynaraColor.primaryText)
                    .clipShape(Capsule())
                    .accessibilityLabel("\(room.unreadCount) unread")
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(room.name), \(room.lastMessagePreview)")
    }
}

struct RoomListView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationStack {
            RoomListView()
        }
        .environment(\.appEnvironment, AppEnvironment.mock())
    }
}
