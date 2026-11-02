import SwiftUI

private struct NotificationsTabView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: RoomListState = .idle
    @State private var isUnreadRoomsExpanded = false
    @State private var roomUpdatesTask: Task<Void, Never>?
    @State private var membershipError: String?

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
                SynaraErrorState(title: String(localized: "Could Not Load Notifications"), message: message) {
                    loadInbox()
                }
            case .loaded(let rooms):
                notificationsContent(for: rooms)
            }
        }
        .refreshable {
            await reloadInbox()
        }
        .safeAreaInset(edge: .bottom) {
            if let membershipError {
                Text(membershipError)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.criticalText)
                    .padding(SynaraSpacing.medium)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(SynaraColor.secondarySurface)
                    .accessibilityIdentifier("NotificationsMembershipErrorText")
            }
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
        let roomsByID = Dictionary(rooms.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })

        if NotificationsInboxSections.isCaughtUp(sections: sections, agentPendingCount: agentPendingApprovals.count) {
            caughtUpEmptyState
        } else {
            List {
                // A labelled way into Approvals; the toolbar glyph stays for
                // the caught-up state, which has no list.
                Section {
                    approvalsRow(pendingCount: agentPendingApprovals.count)
                }

                if sections.mentions.isEmpty == false {
                    Section {
                        ForEach(sections.mentions) { room in
                            notificationsRow(room)
                        }
                    } header: {
                        RoomListSectionHeader(title: String(localized: "Mentions"), count: sections.mentions.count, identifier: "mentions")
                    }
                }

                if sections.invites.isEmpty == false {
                    Section {
                        ForEach(sections.invites) { room in
                            inviteRow(room)
                        }
                    } header: {
                        RoomListSectionHeader(title: String(localized: "Invites"), count: sections.invites.count, identifier: "invites")
                    }
                }

                if agentPendingApprovals.isEmpty == false {
                    Section {
                        ForEach(agentPendingApprovals) { item in
                            agentPendingRow(item, room: roomsByID[item.roomID])
                        }
                    } header: {
                        RoomListSectionHeader(title: String(localized: "Agent actions"), count: agentPendingApprovals.count, identifier: "agentActions")
                    }
                }

                if sections.unreadRooms.isEmpty == false {
                    Section {
                        if isUnreadRoomsExpanded {
                            ForEach(sections.unreadRooms) { room in
                                notificationsRow(room)
                            }
                        }
                    } header: {
                        RoomListSectionHeader(
                            title: String(localized: "Unread rooms"),
                            count: sections.unreadRooms.count,
                            isCollapsed: isUnreadRoomsExpanded == false,
                            identifier: "unreadRooms"
                        ) {
                            withAnimation(.easeInOut(duration: 0.2)) {
                                isUnreadRoomsExpanded.toggle()
                            }
                        }
                        .accessibilityIdentifier("NotificationsUnreadRoomsDisclosure")
                    }
                }
            }
            .listStyle(.plain)
            .accessibilityIdentifier("NotificationsInboxList")
        }
    }

    private var caughtUpEmptyState: some View {
        SynaraEmptyState(
            title: String(localized: "You're Caught Up"),
            systemImage: "bell.slash",
            message: String(localized: "Unread rooms and mentions will appear here. Push alerts still open the relevant room directly.")
        )
    }

    private func approvalsRow(pendingCount: Int) -> some View {
        NavigationLink {
            ApprovalsView()
        } label: {
            HStack(spacing: SynaraSpacing.medium) {
                Image(systemName: "checkmark.shield")
                    .font(SynaraTypography.body.weight(.semibold))
                    .foregroundStyle(SynaraColor.accent)
                    .frame(width: SynaraMetrics.roomGlyph, alignment: .center)
                    .accessibilityHidden(true)
                Text("Approvals")
                    .font(SynaraTypography.body.weight(pendingCount > 0 ? .bold : .regular))
                    .foregroundStyle(SynaraColor.headingText)
                Spacer(minLength: SynaraSpacing.small)
                if pendingCount > 0 {
                    SynaraUnreadBadge(count: pendingCount, highlighted: true)
                }
            }
            .padding(.horizontal, SynaraSpacing.medium)
            .padding(.vertical, SynaraMetrics.roomRowVerticalPadding)
            .frame(minHeight: SynaraMetrics.roomRowMinHeight)
        }
        .listRowInsets(EdgeInsets(top: 1, leading: SynaraSpacing.small, bottom: 1, trailing: SynaraSpacing.small))
        .accessibilityHint("Review pending and recent agent approvals")
        .accessibilityIdentifier("NotificationsApprovalsRow")
    }

    private func notificationsRow(_ room: RoomSummary) -> some View {
        Button {
            environment.router.route(to: .room(id: room.id, title: room.name))
        } label: {
            NotificationsInboxRow(room: room)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, SynaraSpacing.medium)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
        .listRowInsets(EdgeInsets(top: 1, leading: SynaraSpacing.small, bottom: 1, trailing: SynaraSpacing.small))
        .accessibilityIdentifier("NotificationsRow-\(room.id)")
    }

    /// An invited room has no timeline to open yet, so the invites section
    /// acts on the invite in place, like the Rooms list.
    private func inviteRow(_ room: RoomSummary) -> some View {
        InviteRoomListRow(
            room: room,
            onAccept: { updateInvite(roomID: room.id, accept: true) },
            onReject: { updateInvite(roomID: room.id, accept: false) }
        )
        .listRowSeparator(.hidden)
    }

    private func updateInvite(roomID: String, accept: Bool) {
        membershipError = nil

        Task {
            do {
                if accept {
                    try await environment.roomMembership.acceptInvite(roomID: roomID)
                } else {
                    try await environment.roomMembership.rejectInvite(roomID: roomID)
                }
                if accept {
                    await MainActor.run {
                        SynaraHaptics.trigger(.success)
                    }
                }
                await reloadInbox()
            } catch {
                await MainActor.run {
                    membershipError = RoomMembershipError.failed.localizedDescription
                }
            }
        }
    }

    private func agentPendingRow(_ item: AgentPendingApprovalItem, room: RoomSummary?) -> some View {
        Button {
            environment.router.route(
                to: .room(id: item.roomID, eventID: item.eventID, title: item.roomName)
            )
        } label: {
            AgentPendingApprovalRow(item: item, room: room)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, SynaraSpacing.medium)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
        .listRowInsets(EdgeInsets(top: 1, leading: SynaraSpacing.small, bottom: 1, trailing: SynaraSpacing.small))
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

/// Why a room is in the Notifications inbox. An agent approval prompt is
/// labeled as one even when the push rules also highlighted it.
enum NotificationsInboxReason: Equatable {
    case approval
    case mention
    case invite

    init?(room: RoomSummary) {
        if room.lastMessageIsAgentApproval {
            self = .approval
        } else if room.hasHighlight {
            self = .mention
        } else if room.membership == .invited {
            self = .invite
        } else {
            return nil
        }
    }

    var title: String {
        switch self {
        case .approval:
            return String(localized: "Approval")
        case .mention:
            return String(localized: "Mention")
        case .invite:
            return String(localized: "Invite")
        }
    }

    var systemImage: String {
        switch self {
        case .approval:
            return "checkmark.shield"
        case .mention:
            return "at"
        case .invite:
            return "envelope"
        }
    }
}

private struct NotificationsInboxRow: View {
    let room: RoomSummary

    /// Same badge rule as the Rooms list: red count for a mention or an
    /// unread direct message, bold name alone for a plain unread channel.
    private var showsCountBadge: Bool {
        room.hasHighlight || (room.kind == .directMessage && room.hasUnreadActivity)
    }

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            SynaraRoomGlyphView(
                room: room,
                tint: room.hasUnreadActivity ? SynaraColor.headingText : SynaraColor.secondaryText
            )

            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                Text(room.name)
                    .font(SynaraTypography.body.weight(room.hasUnreadActivity ? .bold : .regular))
                    .foregroundStyle(SynaraColor.headingText)
                    .lineLimit(1)

                Text(room.lastMessagePreview)
                    .font(SynaraTypography.roomPreview)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(2)

                HStack(spacing: SynaraSpacing.small) {
                    if let reason = NotificationsInboxReason(room: room) {
                        Label(reason.title, systemImage: reason.systemImage)
                    }
                    if room.lastActivityAt.timeIntervalSince1970 > 0 {
                        Text(room.lastActivityAt, style: .relative)
                    }
                }
                .font(SynaraTypography.messageMeta)
                .foregroundStyle(SynaraColor.tertiaryText)
            }

            Spacer(minLength: SynaraSpacing.small)

            if showsCountBadge {
                SynaraUnreadBadge(count: room.unreadBadgeCount, highlighted: true)
            }
        }
        .frame(maxWidth: .infinity, minHeight: SynaraMetrics.roomRowMinHeight, alignment: .leading)
        .padding(.vertical, SynaraMetrics.roomRowVerticalPadding)
    }
}

