import SwiftUI

struct RoomListView: View {
    @Environment(\.appEnvironment) private var environment
    @State private var state: RoomListState = .idle
    @State private var membershipError: String?
    @State private var searchQuery: String = ProcessInfo.processInfo.environment["SYNARA_UI_TEST_ROOM_SEARCH"] ?? ""
    @State private var selectedFilter: RoomListFilter = .all
    @State private var selectedSpaceID: String?
    @State private var expandedSpaceIDs: Set<String> = []
    @State private var isRoomManagementSheetPresented = ProcessInfo.processInfo.environment["SYNARA_UI_TEST_ROOM_MANAGEMENT_SHEET"] == "1"
    @State private var hasStartedInitialLoad = false
    @State private var loadRoomsTask: Task<Void, Never>?
    @State private var roomUpdatesTask: Task<Void, Never>?
    @State private var isSearchPresented = ProcessInfo.processInfo.environment["SYNARA_UI_TEST_ROOM_SEARCH"] != nil
    @State private var roomPendingLeave: RoomSummary?
    @State private var isResettingSession = false
    @State private var sessionRecoveryError: String?
    @State private var favoriteSort: RoomListSortOrder = RoomListSortOrder.stored(for: .favorites)
    @State private var roomsSort: RoomListSortOrder = RoomListSortOrder.stored(for: .rooms)
    @State private var directsSort: RoomListSortOrder = RoomListSortOrder.stored(for: .directs)
    @FocusState private var isSearchFocused: Bool

    var body: some View {
        Group {
            switch state {
            case .idle, .loading:
                VStack(spacing: 0) {
                    VStack(spacing: SynaraSpacing.medium) {
                        RoomListHeader(
                            title: accountMenuTitle,
                            onAccount: { environment.router.present(.accountSwitcher) },
                            onSearch: { presentSearch() },
                            onNewRoom: { isRoomManagementSheetPresented = true }
                        )
                        RoomFilterStrip(
                            selectedFilter: $selectedFilter,
                            filterBadges: [:]
                        )
                    }
                    .padding(.horizontal, SynaraSpacing.large)
                    .padding(.top, SynaraSpacing.medium)
                    .padding(.bottom, SynaraSpacing.small)
                    .background(SynaraChrome.roomList)

                    List {
                        Section {
                            SynaraSkeletonList(rowCount: 9)
                                .listRowSeparator(.hidden)
                                .listRowInsets(EdgeInsets(top: 3, leading: SynaraSpacing.large, bottom: 3, trailing: SynaraSpacing.large))
                                .listRowBackground(SynaraChrome.roomList)
                        }
                    }
                    .listStyle(.plain)
                    .scrollContentBackground(.hidden)
                    .background(SynaraChrome.roomList)
                    .accessibilityIdentifier("RoomListLoading")
                }
            case .empty:
                HeldConnectionEmptyState(
                    title: "No Rooms",
                    systemImage: "bubble.left.and.bubble.right",
                    store: environment.connectionStatus
                )
            case .failed(let message):
                VStack(spacing: SynaraSpacing.medium) {
                    SynaraErrorState(title: "Could Not Load Rooms", message: message) {
                        loadRooms(startUpdatesAfterLoad: true)
                    }
                    Button("Sign In Again") {
                        signInAgain()
                    }
                    .buttonStyle(.bordered)
                    .disabled(isResettingSession)
                    .accessibilityIdentifier("RoomListSignInAgainButton")

                    if let sessionRecoveryError {
                        Text(sessionRecoveryError)
                            .font(SynaraTypography.supporting)
                            .foregroundStyle(.red)
                            .multilineTextAlignment(.center)
                            .accessibilityIdentifier("RoomListSessionRecoveryError")
                    }
                }
            case .loaded(let rooms):
                let filteredRooms = filteredRooms(from: rooms)
                let favoritesPartition = RoomListFavorites.partition(from: filteredRooms)
                let favoriteRooms = RoomListFavorites.sorted(
                    favoritesPartition.favorites,
                    order: favoriteSort
                )
                let remainingRooms = favoritesPartition.remaining
                let channelRooms = RoomListFavorites.sorted(
                    remainingRooms.filter { $0.kind == .room },
                    order: roomsSort
                )
                let directRooms = RoomListFavorites.sorted(
                    remainingRooms.filter { $0.kind == .directMessage },
                    order: directsSort
                )
                let spaces = spaces(from: rooms)
                let spaceUnreadCounts = RoomListSpaceGrouping.unreadCountsBySpaceID(from: rooms)
                let selectedSpaceTitle = RoomListSpaceGrouping.selectedSpaceName(
                    in: spaces,
                    selectedSpaceID: selectedSpaceID
                )
                let spaceChannelGroups = RoomListSpaceGrouping.spaceChannelGroups(from: channelRooms)
                let ungroupedChannelRooms = RoomListSpaceGrouping.ungroupedChannelRooms(from: channelRooms)
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
                        RoomFilterStrip(
                            selectedFilter: $selectedFilter,
                            filterBadges: filterBadges(from: rooms)
                        )
                        if spaces.isEmpty == false {
                            SpaceFilterStrip(
                                spaces: spaces,
                                unreadCountsBySpaceID: spaceUnreadCounts,
                                selectedSpaceID: $selectedSpaceID
                            )
                        }
                    }
                    .padding(.horizontal, SynaraSpacing.large)
                    .padding(.top, SynaraSpacing.medium)
                    .padding(.bottom, SynaraSpacing.small)
                    .background(SynaraChrome.roomList)

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
                                ForEach(favoriteRooms, id: \.id) { room in
                                    roomRow(room)
                                }
                            } header: {
                                RoomSectionHeader(
                                    title: "Favorites",
                                    count: favoriteRooms.count,
                                    selectedSort: $favoriteSort
                                )
                            }
                        }

