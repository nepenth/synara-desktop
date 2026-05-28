import SwiftUI

struct RoomListView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: RoomListState = .idle
    @State private var membershipError: String?
    @State private var searchQuery: String = ProcessInfo.processInfo.environment["SYNARA_UI_TEST_ROOM_SEARCH"] ?? ""
    @State private var selectedFilter: RoomListFilter = .all
    @State private var selectedSpaceID: String?
    @State private var isRoomManagementSheetPresented = ProcessInfo.processInfo.environment["SYNARA_UI_TEST_ROOM_MANAGEMENT_SHEET"] == "1"

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
                let channelRooms = filteredRooms.filter { $0.kind == .room }
                let directRooms = filteredRooms.filter { $0.kind == .directMessage }
                let spaces = spaces(from: rooms)
                VStack(spacing: 0) {
                    VStack(spacing: SynaraSpacing.medium) {
                        RoomListHeader(
                            onAccount: { environment.router.present(.accountSwitcher) },
                            onNewRoom: { isRoomManagementSheetPresented = true }
                        )
                        RoomSearchField(text: $searchQuery)
                        RoomFilterStrip(selectedFilter: $selectedFilter)
                        if spaces.isEmpty == false {
                            SpaceFilterStrip(spaces: spaces, selectedSpaceID: $selectedSpaceID)
                        }
                    }
                    .padding(.horizontal, SynaraSpacing.large)
                    .padding(.top, SynaraSpacing.medium)
                    .padding(.bottom, SynaraSpacing.small)
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

                        if channelRooms.isEmpty == false {
                            Section {
                                ForEach(channelRooms) { room in
                                    roomRow(room)
                                }
                            } header: {
                                RoomSectionHeader(title: "Rooms", count: channelRooms.count)
                            }
                        }

                        if directRooms.isEmpty == false {
                            Section {
                                ForEach(directRooms) { room in
                                    roomRow(room)
                                }
                            } header: {
                                RoomSectionHeader(title: "Direct messages", count: directRooms.count)
                            }
                        }
                    }
                    .listStyle(.plain)
                    .scrollContentBackground(.hidden)
                    .background(SynaraColor.surface)
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
            } else {
                Color.clear
                    .frame(height: 68)
            }
        }
        .navigationTitle("Rooms")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar(.hidden, for: .navigationBar)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                SynaraToolbarIconButton(systemImage: "person.crop.circle", accessibilityLabel: "Accounts") {
                    environment.router.present(.accountSwitcher)
                }
            }
        }
        .sheet(isPresented: $isRoomManagementSheetPresented) {
            RoomManagementSheet { result in
                isRoomManagementSheetPresented = false
                loadRooms()
                environment.router.route(to: .room(id: result.roomID, title: result.name))
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
        var scopedRooms = rooms

        if let selectedSpaceID {
            scopedRooms = scopedRooms.filter { room in
                room.parentSpaces.contains(where: { $0.id == selectedSpaceID })
            }
        }

        switch selectedFilter {
        case .all:
            break
        case .unread:
            scopedRooms = scopedRooms.filter { $0.unreadCount > 0 }
        case .mentions:
            scopedRooms = scopedRooms.filter(\.hasHighlight)
        case .favorites:
            scopedRooms = scopedRooms.filter { room in
                room.name.localizedCaseInsensitiveContains("favorite")
                    || room.name.localizedCaseInsensitiveContains("star")
            }
        }

        guard query.isEmpty == false else {
            return scopedRooms
        }

        return scopedRooms.filter { room in
            room.name.localizedCaseInsensitiveContains(query)
                || room.lastMessagePreview.localizedCaseInsensitiveContains(query)
        }
    }

    private func spaces(from rooms: [RoomSummary]) -> [SpaceSummary] {
        Array(Set(rooms.flatMap(\.parentSpaces))).sorted { $0.name < $1.name }
    }

    @ViewBuilder
    private func roomRow(_ room: RoomSummary) -> some View {
        if room.membership == .invited {
            InviteRoomListRow(
                room: room,
                onAccept: { updateInvite(roomID: room.id, accept: true) },
                onReject: { updateInvite(roomID: room.id, accept: false) }
            )
            .listRowSeparator(.hidden)
            .listRowBackground(SynaraColor.surface)
        } else {
            NavigationLink(value: AppRoute.room(id: room.id, title: room.name)) {
                RoomListRow(room: room)
                    .accessibilityIdentifier("RoomRow-\(room.id)")
                    .padding(.vertical, SynaraSpacing.xSmall)
            }
            .accessibilityLabel(room.accessibilitySummary)
            .accessibilityHint("Opens the room timeline")
            .accessibilityIdentifier("RoomRow-\(room.id)")
            .listRowSeparator(.hidden)
            .listRowInsets(EdgeInsets(top: 3, leading: SynaraSpacing.large, bottom: 3, trailing: SynaraSpacing.large))
            .listRowBackground(SynaraColor.surface)
        }
    }
}

