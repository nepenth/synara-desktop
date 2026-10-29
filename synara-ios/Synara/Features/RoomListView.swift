import SwiftUI

struct RoomListView: View {
    @Environment(\.appEnvironment) private var environment
    @Environment(\.synaraCanvasLayout) private var canvasLayout
    @Environment(\.synaraSelectedConversationID) private var selectedConversationID
    @State private var state: RoomListState = .idle
    @State private var membershipError: String?
    @State private var searchQuery: String = ProcessInfo.processInfo.environment["SYNARA_UI_TEST_ROOM_SEARCH"] ?? ""
    @State private var selectedFilter: RoomListFilter = .all
    @State private var selectedSpaceID: String?
    @State private var expandedSpaceIDs: Set<String> = []
    @State private var isRoomManagementSheetPresented = ProcessInfo.processInfo.environment["SYNARA_UI_TEST_ROOM_MANAGEMENT_SHEET"] == "1"
    @State private var loadRoomsTask: Task<Void, Never>?
    @State private var liveUpdates = RoomListLiveUpdates()
    @State private var isSearchPresented = ProcessInfo.processInfo.environment["SYNARA_UI_TEST_ROOM_SEARCH"] != nil
    @State private var roomPendingLeave: RoomSummary?
    @State private var isResettingSession = false
    @State private var sessionRecoveryError: String?
    @State private var favoriteSort: RoomListSortOrder = RoomListSortOrder.stored(for: .favorites)
    @State private var roomsSort: RoomListSortOrder = RoomListSortOrder.stored(for: .rooms)
    @State private var directsSort: RoomListSortOrder = RoomListSortOrder.stored(for: .directs)
    /// Direct-message peers whose MSC4426 status says they are in a call.
    @State private var peersInCall: Set<String> = []
    @State private var peerCallStatusGeneration: UInt64 = 0
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
                    .padding(.bottom, SynaraSpacing.xSmall)
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
                    .modifier(RoomListTopMargin())
                    .scrollContentBackground(.hidden)
                    .background(SynaraChrome.roomList)
                    .accessibilityIdentifier("RoomListLoading")
                }
            case .empty:
                HeldConnectionEmptyState(
                    title: String(localized: "No Rooms"),
                    systemImage: "bubble.left.and.bubble.right",
                    store: environment.connectionStatus
                )
            case .failed(let message):
                VStack(spacing: SynaraSpacing.medium) {
                    SynaraErrorState(title: String(localized: "Could Not Load Rooms"), message: message) {
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
                            .foregroundStyle(SynaraColor.criticalText)
                            .multilineTextAlignment(.center)
                            .accessibilityIdentifier("RoomListSessionRecoveryError")
                    }
                }
            case .loaded(let rooms):
                let filteredRooms = filteredRooms(from: rooms)
                let sections = RoomListFavorites.sections(from: filteredRooms)
                let favoriteRooms = RoomListFavorites.sorted(
                    sections.favorites,
                    order: favoriteSort
                )
                let channelRooms = RoomListFavorites.sorted(
                    sections.channels,
                    order: roomsSort
                )
                let directRooms = RoomListFavorites.sorted(
                    sections.directs,
                    order: directsSort
                )
                let lowPriorityRooms = RoomListFavorites.sorted(
                    sections.lowPriority,
                    order: roomsSort
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
                    .padding(.bottom, SynaraSpacing.xSmall)
                    .background(SynaraChrome.roomList)

                    List {
                        if searchQuery.isEmpty == false && filteredRooms.isEmpty {
                            SynaraEmptyState(
                                title: String(localized: "No Matching Rooms"),
                                systemImage: "magnifyingglass",
                                message: String(localized: "Try another room name or message preview.")
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
                                    title: String(localized: "Favorites"),
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
                                    .listRowSeparator(.hidden)
                                    .listRowBackground(SynaraChrome.roomList)
                                }
                            }

                            if ungroupedChannelRooms.isEmpty == false {
                                Section {
                                    ForEach(ungroupedChannelRooms) { room in
                                        roomRow(room)
                                    }
                                } header: {
                                    RoomSectionHeader(
                                        title: String(localized: "Channels"),
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
                                    title: String(localized: "Direct messages"),
                                    count: directRooms.count,
                                    selectedSort: $directsSort
                                )
                            }
                        }

                        if lowPriorityRooms.isEmpty == false {
                            Section {
                                ForEach(lowPriorityRooms) { room in
                                    roomRow(room)
                                }
                            } header: {
                                RoomSectionHeader(
                                    title: String(localized: "Low priority"),
                                    count: lowPriorityRooms.count,
                                    selectedSort: $roomsSort
                                )
                            }
                        }
                    }
                    .listStyle(.plain)
                    .modifier(RoomListTopMargin())
                    .scrollContentBackground(.hidden)
                    .background(SynaraChrome.roomList)
                    .refreshable {
                        await reloadRoomsForRefresh()
                    }
                    .safeAreaInset(edge: .bottom, spacing: 0) {
                        Color.clear
                            .frame(height: SynaraTabRootContentReachability.minimumSeparation)
                            .accessibilityHidden(true)
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
                    .foregroundStyle(SynaraColor.criticalText)
                    .padding(SynaraSpacing.medium)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(SynaraColor.secondarySurface)
                    .accessibilityIdentifier("RoomMembershipErrorText")
            }
        }
        .background {
            // The floating tab bar and hidden navigation bar sit above the
            // content plane. Paint that plane through both safe areas so their
            // translucent edges never sample the window's unrelated default.
            SynaraChrome.roomList
                .ignoresSafeArea()
        }
        .navigationTitle("Rooms")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar(.hidden, for: .navigationBar)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                SynaraToolbarIconButton(systemImage: "person.crop.circle", accessibilityLabel: String(localized: "Accounts")) {
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
        .task(id: peerCallStatusKey) {
            await refreshPeerCallStatus(peerIDs: peerCallStatusKey.peerIDs)
        }
        // `.task` runs on every appearance and is cancelled on disappearance.
        // Opening a room (for example from a notification) can hide the list
        // before its first snapshot arrives, so each appearance starts the
        // updates again whatever `state` holds. The stream yields a cached or
        // freshly loaded snapshot first; it never waits for a change signal.
        .task {
            guard case .signedIn(let session) = environment.session.currentState else {
                return
            }
            guard await environment.sessionReadiness.waitUntilPrepared(for: session),
                  Task.isCancelled == false else {
                return
            }
            startRoomUpdates()
        }
        .onDisappear {
            loadRoomsTask?.cancel()
            liveUpdates.stop()
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

    private var peerCallStatusKey: RoomListPeerCallStatus.Key {
        guard case .loaded(let rooms) = state else {
            return RoomListPeerCallStatus.Key(peerIDs: [], generation: peerCallStatusGeneration)
        }
        return RoomListPeerCallStatus.Key(
            peerIDs: RoomListPeerCallStatus.peerIDs(in: rooms),
            generation: peerCallStatusGeneration
        )
    }

    /// One pass over the direct-message peers when that set changes or the
    /// list is pulled to refresh, instead of a status read per row appearance.
    private func refreshPeerCallStatus(peerIDs: [String]) async {
        let peers = Set(peerIDs)
        peersInCall.formIntersection(peers)
        for peerID in peerIDs {
            let snapshot = await environment.matrix.userStatus(userID: peerID)
            guard Task.isCancelled == false else {
                return
            }
            if snapshot?.inCall != nil {
                peersInCall.insert(peerID)
            } else {
                peersInCall.remove(peerID)
            }
        }
    }

    private func reloadRoomsForRefresh() async {
        loadRoomsTask?.cancel()
        peerCallStatusGeneration &+= 1
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
                    sessionRecoveryError = LocalWipeError.displayMessage(for: error)
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
        liveUpdates.start(service: environment.roomList) { updatedState in
            state = updatedState
            autoOpenRoomIfRequested(from: updatedState)
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
                RoomListRow(room: room, peerInCall: RoomListPeerCallStatus.isInCall(room, peersInCall: peersInCall))
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, SynaraSpacing.medium)
                    .padding(.vertical, SynaraSpacing.small)
                    .background {
                        // Rows sit on the list plane; the open conversation
                        // is marked by a quiet accent fill, as on desktop.
                        if isSelectedConversation(room) {
                            RoundedRectangle(cornerRadius: SynaraRadius.card, style: .continuous)
                                .fill(SynaraColor.accent.opacity(0.14))
                                .synaraDepth(
                                    .layered,
                                    cornerRadius: SynaraRadius.card,
                                    boundaryColor: SynaraColor.accent
                                )
                        }
                    }
                    .contentShape(Rectangle())
            }
            .buttonStyle(SynaraListRowButtonStyle())
            .frame(maxWidth: .infinity, alignment: .leading)
            .contentShape(Rectangle())
            .accessibilityLabel(room.accessibilitySummary)
            .accessibilityHint("Opens the room timeline")
            .accessibilityAddTraits(isSelectedConversation(room) ? .isSelected : [])
            .accessibilityIdentifier("RoomRow-\(room.id)")
            .listRowSeparator(.hidden)
            .listRowInsets(EdgeInsets(top: 1, leading: SynaraSpacing.small, bottom: 1, trailing: SynaraSpacing.small))
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

                if room.hasUnreadActivity {
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

    private func isSelectedConversation(_ room: RoomSummary) -> Bool {
        canvasLayout == .split && selectedConversationID == room.id
    }

    private func markRoomAsRead(_ room: RoomSummary) {
        membershipError = nil
        Task {
            let acknowledged = await environment.readMarkers.markRoomAsRead(roomID: room.id)
            // A nil readback means no fully-read marker reached the server. Keep
            // the row unread and say so instead of treating the tap as finished.
            if let failure = ExplicitRoomReadReceipt.failureMessage(acknowledgedEventID: acknowledged) {
                await MainActor.run {
                    membershipError = failure
                }
                return
            }
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
        // Spaces never carry attention of their own (Core's rule too).
        let rooms = rooms.filter { $0.kind != .space }
        let pendingApprovals = AgentPendingInbox.pendingApprovals(from: rooms).count
        let agentRooms = rooms.filter(\.isAgentRoom).count
        return [
            .unread: rooms.filter(\.hasUnreadActivity).count,
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

    var title: String {
        switch self {
        case .all:
            return String(localized: "All")
        case .unread:
            return String(localized: "Unread")
        case .mentions:
            return String(localized: "Mentions")
        case .agents:
            return String(localized: "Agents")
        }
    }

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
                            .foregroundStyle(SynaraColor.headingText)
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
                    .frame(width: SynaraHitTarget.minimum, height: SynaraHitTarget.minimum)
                    .foregroundStyle(SynaraColor.accent)
            }
            .buttonStyle(SynaraTactileButtonStyle())
            .contentShape(Rectangle())
            .accessibilityLabel("Search rooms")
            .accessibilityIdentifier("RoomSearchButton")

            Button(action: onNewRoom) {
                Image(systemName: "square.and.pencil")
                    .font(.system(size: 17, weight: .semibold))
                    .frame(width: SynaraHitTarget.minimum, height: SynaraHitTarget.minimum)
                    .foregroundStyle(SynaraColor.accent)
            }
            .buttonStyle(SynaraTactileButtonStyle())
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

        var title: String {
            switch self {
            case .room:
                return String(localized: "Room")
            case .dm:
                return String(localized: "DM")
            case .join:
                return String(localized: "Join")
            }
        }
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
                            Text(mode.title).tag(mode)
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
                            .foregroundStyle(SynaraColor.criticalText)
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
                    Text(visibilityTitle(visibility)).tag(visibility)
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
            return String(localized: "Create")
        case .dm:
            return String(localized: "Start")
        case .join:
            return String(localized: "Join")
        }
    }

    private func visibilityTitle(_ visibility: SynaraRoomVisibility) -> String {
        switch visibility {
        case .private:
            return String(localized: "Private")
        case .public:
            return String(localized: "Public")
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
                        title: filter.title,
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
                SynaraFilterChip(title: String(localized: "All spaces"), isSelected: selectedSpaceID == nil) {
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
                .foregroundStyle(SynaraColor.headingText)
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
        Menu {
            ForEach(RoomListSortOrder.allCases) { sort in
                Button {
                    selectedSort = sort
                } label: {
                    if selectedSort == sort {
                        Label(sort.shortTitle, systemImage: "checkmark")
                    } else {
                        Text(sort.shortTitle)
                    }
                }
                .accessibilityLabel(sort.title(for: sectionTitle))
            }
        } label: {
            Image(systemName: selectedSort.systemImage)
                .font(.system(size: 15, weight: .semibold))
                .foregroundStyle(SynaraColor.secondaryText)
                .frame(width: 44, height: 44)
                .contentShape(Rectangle())
        }
        .accessibilityLabel("Sort \(sectionTitle). Current: \(selectedSort.shortTitle)")
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
                .foregroundStyle(SynaraColor.headingText)
            Text("\(roomCount) channels")
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
        .background(
            SynaraColor.mutedControl.opacity(0.6),
            in: RoundedRectangle(cornerRadius: SynaraRadius.control, style: .continuous)
        )
        .contentShape(Rectangle())
        .onTapGesture {
            isFocused.wrappedValue = true
        }
    }
}

private struct RoomListRow: View {
    let room: RoomSummary
    /// The direct-message peer is in a call (batched by `RoomListView`).
    var peerInCall = false
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize

    private var accessibilityChildren: AccessibilityChildBehavior {
        // Production VoiceOver gets one concise, combined row. UI automation
        // exposes descendants only so it can assert the rendered title's real
        // frame instead of passing on the synthesized row label alone.
        SynaraLaunchEnvironment.isUITest ? .contain : .ignore
    }

    private var hasUnreadActivity: Bool {
        room.hasUnreadActivity
    }

    private var roomGlyphColor: Color {
        guard room.isSecureRoom == false else {
            return SynaraColor.secure
        }
        return hasUnreadActivity ? SynaraColor.primaryText : SynaraColor.secondaryText
    }

    private var previewColor: Color {
        SynaraColor.secondaryText
    }

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            switch SynaraRoomGlyph.glyph(for: room) {
            case .channel:
                Text("#")
                    .font(SynaraTypography.body.weight(.semibold))
                    .foregroundStyle(roomGlyphColor)
                    .frame(width: 20, alignment: .center)
                    .accessibilityHidden(true)
            case let .person(initial):
                // Same 20 pt column as "#", so names stay aligned.
                SynaraPersonGlyph(initial: initial, avatarURL: room.avatarURL, size: 20, seed: room.directUserId ?? room.id)
            }

            VStack(alignment: .leading, spacing: SynaraSpacing.xSmall) {
                HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.small) {
                    Text(room.name)
                        .font(SynaraTypography.body.weight(hasUnreadActivity ? .semibold : .regular))
                        .foregroundStyle(SynaraColor.primaryText)
                        .lineLimit(SynaraDynamicTypeLayout.titleLineLimit(dynamicTypeSize))
                        .layoutPriority(1)
                        .accessibilityIdentifier("RoomRowTitle-\(room.id)")

                    if room.isSecureRoom {
                        Image(systemName: "lock.fill")
                            .font(SynaraTypography.chipLabel)
                            .foregroundStyle(SynaraColor.secondaryText)
                            .accessibilityLabel("Encrypted room")
                    }

                    if room.isFavorite {
                        Image(systemName: "star.fill")
                            .font(.system(size: 11, weight: .semibold))
                            .foregroundStyle(SynaraColor.secondaryText)
                            .accessibilityLabel("Favorite")
                    }

                    if room.hasActiveCall {
                        Text(room.liveCallChipLabel)
                            .font(SynaraTypography.chipLabel)
                            .foregroundStyle(SynaraColor.accent)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(SynaraColor.accent.opacity(0.12), in: Capsule())
                            .accessibilityLabel(room.liveCallChipLabel)
                            .accessibilityIdentifier("RoomRowLiveCall-\(room.id)")
                    } else if peerInCall {
                        Text("In a call")
                            .font(SynaraTypography.chipLabel)
                            .foregroundStyle(SynaraColor.accent)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(SynaraColor.accent.opacity(0.12), in: Capsule())
                            .accessibilityLabel("In a call")
                            .accessibilityIdentifier("RoomRowPeerInCall-\(room.id)")
                    }

                    Spacer(minLength: 0)
                }

                // At accessibility sizes the metadata takes its own line so the
                // preview keeps the row's width.
                let stacksMetadata = SynaraDynamicTypeLayout.stacksRoomRowMetadata(dynamicTypeSize)
                let previewLayout = stacksMetadata
                    ? AnyLayout(VStackLayout(alignment: .leading, spacing: SynaraSpacing.xSmall))
                    : AnyLayout(HStackLayout(alignment: .firstTextBaseline, spacing: SynaraSpacing.xSmall))
                previewLayout {
                    Text(room.lastMessagePreview)
                        .font(SynaraTypography.roomPreview.weight(hasUnreadActivity ? .medium : .regular))
                        .foregroundStyle(previewColor)
                        .lineLimit(SynaraDynamicTypeLayout.previewLineLimit(dynamicTypeSize))
                        .layoutPriority(1)

                    if stacksMetadata == false {
                        Spacer(minLength: SynaraSpacing.xSmall)
                    }

                    HStack(spacing: SynaraSpacing.xSmall) {
                        if room.hasHighlight {
                            RoomRowCompactStatusIcon(
                                systemImage: "at",
                                accessibilityLabel: String(localized: "Mention"),
                                tint: SynaraColor.accent
                            )
                            .accessibilityIdentifier("RoomRowMentionIndicator-\(room.id)")
                        }

                        if room.requiresAgentApproval {
                            RoomRowCompactStatusIcon(
                                systemImage: "hand.raised.fill",
                                accessibilityLabel: String(localized: "Approval required"),
                                tint: SynaraColor.agent
                            )
                            .accessibilityIdentifier("RoomRowApprovalChip-\(room.id)")
                        } else if room.isAgentRoom {
                            RoomRowCompactStatusIcon(
                                systemImage: "terminal",
                                accessibilityLabel: String(localized: "Agent room"),
                                tint: SynaraColor.agent
                            )
                        }

                        if room.relativeActivity.isEmpty == false {
                            Text(room.relativeActivity)
                                .font(SynaraTypography.messageMeta.weight(hasUnreadActivity ? .medium : .regular))
                                .foregroundStyle(hasUnreadActivity ? SynaraColor.secondaryText : SynaraColor.tertiaryText)
                                .lineLimit(1)
                        }

                        SynaraUnreadBadge(count: room.unreadBadgeCount, highlighted: room.hasHighlight)
                    }
                    .fixedSize(horizontal: true, vertical: false)
                }
            }
        }
        .frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)
        .accessibilityElement(children: accessibilityChildren)
        .accessibilityLabel(rowAccessibilityLabel)
    }

    private var rowAccessibilityLabel: String {
        if peerInCall && room.hasActiveCall == false {
            return String(localized: "\(room.accessibilitySummary), In a call")
        }
        return room.accessibilitySummary
    }
}

