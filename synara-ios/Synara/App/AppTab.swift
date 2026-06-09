import SwiftUI

private struct NotificationsTabView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: RoomListState = .idle
    @State private var agentPendingCount = 0
    @State private var isUnreadRoomsExpanded = false
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
                caughtUpEmptyState
            case .failed(let message):
                SynaraErrorState(title: "Could Not Load Notifications", message: message) {
                    loadInbox()
                }
            case .loaded(let rooms):
                notificationsContent(for: rooms)
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

    @ViewBuilder
    private func notificationsContent(for rooms: [RoomSummary]) -> some View {
        let sections = NotificationsInboxSections.make(from: rooms)
        let showsAgentEmptyState = environment.agentApprovals.supportsPendingApprovalInbox == false
        let hasVisibleContent = sections.hasRoomSections || agentPendingCount > 0 || showsAgentEmptyState

        if hasVisibleContent == false {
            caughtUpEmptyState
        } else {
            List {
                if sections.mentions.isEmpty == false {
                    Section {
                        ForEach(sections.mentions) { room in
                            notificationsRow(room)
                        }
                    } header: {
                        notificationsSectionHeader("Mentions", count: sections.mentions.count)
                    }
                }

                if sections.invites.isEmpty == false {
                    Section {
                        ForEach(sections.invites) { room in
                            notificationsRow(room)
                        }
                    } header: {
                        notificationsSectionHeader("Invites", count: sections.invites.count)
                    }
                }

                if agentPendingCount > 0 {
                    Section {
                        Label {
                            Text(agentPendingCount == 1 ? "1 pending agent action" : "\(agentPendingCount) pending agent actions")
                                .font(SynaraTypography.body)
                                .foregroundStyle(SynaraColor.primaryText)
                        } icon: {
                            Image(systemName: "cpu")
                                .foregroundStyle(SynaraColor.agent)
                        }
                        .accessibilityIdentifier("NotificationsAgentPendingSummary")
                    } header: {
                        notificationsSectionHeader("Agent actions", count: agentPendingCount)
                    }
                } else if showsAgentEmptyState {
                    Section {
                        Text("No pending agent actions")
                            .font(SynaraTypography.supporting)
                            .foregroundStyle(SynaraColor.secondaryText)
                            .accessibilityIdentifier("NotificationsAgentEmptyState")
                    } header: {
                        Text("Agent actions")
                            .font(SynaraTypography.supporting)
                            .foregroundStyle(SynaraColor.secondaryText)
                    }
                }

                if sections.unreadRooms.isEmpty == false {
                    Section {
                        DisclosureGroup(isExpanded: $isUnreadRoomsExpanded) {
                            ForEach(sections.unreadRooms) { room in
                                notificationsRow(room)
                            }
                        } label: {
                            Text("Unread rooms")
                                .font(SynaraTypography.body.weight(.medium))
                                .foregroundStyle(SynaraColor.primaryText)
                        }
                    } header: {
                        notificationsSectionHeader("Unread rooms", count: sections.unreadRooms.count)
                    }
                }
            }
            .listStyle(.plain)
            .accessibilityIdentifier("NotificationsInboxList")
        }
    }

    private var caughtUpEmptyState: some View {
        SynaraEmptyState(
            title: "You're Caught Up",
            systemImage: "bell.slash",
            message: "Unread rooms and mentions will appear here. Push alerts still open the relevant room directly."
        )
    }

    private func notificationsSectionHeader(_ title: String, count: Int) -> some View {
        Text("\(title) · \(count)")
            .font(SynaraTypography.supporting)
            .foregroundStyle(SynaraColor.secondaryText)
    }

    private func notificationsRow(_ room: RoomSummary) -> some View {
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

    private func loadInbox() {
        state = .loading
        Task {
            await reloadInbox()
        }
    }

    private func reloadInbox() async {
        async let nextState = environment.roomList.loadRooms()
        async let pendingApprovals = environment.agentApprovals.pendingApprovalCount()
        let resolvedState = await nextState
        let resolvedPendingApprovals = await pendingApprovals

        await MainActor.run {
            state = resolvedState
            agentPendingCount = resolvedPendingApprovals
        }
    }

    private func startRoomUpdates() {
        roomUpdatesTask?.cancel()
        roomUpdatesTask = Task {
            for await update in environment.roomList.roomUpdates() {
                guard Task.isCancelled == false else {
                    return
                }
                let pendingApprovals = await environment.agentApprovals.pendingApprovalCount()
                await MainActor.run {
                    state = update
                    agentPendingCount = pendingApprovals
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
    func label(badgeCounts: TabBadgeCounts) -> some View {
        switch self {
        case .rooms:
            Label("Rooms", systemImage: "bubble.left.and.bubble.right")
                .accessibilityIdentifier("RoomsTab")
                .badge(badgeCounts.rooms > 0 ? badgeCounts.rooms : 0)
        case .notifications:
            Label("Notifications", systemImage: "bell")
                .accessibilityIdentifier("NotificationsTab")
                .badge(badgeCounts.notifications > 0 ? badgeCounts.notifications : 0)
        case .later:
            Label("Later", systemImage: "clock")
                .accessibilityIdentifier("LaterTab")
        case .settings:
            Label("Settings", systemImage: "gearshape")
                .accessibilityIdentifier("SettingsTab")
        }
    }
}