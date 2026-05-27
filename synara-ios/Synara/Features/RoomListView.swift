import SwiftUI

struct RoomListView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: RoomListState = .idle
    @State private var membershipError: String?

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
                List {
                    ForEach(rooms) { room in
                        if room.membership == .invited {
                            InviteRoomListRow(
                                room: room,
                                onAccept: { updateInvite(roomID: room.id, accept: true) },
                                onReject: { updateInvite(roomID: room.id, accept: false) }
                            )
                        } else {
                            NavigationLink(value: AppRoute.room(id: room.id, title: room.name)) {
                                RoomListRow(room: room)
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
        .frame(maxWidth: .infinity, alignment: .leading)
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
        .background(SynaraColor.secondarySurface)
        .clipShape(RoundedRectangle(cornerRadius: 8))
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
}

struct RoomListView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationStack {
            RoomListView()
        }
        .environment(\.appEnvironment, AppEnvironment.mock())
    }
}
