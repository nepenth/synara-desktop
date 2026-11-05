import SwiftUI

/// Switches kept so the review-2 density pass is easy to revert.
/// `showsSpaceChipStrip` brings back the second chip row (All spaces / Space…)
/// under the filters; spaces are otherwise reached from the title menu.
enum RoomListChromePolicy {
    static let showsSpaceChipStrip = false
}

struct RoomListView: View {
    @Environment(\.appEnvironment) private var environment
    @Environment(\.synaraCanvasLayout) private var canvasLayout
    @Environment(\.synaraSelectedConversationID) private var selectedConversationID
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var state: RoomListState = .idle
    @State private var membershipError: String?
    @State private var searchQuery: String = ProcessInfo.processInfo.environment["SYNARA_UI_TEST_ROOM_SEARCH"] ?? ""
    @State private var selectedFilter: RoomListFilter = .all
    @State private var selectedSpaceID: String?
    @State private var collapsedSections: Set<String> = RoomListSectionCollapse.stored()
    @State private var isRoomManagementSheetPresented = ProcessInfo.processInfo.environment["SYNARA_UI_TEST_ROOM_MANAGEMENT_SHEET"] == "1"
    @State private var loadRoomsTask: Task<Void, Never>?
    @State private var liveUpdates = RoomListLiveUpdates()
    @State private var isSearchPresented = ProcessInfo.processInfo.environment["SYNARA_UI_TEST_ROOM_SEARCH"] != nil
    @State private var roomPendingLeave: RoomSummary?
    @State private var isResettingSession = false
    @State private var sessionRecoveryError: String?
    /// One order for every section (desktop stores one `synara.roomListSort`).
    @State private var sortOrder: RoomListSortOrder = RoomListSortOrder.storedShared()
    /// Direct-message peers whose MSC4426 status says they are in a call.
    @State private var peersInCall: Set<String> = []
    @State private var peerCallStatusGeneration: UInt64 = 0
    /// Direct-message peers' presence from Core; a missing entry is unknown.
    @State private var peerPresence: [String: RoomListPeerPresence] = [:]
    @FocusState private var isSearchFocused: Bool