private enum RoomListFilter: String, CaseIterable, Identifiable {
    case all = "All"
    case unread = "Unread"
    case mentions = "Mentions"
    case favorites = "Favorites"

    var id: String { rawValue }
}

private struct RoomListHeader: View {
    let onAccount: () -> Void
    let onNewRoom: () -> Void

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            Button(action: onAccount) {
                ZStack(alignment: .bottomTrailing) {
                    SynaraBrandMark(size: 38)
                    Circle()
                        .fill(SynaraColor.success)
                        .frame(width: 10, height: 10)
                        .overlay(Circle().stroke(SynaraColor.surface, lineWidth: 2))
                }
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Accounts")

            HStack(spacing: SynaraSpacing.xSmall) {
                Text("Rooms")
                    .font(.title3.weight(.semibold))
                    .foregroundStyle(SynaraColor.primaryText)
                Image(systemName: "chevron.down")
                    .font(.caption.weight(.semibold))
                    .foregroundStyle(SynaraColor.secondaryText)
            }

            Spacer()

            Button(action: onNewRoom) {
                Image(systemName: "square.and.pencil")
                    .font(.system(size: 17, weight: .semibold))
                    .frame(width: 44, height: 44)
                    .background(SynaraColor.secondaryText.opacity(0.12))
                    .foregroundStyle(SynaraColor.secondaryText)
                    .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control))
            }
            .buttonStyle(.plain)
            .contentShape(Rectangle())
            .accessibilityLabel("New room")
            .accessibilityIdentifier("NewRoomButton")
        }
    }
}

private struct RoomManagementSheet: View {
    enum Mode: String, CaseIterable, Identifiable {
        case room = "Room"
        case dm = "DM"
        case join = "Join"

        var id: String { rawValue }
    }

