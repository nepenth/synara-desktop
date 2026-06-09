import XCTest
@testable import Synara

final class RoomListServiceTests: XCTestCase {
    func testRoomsSortByHighlightUnreadThenActivity() {
        let rooms = RoomListFixtures.small().reversed()

        let sorted = RoomListFixtures.sorted(Array(rooms))

        XCTAssertEqual(sorted.first?.id, "!project:matrix.org")
    }

    func testLargeFixtureHasStableIdentifiers() {
        let rooms = RoomListFixtures.large()

        XCTAssertEqual(rooms.count, 1_000)
        XCTAssertEqual(Set(rooms.map(\.id)).count, 1_000)
    }

    func testMockRoomListReturnsSortedLoadedState() async {
        let rooms = RoomListFixtures.small().reversed()
        let service = MockRoomListService(state: .loaded(Array(rooms)))

        let state = await service.loadRooms()

        XCTAssertEqual(state, .loaded(RoomListFixtures.sorted(RoomListFixtures.small())))
        XCTAssertEqual(service.loadCallCount, 1)
    }

    func testRoomDisplayNameLookupResolvesCachedNames() async {
        let service = MockRoomListService(state: .loaded(RoomListFixtures.small()))
        let state = await service.loadRooms()

        XCTAssertEqual(service.roomDisplayName(roomID: "!project:matrix.org"), "Product")
        XCTAssertEqual(
            RoomDisplayNameLookup.resolve(
                roomID: "!project:matrix.org",
                names: RoomDisplayNameLookup.names(from: state)
            ),
            "Product"
        )
        XCTAssertEqual(
            RoomDisplayNameLookup.resolve(roomID: "!missing:matrix.org", names: [:]),
            "!missing:matrix.org"
        )
    }

    func testClearCacheReturnsEmptyState() async {
        let service = MockRoomListService()

        service.clearCache()
        let state = await service.loadRooms()

        XCTAssertEqual(state, .empty)
        XCTAssertEqual(service.clearCallCount, 1)
    }

    func testMockInviteTransitionAcceptsInviteIntoJoinedRoom() async throws {
        let service = MockInviteTransitionService()

        try await service.acceptInvite(roomID: "!alerts:matrix.org")
        let state = await service.loadRooms()

        guard case .loaded(let rooms) = state else {
            XCTFail("Expected loaded rooms")
            return
        }

        XCTAssertEqual(rooms.first?.id, "!alerts:matrix.org")
        XCTAssertEqual(rooms.first?.membership, .joined)
        XCTAssertEqual(rooms.first?.lastMessagePreview, "Joined room")
    }

    func testMockInviteTransitionRejectsInviteIntoEmptyState() async throws {
        let service = MockInviteTransitionService()

        try await service.rejectInvite(roomID: "!alerts:matrix.org")
        let state = await service.loadRooms()

        XCTAssertEqual(state, .empty)
    }

    func testSearchFilterExcludesNonMatchingInvites() {
        let invite = RoomSummary(
            id: "!alerts:matrix.org",
            name: "Alerts",
            lastMessagePreview: "You are invited",
            unreadCount: 1,
            hasHighlight: false,
            kind: .room,
            membership: .invited,
            lastActivityAt: RoomListFixtures.now
        )
        let joined = RoomSummary(
            id: "!alice:matrix.org",
            name: "Alice",
            lastMessagePreview: "Hello from Alice",
            unreadCount: 0,
            hasHighlight: false,
            kind: .directMessage,
            membership: .joined,
            lastActivityAt: RoomListFixtures.now
        )
        let rooms = [invite, joined]

        let filtered = RoomListSearchFilter.applySearchQuery("Alice", to: rooms, invitedRooms: [invite])

        XCTAssertEqual(filtered.map(\.id), ["!alice:matrix.org"])
    }

