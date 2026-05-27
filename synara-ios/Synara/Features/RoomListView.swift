import SwiftUI

struct RoomListView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: RoomListState = .idle
    @State private var membershipError: String?
    @State private var searchQuery: String = ProcessInfo.processInfo.environment["SYNARA_UI_TEST_ROOM_SEARCH"] ?? ""

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
                let filteredRooms = filteredRooms(from: rooms)
                VStack(spacing: 0) {
                    VStack(spacing: SynaraSpacing.xSmall) {
                        RoomSearchField(text: $searchQuery)
                        RoomListSyncBanner(status: environment.matrix.syncStatusDescription, roomCount: filteredRooms.count)
                    }
                    .padding(.horizontal, SynaraSpacing.large)
                    .padding(.top, SynaraSpacing.small)
                    .background(SynaraColor.surface)

                    List {
                        if searchQuery.isEmpty == false && filteredRooms.isEmpty {
                            SynaraEmptyState(
                                title: "No Matching Rooms",
                                systemImage: "magnifyingglass",
                                message: "Try another room name or message preview."
                            )
                            .listRowInsets(EdgeInsets())
                        }

                        ForEach(filteredRooms) { room in
                            if room.membership == .invited {
                                InviteRoomListRow(
                                    room: room,
                                    onAccept: { updateInvite(roomID: room.id, accept: true) },
                                    onReject: { updateInvite(roomID: room.id, accept: false) }
                                )
                            } else {
                                NavigationLink(value: AppRoute.room(id: room.id, title: room.name)) {
                                    RoomListRow(room: room)
                                        .accessibilityIdentifier("RoomRow-\(room.id)")
                                        .padding(.vertical, SynaraSpacing.xSmall)
                                }
                                .accessibilityLabel(room.accessibilitySummary)
                                .accessibilityHint("Opens the room timeline")
                                .accessibilityIdentifier("RoomRow-\(room.id)")
                            }
                        }
                    }
                    .listStyle(.plain)
                    .accessibilityIdentifier("RoomList")
                }
            }
        }
        .safeAreaInset(edge: .bottom) {
            if let membershipError {
                Text(membershipError)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(.red)
                    .padding(SynaraSpacing.medium)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(SynaraColor.secondarySurface)
                    .accessibilityIdentifier("RoomMembershipErrorText")
            }
        }
        .navigationTitle("Rooms")
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                SynaraToolbarIconButton(systemImage: "person.crop.circle", accessibilityLabel: "Accounts") {
                    environment.router.present(.accountSwitcher)
                }
            }
        }
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
                autoOpenRoomIfRequested(from: loadedState)
            }
        }
    }

    private func autoOpenRoomIfRequested(from state: RoomListState) {
        guard environment.router.roomsPath.isEmpty,
              case .loaded(let rooms) = state else {
            return
        }

        let processEnvironment = ProcessInfo.processInfo.environment
        let requestedRoomID = processEnvironment["SYNARA_AUTO_OPEN_ROOM_ID"]
        let requestedRoomName = processEnvironment["SYNARA_AUTO_OPEN_ROOM_NAME"]
        guard requestedRoomID != nil || requestedRoomName != nil else {
            return
        }

        let room = rooms.first { room in
            if let requestedRoomID, room.id == requestedRoomID {
                return true
            }
            if let requestedRoomName {
                let normalizedRoomName = room.name.trimmingCharacters(in: CharacterSet(charactersIn: "#"))
                if normalizedRoomName.localizedCaseInsensitiveContains(requestedRoomName)
                    || requestedRoomName.localizedCaseInsensitiveContains(normalizedRoomName) {
                    return true
                }
            }
            return false
        }

        if let room {
            environment.router.route(to: .room(id: room.id, title: room.name))
        }
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
                await MainActor.run {
                    loadRooms()
                }
            } catch {
                await MainActor.run {
                    membershipError = RoomMembershipError.failed.localizedDescription
                }
            }
        }
    }

    private func filteredRooms(from rooms: [RoomSummary]) -> [RoomSummary] {
        let query = searchQuery.trimmingCharacters(in: .whitespacesAndNewlines)
        guard query.isEmpty == false else {
            return rooms
        }

        return rooms.filter { room in
            room.name.localizedCaseInsensitiveContains(query)
                || room.lastMessagePreview.localizedCaseInsensitiveContains(query)
        }
    }
}