    @Environment(\.appEnvironment) private var environment
    @Environment(\.dismiss) private var dismiss
    @State private var mode: Mode = .room
    @State private var roomName = ""
    @State private var roomTopic = ""
    @State private var roomVisibility: SynaraRoomVisibility = .private
    @State private var encryptRoom = true
    @State private var userID = ""
    @State private var joinReference = ""
    @State private var directoryQuery = ""
    @State private var directoryResults: [PublicRoomSummary] = []
    @State private var state: SheetState = .idle
    let onComplete: (RoomOperationResult) -> Void

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    Picker("Action", selection: $mode) {
                        ForEach(Mode.allCases) { mode in
                            Text(mode.rawValue).tag(mode)
                        }
                    }
                    .pickerStyle(.segmented)
                    .accessibilityIdentifier("RoomManagementModePicker")
                }

                switch mode {
                case .room:
                    createRoomSection
                case .dm:
                    directMessageSection
                case .join:
                    joinRoomSection
                }

                if case .failed(let message) = state {
                    Section {
                        Text(message)
                            .foregroundStyle(.red)
                            .accessibilityIdentifier("RoomManagementErrorText")
                    }
                }
            }
            .navigationTitle("New")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button(actionTitle, action: submit)
                        .disabled(state.isLoading)
                        .accessibilityIdentifier("RoomManagementSubmitButton")
                }
            }
            .accessibilityIdentifier("RoomManagementSheet")
        }
    }

    private var createRoomSection: some View {
        Section("Create Room") {
            TextField("Name", text: $roomName)
                .accessibilityIdentifier("CreateRoomNameField")
            TextField("Topic", text: $roomTopic)
                .accessibilityIdentifier("CreateRoomTopicField")
            Picker("Visibility", selection: $roomVisibility) {
                ForEach(SynaraRoomVisibility.allCases) { visibility in
                    Text(visibility.rawValue).tag(visibility)
                }
            }
            Toggle("Encrypt room", isOn: $encryptRoom)
                .accessibilityIdentifier("CreateRoomEncryptionToggle")
        }
    }

    private var directMessageSection: some View {
        Section("Start Direct Message") {
            TextField("@user:server", text: $userID)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .accessibilityIdentifier("CreateDMUserField")
            Toggle("Encrypt DM", isOn: $encryptRoom)
                .accessibilityIdentifier("CreateDMEncryptionToggle")
        }
    }

    private var joinRoomSection: some View {
        Group {
            Section("Join Room") {
                TextField("#room:server or !room:server", text: $joinReference)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .accessibilityIdentifier("JoinRoomReferenceField")
            }

            Section("Public Directory") {
                TextField("Search public rooms", text: $directoryQuery)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .accessibilityIdentifier("PublicRoomSearchField")
                Button("Search Directory", action: searchDirectory)
                    .disabled(state.isLoading || directoryQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    .accessibilityIdentifier("PublicRoomSearchButton")

                ForEach(directoryResults) { result in
                    Button {
                        joinReference = result.joinReference
                        submit()
                    } label: {
                        VStack(alignment: .leading, spacing: 3) {
                            Text(result.name)
                                .font(.body.weight(.semibold))
                            if let topic = result.topic, topic.isEmpty == false {
                                Text(topic)
                                    .font(.caption)
                                    .foregroundStyle(SynaraColor.secondaryText)
                                    .lineLimit(2)
                            }
                            Text("\(result.memberCount) members")
                                .font(.caption2)
                                .foregroundStyle(SynaraColor.secondaryText)
                        }
                    }
                    .accessibilityIdentifier("PublicRoomResult-\(result.id)")
                }
            }
        }
    }

    private var actionTitle: String {
        switch mode {
        case .room:
            return "Create"
        case .dm:
            return "Start"
        case .join:
            return "Join"
        }
    }

    private func submit() {
        state = .loading
        Task {
            do {
                let result: RoomOperationResult
                switch mode {
                case .room:
                    result = try await environment.roomManagement.createRoom(
                        RoomCreateRequest(
                            name: roomName,
                            topic: roomTopic,
                            visibility: roomVisibility,
                            isEncrypted: encryptRoom
                        )
                    )
                case .dm:
                    result = try await environment.roomManagement.createDirectMessage(
                        DirectMessageCreateRequest(userID: userID, isEncrypted: encryptRoom)
                    )
                case .join:
                    result = try await environment.roomManagement.joinRoom(RoomJoinRequest(reference: joinReference))
                }

                await MainActor.run {
                    state = .idle
                    onComplete(result)
                }
            } catch let error as RoomManagementError {
                await MainActor.run {
                    state = .failed(error.localizedDescription)
                }
            } catch {
                await MainActor.run {
                    state = .failed(RoomManagementError.failed.localizedDescription)
                }
            }
        }
    }

    private func searchDirectory() {
        state = .loading
        Task {
            do {
                let results = try await environment.roomManagement.searchPublicRooms(query: directoryQuery)
                await MainActor.run {
                    directoryResults = results
                    state = .idle
                }
            } catch let error as RoomManagementError {
                await MainActor.run {
                    state = .failed(error.localizedDescription)
                }
            } catch {
                await MainActor.run {
                    state = .failed(RoomManagementError.failed.localizedDescription)
                }
            }
        }
    }

    private enum SheetState: Equatable {
        case idle
        case loading
        case failed(String)

        var isLoading: Bool {
            if case .loading = self {
                return true
            }
            return false
        }
    }
}

