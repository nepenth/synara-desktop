import SwiftUI

private struct NotificationsTabView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: RoomListState = .idle
    @State private var isUnreadRoomsExpanded = false
    @State private var roomUpdatesTask: Task<Void, Never>?

    var body: some View {
        Group {
            switch state {
            case .idle, .loading:
                List {
                    Section {
                        SynaraSkeletonList(rowCount: 8)
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
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                NavigationLink {
                    ApprovalsView()
                } label: {
                    Image(systemName: "checkmark.shield")
                }
                .accessibilityLabel("Approvals")
                .accessibilityHint("Review pending and recent agent approvals")
                .accessibilityIdentifier("NotificationsApprovalsButton")
            }
        }
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
        let agentPendingApprovals = AgentPendingInbox.pendingApprovals(from: rooms)

        if NotificationsInboxSections.isCaughtUp(sections: sections, agentPendingCount: agentPendingApprovals.count) {
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

                if agentPendingApprovals.isEmpty == false {
                    Section {
                        ForEach(agentPendingApprovals) { item in
                            agentPendingRow(item)
                        }
                    } header: {
                        notificationsSectionHeader("Agent actions", count: agentPendingApprovals.count)
                    }
                }

                if sections.unreadRooms.isEmpty == false {
                    Section {
                        Button {
                            withAnimation(.easeInOut(duration: 0.2)) {
                                isUnreadRoomsExpanded.toggle()
                            }
                        } label: {
                            HStack(spacing: SynaraSpacing.small) {
                                Text("Unread rooms")
                                    .font(SynaraTypography.body.weight(.medium))
                                    .foregroundStyle(SynaraColor.primaryText)
                                Spacer(minLength: SynaraSpacing.small)
                                Image(systemName: "chevron.right")
                                    .font(.caption.weight(.semibold))
                                    .foregroundStyle(SynaraColor.secondaryText)
                                    .rotationEffect(.degrees(isUnreadRoomsExpanded ? 90 : 0))
                            }
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Unread rooms")
                        .accessibilityValue(isUnreadRoomsExpanded ? "Expanded" : "Collapsed")
                        .accessibilityHint(isUnreadRoomsExpanded ? "Collapses unread rooms" : "Expands unread rooms")
                        .accessibilityIdentifier("NotificationsUnreadRoomsDisclosure")

                        if isUnreadRoomsExpanded {
                            ForEach(sections.unreadRooms) { room in
                                notificationsRow(room)
                            }
                        }
                    } header: {
                        notificationsSectionHeader("Messages", count: sections.unreadRooms.count)
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
        .buttonStyle(.plain)
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
        .accessibilityIdentifier("NotificationsRow-\(room.id)")
    }

    private func agentPendingRow(_ item: AgentPendingApprovalItem) -> some View {
        Button {
            environment.router.route(
                to: .room(id: item.roomID, eventID: item.eventID, title: item.roomName)
            )
        } label: {
            AgentPendingApprovalRow(item: item)
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
        .accessibilityLabel("\(item.title) in \(item.roomName)")
        .accessibilityHint("Opens the room to review this agent action")
        .accessibilityIdentifier("NotificationsAgentRow-\(item.roomID)")
    }

    private func loadInbox() {
        state = .loading
        Task {
            await reloadInbox()
        }
    }

    private func reloadInbox() async {
        let resolvedState = await environment.roomList.loadRooms()
        await MainActor.run {
            state = resolvedState
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
            Text("#")
                .font(SynaraTypography.body.weight(.semibold))
                .foregroundStyle(SynaraColor.secondaryText)
                .frame(width: 24, alignment: .center)
                .accessibilityHidden(true)

            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                Text(room.name)
                    .font(SynaraTypography.body.weight(room.hasUnreadActivity ? .semibold : .regular))
                    .foregroundStyle(SynaraColor.headingText)
                    .lineLimit(1)

                Text(room.lastMessagePreview)
                    .font(SynaraTypography.roomPreview)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(2)

                HStack(spacing: SynaraSpacing.small) {
                    if room.hasHighlight {
                        Label("Mention", systemImage: "at")
                    } else if room.membership == .invited {
                        Label("Invite", systemImage: "envelope")
                    }
                    if room.lastActivityAt.timeIntervalSince1970 > 0 {
                        Text(room.lastActivityAt, style: .relative)
                    }
                }
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.tertiaryText)
            }

            Spacer(minLength: SynaraSpacing.small)

            SynaraUnreadBadge(count: room.unreadBadgeCount, highlighted: room.hasHighlight)
        }
        .frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)
        .padding(.vertical, SynaraSpacing.xSmall)
    }
}

private struct AgentPendingApprovalRow: View {
    let item: AgentPendingApprovalItem

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            Text("#")
                .font(SynaraTypography.body.weight(.semibold))
                .foregroundStyle(SynaraColor.secondaryText)
                .frame(width: 24, alignment: .center)
                .accessibilityHidden(true)

            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                Text(item.title)
                    .font(SynaraTypography.body.weight(.semibold))
                    .foregroundStyle(SynaraColor.headingText)
                    .lineLimit(2)

                Text(item.summary ?? item.roomName)
                    .font(SynaraTypography.roomPreview)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(2)

                HStack(spacing: SynaraSpacing.small) {
                    Label("Agent", systemImage: "sparkles")
                    Text("# \(item.roomName)")
                        .lineLimit(1)
                    if let status = item.status, status.isEmpty == false {
                        Text(status)
                            .fontWeight(.semibold)
                    }
                }
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.tertiaryText)
            }

            Spacer(minLength: SynaraSpacing.small)

            Image(systemName: "chevron.right")
                .font(SynaraTypography.chipLabel)
                .foregroundStyle(SynaraColor.tertiaryText)
                .accessibilityHidden(true)
        }
        .frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)
        .padding(.vertical, SynaraSpacing.xSmall)
    }
}