    func testSearchFilterIncludesMatchingInvites() {
        let invite = RoomSummary(
            id: "!alerts:matrix.org",
            name: "Alerts",
            lastMessagePreview: "You are invited",
            unreadCount: 1,
            hasHighlight: false,
            kind: .room,
            membership: .invited,
            lastActivityAt: RoomListFixtures.now
        )
        let joined = RoomSummary(
            id: "!project:matrix.org",
            name: "Project",
            lastMessagePreview: "Latest update",
            unreadCount: 2,
            hasHighlight: false,
            kind: .room,
            membership: .joined,
            lastActivityAt: RoomListFixtures.now
        )
        let rooms = [invite, joined]

        let filtered = RoomListSearchFilter.applySearchQuery("Alerts", to: rooms, invitedRooms: [invite])

        XCTAssertEqual(filtered.map(\.id), ["!alerts:matrix.org"])
    }

    func testScopeFilterUnreadKeepsInvitesAndUnreadRooms() {
        let invite = RoomSummary(
            id: "!alerts:matrix.org",
            name: "Alerts",
            lastMessagePreview: "You are invited",
            unreadCount: 1,
            hasHighlight: false,
            kind: .room,
            membership: .invited,
            lastActivityAt: RoomListFixtures.now
        )
        let unread = RoomSummary(
            id: "!project:matrix.org",
            name: "Product",
            lastMessagePreview: "Mina: Here's the latest spec",
            unreadCount: 3,
            hasHighlight: true,
            kind: .room,
            membership: .joined,
            lastActivityAt: RoomListFixtures.now
        )
        let read = RoomSummary(
            id: "!general:matrix.org",
            name: "General",
            lastMessagePreview: "Kai: Project update looks good",
            unreadCount: 0,
            hasHighlight: false,
            kind: .room,
            membership: .joined,
            lastActivityAt: RoomListFixtures.now
        )

        let filtered = RoomListScopeFilter.filteredRooms(
            from: [invite, unread, read],
            filter: .unread,
            selectedSpaceID: nil,
            searchQuery: ""
        )

        XCTAssertEqual(Set(filtered.map(\.id)), Set(["!alerts:matrix.org", "!project:matrix.org"]))
    }

    func testScopeFilterMentionsExcludesNonHighlightedRooms() {
        let rooms = RoomListFixtures.small()

        let filtered = RoomListScopeFilter.filteredRooms(
            from: rooms,
            filter: .mentions,
            selectedSpaceID: nil,
            searchQuery: ""
        )

        XCTAssertTrue(filtered.allSatisfy(\.hasHighlight))
        XCTAssertEqual(filtered.map(\.id), ["!project:matrix.org"])
    }

    func testScopeFilterRespectsSelectedSpace() {
        let rooms = RoomListFixtures.small()

        let filtered = RoomListScopeFilter.filteredRooms(
            from: rooms,
            filter: .all,
            selectedSpaceID: "!workspace:matrix.org",
            searchQuery: ""
        )

        XCTAssertEqual(
            filtered.map(\.id),
            ["!general:matrix.org", "!project:matrix.org", "!design:matrix.org"]
        )
    }

    func testScopeFilterAgentsIncludesOnlyAgentRooms() {
        let rooms = RoomListFixtures.small()

        let filtered = RoomListScopeFilter.filteredRooms(
            from: rooms,
            filter: .agents,
            selectedSpaceID: nil,
            searchQuery: ""
        )

        XCTAssertTrue(filtered.allSatisfy(\.isAgentRoom))
        XCTAssertEqual(filtered.map(\.id), ["!agent-workflows:matrix.org"])
    }