private struct RoomSearchField: View {
    @Binding var text: String
    @FocusState private var isFocused: Bool

    var body: some View {
        HStack(spacing: SynaraSpacing.small) {
            Image(systemName: "magnifyingglass")
                .foregroundStyle(SynaraColor.secondaryText)
                .accessibilityHidden(true)
            TextField("Search rooms", text: $text)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .focused($isFocused)
                .accessibilityIdentifier("RoomSearchField")
        }
        .padding(SynaraSpacing.medium)
        .synaraCard(fill: SynaraColor.secondarySurface)
        .contentShape(Rectangle())
        .onTapGesture {
            isFocused = true
        }
    }
}

private struct RoomListRow: View {
    let room: RoomSummary

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            SynaraAvatar(
                title: room.name,
                systemImage: room.kind == .directMessage ? "person.fill" : nil,
                tint: room.hasHighlight ? SynaraColor.accent : room.kind == .directMessage ? SynaraColor.agent : SynaraColor.secondaryText
            )

            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.small) {
                    Text(room.name)
                        .font(SynaraTypography.body.weight(room.hasHighlight ? .semibold : .regular))
                        .foregroundStyle(SynaraColor.primaryText)
                        .lineLimit(1)

                    if room.hasHighlight {
                        SynaraStatusChip(title: "Mention", tint: SynaraColor.accent, systemImage: "at")
                    }

                    Spacer(minLength: SynaraSpacing.small)

                    Text(room.relativeActivity)
                        .font(.caption)
                        .foregroundStyle(SynaraColor.tertiaryText)
                        .lineLimit(1)
                }

                Text(room.lastMessagePreview)
                    .font(SynaraTypography.supporting)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .lineLimit(2)
            }

            SynaraUnreadBadge(count: room.unreadCount, highlighted: room.hasHighlight)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct RoomListSyncBanner: View {
    let status: String
    let roomCount: Int

    var body: some View {
        HStack(spacing: SynaraSpacing.small) {
            SynaraStatusChip(title: status, tint: SynaraColor.agent, systemImage: "arrow.triangle.2.circlepath")
            Spacer()
            Text("\(roomCount) rooms")
                .font(.caption)
                .foregroundStyle(SynaraColor.secondaryText)
        }
        .padding(.vertical, SynaraSpacing.xSmall)
    }
}

private struct InviteRoomListRow: View {
    let room: RoomSummary
    let onAccept: () -> Void
    let onReject: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.small) {
            RoomListRow(room: room)

            HStack {
                Button("Accept", action: onAccept)
                    .buttonStyle(.borderedProminent)
                    .accessibilityHint("Joins \(room.name)")
                    .accessibilityIdentifier("AcceptInvite-\(room.id)")

                Button("Decline", role: .destructive, action: onReject)
                    .buttonStyle(.bordered)
                    .accessibilityHint("Declines the invitation to \(room.name)")
                    .accessibilityIdentifier("RejectInvite-\(room.id)")
            }
        }
        .padding(SynaraSpacing.medium)
        .synaraCard(fill: SynaraColor.warning.opacity(0.08), stroke: SynaraColor.warning.opacity(0.25))
    }
}

private extension RoomSummary {
    var accessibilitySummary: String {
        var parts = [name, lastMessagePreview]
        if unreadCount > 0 {
            parts.append("\(unreadCount) unread")
        }
        if hasHighlight {
            parts.append("highlighted")
        }
        return parts.joined(separator: ", ")
    }

    var relativeActivity: String {
        guard lastActivityAt > .distantPast else {
            return ""
        }

        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .abbreviated
        return formatter.localizedString(for: lastActivityAt, relativeTo: RoomListFixtures.now)
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