private struct RoomRowCompactStatusIcon: View {
    let systemImage: String
    let accessibilityLabel: String
    let tint: Color
    @ScaledMetric(relativeTo: .caption) private var width: CGFloat = 16
    @ScaledMetric(relativeTo: .caption) private var height: CGFloat = 20

    var body: some View {
        Image(systemName: systemImage)
            .font(.caption.weight(.semibold))
            .foregroundStyle(tint)
            .frame(width: width, height: height)
            .accessibilityLabel(accessibilityLabel)
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

/// Invite card with Accept / Decline, shared by Rooms and the Notifications
/// invites section.
struct InviteRoomListRow: View {
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
        .synaraCard(fill: SynaraColor.warning.opacity(0.08), stroke: SynaraColor.warning, depth: .layered)
    }
}

private extension RoomSummary {
    var accessibilitySummary: String {
        var parts = [name, lastMessagePreview]
        if unreadBadgeCount > 0 {
            parts.append(String(localized: "\(unreadBadgeCount) unread"))
        }
        if hasHighlight {
            parts.append(String(localized: "mentioned"))
        }
        if isFavorite {
            parts.append(String(localized: "favorite"))
        }
        if hasActiveCall {
            parts.append(liveCallChipLabel)
        }
        if isSecureRoom {
            parts.append(String(localized: "encrypted"))
        }
        if requiresAgentApproval {
            parts.append(String(localized: "approval required"))
        } else if isAgentRoom {
            parts.append(String(localized: "agent room"))
        }
        if relativeActivity.isEmpty == false {
            parts.append(relativeActivity)
        }
        return parts.joined(separator: ", ")
    }

