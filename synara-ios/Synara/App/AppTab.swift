import SwiftUI

private struct NotificationsTabView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: RoomListState = .idle
    @State private var roomUpdatesTask: Task<Void, Never>?

    var body: some View {
        Group {
            switch state {
            case .idle, .loading:
                List {
                    Section {
                        SynaraSkeletonList(rowCount: 8, showsAvatar: false)
                            .listRowSeparator(.hidden)
                            .listRowInsets(EdgeInsets(top: 3, leading: SynaraSpacing.large, bottom: 3, trailing: SynaraSpacing.large))
                    }
                }
                .listStyle(.plain)
                .accessibilityIdentifier("NotificationsLoading")
            case .empty:
                SynaraEmptyState(
                    title: "You're Caught Up",
                    systemImage: "bell.slash",
                    message: "Unread rooms and mentions will appear here. Push alerts still open the relevant room directly."
                )
            case .failed(let message):
                SynaraErrorState(title: "Could Not Load Notifications", message: message) {
                    loadInbox()
                }
            case .loaded(let rooms):
                let unreadRooms = rooms.filter { $0.unreadCount > 0 || $0.hasHighlight || $0.membership == .invited }
                if unreadRooms.isEmpty {
                    SynaraEmptyState(
                        title: "You're Caught Up",
                        systemImage: "bell.slash",
                        message: "Unread rooms and mentions will appear here. Push alerts still open the relevant room directly."
                    )
                } else {
                    List {
                        Section {
                            ForEach(unreadRooms) { room in
                                Button {
                                    environment.router.route(to: .room(id: room.id, title: room.name))
                                } label: {
                                    NotificationsInboxRow(room: room)
                                        .frame(maxWidth: .infinity, alignment: .leading)
                                        .contentShape(Rectangle())
                                }
                                .buttonStyle(SynaraListRowButtonStyle())
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .contentShape(Rectangle())
                                .accessibilityIdentifier("NotificationsRow-\(room.id)")
                            }
                        } header: {
                            Text("\(unreadRooms.count) unread")
                                .font(SynaraTypography.supporting)
                                .foregroundStyle(SynaraColor.secondaryText)
                        }
                    }
                    .listStyle(.plain)
                    .accessibilityIdentifier("NotificationsInboxList")
                }
            }
        }
        .refreshable {
            await reloadInbox()
        }
        .navigationTitle("Notifications")
        .accessibilityIdentifier("NotificationsScreen")
        .task {
            loadInbox()
            startRoomUpdates()
        }
        .onDisappear {
            roomUpdatesTask?.cancel()
            roomUpdatesTask = nil
        }
    }

    private func loadInbox() {
        state = .loading
        Task {
            await reloadInbox()
        }
    }

    private func reloadInbox() async {
        let nextState = await environment.roomList.loadRooms()
        await MainActor.run {
            state = nextState
        }
    }

    private func startRoomUpdates() {
        roomUpdatesTask?.cancel()
        roomUpdatesTask = Task {
            for await update in environment.roomList.roomUpdates() {
                guard Task.isCancelled == false else {
                    return
                }
                await MainActor.run {
                    state = update
                }
            }
        }
    }
}

private struct NotificationsInboxRow: View {
    let room: RoomSummary

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                HStack(spacing: SynaraSpacing.small) {
                    Text(room.name)
                        .font(SynaraTypography.body.weight(room.hasHighlight ? .semibold : .regular))
                        .foregroundStyle(SynaraColor.primaryText)
                        .lineLimit(1)

                    if room.hasHighlight {
                        SynaraStatusChip(title: "Mention", tint: SynaraColor.accent, systemImage: "at")
                    }

                    if room.membership == .invited {
                        SynaraStatusChip(title: "Invite", tint: SynaraColor.accent, systemImage: "envelope")
                    }
                }

                Text(room.lastMessagePreview)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(2)
            }

            Spacer(minLength: SynaraSpacing.small)

            SynaraUnreadBadge(count: room.unreadCount, highlighted: room.hasHighlight)
        }
        .frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)
        .padding(.vertical, SynaraSpacing.xSmall)
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