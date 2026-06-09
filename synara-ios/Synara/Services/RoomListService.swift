import Foundation

struct RoomSummary: Identifiable, Equatable {
    enum RoomKind: Equatable {
        case room
        case directMessage
    }

    enum Membership: Equatable {
        case joined
        case invited
    }

    let id: String
    let name: String
    let lastMessagePreview: String
    let unreadCount: Int
    let hasHighlight: Bool
    let kind: RoomKind
    let membership: Membership
    let lastActivityAt: Date
    let parentSpaces: [SpaceSummary]

    init(
        id: String,
        name: String,
        lastMessagePreview: String,
        unreadCount: Int,
        hasHighlight: Bool,
        kind: RoomKind,
        membership: Membership,
        lastActivityAt: Date,
        parentSpaces: [SpaceSummary] = []
    ) {
        self.id = id
        self.name = name
        self.lastMessagePreview = lastMessagePreview
        self.unreadCount = unreadCount
        self.hasHighlight = hasHighlight
        self.kind = kind
        self.membership = membership
        self.lastActivityAt = lastActivityAt
        self.parentSpaces = parentSpaces
    }
}

struct SpaceSummary: Identifiable, Equatable, Hashable {
    let id: String
    let name: String
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
    func roomUpdates() -> AsyncStream<RoomListState>
    func clearCache()
}

extension RoomListServicing {
    func roomUpdates() -> AsyncStream<RoomListState> {
        AsyncStream { continuation in
            continuation.finish()
        }
    }
}

enum RoomMembershipError: LocalizedError, Equatable {
    case signedOut
    case failed

    var errorDescription: String? {
        switch self {
        case .signedOut:
            return "Sign in before changing room membership."
        case .failed:
            return "Could not update the room invite. Try again."
        }
    }
}

protocol RoomMembershipServicing: AnyObject {
    func acceptInvite(roomID: String) async throws
    func rejectInvite(roomID: String) async throws
}

enum RoomListFixtures {
    private static let baseTimestamp: TimeInterval = 1_700_000_000

    static var now: Date {
        Date(timeIntervalSince1970: baseTimestamp)
    }

