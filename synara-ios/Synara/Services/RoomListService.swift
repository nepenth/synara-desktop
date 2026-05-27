import Foundation

struct RoomSummary: Identifiable, Equatable {
    enum RoomKind: Equatable {
        case room
        case directMessage
    }

    let id: String
    let name: String
    let lastMessagePreview: String
    let unreadCount: Int
    let hasHighlight: Bool
    let kind: RoomKind
    let lastActivityAt: Date
}

enum RoomListState: Equatable {
    case idle
    case loading
    case empty
    case failed(String)
    case loaded([RoomSummary])
}

protocol RoomListServicing: AnyObject {
    func loadRooms() async -> RoomListState
    func clearCache()
}

enum RoomListFixtures {
    private static let baseTimestamp: TimeInterval = 1_700_000_000

    static var now: Date {
        Date(timeIntervalSince1970: baseTimestamp)
    }

    static func small() -> [RoomSummary] {
        [
            RoomSummary(
                id: "!project:matrix.org",
                name: "Project",
                lastMessagePreview: "Build validated",
                unreadCount: 3,
                hasHighlight: true,
                kind: .room,
                lastActivityAt: now.addingTimeInterval(60)
            ),
            RoomSummary(
                id: "!alice:matrix.org",
                name: "Alice",
                lastMessagePreview: "See you soon",
                unreadCount: 0,
                hasHighlight: false,
                kind: .directMessage,
                lastActivityAt: now
            )
        ]
    }

    static func large(count: Int = 1_000) -> [RoomSummary] {
        var rooms: [RoomSummary] = []
        rooms.reserveCapacity(count)

        for index in 0..<count {
            let kind: RoomSummary.RoomKind = index % 5 == 0 ? .directMessage : .room
            let room = RoomSummary(
                id: "!room-\(index):matrix.org",
                name: "Room \(index)",
                lastMessagePreview: "Message \(index)",
                unreadCount: index % 7,
                hasHighlight: index % 23 == 0,
                kind: kind,
                lastActivityAt: now.addingTimeInterval(TimeInterval(-index))
            )
            rooms.append(room)
        }

        return rooms
    }

    static func sorted(_ rooms: [RoomSummary]) -> [RoomSummary] {
        rooms.sorted { lhs, rhs in
            if lhs.hasHighlight != rhs.hasHighlight {
                return lhs.hasHighlight
            }
            if lhs.unreadCount != rhs.unreadCount {
                return lhs.unreadCount > rhs.unreadCount
            }
            return lhs.lastActivityAt > rhs.lastActivityAt
        }
    }
}

final class PlaceholderRoomListService: RoomListServicing {
    private var cachedRooms: [RoomSummary] = RoomListFixtures.small()

    func loadRooms() async -> RoomListState {
        cachedRooms.isEmpty ? .empty : .loaded(RoomListFixtures.sorted(cachedRooms))
    }

    func clearCache() {
        cachedRooms = []
    }
}

final class MockRoomListService: RoomListServicing {
    var state: RoomListState
    private(set) var loadCallCount = 0
    private(set) var clearCallCount = 0

    init(state: RoomListState = .loaded(RoomListFixtures.small())) {
        self.state = state
    }

    func loadRooms() async -> RoomListState {
        loadCallCount += 1
        if case .loaded(let rooms) = state {
            return .loaded(RoomListFixtures.sorted(rooms))
        }
        return state
    }

    func clearCache() {
        clearCallCount += 1
        state = .empty
    }
}