    func testAgentRoomRequiresRecentAgentActivity() {
        let agentRoom = RoomSummary(
            id: "!agent:matrix.org",
            name: "Agent Workflows",
            lastMessagePreview: "Approval required",
            unreadCount: 1,
            hasHighlight: false,
            kind: .room,
            membership: .joined,
            lastActivityAt: RoomListFixtures.now,
            hasAgentActivity: true
        )
        let generalRoom = RoomSummary(
            id: "!general:matrix.org",
            name: "General",
            lastMessagePreview: "Hello",
            unreadCount: 0,
            hasHighlight: false,
            kind: .room,
            membership: .joined,
            lastActivityAt: RoomListFixtures.now,
            hasAgentActivity: false
        )

        XCTAssertTrue(agentRoom.isAgentRoom)
        XCTAssertFalse(generalRoom.isAgentRoom)
    }

    func testNotificationsInboxCaughtUpWhenNoSectionsOrPendingAgents() {
        let sections = NotificationsInboxSections(
            mentions: [],
            invites: [],
            unreadRooms: []
        )

        XCTAssertTrue(NotificationsInboxSections.isCaughtUp(sections: sections, agentPendingCount: 0))
        XCTAssertFalse(NotificationsInboxSections.isCaughtUp(sections: sections, agentPendingCount: 2))
    }

    func testSpaceUnreadCountsSumUnreadRoomsPerSpace() {
        let rooms = RoomListFixtures.small()

        let counts = RoomListSpaceGrouping.unreadCountsBySpaceID(from: rooms)

        XCTAssertEqual(counts["!workspace:matrix.org"], 10)
        XCTAssertEqual(counts["!ops:matrix.org"], 3)
    }

    func testSpaceChannelGroupsUsePrimaryParentSpace() {
        let rooms = RoomListFixtures.small().filter { $0.kind == .room }

        let groups = RoomListSpaceGrouping.spaceChannelGroups(from: rooms)

        XCTAssertEqual(groups.map(\.space.name), ["Ops", "Workspace"])
        XCTAssertEqual(
            groups.first(where: { $0.space.id == "!workspace:matrix.org" })?.rooms.map(\.id),
            ["!general:matrix.org", "!project:matrix.org", "!design:matrix.org"]
        )
        XCTAssertEqual(
            groups.first(where: { $0.space.id == "!ops:matrix.org" })?.rooms.map(\.id),
            ["!security:matrix.org", "!agent-workflows:matrix.org"]
        )
    }

    func testUngroupedChannelRoomsExcludeParentedChannels() {
        let rooms = RoomListFixtures.small().filter { $0.kind == .room }

        let ungrouped = RoomListSpaceGrouping.ungroupedChannelRooms(from: rooms)

        XCTAssertTrue(ungrouped.isEmpty)
    }

    func testRoomSummarySpaceConveniencesExposePrimaryParent() {
        let room = RoomListFixtures.small().first { $0.id == "!general:matrix.org" }

        XCTAssertEqual(room?.parentSpaceID, "!workspace:matrix.org")
        XCTAssertEqual(room?.spaceName, "Workspace")
    }

    func testSelectedSpaceNameResolvesFromSpaceSummaries() {
        let spaces = Array(Set(RoomListFixtures.small().flatMap(\.parentSpaces)))

        XCTAssertEqual(
            RoomListSpaceGrouping.selectedSpaceName(in: spaces, selectedSpaceID: "!ops:matrix.org"),
            "Ops"
        )
        XCTAssertNil(RoomListSpaceGrouping.selectedSpaceName(in: spaces, selectedSpaceID: nil))
    }

    func testNotificationBadgeSummaryMatchesSharedContractFixture() {
        let summary = NotificationBadgeSummary.summarizeNotifications(
            NotificationSummaryInput(
                unreadCounts: [
                    BadgeUnreadSource(total: 4, highlight: 2),
                    BadgeUnreadSource(total: 3)
                ],
                laterActiveCount: 5,
                inviteCount: 2,
                agentApprovalCount: 1
            )
        )

        XCTAssertEqual(summary?.appBadgeCount, 10)
        XCTAssertEqual(summary?.inboxBadgeCount, 8)
        XCTAssertEqual(summary?.laterActiveCount, 5)
        XCTAssertEqual(summary?.inviteCount, 2)
        XCTAssertEqual(summary?.agentApprovalCount, 1)
        XCTAssertEqual(summary?.highlightCount, 2)
        XCTAssertEqual(summary?.unreadCount, 3)
    }