                        if let selectedSpaceTitle {
                            if channelRooms.isEmpty == false {
                                Section {
                                    ForEach(channelRooms) { room in
                                        roomRow(room)
                                    }
                                } header: {
                                    SpaceSelectedHeader(
                                        title: selectedSpaceTitle,
                                        roomCount: channelRooms.count
                                    )
                                }
                            }
                        } else if channelRooms.isEmpty == false {
                            ForEach(spaceChannelGroups) { group in
                                Section {
                                    DisclosureGroup(
                                        isExpanded: spaceExpansionBinding(for: group.id)
                                    ) {
                                        ForEach(group.rooms) { room in
                                            roomRow(room)
                                        }
                                    } label: {
                                        SpaceDisclosureLabel(
                                            title: group.space.name,
                                            roomCount: group.rooms.count,
                                            unreadCount: spaceUnreadCounts[group.space.id] ?? 0
                                        )
                                    }
                                }
                            }

                            if ungroupedChannelRooms.isEmpty == false {
                                Section {
                                    ForEach(ungroupedChannelRooms) { room in
                                        roomRow(room)
                                    }
                                } header: {
                                    RoomSectionHeader(
                                        title: "Channels",
                                        count: ungroupedChannelRooms.count,
                                        selectedSort: $roomsSort
                                    )
                                }
                            }
                        }