private struct AgentPendingApprovalRow: View {
    let item: AgentPendingApprovalItem
    /// The room's list summary, for the `#` / person glyph; nil falls back to `#`.
    var room: RoomSummary? = nil

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            SynaraRoomGlyphView(room: room, tint: SynaraColor.headingText)

            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                Text(item.title)
                    .font(SynaraTypography.body.weight(.bold))
                    .foregroundStyle(SynaraColor.headingText)
                    .lineLimit(2)

                Text(item.summary ?? item.roomName)
                    .font(SynaraTypography.roomPreview)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(2)

                HStack(spacing: SynaraSpacing.small) {
                    Label("Agent", systemImage: "terminal")
                    Text(item.roomName)
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
        .frame(maxWidth: .infinity, minHeight: SynaraMetrics.roomRowMinHeight, alignment: .leading)
        .padding(.vertical, SynaraMetrics.roomRowVerticalPadding)
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
            return String(localized: "Select a Room")
        case .notifications:
            return String(localized: "Select a Notification")
        case .later:
            return String(localized: "Select an Item")
        case .settings:
            return String(localized: "Select a Setting")
        }
    }

    var emptyConversationSystemImage: String {
        switch self {
        case .rooms:
            return "bubble.left.and.bubble.right"
        case .notifications:
            return "bell"
        case .later:
            return "bookmark"
        case .settings:
            return "gearshape"
        }
    }

    var emptyConversationMessage: String {
        switch self {
        case .rooms:
            return String(localized: "Pick a conversation. The room list stays open beside it.")
        case .notifications:
            return String(localized: "Open a mention, invite, or unread room. The inbox stays visible.")
        case .later:
            return String(localized: "Open a saved message. Your Later list stays visible.")
        case .settings:
            return String(localized: "Choose a settings page to open beside the list.")
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
            // The bookmark is Later's glyph on desktop too (DS-7).
            Label("Later", systemImage: "bookmark")
                .accessibilityIdentifier("LaterTab")
        case .settings:
            Label("Settings", systemImage: "gearshape")
                .accessibilityIdentifier("SettingsTab")
        }
    }
}