private struct RoomFilterStrip: View {
    @Binding var selectedFilter: RoomListFilter

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: SynaraSpacing.small) {
                ForEach(RoomListFilter.allCases) { filter in
                    SynaraFilterChip(title: filter.rawValue, isSelected: filter == selectedFilter) {
                        selectedFilter = filter
                    }
                }
            }
            .padding(.trailing, SynaraSpacing.large)
        }
        .accessibilityIdentifier("RoomFilterStrip")
    }
}

private struct SpaceFilterStrip: View {
    let spaces: [SpaceSummary]
    @Binding var selectedSpaceID: String?

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: SynaraSpacing.small) {
                SynaraFilterChip(title: "All spaces", isSelected: selectedSpaceID == nil) {
                    selectedSpaceID = nil
                }

                ForEach(spaces) { space in
                    SynaraFilterChip(title: space.name, isSelected: selectedSpaceID == space.id) {
                        selectedSpaceID = space.id
                    }
                    .accessibilityIdentifier("SpaceFilter-\(space.id)")
                }
            }
            .padding(.trailing, SynaraSpacing.large)
        }
        .accessibilityIdentifier("SpaceFilterStrip")
    }
}

private struct RoomSectionHeader: View {
    let title: String
    let count: Int

    var body: some View {
        HStack {
            Text(title)
                .font(.subheadline.weight(.semibold))
                .textCase(nil)
                .foregroundStyle(SynaraColor.primaryText)
            Spacer()
            Text("\(count)")
                .font(.caption.weight(.semibold))
                .foregroundStyle(SynaraColor.secondaryText)
        }
    }
}

private struct RoomSearchField: View {
    @Binding var text: String
    @FocusState private var isFocused: Bool

    var body: some View {
        HStack(spacing: SynaraSpacing.small) {
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
            .frame(height: 44)
            .synaraCard(fill: SynaraColor.secondarySurface)
            .contentShape(Rectangle())
            .onTapGesture {
                isFocused = true
            }

            SynaraActionIconButton(systemImage: "line.3.horizontal.decrease", accessibilityLabel: "Room filters", tint: SynaraColor.secondaryText) {
                isFocused = true
            }
        }
    }
}

private struct RoomListRow: View {
    let room: RoomSummary

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            SynaraIconTile(title: room.name, systemImage: room.roomIconName, tint: room.roomTint, size: 40)

            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.small) {
                    if room.kind == .room {
                        Image(systemName: room.isSecureRoom ? "lock.fill" : "number")
                            .font(.caption.weight(.bold))
                            .foregroundStyle(room.isSecureRoom ? SynaraColor.secure : SynaraColor.secondaryText)
                            .accessibilityHidden(true)
                    }

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
                    .lineLimit(1)
            }

            SynaraUnreadBadge(count: room.unreadCount, highlighted: room.hasHighlight)
        }
        .frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)
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

    var isSecureRoom: Bool {
        name.localizedCaseInsensitiveContains("security")
            || name.localizedCaseInsensitiveContains("secure")
            || name.localizedCaseInsensitiveContains("e2e")
    }

    var isAgentRoom: Bool {
        name.localizedCaseInsensitiveContains("agent")
            || name.localizedCaseInsensitiveContains("workflow")
    }

    var roomIconName: String {
        if kind == .directMessage {
            return "person.fill"
        }
        if isAgentRoom {
            return "sparkles"
        }
        if isSecureRoom {
            return "lock.fill"
        }
        if name.localizedCaseInsensitiveContains("design") {
            return "megaphone.fill"
        }
        if name.localizedCaseInsensitiveContains("ops") {
            return "briefcase.fill"
        }
        return "number"
    }

    var roomTint: Color {
        if kind == .directMessage {
            return SynaraColor.secondaryText
        }
        if isAgentRoom {
            return SynaraColor.design
        }
        if isSecureRoom {
            return SynaraColor.secure
        }
        if name.localizedCaseInsensitiveContains("design") {
            return SynaraColor.design
        }
        if name.localizedCaseInsensitiveContains("ops") {
            return SynaraColor.ops
        }
        return SynaraColor.primaryText
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