    func testNotificationBadgeSummaryClampsInvalidCounts() {
        let summary = NotificationBadgeSummary.summarizeNotifications(
            NotificationSummaryInput(
                unreadCounts: [
                    BadgeUnreadSource(total: -1, highlight: -2),
                    BadgeUnreadSource(total: 3)
                ],
                laterActiveCount: 2,
                inviteCount: -1,
                agentApprovalCount: -4
            )
        )

        XCTAssertEqual(summary?.appBadgeCount, 5)
        XCTAssertEqual(summary?.inboxBadgeCount, 2)
        XCTAssertEqual(summary?.highlightCount, 0)
        XCTAssertEqual(summary?.unreadCount, 3)
    }

    func testNotificationsInboxSectionsPartitionRooms() {
        let rooms = [
            RoomSummary(
                id: "!project:matrix.org",
                name: "Product",
                lastMessagePreview: "Mention",
                unreadCount: 3,
                hasHighlight: true,
                kind: .room,
                membership: .joined,
                lastActivityAt: RoomListFixtures.now
            ),
            RoomSummary(
                id: "!alerts:matrix.org",
                name: "Alerts",
                lastMessagePreview: "Invite",
                unreadCount: 1,
                hasHighlight: true,
                kind: .room,
                membership: .invited,
                lastActivityAt: RoomListFixtures.now
            ),
            RoomSummary(
                id: "!general:matrix.org",
                name: "General",
                lastMessagePreview: "Unread only",
                unreadCount: 2,
                hasHighlight: false,
                kind: .room,
                membership: .joined,
                lastActivityAt: RoomListFixtures.now
            ),
            RoomSummary(
                id: "!design:matrix.org",
                name: "Design",
                lastMessagePreview: "Read",
                unreadCount: 0,
                hasHighlight: false,
                kind: .room,
                membership: .joined,
                lastActivityAt: RoomListFixtures.now
            )
        ]

        let sections = NotificationsInboxSections.make(from: rooms)

        XCTAssertEqual(sections.mentions.map(\.id), ["!project:matrix.org"])
        XCTAssertEqual(sections.invites.map(\.id), ["!alerts:matrix.org"])
        XCTAssertEqual(sections.unreadRooms.map(\.id), ["!general:matrix.org"])
        XCTAssertEqual(NotificationsInboxSections.notificationRooms(from: rooms).count, 3)
    }

    func testTabBadgeCountsUseRoomListState() {
        let badges = TabBadgeCounts.make(from: RoomListFixtures.small(), agentApprovalCount: 2)

        XCTAssertEqual(badges.notifications, 7)
        XCTAssertEqual(badges.rooms, 12)
    }

    func testMockRoomListStreamYieldsMultipleStates() async {
        let initialRooms = RoomListFixtures.small()
        let updatedRooms = [
            RoomSummary(
                id: "!new:matrix.org",
                name: "New room",
                lastMessagePreview: "Fresh activity",
                unreadCount: 2,
                hasHighlight: true,
                kind: .room,
                membership: .joined,
                lastActivityAt: RoomListFixtures.now
            )
        ] + initialRooms
        let service = MockRoomListService(state: .loaded(initialRooms))
        service.updateStates = [
            .loaded(initialRooms),
            .loaded(updatedRooms)
        ]

        var states: [RoomListState] = []
        for await state in service.roomUpdates() {
            states.append(state)
        }

        XCTAssertEqual(states.count, 2)
        guard case .loaded(let firstBatch) = states[0],
              case .loaded(let secondBatch) = states[1] else {
            XCTFail("Expected loaded room list states")
            return
        }
        XCTAssertEqual(firstBatch.count, initialRooms.count)
        XCTAssertEqual(secondBatch.first?.id, "!new:matrix.org")
    }

}
