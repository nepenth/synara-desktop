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

final class MatrixRoomListService: RoomListServicing {
    private let sessionStore: AppSessionStore
    private let httpClient: AuthHTTPClient
    private let jsonDecoder: JSONDecoder
    private var cachedRooms: [RoomSummary] = []

    init(
        sessionStore: AppSessionStore,
        httpClient: AuthHTTPClient = URLSession.shared,
        jsonDecoder: JSONDecoder = JSONDecoder()
    ) {
        self.sessionStore = sessionStore
        self.httpClient = httpClient
        self.jsonDecoder = jsonDecoder
    }

    func loadRooms() async -> RoomListState {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .empty
        }

        do {
            var request = URLRequest(url: matrixSyncURL(for: session.homeserverURL))
            request.httpMethod = "GET"
            request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")

            let (data, response) = try await httpClient.data(for: request)
            guard let httpResponse = response as? HTTPURLResponse else {
                return .failed("Could not reach the homeserver. Try again.")
            }

            guard httpResponse.statusCode == 200 else {
                return .failed("Could not load rooms. Try again.")
            }

            let sync = try jsonDecoder.decode(MatrixSyncResponse.self, from: data)
            cachedRooms = RoomListFixtures.sorted(mapRooms(from: sync))
            return cachedRooms.isEmpty ? .empty : .loaded(cachedRooms)
        } catch {
            return .failed("Could not load rooms. Try again.")
        }
    }

    func clearCache() {
        cachedRooms = []
    }

    private func matrixSyncURL(for homeserverURL: URL) -> URL {
        var url = homeserverURL
        url.appendPathComponent("_matrix")
        url.appendPathComponent("client")
        url.appendPathComponent("v3")
        url.appendPathComponent("sync")

        var components = URLComponents(url: url, resolvingAgainstBaseURL: false)
        components?.queryItems = [
            URLQueryItem(name: "timeout", value: "0")
        ]

        return components?.url ?? url
    }

    private func mapRooms(from sync: MatrixSyncResponse) -> [RoomSummary] {
        let joinedRooms: [RoomSummary] = sync.rooms?.join.map { roomID, joinedRoom in
            let stateEvents = joinedRoom.state?.events ?? []
            let timelineEvents = joinedRoom.timeline?.events ?? []
            let latestMessage = timelineEvents.reversed().first(where: { $0.type == "m.room.message" })
            let lastActivityAt = latestActivityDate(from: timelineEvents + stateEvents)

            return RoomSummary(
                id: roomID,
                name: roomName(from: stateEvents, fallback: roomID),
                lastMessagePreview: latestMessage?.content.body ?? "No recent messages",
                unreadCount: joinedRoom.unreadNotifications?.notificationCount ?? 0,
                hasHighlight: (joinedRoom.unreadNotifications?.highlightCount ?? 0) > 0,
                kind: .room,
                lastActivityAt: lastActivityAt
            )
        } ?? []

        let invitedRooms: [RoomSummary] = sync.rooms?.invite.map { roomID, invitedRoom in
            let inviteEvents = invitedRoom.inviteState?.events ?? []
            return RoomSummary(
                id: roomID,
                name: roomName(from: inviteEvents, fallback: roomID),
                lastMessagePreview: "Invited to room",
                unreadCount: 1,
                hasHighlight: true,
                kind: .room,
                lastActivityAt: latestActivityDate(from: inviteEvents)
            )
        } ?? []

        return joinedRooms + invitedRooms
    }

    private func roomName(from events: [MatrixSyncEvent], fallback: String) -> String {
        if let name = events.last(where: { $0.type == "m.room.name" })?.content.name,
           name.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false {
            return name
        }

        if let alias = events.last(where: { $0.type == "m.room.canonical_alias" })?.content.alias,
           alias.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false {
            return alias
        }

        return fallback
    }

    private func latestActivityDate(from events: [MatrixSyncEvent]) -> Date {
        let timestamp = events.compactMap(\.originServerTimestamp).max() ?? 0
        guard timestamp > 0 else {
            return .distantPast
        }

        return Date(timeIntervalSince1970: TimeInterval(timestamp) / 1_000)
    }
}

private struct MatrixSyncResponse: Decodable {
    let rooms: MatrixSyncRooms?
}

private struct MatrixSyncRooms: Decodable {
    let join: [String: MatrixJoinedRoom]
    let invite: [String: MatrixInvitedRoom]

    enum CodingKeys: String, CodingKey {
        case join
        case invite
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        join = try container.decodeIfPresent([String: MatrixJoinedRoom].self, forKey: .join) ?? [:]
        invite = try container.decodeIfPresent([String: MatrixInvitedRoom].self, forKey: .invite) ?? [:]
    }
}

private struct MatrixJoinedRoom: Decodable {
    let state: MatrixEventBatch?
    let timeline: MatrixEventBatch?
    let unreadNotifications: MatrixUnreadNotifications?

    enum CodingKeys: String, CodingKey {
        case state
        case timeline
        case unreadNotifications = "unread_notifications"
    }
}

private struct MatrixInvitedRoom: Decodable {
    let inviteState: MatrixEventBatch?

    enum CodingKeys: String, CodingKey {
        case inviteState = "invite_state"
    }
}

private struct MatrixEventBatch: Decodable {
    let events: [MatrixSyncEvent]
}

private struct MatrixSyncEvent: Decodable {
    let type: String
    let originServerTimestamp: Int?
    let content: MatrixEventContent

    enum CodingKeys: String, CodingKey {
        case type
        case originServerTimestamp = "origin_server_ts"
        case content
    }
}

private struct MatrixEventContent: Decodable {
    let name: String?
    let alias: String?
    let body: String?
}

private struct MatrixUnreadNotifications: Decodable {
    let notificationCount: Int?
    let highlightCount: Int?

    enum CodingKeys: String, CodingKey {
        case notificationCount = "notification_count"
        case highlightCount = "highlight_count"
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
