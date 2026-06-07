import SwiftUI

struct RoomListView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: RoomListState = .idle
    @State private var membershipError: String?
    @State private var searchQuery: String = ProcessInfo.processInfo.environment["SYNARA_UI_TEST_ROOM_SEARCH"] ?? ""
    @State private var selectedFilter: RoomListFilter = .all
    @State private var selectedSpaceID: String?
    @State private var isRoomManagementSheetPresented = ProcessInfo.processInfo.environment["SYNARA_UI_TEST_ROOM_MANAGEMENT_SHEET"] == "1"
    @State private var hasStartedInitialLoad = false
    @State private var isSearchPresented = ProcessInfo.processInfo.environment["SYNARA_UI_TEST_ROOM_SEARCH"] != nil
    @FocusState private var isSearchFocused: Bool

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
                let favoriteRooms = favoriteRooms(from: channelRooms)
                let favoriteRoomIDs = Set(favoriteRooms.map(\.id))
                let otherRooms = channelRooms.filter { favoriteRoomIDs.contains($0.id) == false }
                let spaces = spaces(from: rooms)
                VStack(spacing: 0) {
                    VStack(spacing: SynaraSpacing.medium) {
                        RoomListHeader(
                            title: accountMenuTitle,
                            onAccount: { environment.router.present(.accountSwitcher) },
                            onSearch: { presentSearch() },
                            onNewRoom: { isRoomManagementSheetPresented = true }
                        )
                        if isSearchPresented {
                            RoomSearchField(text: $searchQuery, isFocused: $isSearchFocused) {
                                dismissSearch(clearQuery: false)
                            }
                            .transition(.move(edge: .top).combined(with: .opacity))
                        }
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

                        if favoriteRooms.isEmpty == false {
                            Section {
                                ForEach(favoriteRooms) { room in
                                    roomRow(room)
                                }
                            } header: {
                                RoomSectionHeader(title: "Favorites", count: favoriteRooms.count)
                            }
                        }

                        if otherRooms.isEmpty == false {
                            Section {
                                ForEach(otherRooms) { room in
                                    roomRow(room)
                                }
                            } header: {
                                RoomSectionHeader(title: "Other", count: otherRooms.count)
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
            guard hasStartedInitialLoad == false else {
                return
            }
            hasStartedInitialLoad = true
            loadRooms()
        }
    }

    private func loadRooms() {
        state = .loading
        Task {
            let signpostID = PerformanceTrace.begin("RoomListLoad")
            defer {
                PerformanceTrace.end("RoomListLoad", id: signpostID)
            }
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
            scopedRooms = scopedRooms.filter(\.isFavoriteLike)
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

    private func favoriteRooms(from rooms: [RoomSummary]) -> [RoomSummary] {
        let explicitFavorites = rooms.filter(\.isFavoriteLike)
        if explicitFavorites.isEmpty == false {
            return explicitFavorites
        }
        return Array(rooms.prefix(4))
    }

    private var accountMenuTitle: String {
        guard case .signedIn(let session) = environment.session.currentState else {
            return "Synara"
        }
        if let host = session.homeserverURL.host(percentEncoded: false), host.isEmpty == false {
            return host
        }
        return session.userID
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
            Button {
                dismissSearch(clearQuery: false)
                environment.router.route(to: .room(id: room.id, title: room.name))
            } label: {
                RoomListRow(room: room)
                    .accessibilityIdentifier("RoomRow-\(room.id)")
                    .padding(.vertical, SynaraSpacing.xSmall)
            }
            .buttonStyle(.plain)
            .contentShape(Rectangle())
            .accessibilityLabel(room.accessibilitySummary)
            .accessibilityHint("Opens the room timeline")
            .accessibilityIdentifier("RoomRow-\(room.id)")
            .listRowSeparator(.hidden)
            .listRowInsets(EdgeInsets(top: 3, leading: SynaraSpacing.large, bottom: 3, trailing: SynaraSpacing.large))
            .listRowBackground(SynaraColor.surface)
        }
    }

    private func presentSearch() {
        withAnimation(.easeInOut(duration: 0.18)) {
            isSearchPresented = true
        }
        isSearchFocused = true
    }

    private func dismissSearch(clearQuery: Bool) {
        isSearchFocused = false
        if clearQuery {
            searchQuery = ""
        }
        if searchQuery.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            withAnimation(.easeInOut(duration: 0.18)) {
                isSearchPresented = false
            }
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
    let title: String
    let onAccount: () -> Void
    let onSearch: () -> Void
    let onNewRoom: () -> Void

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            Button(action: onAccount) {
                HStack(spacing: SynaraSpacing.small) {
                    ZStack(alignment: .bottomTrailing) {
                        SynaraBrandMark(size: 38)
                        Circle()
                            .fill(SynaraColor.success)
                            .frame(width: 10, height: 10)
                            .overlay(Circle().stroke(SynaraColor.surface, lineWidth: 2))
                    }

                    HStack(spacing: SynaraSpacing.xSmall) {
                        Text(title)
                            .font(.title3.weight(.semibold))
                            .foregroundStyle(SynaraColor.primaryText)
                            .lineLimit(1)
                            .minimumScaleFactor(0.78)
                    }
                    .frame(maxWidth: 210, alignment: .leading)
                }
            }
            .buttonStyle(.plain)
            .contentShape(Rectangle())
            .accessibilityLabel("Account menu")
            .accessibilityHint("Shows account details, settings, and logout")
            .accessibilityIdentifier("RoomHeaderAccountMenuButton")

            Spacer()

            Button(action: onSearch) {
                Image(systemName: "magnifyingglass")
                    .font(.system(size: 17, weight: .semibold))
                    .frame(width: 42, height: 42)
                    .background(SynaraColor.secondaryText.opacity(0.10))
                    .foregroundStyle(SynaraColor.secondaryText)
                    .clipShape(RoundedRectangle(cornerRadius: SynaraRadius.control))
            }
            .buttonStyle(.plain)
            .contentShape(Rectangle())
            .accessibilityLabel("Search rooms")
            .accessibilityIdentifier("RoomSearchButton")

            Button(action: onNewRoom) {
                Image(systemName: "square.and.pencil")
                    .font(.system(size: 17, weight: .semibold))
                    .frame(width: 42, height: 42)
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
    let isFocused: FocusState<Bool>.Binding
    let onDismiss: () -> Void

    var body: some View {
        HStack(spacing: SynaraSpacing.small) {
            Image(systemName: "magnifyingglass")
                .foregroundStyle(SynaraColor.secondaryText)
                .accessibilityHidden(true)
            TextField("Search rooms", text: $text)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .submitLabel(.done)
                .focused(isFocused)
                .onSubmit(onDismiss)
                .accessibilityIdentifier("RoomSearchField")
            if text.isEmpty == false {
                Button {
                    text = ""
                    onDismiss()
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .foregroundStyle(SynaraColor.tertiaryText)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Clear room search")
            }
        }
        .padding(SynaraSpacing.medium)
        .frame(height: 44)
        .synaraCard(fill: SynaraColor.secondarySurface)
        .contentShape(Rectangle())
        .onTapGesture {
            isFocused.wrappedValue = true
        }
    }
}

private struct RoomListRow: View {
    let room: RoomSummary

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            RoomAvatarTile(room: room, size: 42)

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

                    if room.relativeActivity.isEmpty == false {
                        Text(room.relativeActivity)
                            .font(.caption)
                            .foregroundStyle(SynaraColor.tertiaryText)
                            .lineLimit(1)
                    }
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

private struct RoomAvatarTile: View {
    let room: RoomSummary
    let size: CGFloat

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: 11, style: .continuous)
                .fill(room.avatarGradient)
                .overlay(alignment: .topLeading) {
                    RoundedRectangle(cornerRadius: 11, style: .continuous)
                        .fill(Color.white.opacity(0.18))
                        .blendMode(.softLight)
                }

            if let systemImage = room.avatarSystemImage {
                Image(systemName: systemImage)
                    .font(.system(size: size * 0.43, weight: .bold))
                    .foregroundStyle(.white)
                    .symbolRenderingMode(.hierarchical)
            } else {
                Text(room.avatarInitials)
                    .font(.system(size: size * 0.34, weight: .bold))
                    .foregroundStyle(.white)
                    .minimumScaleFactor(0.72)
            }
        }
        .frame(width: size, height: size)
        .shadow(color: room.avatarShadow.opacity(0.22), radius: 5, x: 0, y: 2)
        .accessibilityHidden(true)
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

        let isUITest = ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1"
        let referenceDate = isUITest ? RoomListFixtures.now : Date()

        if isUITest == false, lastActivityAt > referenceDate.addingTimeInterval(60) {
            return ""
        }

        let calendar = Calendar.current
        if calendar.isDate(lastActivityAt, inSameDayAs: referenceDate) {
            let formatter = DateFormatter()
            formatter.dateStyle = .none
            formatter.timeStyle = .short
            return formatter.string(from: lastActivityAt)
        }

        if calendar.isDateInYesterday(lastActivityAt) {
            return "Yesterday"
        }

        if let days = calendar.dateComponents([.day], from: calendar.startOfDay(for: lastActivityAt), to: calendar.startOfDay(for: referenceDate)).day,
           days > 0,
           days < 7 {
            let formatter = DateFormatter()
            formatter.dateFormat = "EEE"
            return formatter.string(from: lastActivityAt)
        }

        let formatter = DateFormatter()
        formatter.dateFormat = calendar.component(.year, from: lastActivityAt) == calendar.component(.year, from: referenceDate) ? "MMM d" : "MMM d, yyyy"
        return formatter.string(from: lastActivityAt)
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

    var isFavoriteLike: Bool {
        hasHighlight
            || isAgentRoom
            || isSecureRoom
            || name.localizedCaseInsensitiveContains("product")
            || name.localizedCaseInsensitiveContains("design")
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

    var avatarSystemImage: String? {
        if kind == .directMessage {
            return "person.fill"
        }
        if isAgentRoom {
            return "sparkles"
        }
        if isSecureRoom {
            return "lock.fill"
        }
        if name.localizedCaseInsensitiveContains("alert") || name.localizedCaseInsensitiveContains("incident") {
            return "bell.badge.fill"
        }
        if name.localizedCaseInsensitiveContains("ops") || name.localizedCaseInsensitiveContains("infra") {
            return "briefcase.fill"
        }
        if name.localizedCaseInsensitiveContains("design") || name.localizedCaseInsensitiveContains("creative") {
            return "paintpalette.fill"
        }
        return nil
    }

    var avatarInitials: String {
        let cleaned = name
            .replacingOccurrences(of: "#", with: " ")
            .replacingOccurrences(of: "_", with: " ")
            .replacingOccurrences(of: "-", with: " ")
            .trimmingCharacters(in: .whitespacesAndNewlines)

        let ignoredWords: Set<String> = ["the", "and", "room", "channel", "chat"]
        let words = cleaned
            .split(separator: " ")
            .map(String.init)
            .filter { ignoredWords.contains($0.lowercased()) == false }

        if words.count >= 2 {
            return words.prefix(2).compactMap(\.first).map(String.init).joined().uppercased()
        }

        if let word = words.first, word.count > 1 {
            let letters = word.filter(\.isLetter)
            let first = letters.first.map(String.init) ?? String(word.prefix(1))
            let second = letters.dropFirst().first(where: { !"AEIOUaeiou".contains($0) }).map(String.init)
                ?? letters.dropFirst().first.map(String.init)
                ?? ""
            return "\(first)\(second)".uppercased()
        }

        return cleaned.first.map { String($0).uppercased() } ?? "S"
    }

    var avatarGradient: LinearGradient {
        let palette = avatarPalette
        return LinearGradient(colors: palette, startPoint: .topLeading, endPoint: .bottomTrailing)
    }

    var avatarShadow: Color {
        avatarPalette.last ?? SynaraColor.accent
    }

    private var avatarPalette: [Color] {
        if kind == .directMessage {
            return [Color(red: 0.35, green: 0.42, blue: 0.53), Color(red: 0.12, green: 0.16, blue: 0.24)]
        }
        if isAgentRoom {
            return [Color(red: 0.58, green: 0.32, blue: 0.94), Color(red: 0.20, green: 0.63, blue: 0.86)]
        }
        if isSecureRoom {
            return [Color(red: 0.05, green: 0.55, blue: 0.43), Color(red: 0.02, green: 0.26, blue: 0.25)]
        }
        if name.localizedCaseInsensitiveContains("alert") || name.localizedCaseInsensitiveContains("incident") {
            return [Color(red: 0.97, green: 0.42, blue: 0.22), Color(red: 0.78, green: 0.12, blue: 0.24)]
        }
        if name.localizedCaseInsensitiveContains("design") || name.localizedCaseInsensitiveContains("creative") {
            return [Color(red: 0.49, green: 0.29, blue: 0.95), Color(red: 0.95, green: 0.32, blue: 0.58)]
        }
        if name.localizedCaseInsensitiveContains("ops") || name.localizedCaseInsensitiveContains("infra") {
            return [Color(red: 0.04, green: 0.48, blue: 0.46), Color(red: 0.05, green: 0.23, blue: 0.38)]
        }

        let palettes: [[Color]] = [
            [Color(red: 0.12, green: 0.45, blue: 0.91), Color(red: 0.26, green: 0.24, blue: 0.77)],
            [Color(red: 0.04, green: 0.58, blue: 0.74), Color(red: 0.02, green: 0.31, blue: 0.58)],
            [Color(red: 0.80, green: 0.25, blue: 0.43), Color(red: 0.48, green: 0.20, blue: 0.74)],
            [Color(red: 0.12, green: 0.60, blue: 0.38), Color(red: 0.08, green: 0.35, blue: 0.42)],
            [Color(red: 0.90, green: 0.45, blue: 0.16), Color(red: 0.68, green: 0.20, blue: 0.34)]
        ]
        let seed = "\(id)|\(name)".unicodeScalars.reduce(0) { partial, scalar in
            (partial &* 31 &+ Int(scalar.value)) & 0x7fffffff
        }
        let index = seed % palettes.count
        return palettes[index]
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