private struct OptionalTabBadge: ViewModifier {
    let count: Int

    func body(content: Content) -> some View {
        if count > 0 {
            content.badge(count)
        } else {
            content
        }
    }
}

enum AppTab: String, CaseIterable, Identifiable {
    case rooms
    case later
    case notifications
    case settings

    var id: String { rawValue }

    var usesConversationCanvas: Bool {
        switch self {
        case .rooms, .later, .notifications:
            return true
        case .settings:
            return false
        }
    }

    var emptyConversationTitle: String {
        switch self {
        case .rooms:
            return "Select a Room"
        case .notifications:
            return "Select a Notification"
        case .later:
            return "Select an Item"
        case .settings:
            return "Select a Setting"
        }
    }

    var emptyConversationSystemImage: String {
        switch self {
        case .rooms:
            return "bubble.left.and.bubble.right"
        case .notifications:
            return "bell"
        case .later:
            return "clock"
        case .settings:
            return "gearshape"
        }
    }

    var emptyConversationMessage: String {
        switch self {
        case .rooms:
            return "Pick a conversation. The room list stays open beside it."
        case .notifications:
            return "Open a mention, invite, or unread room. The inbox stays visible."
        case .later:
            return "Open a saved message. Your Later list stays visible."
        case .settings:
            return "Choose a settings page to open beside the list."
        }
    }

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
                .modifier(OptionalTabBadge(count: badgeCounts.rooms))
        case .notifications:
            Label("Notifications", systemImage: "bell")
                .accessibilityIdentifier("NotificationsTab")
                .modifier(OptionalTabBadge(count: badgeCounts.notifications))
        case .later:
            Label("Later", systemImage: "clock")
                .accessibilityIdentifier("LaterTab")
        case .settings:
            Label("Settings", systemImage: "gearshape")
                .accessibilityIdentifier("SettingsTab")
        }
    }
}
