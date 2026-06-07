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

}