    var body: some View {
        Group {
            switch state {
            case .idle, .loading:
                VStack(spacing: 0) {
                    roomListChrome(spaces: [], unreadCounts: [:], selectedSpaceTitle: nil, filterBadges: [:])
                    ConnectionStatusChrome()

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
                    title: String(localized: "No rooms yet"),
                    systemImage: "bubble.left.and.bubble.right",
                    store: environment.connectionStatus,
                    connectedMessage: String(localized: "Join a room or start a direct message."),
                    action: HeldConnectionEmptyState.Action(
                        title: String(localized: "Create or join a room"),
                        identifier: "RoomListEmptyStateButton"
                    ) {
                        isRoomManagementSheetPresented = true
                    }
                )
                .synaraConnectionChrome()
            case .failed(let message):
                VStack(spacing: SynaraSpacing.medium) {
                    SynaraErrorState(title: String(localized: "Could Not Load Rooms"), message: message) {
                        loadRooms(startUpdatesAfterLoad: true)
                    }
                    Button("Sign in again") {
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
                .synaraConnectionChrome()
            case .loaded(let rooms):
                let filteredRooms = filteredRooms(from: rooms)
                let sections = RoomListFavorites.sections(from: filteredRooms)
                let favoriteRooms = RoomListFavorites.sorted(sections.favorites, order: sortOrder)
                let channelRooms = RoomListFavorites.sorted(sections.channels, order: sortOrder)
                let directRooms = RoomListFavorites.sorted(sections.directs, order: sortOrder)
                let lowPriorityRooms = RoomListFavorites.sorted(sections.lowPriority, order: sortOrder)
                let spaces = spaces(from: rooms)
                let spaceUnreadCounts = RoomListSpaceGrouping.unreadCountsBySpaceID(from: rooms)
                let selectedSpaceTitle = RoomListSpaceGrouping.selectedSpaceName(
                    in: spaces,
                    selectedSpaceID: selectedSpaceID
                )
                let spaceChannelGroups = RoomListSpaceGrouping.spaceChannelGroups(from: channelRooms)
                let ungroupedChannelRooms = RoomListSpaceGrouping.ungroupedChannelRooms(from: channelRooms)
                // The first visible section's header gets no top spacing.
                let firstSectionKey = Self.firstSectionKey(
                    favorites: favoriteRooms,
                    middle: selectedSpaceTitle != nil && selectedSpaceID != nil
                        ? [(RoomListSectionKey.space(selectedSpaceID ?? ""), channelRooms)]
                        : spaceChannelGroups.map { (RoomListSectionKey.space($0.space.id), $0.rooms) }
                            + [(RoomListSectionKey.channels, ungroupedChannelRooms)],
                    directs: directRooms,
                    lowPriority: lowPriorityRooms
                )
                VStack(spacing: 0) {
                    roomListChrome(
                        spaces: spaces,
                        unreadCounts: spaceUnreadCounts,
                        selectedSpaceTitle: selectedSpaceTitle,
                        filterBadges: filterBadges(from: rooms)
                    )
                    // Under the server name: the connection banner and beam.
                    ConnectionStatusChrome()

                    List {
                        if searchQuery.isEmpty == false && filteredRooms.isEmpty {
                            SynaraEmptyState(
                                title: String(localized: "No Matching Rooms"),
                                systemImage: "magnifyingglass",
                                message: String(localized: "Try another room name or message preview."),
                                paintsBackground: false
                            )
                            .listRowInsets(EdgeInsets())
                            .listRowBackground(SynaraChrome.roomList)
                        }

                        roomSection(RoomListSectionKey.favorites, title: String(localized: "Favorites"), rooms: favoriteRooms, firstKey: firstSectionKey)

                        if let selectedSpaceTitle, let selectedSpaceID {
                            roomSection(RoomListSectionKey.space(selectedSpaceID), title: selectedSpaceTitle, rooms: channelRooms, firstKey: firstSectionKey)
                        } else {
                            ForEach(spaceChannelGroups) { group in
                                roomSection(RoomListSectionKey.space(group.space.id), title: group.space.name, rooms: group.rooms, firstKey: firstSectionKey)
                            }

                            roomSection(RoomListSectionKey.channels, title: String(localized: "Rooms"), rooms: ungroupedChannelRooms, firstKey: firstSectionKey)
                        }

                        roomSection(RoomListSectionKey.directs, title: String(localized: "Direct messages"), rooms: directRooms, firstKey: firstSectionKey)
                        roomSection(RoomListSectionKey.lowPriority, title: String(localized: "Low priority"), rooms: lowPriorityRooms, firstKey: firstSectionKey)
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
        .task(id: peerPresenceKey) {
            await refreshPeerPresence(peerIDs: peerPresenceKey.peerIDs)
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
        .onChange(of: sortOrder) { sort in
            RoomListSortOrder.persistShared(sort)
        }
    }

    /// One 44 pt header row (account mark, title menu, search, new room), the
    /// search field while open, and one filter chip row.
    @ViewBuilder
    private func roomListChrome(
        spaces: [SpaceSummary],
        unreadCounts: [String: Int],
        selectedSpaceTitle: String?,
        filterBadges: [RoomListFilter: Int]
    ) -> some View {
        VStack(spacing: SynaraSpacing.small) {
            RoomListHeader(
                title: selectedSpaceTitle ?? accountMenuTitle,
                spaces: spaces,
                unreadCountsBySpaceID: unreadCounts,
                selectedSpaceID: $selectedSpaceID,
                sortOrder: $sortOrder,
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
                filterBadges: filterBadges
            )
            if RoomListChromePolicy.showsSpaceChipStrip, spaces.isEmpty == false {
                SpaceFilterStrip(
                    spaces: spaces,
                    unreadCountsBySpaceID: unreadCounts,
                    selectedSpaceID: $selectedSpaceID
                )
            }
        }
        .padding(.horizontal, SynaraSpacing.large)
        .padding(.top, SynaraSpacing.small)
        .padding(.bottom, SynaraSpacing.xSmall)
        .background(SynaraChrome.roomList)
    }

    /// The key of the first section that will draw (empty sections are
    /// skipped), so only later headers carry the top spacing.
    private static func firstSectionKey(
        favorites: [RoomSummary],
        middle: [(String, [RoomSummary])],
        directs: [RoomSummary],
        lowPriority: [RoomSummary]
    ) -> String? {
        var sections: [(String, [RoomSummary])] = [(RoomListSectionKey.favorites, favorites)]
        sections.append(contentsOf: middle)
        sections.append((RoomListSectionKey.directs, directs))
        sections.append((RoomListSectionKey.lowPriority, lowPriority))
        return sections.first { $0.1.isEmpty == false }?.0
    }

    /// A collapsible section. Collapsing hides the read rows only; unread
    /// rooms stay listed, as in Slack's sidebar.
    @ViewBuilder
    private func roomSection(_ key: String, title: String, rooms: [RoomSummary], firstKey: String?) -> some View {
        if rooms.isEmpty == false {
            let isCollapsed = collapsedSections.contains(key)
            let visibleRooms = isCollapsed ? rooms.filter(\.hasUnreadActivity) : rooms
            Section {
                ForEach(visibleRooms) { room in
                    roomRow(room)
                }
            } header: {
                RoomListSectionHeader(
                    title: title,
                    count: rooms.count,
                    isCollapsed: isCollapsed,
                    identifier: key,
                    isFirst: key == firstKey
                ) {
                    toggleSection(key)
                }
                // The header carries its own top spacing; its chevron sits
                // in the rows' glyph column (row inset + row padding).
                .listRowInsets(EdgeInsets(
                    top: 0,
                    leading: SynaraSpacing.small + SynaraSpacing.medium,
                    bottom: 0,
                    trailing: SynaraSpacing.small + SynaraSpacing.medium
                ))
            }
        }
    }

    private func toggleSection(_ key: String) {
        withAnimation(reduceMotion ? nil : .easeInOut(duration: 0.2)) {
            if collapsedSections.contains(key) {
                collapsedSections.remove(key)
            } else {
                collapsedSections.insert(key)
            }
        }
        RoomListSectionCollapse.persist(collapsedSections)
        SynaraHaptics.trigger(.lightImpact)
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

    private var peerPresenceKey: RoomListPeerPresence.Key {
        guard case .loaded(let rooms) = state else {
            return RoomListPeerPresence.Key(peerIDs: [], generation: peerCallStatusGeneration)
        }
        return RoomListPeerPresence.Key(
            peerIDs: RoomListPeerPresence.peerIDs(in: rooms),
            generation: peerCallStatusGeneration
        )
    }

    /// One pass over the direct-message peers' presence when that set
    /// changes, the list reappears or is pulled to refresh. Core caches the
    /// reads (`PRESENCE_FETCH_TTL`), so this is not a request per row.
    private func refreshPeerPresence(peerIDs: [String]) async {
        let peers = Set(peerIDs)
        peerPresence = peerPresence.filter { peers.contains($0.key) }
        for peerID in peerIDs {
            let snapshot = await environment.matrix.presence(userID: peerID)
            guard Task.isCancelled == false else {
                return
            }
            peerPresence[peerID] = RoomListPeerPresence.presence(from: snapshot)
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
                RoomListRow(
                    room: room,
                    peerInCall: RoomListPeerCallStatus.isInCall(room, peersInCall: peersInCall),
                    isSelected: isSelectedConversation(room),
                    presence: RoomListPeerPresence.peerID(for: room).flatMap { peerPresence[$0] }
                )
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.horizontal, SynaraSpacing.medium)
                    .padding(.vertical, SynaraMetrics.roomRowVerticalPadding)
                    .background {
                        // Rows sit on the list plane; the open conversation
                        // is marked by the accent fill, as on desktop.
                        if isSelectedConversation(room) {
                            RoundedRectangle(cornerRadius: SynaraRadius.card, style: .continuous)
                                .fill(SynaraColor.accent.opacity(SynaraMetrics.selectedRowFillOpacity))
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
            .overlay(alignment: .leading) {
                // Drawn in the row's leading list inset, so the text does not
                // move; it slides with the row when a swipe action opens.
                if RoomListUnreadMarker.isShown(for: room) {
                    RoomListUnreadMarker()
                        .offset(x: -RoomListUnreadMarker.gutterOffset)
                }
            }
            .accessibilityLabel(rowAccessibilityLabel(room))
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
                        Label("Mark read", systemImage: "envelope.open")
                    }
                    .tint(SynaraColor.accent)
                    .accessibilityIdentifier("MarkRead-\(room.id)")
                }
            }
            .swipeActions(edge: .trailing, allowsFullSwipe: false) {
                Button {
                    toggleMute(room)
                } label: {
                    Label(
                        room.isMuted ? "Unmute" : "Mute",
                        systemImage: room.isMuted ? "bell" : "bell.slash"
                    )
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
                if room.hasUnreadActivity {
                    Button {
                        markRoomAsRead(room)
                    } label: {
                        Label("Mark read", systemImage: "envelope.open")
                    }
                }
                Button {
                    toggleFavorite(room)
                } label: {
                    Label(
                        room.isFavorite ? "Remove from Favorites" : "Add to Favorites",
                        systemImage: room.isFavorite ? "star.slash" : "star"
                    )
                }
                Button {
                    toggleMute(room)
                } label: {
                    Label(
                        room.isMuted ? "Unmute" : "Mute",
                        systemImage: room.isMuted ? "bell" : "bell.slash"
                    )
                }
            }
        }
    }

    private func rowAccessibilityLabel(_ room: RoomSummary) -> String {
        guard let presence = RoomListPeerPresence.peerID(for: room).flatMap({ peerPresence[$0] }) else {
            return room.accessibilitySummary
        }
        let presenceLabel = presence == .online ? String(localized: "Online") : String(localized: "Offline")
        return [room.accessibilitySummary, presenceLabel].joined(separator: ", ")
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
                SynaraHaptics.trigger(.success)
                loadRooms(showLoading: false)
            }
        }
    }

    private func toggleFavorite(_ room: RoomSummary) {
        Task {
            do {
                try await environment.roomManagement.setRoomFavorite(!room.isFavorite, roomID: room.id)
                await MainActor.run {
                    SynaraHaptics.trigger(.lightImpact)
                    loadRooms(showLoading: false)
                }
            } catch {
                await MainActor.run {
                    membershipError = RoomManagementError.failed.localizedDescription
                }
            }
        }
    }

    /// Mute ↔ default. The reload shows the muted row style (or clears it) and
    /// flips the swipe label.
    private func toggleMute(_ room: RoomSummary) {
        Task {
            do {
                try await environment.roomManagement.setNotificationMode(
                    room.isMuted ? .default : .mute,
                    roomID: room.id
                )
                await MainActor.run {
                    SynaraHaptics.trigger(.lightImpact)
                    loadRooms(showLoading: false)
                }
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

/// Keys of the collapsible room-list sections.
enum RoomListSectionKey {
    static let favorites = "favorites"
    static let channels = "channels"
    static let directs = "directs"
    static let lowPriority = "lowPriority"

    static func space(_ id: String) -> String {
        "space:\(id)"
    }
}

/// Collapsed sections survive relaunch, like the sort order. UI tests start
/// expanded whatever the simulator's defaults hold.
enum RoomListSectionCollapse {
    static let storageKey = "synara.roomListCollapsedSections"

    static func stored(defaults: UserDefaults = .standard) -> Set<String> {
        guard SynaraLaunchEnvironment.isUITest == false else {
            return []
        }
        return Set(defaults.stringArray(forKey: storageKey) ?? [])
    }

    static func persist(_ keys: Set<String>, defaults: UserDefaults = .standard) {
        defaults.set(Array(keys).sorted(), forKey: storageKey)
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

/// The one header row: account mark, a title menu (space switcher + sort),
/// search and new room. 44 pt, the shared `SynaraMetrics.columnHeaderHeight`.
private struct RoomListHeader: View {
    let title: String
    let spaces: [SpaceSummary]
    let unreadCountsBySpaceID: [String: Int]
    @Binding var selectedSpaceID: String?
    @Binding var sortOrder: RoomListSortOrder
    let onAccount: () -> Void
    let onSearch: () -> Void
    let onNewRoom: () -> Void

    var body: some View {
        HStack(spacing: SynaraSpacing.small) {
            Button(action: onAccount) {
                SynaraBrandMark(size: 28)
                    .frame(width: SynaraHitTarget.minimum, height: SynaraHitTarget.minimum)
                    .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Account menu")
            .accessibilityHint("Shows account details, settings, and logout")
            .accessibilityIdentifier("RoomHeaderAccountMenuButton")

            titleMenu

            Spacer(minLength: SynaraSpacing.small)

            Button(action: onSearch) {
                Image(systemName: "magnifyingglass")
                    .font(.body.weight(.semibold))
                    .frame(width: SynaraHitTarget.minimum, height: SynaraHitTarget.minimum)
                    .foregroundStyle(SynaraColor.accent)
            }
            .buttonStyle(SynaraTactileButtonStyle())
            .contentShape(Rectangle())
            .accessibilityLabel("Search rooms")
            .accessibilityIdentifier("RoomSearchButton")

            Button(action: onNewRoom) {
                Image(systemName: "square.and.pencil")
                    .font(.body.weight(.semibold))
                    .frame(width: SynaraHitTarget.minimum, height: SynaraHitTarget.minimum)
                    .foregroundStyle(SynaraColor.accent)
            }
            .buttonStyle(SynaraTactileButtonStyle())
            .contentShape(Rectangle())
            .accessibilityLabel("New room")
            .accessibilityIdentifier("NewRoomButton")
        }
        .frame(minHeight: SynaraMetrics.columnHeaderHeight)
    }

    /// Spaces (when the account has any) and the list order live behind the
    /// title, so the chrome stays one row.
    private var titleMenu: some View {
        Menu {
            if spaces.isEmpty == false {
                // Inside a space, the way back to every room comes first and
                // reads as a way back, not as one more checkable row.
                if selectedSpaceID != nil {
                    Section {
                        Button {
                            selectedSpaceID = nil
                        } label: {
                            Label(String(localized: "All spaces"), systemImage: "arrow.uturn.backward")
                        }
                        .accessibilityHint("Shows the rooms of every space")
                        .accessibilityIdentifier("SpaceFilter-all")
                    }
                }

                Section {
                    if selectedSpaceID == nil {
                        Button {
                            selectedSpaceID = nil
                        } label: {
                            menuItem(String(localized: "All spaces"), isSelected: true)
                        }
                        .accessibilityIdentifier("SpaceFilter-all")
                    }

                    ForEach(spaces) { space in
                        Button {
                            selectedSpaceID = space.id
                        } label: {
                            spaceMenuItem(space, isSelected: selectedSpaceID == space.id)
                        }
                        .accessibilityLabel(spaceAccessibilityLabel(space))
                        .accessibilityIdentifier("SpaceFilter-\(space.id)")
                    }
                } header: {
                    Text("Spaces")
                }
            }

            Section {
                ForEach(RoomListSortOrder.allCases) { sort in
                    Button {
                        sortOrder = sort
                    } label: {
                        menuItem(sort.shortTitle, isSelected: sortOrder == sort)
                    }
                    .accessibilityLabel(sort.title)
                    .accessibilityIdentifier("RoomListSortMenu-\(sort.rawValue)")
                }
            } header: {
                Text("Sort by")
            }
        } label: {
            HStack(spacing: SynaraSpacing.xSmall) {
                Text(title)
                    .font(.title3.weight(.semibold))
                    .foregroundStyle(SynaraColor.headingText)
                    .lineLimit(1)
                Image(systemName: "chevron.down")
                    .font(.caption.weight(.bold))
                    .foregroundStyle(SynaraColor.secondaryText)
                    .accessibilityHidden(true)
            }
            .frame(minHeight: SynaraHitTarget.minimum)
            .contentShape(Rectangle())
        }
        .accessibilityLabel(Text(title))
        .accessibilityValue(Text(sortOrder.title))
        .accessibilityHint("Switches space and list order")
        .accessibilityIdentifier("RoomListTitleMenu")
    }

    @ViewBuilder
    private func menuItem(_ title: String, isSelected: Bool) -> some View {
        if isSelected {
            Label(title, systemImage: "checkmark")
        } else {
            Text(title)
        }
    }

    /// The space name with its unread count as the menu row's subtitle.
    @ViewBuilder
    private func spaceMenuItem(_ space: SpaceSummary, isSelected: Bool) -> some View {
        let unread = unreadCountsBySpaceID[space.id] ?? 0
        if isSelected {
            Label {
                Text(space.name)
                if unread > 0 {
                    Text(String(localized: "\(unread) unread"))
                }
            } icon: {
                Image(systemName: "checkmark")
            }
        } else {
            Text(space.name)
            if unread > 0 {
                Text(String(localized: "\(unread) unread"))
            }
        }
    }

    private func spaceAccessibilityLabel(_ space: SpaceSummary) -> String {
        let unread = unreadCountsBySpaceID[space.id] ?? 0
        return unread > 0 ? String(localized: "\(space.name), \(unread) unread") : space.name
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

/// The pre-review-2 space chip row. Kept behind
/// `RoomListChromePolicy.showsSpaceChipStrip` for an easy revert; the title
/// menu carries the same identifiers.
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

/// The one section header of the Rooms list and the Notifications inbox:
/// title, count, and (when `onToggle` is set) a chevron that collapses the
/// section. Collapsed state is persisted by the owner.
struct RoomListSectionHeader: View {
    let title: String
    let count: Int
    var isCollapsed = false
    /// Stable, non-localized suffix of the accessibility identifier.
    var identifier: String? = nil
    /// The list's first header sits flush; later ones get
    /// `SynaraMetrics.roomListSectionHeaderTopSpacing` above them.
    var isFirst = false
    var onToggle: (() -> Void)? = nil

    var body: some View {
        if let onToggle {
            Button(action: onToggle) {
                label
            }
            .buttonStyle(.plain)
            .accessibilityLabel(Text(title))
            .accessibilityValue(isCollapsed ? Text("Collapsed") : Text("Expanded"))
            .accessibilityHint(isCollapsed ? Text("Expands this section") : Text("Collapses this section"))
            .accessibilityIdentifier("RoomListSectionHeader-\(identifier ?? title)")
        } else {
            label
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("RoomListSectionHeader-\(identifier ?? title)")
        }
    }

    /// Wave 3: a quiet caption (12 pt scaled, semibold, secondary, 0.02 em
    /// tracking) with an always-visible leading chevron that turns when the
    /// section is collapsed, so headers frame the rows instead of competing.
    private var label: some View {
        // The chevron takes the rows' glyph column, so the title lines up
        // with the room names below it.
        HStack(spacing: SynaraSpacing.medium) {
            if onToggle != nil {
                Image(systemName: "chevron.down")
                    .font(SynaraTypography.sectionCaption)
                    .imageScale(.small)
                    .foregroundStyle(SynaraColor.secondaryText)
                    .rotationEffect(.degrees(isCollapsed ? -90 : 0))
                    .frame(width: SynaraMetrics.roomGlyph)
                    .accessibilityHidden(true)
            }
            Text(title)
                .font(SynaraTypography.sectionCaption)
                .tracking(SynaraMetrics.roomListSectionHeaderTracking)
                .textCase(nil)
                .foregroundStyle(SynaraColor.secondaryText)
                .lineLimit(1)
                .truncationMode(.middle)
            Spacer(minLength: SynaraSpacing.small)
            Text("\(count)")
                .font(SynaraTypography.messageMeta)
                .monospacedDigit()
                .foregroundStyle(SynaraColor.tertiaryText)
        }
        .frame(maxWidth: .infinity, minHeight: 28, alignment: .leading)
        .padding(.top, isFirst ? 0 : SynaraMetrics.roomListSectionHeaderTopSpacing)
        .contentShape(Rectangle())
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
        .frame(minHeight: SynaraHitTarget.minimum)
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
    /// The open conversation on a split canvas (iPad regular width).
    var isSelected = false
    /// The direct-message peer's presence; nil draws no dot.
    var presence: RoomListPeerPresence? = nil
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

    /// Wave 3 reading hierarchy, mirrored from desktop: read rows recede
    /// (secondary name, tertiary glyph, regular), unread rows step forward
    /// (strongest text colour, semibold, secondary glyph), muted rows stay
    /// tertiary, and the open conversation takes the accent.
    private var nameColor: Color {
        if isSelected {
            return SynaraColor.accentText
        }
        if room.isMuted {
            return SynaraColor.tertiaryText
        }
        return isUnreadAndAudible ? SynaraColor.headingText : SynaraColor.secondaryText
    }

    private var glyphColor: Color {
        if isSelected {
            return SynaraColor.accent
        }
        if room.isMuted {
            return SynaraColor.tertiaryText
        }
        return isUnreadAndAudible ? SynaraColor.secondaryText : SynaraColor.tertiaryText
    }

    private var nameWeight: Font.Weight {
        isSelected || isUnreadAndAudible ? .semibold : .regular
    }

    private var isUnreadAndAudible: Bool {
        hasUnreadActivity && room.isMuted == false
    }

    private var previewColor: Color {
        hasUnreadActivity && room.isMuted == false ? SynaraColor.secondaryText : SynaraColor.tertiaryText
    }

    /// Mentions and unread direct messages get the red count; a plain unread
    /// channel is marked by its bold name alone. Muted rooms badge mentions only.
    private var showsCountBadge: Bool {
        if room.hasHighlight {
            return true
        }
        return room.isMuted == false && room.kind == .directMessage && hasUnreadActivity
    }

    var body: some View {
        HStack(spacing: SynaraSpacing.medium) {
            SynaraRoomGlyphView(room: room, tint: glyphColor, presence: presence)

            VStack(alignment: .leading, spacing: 2) {
                HStack(alignment: .firstTextBaseline, spacing: SynaraSpacing.small) {
                    // One line at standard sizes, truncated in the middle so
                    // "Project - Home Management" keeps its distinguishing tail.
                    Text(room.name)
                        .font(SynaraTypography.body.weight(nameWeight))
                        .foregroundStyle(nameColor)
                        .lineLimit(SynaraDynamicTypeLayout.titleLineLimit(dynamicTypeSize))
                        .truncationMode(.middle)
                        .layoutPriority(1)
                        .accessibilityIdentifier("RoomRowTitle-\(room.id)")

                    if room.isEncrypted {
                        Image(systemName: "lock.fill")
                            .font(SynaraTypography.chipLabel)
                            .foregroundStyle(SynaraColor.tertiaryText)
                            .accessibilityLabel("Encrypted room")
                    }

                    if room.isMuted {
                        Image(systemName: "bell.slash.fill")
                            .font(SynaraTypography.chipLabel)
                            .foregroundStyle(SynaraColor.tertiaryText)
                            .accessibilityLabel("Muted")
                            .accessibilityIdentifier("RoomRowMuted-\(room.id)")
                    }

                    if room.isFavorite {
                        Image(systemName: "star.fill")
                            .font(SynaraTypography.fineMeta.weight(.semibold))
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
                        .font(SynaraTypography.roomPreview)
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

                        if showsCountBadge {
                            SynaraUnreadBadge(count: room.unreadBadgeCount, highlighted: true)
                        }
                    }
                    .fixedSize(horizontal: true, vertical: false)
                }
            }
        }
        .frame(maxWidth: .infinity, minHeight: SynaraMetrics.roomRowMinHeight, alignment: .leading)
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

/// Wave 3 unread gutter marker: a 3 x 16 capsule in the strongest text
/// colour, centred in the row's leading list inset of unread rows (mentions
/// included). Read and muted rows have none.
struct RoomListUnreadMarker: View {
    /// From the row's leading edge back into the middle of its
    /// `SynaraSpacing.small` list inset.
    static let gutterOffset = (SynaraSpacing.small + SynaraMetrics.unreadMarkerWidth) / 2

    static func isShown(for room: RoomSummary) -> Bool {
        room.membership != .invited && room.hasUnreadActivity && room.isMuted == false
    }

    var body: some View {
        Capsule()
            .fill(SynaraColor.headingText)
            .frame(width: SynaraMetrics.unreadMarkerWidth, height: SynaraMetrics.unreadMarkerHeight)
            .accessibilityHidden(true)
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
        if isMuted {
            parts.append(String(localized: "muted"))
        }
        if hasActiveCall {
            parts.append(liveCallChipLabel)
        }
        if isEncrypted {
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
}

struct RoomListView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationStack {
            RoomListView()
        }
        .environment(\.appEnvironment, AppEnvironment.mock())
    }
}

/// A custom account/filter header already provides the list's top spacing,
/// and sections sit close together (Slack density) instead of the default
/// grouped gap.
private struct RoomListTopMargin: ViewModifier {
    @ViewBuilder
    func body(content: Content) -> some View {
        if #available(iOS 17.0, *) {
            content
                .contentMargins(.top, 0, for: .scrollContent)
                .listSectionSpacing(SynaraMetrics.roomListSectionSpacing)
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