                        if directRooms.isEmpty == false {
                            Section {
                                ForEach(directRooms) { room in
                                    roomRow(room)
                                }
                            } header: {
                                RoomSectionHeader(
                                    title: "Direct messages",
                                    count: directRooms.count,
                                    selectedSort: $directsSort
                                )
                            }
                        }
                    }
                    .listStyle(.plain)
                    .scrollContentBackground(.hidden)
                    .background(SynaraChrome.roomList)
                    .refreshable {
                        await reloadRoomsForRefresh()
                    }
                    .accessibilityIdentifier("RoomList")
                    .onAppear {
                        seedExpandedSpacesIfNeeded(
                            groups: spaceChannelGroups,
                            unreadCounts: spaceUnreadCounts
                        )
                    }
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
        .confirmationDialog(
            "Leave this room?",
            isPresented: Binding(
                get: { roomPendingLeave != nil },
                set: { isPresented in
                    if isPresented == false {
                        roomPendingLeave = nil
                    }
                }
            ),
            titleVisibility: .visible
        ) {
            if let roomPendingLeave {
                Button("Leave Room", role: .destructive) {
                    leaveRoom(roomPendingLeave)
                }
                .accessibilityIdentifier("LeaveRoomConfirm-\(roomPendingLeave.id)")
            }
            Button("Cancel", role: .cancel) {
                roomPendingLeave = nil
            }
        } message: {
            if let roomPendingLeave {
                Text("\(roomPendingLeave.name) will be removed from your joined room list.")
            }
        }
        .task {
            guard hasStartedInitialLoad == false else {
                return
            }
            guard case .signedIn(let session) = environment.session.currentState else {
                return
            }
            hasStartedInitialLoad = true
            await environment.sessionReadiness.waitUntilPrepared(for: session)
            startRoomUpdates()
        }
        .onAppear {
            if hasStartedInitialLoad {
                startRoomUpdatesIfReady(for: state)
            }
        }
        .onDisappear {
            loadRoomsTask?.cancel()
            roomUpdatesTask?.cancel()
        }
        .onChange(of: favoriteSort) { sort in
            RoomListSortOrder.persist(sort, for: .favorites)
        }
        .onChange(of: roomsSort) { sort in
            RoomListSortOrder.persist(sort, for: .rooms)
        }
        .onChange(of: directsSort) { sort in
            RoomListSortOrder.persist(sort, for: .directs)
        }
    }

    private func reloadRoomsForRefresh() async {
        loadRoomsTask?.cancel()
        let signpostID = PerformanceTrace.begin("RoomListLoad")
        defer {
            PerformanceTrace.end("RoomListLoad", id: signpostID)
        }
        let loadedState = await environment.roomList.loadRooms()
        guard Task.isCancelled == false else {
            return
        }
        await MainActor.run {
            state = loadedState
            autoOpenRoomIfRequested(from: loadedState)
            startRoomUpdatesIfReady(for: loadedState)
        }
    }

    private func loadRooms(showLoading: Bool = true, startUpdatesAfterLoad: Bool = false) {
        if showLoading {
            state = .loading
        } else if case .idle = state {
            state = .loading
        }
        loadRoomsTask?.cancel()
        loadRoomsTask = Task {
            let signpostID = PerformanceTrace.begin("RoomListLoad")
            defer {
                PerformanceTrace.end("RoomListLoad", id: signpostID)
            }
            let loadedState = await environment.roomList.loadRooms()
            guard Task.isCancelled == false else {
                return
            }
            await MainActor.run {
                state = loadedState
                autoOpenRoomIfRequested(from: loadedState)
                if startUpdatesAfterLoad {
                    startRoomUpdatesIfReady(for: loadedState)
                }
            }
        }
    }

    private func signInAgain() {
        guard isResettingSession == false else {
            return
        }

        isResettingSession = true
        sessionRecoveryError = nil
        Task {
            do {
                try await environment.wipe.logoutAndWipe()
            } catch {
                await MainActor.run {
                    isResettingSession = false
                    sessionRecoveryError = LocalWipeError.sessionDeleteFailed.localizedDescription
                }
            }
        }
    }

    private func startRoomUpdatesIfReady(for state: RoomListState) {
        switch state {
        case .loaded, .empty:
            startRoomUpdates()
        case .idle, .loading, .failed:
            break
        }
    }

    private func startRoomUpdates() {
        roomUpdatesTask?.cancel()
        roomUpdatesTask = Task {
            for await updatedState in environment.roomList.roomUpdates() {
                guard Task.isCancelled == false else {
                    return
                }
                await MainActor.run {
                    state = updatedState
                    autoOpenRoomIfRequested(from: updatedState)
                }
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
                    if accept {
                        SynaraHaptics.trigger(.success)
                    }
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
        RoomListScopeFilter.filteredRooms(
            from: rooms,
            filter: selectedFilter.scopeFilter,
            selectedSpaceID: selectedSpaceID,
            searchQuery: searchQuery
        )
    }

    private func spaces(from rooms: [RoomSummary]) -> [SpaceSummary] {
        Array(Set(rooms.flatMap(\.parentSpaces))).sorted {
            $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending
        }
    }

    private func spaceExpansionBinding(for spaceID: String) -> Binding<Bool> {
        Binding(
            get: { expandedSpaceIDs.contains(spaceID) },
            set: { isExpanded in
                if isExpanded {
                    expandedSpaceIDs.insert(spaceID)
                } else {
                    expandedSpaceIDs.remove(spaceID)
                }
            }
        )
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
            .listRowBackground(SynaraChrome.roomList)
        } else {
            Button {
                dismissSearch(clearQuery: false)
                environment.router.route(to: .room(id: room.id, title: room.name))
            } label: {
                RoomListRow(room: room)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .contentShape(Rectangle())
                    .accessibilityIdentifier("RoomRow-\(room.id)")
                    .padding(.vertical, SynaraSpacing.xSmall)
            }
            .buttonStyle(SynaraListRowButtonStyle())
            .frame(maxWidth: .infinity, alignment: .leading)
            .contentShape(Rectangle())
            .accessibilityLabel(room.accessibilitySummary)
            .accessibilityHint("Opens the room timeline")
            .accessibilityIdentifier("RoomRow-\(room.id)")
            .listRowSeparator(.hidden)
            .listRowInsets(EdgeInsets(top: 3, leading: SynaraSpacing.large, bottom: 3, trailing: SynaraSpacing.large))
            .listRowBackground(SynaraChrome.roomList)
            .swipeActions(edge: .leading, allowsFullSwipe: true) {
                Button {
                    toggleFavorite(room)
                } label: {
                    Label(
                        room.isFavorite ? "Unfavorite" : "Favorite",
                        systemImage: room.isFavorite ? "star.slash.fill" : "star.fill"
                    )
                }
                .tint(SynaraColor.secondaryText)
                .accessibilityIdentifier("FavoriteRoom-\(room.id)")

                if room.unreadCount > 0 {
                    Button {
                        markRoomAsRead(room)
                    } label: {
                        Label("Read", systemImage: "envelope.open")
                    }
                    .tint(SynaraColor.accent)
                    .accessibilityIdentifier("MarkRead-\(room.id)")
                }
            }
            .swipeActions(edge: .trailing, allowsFullSwipe: false) {
                Button {
                    muteRoom(room)
                } label: {
                    Label("Mute", systemImage: "bell.slash")
                }
                .tint(SynaraColor.secondaryText)
                .accessibilityIdentifier("MuteRoom-\(room.id)")

                Button(role: .destructive) {
                    roomPendingLeave = room
                } label: {
                    Label("Leave", systemImage: "rectangle.portrait.and.arrow.right")
                }
                .accessibilityIdentifier("LeaveRoom-\(room.id)")
            }
            .contextMenu {
                Button {
                    toggleFavorite(room)
                } label: {
                    Label(
                        room.isFavorite ? "Remove from Favorites" : "Add to Favorites",
                        systemImage: room.isFavorite ? "star.slash" : "star"
                    )
                }
            }
        }
    }

    private func markRoomAsRead(_ room: RoomSummary) {
        Task {
            _ = await environment.readMarkers.markRoomAsRead(roomID: room.id)
            await MainActor.run {
                loadRooms(showLoading: false)
            }
        }
    }

    private func toggleFavorite(_ room: RoomSummary) {
        Task {
            do {
                try await environment.roomManagement.setRoomFavorite(!room.isFavorite, roomID: room.id)
                await MainActor.run {
                    loadRooms(showLoading: false)
                }
            } catch {
                await MainActor.run {
                    membershipError = RoomManagementError.failed.localizedDescription
                }
            }
        }
    }

    private func muteRoom(_ room: RoomSummary) {
        Task {
            do {
                try await environment.roomManagement.setNotificationMode(.mute, roomID: room.id)
            } catch {
                await MainActor.run {
                    membershipError = RoomManagementError.failed.localizedDescription
                }
            }
        }
    }

    private func leaveRoom(_ room: RoomSummary) {
        membershipError = nil
        roomPendingLeave = nil

        Task {
            do {
                try await environment.roomManagement.leaveRoom(roomID: room.id)
                await MainActor.run {
                    loadRooms()
                }
            } catch {
                await MainActor.run {
                    membershipError = RoomManagementError.failed.localizedDescription
                }
            }
        }
    }

    private func presentSearch() {
        withAnimation(.easeInOut(duration: 0.18)) {
            isSearchPresented = true
        }
        isSearchFocused = true
    }

    private func filterBadges(from rooms: [RoomSummary]) -> [RoomListFilter: Int] {
        let pendingApprovals = AgentPendingInbox.pendingApprovals(from: rooms).count
        let agentRooms = rooms.filter(\.isAgentRoom).count
        return [
            .unread: rooms.filter { $0.unreadCount > 0 }.count,
            .mentions: rooms.filter(\.hasHighlight).count,
            .agents: pendingApprovals > 0 ? pendingApprovals : (agentRooms > 0 ? agentRooms : 0)
        ]
    }

    private func seedExpandedSpacesIfNeeded(
        groups: [SpaceChannelGroup],
        unreadCounts: [String: Int]
    ) {
        guard expandedSpaceIDs.isEmpty else {
            return
        }

        let unreadSpaceIDs = groups
            .filter { unreadCounts[$0.space.id, default: 0] > 0 }
            .map(\.id)
        guard unreadSpaceIDs.isEmpty == false else {
            return
        }
        expandedSpaceIDs = Set(unreadSpaceIDs)
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
    case agents = "Agents"

    var id: String { rawValue }

    var scopeFilter: RoomListScopeFilter.Kind {
        switch self {
        case .all:
            return .all
        case .unread:
            return .unread
        case .mentions:
            return .mentions
        case .agents:
            return .agents
        }
    }
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
                    SynaraBrandMark(size: 38)

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
                                .font(SynaraTypography.emphasis)
                            if let topic = result.topic, topic.isEmpty == false {
                                Text(topic)
                                    .font(SynaraTypography.messageMeta)
                                    .foregroundStyle(SynaraColor.secondaryText)
                                    .lineLimit(2)
                            }
                            Text("\(result.memberCount) members")
                                .font(SynaraTypography.fineMeta)
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
    let filterBadges: [RoomListFilter: Int]
    @State private var hapticTrigger = 0

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: SynaraSpacing.small) {
                ForEach(RoomListFilter.allCases) { filter in
                    SynaraFilterChip(
                        title: filter.rawValue,
                        badgeCount: filterBadges[filter],
                        isSelected: filter == selectedFilter
                    ) {
                        guard filter != selectedFilter else {
                            return
                        }
                        selectedFilter = filter
                        hapticTrigger += 1
                    }
                    .accessibilityIdentifier("RoomFilter-\(filter.rawValue)")
                }
            }
            .padding(.trailing, SynaraSpacing.large)
        }
        .accessibilityIdentifier("RoomFilterStrip")
        .synaraHapticFeedback(.selection, trigger: hapticTrigger)
    }
}