    static func small() -> [RoomSummary] {
        [
            RoomSummary(
                id: "!general:matrix.org",
                name: "General",
                lastMessagePreview: "Kai: Project update looks good",
                unreadCount: 7,
                hasHighlight: false,
                kind: .room,
                membership: .joined,
                lastActivityAt: now.addingTimeInterval(120),
                parentSpaces: [SpaceSummary(id: "!workspace:matrix.org", name: "Workspace")]
            ),
            RoomSummary(
                id: "!project:matrix.org",
                name: "Product",
                lastMessagePreview: "Mina: Here's the latest spec",
                unreadCount: 3,
                hasHighlight: true,
                kind: .room,
                membership: .joined,
                lastActivityAt: now.addingTimeInterval(60),
                parentSpaces: [SpaceSummary(id: "!workspace:matrix.org", name: "Workspace")]
            ),
            RoomSummary(
                id: "!design:matrix.org",
                name: "Design",
                lastMessagePreview: "You: Figma file updated",
                unreadCount: 0,
                hasHighlight: false,
                kind: .room,
                membership: .joined,
                lastActivityAt: now.addingTimeInterval(-180),
                parentSpaces: [SpaceSummary(id: "!workspace:matrix.org", name: "Workspace")]
            ),
            RoomSummary(
                id: "!security:matrix.org",
                name: "Security",
                lastMessagePreview: "Ravi: Please review the audit",
                unreadCount: 2,
                hasHighlight: false,
                kind: .room,
                membership: .joined,
                lastActivityAt: now.addingTimeInterval(-780),
                parentSpaces: [SpaceSummary(id: "!ops:matrix.org", name: "Ops")]
            ),
            RoomSummary(
                id: "!agent-workflows:matrix.org",
                name: "Agent Workflows",
                lastMessagePreview: "Workflow run completed",
                unreadCount: 1,
                hasHighlight: false,
                kind: .room,
                membership: .joined,
                lastActivityAt: now.addingTimeInterval(-1_200),
                parentSpaces: [SpaceSummary(id: "!ops:matrix.org", name: "Ops")]
            ),
            RoomSummary(
                id: "!alice:matrix.org",
                name: "Alice",
                lastMessagePreview: "See you soon",
                unreadCount: 0,
                hasHighlight: false,
                kind: .directMessage,
                membership: .joined,
                lastActivityAt: now
            ),
            RoomSummary(
                id: "!mina:matrix.org",
                name: "Mina",
                lastMessagePreview: "You: Sounds good!",
                unreadCount: 1,
                hasHighlight: false,
                kind: .directMessage,
                membership: .joined,
                lastActivityAt: now.addingTimeInterval(-300)
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
                membership: .joined,
                lastActivityAt: now.addingTimeInterval(TimeInterval(-index)),
                parentSpaces: kind == .room && index % 4 == 0 ? [SpaceSummary(id: "!space-\(index % 3):matrix.org", name: "Space \(index % 3)")] : []
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

enum RoomListScopeFilter {
    enum Kind: Equatable {
        case all
        case unread
        case mentions
    }

    static func apply(_ filter: Kind, to rooms: [RoomSummary]) -> [RoomSummary] {
        switch filter {
        case .all:
            return rooms
        case .unread:
            return rooms.filter { $0.unreadCount > 0 }
        case .mentions:
            return rooms.filter(\.hasHighlight)
        }
    }

    static func filteredRooms(
        from rooms: [RoomSummary],
        filter: Kind,
        selectedSpaceID: String?,
        searchQuery: String
    ) -> [RoomSummary] {
        let query = searchQuery.trimmingCharacters(in: .whitespacesAndNewlines)
        let invitedRooms = rooms.filter { $0.membership == .invited }
        var scopedRooms = rooms.filter { $0.membership != .invited }

        if let selectedSpaceID {
            scopedRooms = scopedRooms.filter { room in
                room.parentSpaces.contains(where: { $0.id == selectedSpaceID })
            }
        }

        scopedRooms = apply(filter, to: scopedRooms)
        var filtered = RoomListSearchFilter.mergeInvitedRooms(invitedRooms, into: scopedRooms)

        guard query.isEmpty == false else {
            return filtered
        }

        return RoomListSearchFilter.applySearchQuery(query, to: filtered, invitedRooms: invitedRooms)
    }
}

enum RoomListSearchFilter {
    static func roomMatchesQuery(_ room: RoomSummary, query: String) -> Bool {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false else {
            return true
        }

        return room.name.localizedCaseInsensitiveContains(trimmed)
            || room.lastMessagePreview.localizedCaseInsensitiveContains(trimmed)
    }

    static func mergeInvitedRooms(_ invitedRooms: [RoomSummary], into rooms: [RoomSummary]) -> [RoomSummary] {
        guard invitedRooms.isEmpty == false else {
            return rooms
        }

        let existingIDs = Set(rooms.map(\.id))
        let missingInvites = invitedRooms.filter { existingIDs.contains($0.id) == false }
        return missingInvites + rooms
    }

    static func applySearchQuery(
        _ query: String,
        to rooms: [RoomSummary],
        invitedRooms: [RoomSummary]
    ) -> [RoomSummary] {
        let trimmed = query.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false else {
            return rooms
        }

        let filtered = rooms.filter { roomMatchesQuery($0, query: trimmed) }
        let matchingInvites = invitedRooms.filter { roomMatchesQuery($0, query: trimmed) }
        return mergeInvitedRooms(matchingInvites, into: filtered.filter { $0.membership != .invited })
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

final class MockRoomMembershipService: RoomMembershipServicing {
    private(set) var acceptedRoomIDs: [String] = []
    private(set) var rejectedRoomIDs: [String] = []

    func acceptInvite(roomID: String) async throws {
        acceptedRoomIDs.append(roomID)
    }

    func rejectInvite(roomID: String) async throws {
        rejectedRoomIDs.append(roomID)
    }
}

final class MockRoomListService: RoomListServicing {
    var state: RoomListState
    var updateStates: [RoomListState] = []
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

    func roomUpdates() -> AsyncStream<RoomListState> {
        AsyncStream { continuation in
            let task = Task {
                let states: [RoomListState]
                if updateStates.isEmpty {
                    states = [await loadRooms()]
                } else {
                    states = updateStates
                }

                for state in states {
                    continuation.yield(state)
                }
                continuation.finish()
            }

            continuation.onTermination = { _ in
                task.cancel()
            }
        }
    }

    func clearCache() {
        clearCallCount += 1
        state = .empty
    }
}

final class MockInviteTransitionService: RoomListServicing, RoomMembershipServicing {
    private var rooms: [RoomSummary]
    private(set) var acceptedRoomIDs: [String] = []
    private(set) var rejectedRoomIDs: [String] = []

    init(rooms: [RoomSummary] = [
        RoomSummary(
            id: "!alerts:matrix.org",
            name: "Alerts",
            lastMessagePreview: "Invited to room",
            unreadCount: 1,
            hasHighlight: true,
            kind: .room,
            membership: .invited,
            lastActivityAt: RoomListFixtures.now
        )
    ]) {
        self.rooms = rooms
    }

    func loadRooms() async -> RoomListState {
        rooms.isEmpty ? .empty : .loaded(RoomListFixtures.sorted(rooms))
    }

    func clearCache() {
        rooms = []
    }

    func acceptInvite(roomID: String) async throws {
        acceptedRoomIDs.append(roomID)
        rooms = rooms.map { room in
            guard room.id == roomID else {
                return room
            }

            return RoomSummary(
                id: room.id,
                name: room.name,
                lastMessagePreview: "Joined room",
                unreadCount: room.unreadCount,
                hasHighlight: room.hasHighlight,
                kind: room.kind,
                membership: .joined,
                lastActivityAt: Date(),
                parentSpaces: room.parentSpaces
            )
        }
    }

    func rejectInvite(roomID: String) async throws {
        rejectedRoomIDs.append(roomID)
        rooms.removeAll { $0.id == roomID }
    }
}