    var relativeActivity: String {
        guard lastActivityAt.timeIntervalSince1970 > 0 else {
            return ""
        }

        let isUITest = SynaraLaunchEnvironment.isUITest
        let referenceDate = isUITest ? RoomListFixtures.now : Date()

        if isUITest == false, lastActivityAt > referenceDate.addingTimeInterval(60) {
            return ""
        }

        let calendar = Calendar.current
        let formatters = SynaraTimeFormatting.formatters()
        if calendar.isDate(lastActivityAt, inSameDayAs: referenceDate) {
            return formatters.clockTime.format(lastActivityAt)
        }

        if calendar.isDateInYesterday(lastActivityAt) {
            return formatters.relativeDayName(offset: -1)
        }

        if let days = calendar.dateComponents([.day], from: calendar.startOfDay(for: lastActivityAt), to: calendar.startOfDay(for: referenceDate)).day,
           days > 0,
           days < 7 {
            return formatters.weekday.format(lastActivityAt)
        }

        let isThisYear = calendar.component(.year, from: lastActivityAt) == calendar.component(.year, from: referenceDate)
        let formatter = isThisYear
            ? formatters.monthDay
            : formatters.monthDayYear
        return formatter.format(lastActivityAt)
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

/// A custom account/filter header already provides the list's top spacing.
private struct RoomListTopMargin: ViewModifier {
    @ViewBuilder
    func body(content: Content) -> some View {
        if #available(iOS 17.0, *) {
            content.contentMargins(.top, 0, for: .scrollContent)
        } else {
            content
        }
    }
}

/// Owns the room list's update stream across appearances. `start` replaces
/// any running stream, so calling it on every appearance is safe, and a
/// stream stopped before its first snapshot simply starts over.
@MainActor
final class RoomListLiveUpdates {
    private var task: Task<Void, Never>?

    var isRunning: Bool {
        task != nil
    }

    func start(service: RoomListServicing, deliver: @escaping @MainActor (RoomListState) -> Void) {
        task?.cancel()
        task = Task { @MainActor in
            for await updatedState in service.roomUpdates() {
                guard Task.isCancelled == false else {
                    return
                }
                deliver(updatedState)
            }
        }
    }

    func stop() {
        task?.cancel()
        task = nil
    }
}
