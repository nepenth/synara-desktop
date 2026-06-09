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