private struct SpaceFilterStrip: View {
    let spaces: [SpaceSummary]
    let unreadCountsBySpaceID: [String: Int]
    @Binding var selectedSpaceID: String?

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: SynaraSpacing.small) {
                SynaraFilterChip(title: "All spaces", isSelected: selectedSpaceID == nil) {
                    selectedSpaceID = nil
                }
                .accessibilityIdentifier("SpaceFilter-all")

                ForEach(spaces) { space in
                    SynaraFilterChip(
                        title: space.name,
                        badgeCount: unreadCountsBySpaceID[space.id],
                        isSelected: selectedSpaceID == space.id
                    ) {
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
    var selectedSort: Binding<RoomListSortOrder>?

    var body: some View {
        HStack {
            Text(title)
                .font(SynaraTypography.emphasis)
                .textCase(nil)
                .foregroundStyle(SynaraColor.primaryText)
            Spacer()
            if let selectedSort {
                RoomListSortIcons(sectionTitle: title, selectedSort: selectedSort)
            }
            Text("\(count)")
                .font(SynaraTypography.chipLabel)
                .foregroundStyle(SynaraColor.secondaryText)
        }
    }
}

private struct RoomListSortIcons: View {
    let sectionTitle: String
    @Binding var selectedSort: RoomListSortOrder

    var body: some View {
        HStack(spacing: SynaraSpacing.xSmall) {
            ForEach(RoomListSortOrder.allCases) { sort in
                Button {
                    selectedSort = sort
                } label: {
                    Image(systemName: sort.systemImage)
                        .font(.system(size: 11, weight: selectedSort == sort ? .semibold : .regular))
                        .foregroundStyle(
                            selectedSort == sort ? SynaraColor.primaryText : SynaraColor.secondaryText
                        )
                        .frame(width: 16, height: 16)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(sort.title(for: sectionTitle))
                .accessibilityAddTraits(selectedSort == sort ? .isSelected : [])
            }
        }
        .accessibilityIdentifier("RoomListSortMenu")
    }
}

private struct SpaceSelectedHeader: View {
    let title: String
    let roomCount: Int

    var body: some View {
        VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
            Text(title)
                .font(SynaraTypography.screenTitle.weight(.bold))
                .foregroundStyle(SynaraColor.primaryText)
            Text("\(roomCount) channel\(roomCount == 1 ? "" : "s")")
                .font(SynaraTypography.messageMeta.weight(.medium))
                .foregroundStyle(SynaraColor.secondaryText)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .textCase(nil)
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("SpaceSelectedHeader-\(title)")
    }
}

private struct SpaceDisclosureLabel: View {
    let title: String
    let roomCount: Int
    let unreadCount: Int

    var body: some View {
        HStack(spacing: SynaraSpacing.small) {
            Text(title)
                .font(SynaraTypography.emphasis)
                .foregroundStyle(SynaraColor.primaryText)

            Spacer(minLength: SynaraSpacing.small)

            if unreadCount > 0 {
                SynaraUnreadBadge(count: unreadCount, highlighted: false)
            }

            Text("\(roomCount)")
                .font(SynaraTypography.chipLabel)
                .foregroundStyle(SynaraColor.secondaryText)
        }
        .accessibilityElement(children: .combine)
        .accessibilityIdentifier("SpaceDisclosureLabel-\(title)")
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

    private var hasUnreadActivity: Bool {
        room.unreadCount > 0 || room.hasHighlight
    }

    private var roomGlyphColor: Color {
        guard room.isSecureRoom == false else {
            return SynaraColor.secure
        }
        return hasUnreadActivity ? SynaraColor.primaryText : SynaraColor.secondaryText
    }

    private var previewColor: Color {
        hasUnreadActivity ? SynaraColor.primaryText.opacity(0.82) : SynaraColor.secondaryText
    }

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            SynaraRoomAvatarTile(room: room, size: 32)

            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.small) {
                    if room.kind == .room {
                        Image(systemName: room.isSecureRoom ? "lock.fill" : "number")
                            .font(SynaraTypography.chipLabel.weight(.bold))
                            .foregroundStyle(roomGlyphColor)
                            .accessibilityHidden(true)
                    }

                    Text(room.name)
                        .font(SynaraTypography.body.weight(hasUnreadActivity ? .semibold : .regular))
                        .foregroundStyle(SynaraColor.primaryText)
                        .lineLimit(1)

                    if room.isFavorite {
                        Image(systemName: "star.fill")
                            .font(.system(size: 11, weight: .semibold))
                            .foregroundStyle(SynaraColor.secondaryText)
                            .accessibilityLabel("Favorite")
                    }

                    if room.hasHighlight {
                        SynaraStatusChip(title: "Mention", tint: SynaraColor.accent, systemImage: "at")
                    }

                    if room.requiresAgentApproval {
                        SynaraStatusChip(title: "Approval", tint: SynaraColor.agent, systemImage: "hand.raised")
                            .accessibilityIdentifier("RoomRowApprovalChip-\(room.id)")
                    } else if room.isAgentRoom {
                        SynaraStatusChip(title: "Agent", tint: SynaraColor.agent, systemImage: "sparkles")
                    }

                    Spacer(minLength: SynaraSpacing.small)

                    if room.relativeActivity.isEmpty == false {
                        Text(room.relativeActivity)
                            .font(SynaraTypography.messageMeta.weight(hasUnreadActivity ? .medium : .regular))
                            .foregroundStyle(hasUnreadActivity ? SynaraColor.secondaryText : SynaraColor.tertiaryText)
                            .lineLimit(1)
                    }
                }

                Text(room.lastMessagePreview)
                    .font(SynaraTypography.roomPreview.weight(hasUnreadActivity ? .medium : .regular))
                    .foregroundStyle(previewColor)
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
                .font(SynaraTypography.messageMeta)
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
        if isFavorite {
            parts.append("favorite")
        }
        return parts.joined(separator: ", ")
    }

    var relativeActivity: String {
        guard lastActivityAt.timeIntervalSince1970 > 0 else {
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
            if SynaraSharedConstants.boolSetting(SynaraSharedConstants.hour24ClockKey) {
                formatter.dateFormat = "HH:mm"
            } else {
                formatter.timeStyle = .short
            }
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
