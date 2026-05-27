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
    static let now = Date(timeIntervalSince1970: 1_700_000_000)

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
        (0..<count).map { index in
            RoomSummary(
                id: "!room-\(index):matrix.org",
                name: "Room \(index)",
                lastMessagePreview: "Message \(index)",
                unreadCount: index % 7,
                hasHighlight: index % 23 == 0,
                kind: index % 5 == 0 ? .directMessage : .room,
                lastActivityAt: now.addingTimeInterval(TimeInterval(-index))
            )
        }
    }

    static func sorted(_ rooms: [RoomSummary]) -> [RoomSummary] {
        rooms.sorted {
            if $0.hasHighlight != $1.hasHighlight {
                return $0.hasHighlight
            }
            if $0.unreadCount != $1.unreadCount {
                return $0.unreadCount > $1.unreadCount
            }
            return $0.lastActivityAt > $1.lastActivityAt
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
