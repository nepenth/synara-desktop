import Foundation
import SynaraCore

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
    let avatarURL: URL?
    let hasAgentActivity: Bool
    let latestAgentCard: SynaraAgentCard?
    let latestAgentCardEventID: String?
    let pendingAgentApprovals: [PendingAgentCardRef]

    init(
        id: String,
        name: String,
        lastMessagePreview: String,
        unreadCount: Int,
        hasHighlight: Bool,
        kind: RoomKind,
        membership: Membership,
        lastActivityAt: Date,
        parentSpaces: [SpaceSummary] = [],
        avatarURL: URL? = nil,
        hasAgentActivity: Bool = false,
        latestAgentCard: SynaraAgentCard? = nil,
        latestAgentCardEventID: String? = nil,
        pendingAgentApprovals: [PendingAgentCardRef] = []
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
        self.avatarURL = avatarURL
        self.hasAgentActivity = hasAgentActivity
        self.latestAgentCard = latestAgentCard
        self.latestAgentCardEventID = latestAgentCardEventID
        self.pendingAgentApprovals = pendingAgentApprovals
    }

    var isAgentRoom: Bool {
        hasAgentActivity || pendingAgentApprovals.isEmpty == false || latestAgentCard != nil
    }

    var requiresAgentApproval: Bool {
        latestAgentCard?.requiresUserApproval == true
    }

    var primaryParentSpace: SpaceSummary? {
        parentSpaces.sorted { $0.name.localizedCaseInsensitiveCompare($1.name) == .orderedAscending }.first
    }

    var parentSpaceID: String? {
        primaryParentSpace?.id
    }

    var spaceName: String? {
        primaryParentSpace?.name
    }
}

struct SpaceSummary: Identifiable, Equatable, Hashable {
    let id: String
    let name: String
}

enum RoomListRecentActivity {
    static let window: TimeInterval = 86400

    struct Partition: Equatable {
        let recent: [RoomSummary]
        let remaining: [RoomSummary]
    }

    static func recent(from rooms: [RoomSummary], referenceDate: Date = Date()) -> [RoomSummary] {
        partition(from: rooms, referenceDate: referenceDate).recent
    }

    static func partition(from rooms: [RoomSummary], referenceDate: Date = Date()) -> Partition {
        let cutoff = referenceDate.addingTimeInterval(-window)
        let recent = rooms
            .filter { $0.lastActivityAt > cutoff }
            .sorted {
                if $0.lastActivityAt != $1.lastActivityAt {
                    return $0.lastActivityAt > $1.lastActivityAt
                }
                let nameOrder = $0.name.localizedCaseInsensitiveCompare($1.name)
                if nameOrder != .orderedSame {
                    return nameOrder == .orderedAscending
                }
                return $0.id < $1.id
            }
        let recentIDs = Set(recent.map(\.id))
        return Partition(
            recent: recent,
            remaining: rooms.filter { recentIDs.contains($0.id) == false }
        )
    }

    static func nextExpirationDate(from rooms: [RoomSummary], referenceDate: Date = Date()) -> Date? {
        rooms
            .map { $0.lastActivityAt.addingTimeInterval(window) }
            .filter { $0 > referenceDate }
            .min()
    }
}

enum RoomActivityTimestamp {
    static func resolve(latest: Date?, previous: Date?) -> Date {
        switch (latest, previous) {
        case let (latest?, previous?):
            return max(latest, previous)
        case let (latest?, nil):
            return latest
        case let (nil, previous?):
            return previous
        case (nil, nil):
            return .distantPast
        }
    }
}

enum RoomActivityEventKind {
    case messageLike
    case localEcho
    case invite
    case receipt
    case typing
    case state
}

enum RoomActivityQualification {
    static func qualifies(_ kind: RoomActivityEventKind) -> Bool {
        switch kind {
        case .messageLike, .localEcho, .invite:
            return true
        case .receipt, .typing, .state:
            return false
        }
    }
}

/// Recovers a room's newest message-like activity when the SDK's latest-event
/// summary happens to be a state event. Recovery is deliberately cold-start
/// only and bounded to one small SDK timeline page per affected room.
enum RoomActivityRecoveryPolicy {
    static let maximumTimelineEvents = 24

    struct Candidate: Equatable {
        let timestamp: Date
        let kind: RoomActivityEventKind
    }

    static func shouldRecover(
        latestRequiresRecovery: Bool,
        previousActivityAt: Date?
    ) -> Bool {
        let previousState: SynaraCore.RoomActivityPreviousState
        if previousActivityAt == nil || previousActivityAt == .distantPast {
            previousState = .missing
        } else {
            previousState = .known
        }
        return SynaraCore.roomActivityRecoveryRequired(
            latestRequiresRecovery: latestRequiresRecovery,
            previousState: previousState
        )
    }

    static func newestQualifyingTimestamp(from candidates: [Candidate]) -> Date? {
        candidates
            .suffix(maximumTimelineEvents)
            .reversed()
            .first { RoomActivityQualification.qualifies($0.kind) }?
            .timestamp
    }
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
    func roomDisplayName(roomID: String) -> String?
    func isAgentRoom(roomID: String) -> Bool
    func hasUnreadMessages(roomID: String) -> Bool
    func clearCache()
}

extension RoomListServicing {
    func roomUpdates() -> AsyncStream<RoomListState> {
        AsyncStream { continuation in
            continuation.finish()
        }
    }

    func roomDisplayName(roomID: String) -> String? {
        nil
    }

    func isAgentRoom(roomID: String) -> Bool {
        false
    }

    func hasUnreadMessages(roomID: String) -> Bool {
        false
    }
}

enum RoomDisplayNameLookup {
    static func names(from state: RoomListState) -> [String: String] {
        guard case .loaded(let rooms) = state else {
            return [:]
        }

        return Dictionary(uniqueKeysWithValues: rooms.map { ($0.id, $0.name) })
    }

    static func resolve(roomID: String, names: [String: String]) -> String {
        guard roomID.isEmpty == false else {
            return "Unknown room"
        }

        return names[roomID] ?? roomID
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

    static func pendingAgentApprovalCard(title: String = "Deploy approval required") -> SynaraAgentCard {
        try! SynaraAgentCard(
            title: title,
            status: "pending",
            summary: "Review the proposed deployment before it runs.",
            actions: [
                try! SynaraAgentCardAction(
                    id: "approve-deploy",
                    title: "Approve",
                    kind: "approve",
                    prompt: "approve"
                ),
                try! SynaraAgentCardAction(
                    id: "reject-deploy",
                    title: "Reject",
                    kind: "reject",
                    prompt: "reject"
                )
            ]
        )
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
                lastMessagePreview: "Agent: Deploy approval required",
                unreadCount: 1,
                hasHighlight: false,
                kind: .room,
                membership: .joined,
                lastActivityAt: now.addingTimeInterval(-1_200),
                parentSpaces: [SpaceSummary(id: "!ops:matrix.org", name: "Ops")],
                hasAgentActivity: true,
                latestAgentCard: RoomListFixtures.pendingAgentApprovalCard(),
                latestAgentCardEventID: "$agent-deploy-approval:matrix.org"
            ),
            RoomSummary(
                id: "!security-agent:matrix.org",
                name: "Security Agent",
                lastMessagePreview: "Agent: Access review required",
                unreadCount: 1,
                hasHighlight: false,
                kind: .room,
                membership: .joined,
                lastActivityAt: now.addingTimeInterval(-900),
                parentSpaces: [SpaceSummary(id: "!ops:matrix.org", name: "Ops")],
                hasAgentActivity: true,
                latestAgentCard: RoomListFixtures.pendingAgentApprovalCard(title: "Access review required"),
                latestAgentCardEventID: "$agent-access-approval:matrix.org"
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

struct BadgeUnreadSource: Equatable {
    let total: Int
    let highlight: Int?

    init(total: Int, highlight: Int? = nil) {
        self.total = total
        self.highlight = highlight
    }
}

struct NotificationSummaryInput: Equatable {
    let unreadCounts: [BadgeUnreadSource]
    let laterActiveCount: Int
    let inviteCount: Int
    let agentApprovalCount: Int

    init(
        unreadCounts: [BadgeUnreadSource],
        laterActiveCount: Int = 0,
        inviteCount: Int = 0,
        agentApprovalCount: Int = 0
    ) {
        self.unreadCounts = unreadCounts
        self.laterActiveCount = laterActiveCount
        self.inviteCount = inviteCount
        self.agentApprovalCount = agentApprovalCount
    }
}

enum NotificationBadgeSummary {
    private static func clampCount(_ value: Int) -> Int {
        max(0, value)
    }

    static func summarizeNotifications(_ input: NotificationSummaryInput) -> SynaraNotificationSummary? {
        let laterCount = clampCount(input.laterActiveCount)
        let invites = clampCount(input.inviteCount)
        let agentApprovals = clampCount(input.agentApprovalCount)
        var highlightCount = 0
        var unreadCount = 0

        for unread in input.unreadCounts {
            if let highlight = unread.highlight {
                highlightCount += clampCount(highlight)
            } else {
                unreadCount += clampCount(unread.total)
            }
        }

        return try? SynaraNotificationSummary(
            appBadgeCount: laterCount + highlightCount + unreadCount,
            inboxBadgeCount: laterCount + invites + agentApprovals,
            laterActiveCount: laterCount,
            inviteCount: invites,
            agentApprovalCount: agentApprovals,
            highlightCount: highlightCount,
            unreadCount: unreadCount
        )
    }

    static func unreadSources(from rooms: [RoomSummary]) -> [BadgeUnreadSource] {
        rooms.map { room in
            if room.hasHighlight {
                return BadgeUnreadSource(total: room.unreadCount, highlight: 1)
            }
            return BadgeUnreadSource(total: room.unreadCount)
        }
    }

    static func inviteCount(from rooms: [RoomSummary]) -> Int {
        rooms.filter { $0.membership == .invited }.count
    }

    static func roomsTabBadgeCount(from rooms: [RoomSummary]) -> Int {
        summarizeNotifications(
            NotificationSummaryInput(unreadCounts: unreadSources(from: rooms))
        )?.appBadgeCount ?? 0
    }

    static func notificationsTabBadgeCount(from rooms: [RoomSummary]) -> Int {
        let sections = NotificationsInboxSections.make(from: rooms)
        let agentPendingCount = AgentPendingInbox.pendingApprovals(from: rooms).count
        return sections.mentions.count
            + sections.invites.count
            + sections.unreadRooms.count
            + agentPendingCount
    }
}

struct PendingAgentCardRef: Equatable {
    let eventID: String
    let card: SynaraAgentCard
    let timestamp: Date
}

struct AgentPendingApprovalItem: Identifiable, Equatable {
    let id: String
    let eventID: String
    let roomID: String
    let roomName: String
    let title: String
    let summary: String?
    let status: String?
    let avatarURL: URL?
    let lastActivityAt: Date

    var roomSummary: RoomSummary {
        RoomSummary(
            id: roomID,
            name: roomName,
            lastMessagePreview: summary ?? title,
            unreadCount: 0,
            hasHighlight: false,
            kind: .room,
            membership: .joined,
            lastActivityAt: lastActivityAt,
            avatarURL: avatarURL,
            hasAgentActivity: true,
            latestAgentCard: nil
        )
    }
}

enum AgentPendingInbox {
    static func pendingApprovals(from rooms: [RoomSummary]) -> [AgentPendingApprovalItem] {
        rooms.flatMap { room in
            pendingCardRefs(for: room).compactMap { ref in
                guard ref.card.requiresUserApproval else {
                    return nil
                }

                return AgentPendingApprovalItem(
                    id: "\(room.id)-\(ref.eventID)",
                    eventID: ref.eventID,
                    roomID: room.id,
                    roomName: room.name,
                    title: ref.card.title,
                    summary: ref.card.summary,
                    status: ref.card.status,
                    avatarURL: room.avatarURL,
                    lastActivityAt: ref.timestamp
                )
            }
        }
        .sorted { $0.lastActivityAt > $1.lastActivityAt }
    }

    private static func pendingCardRefs(for room: RoomSummary) -> [PendingAgentCardRef] {
        if room.pendingAgentApprovals.isEmpty == false {
            return room.pendingAgentApprovals
        }

        guard let card = room.latestAgentCard else {
            return []
        }

        let eventID = room.latestAgentCardEventID ?? room.id
        return [PendingAgentCardRef(eventID: eventID, card: card, timestamp: room.lastActivityAt)]
    }
}

struct NotificationsInboxSections: Equatable {
    let mentions: [RoomSummary]
    let invites: [RoomSummary]
    let unreadRooms: [RoomSummary]

    var hasRoomSections: Bool {
        mentions.isEmpty == false || invites.isEmpty == false || unreadRooms.isEmpty == false
    }

    static func isCaughtUp(sections: NotificationsInboxSections, agentPendingCount: Int) -> Bool {
        sections.hasRoomSections == false && agentPendingCount == 0
    }

    static func notificationRooms(from rooms: [RoomSummary]) -> [RoomSummary] {
        rooms.filter { $0.unreadCount > 0 || $0.hasHighlight || $0.membership == .invited }
    }

    static func make(from rooms: [RoomSummary]) -> NotificationsInboxSections {
        let invites = rooms.filter { $0.membership == .invited }
        let inviteIDs = Set(invites.map(\.id))
        let mentions = rooms.filter { $0.hasHighlight && inviteIDs.contains($0.id) == false }
        let mentionIDs = Set(mentions.map(\.id))
        let unreadRooms = rooms.filter { room in
            guard inviteIDs.contains(room.id) == false, mentionIDs.contains(room.id) == false else {
                return false
            }
            return room.unreadCount > 0
        }

        return NotificationsInboxSections(
            mentions: mentions,
            invites: invites,
            unreadRooms: unreadRooms
        )
    }
}

struct TabBadgeCounts: Equatable {
    var notifications: Int = 0
    var rooms: Int = 0

    static func make(from rooms: [RoomSummary]) -> TabBadgeCounts {
        TabBadgeCounts(
            notifications: NotificationBadgeSummary.notificationsTabBadgeCount(from: rooms),
            rooms: NotificationBadgeSummary.roomsTabBadgeCount(from: rooms)
        )
    }
}

struct SpaceChannelGroup: Identifiable, Equatable {
    let space: SpaceSummary
    let rooms: [RoomSummary]

    var id: String { space.id }
}

enum RoomListSpaceGrouping {
    static func unreadCountsBySpaceID(from rooms: [RoomSummary]) -> [String: Int] {
        var counts: [String: Int] = [:]

        for room in rooms where room.membership != .invited {
            for space in room.parentSpaces {
                counts[space.id, default: 0] += room.unreadCount
            }
        }

        return counts
    }

    static func spaceChannelGroups(from channelRooms: [RoomSummary]) -> [SpaceChannelGroup] {
        var groupedRooms: [String: [RoomSummary]] = [:]
        var spacesByID: [String: SpaceSummary] = [:]

        for room in channelRooms {
            guard let space = room.primaryParentSpace else {
                continue
            }

            groupedRooms[space.id, default: []].append(room)
            spacesByID[space.id] = space
        }

        return groupedRooms
            .compactMap { spaceID, rooms in
                guard let space = spacesByID[spaceID] else {
                    return nil
                }

                return SpaceChannelGroup(space: space, rooms: rooms)
            }
            .sorted { $0.space.name.localizedCaseInsensitiveCompare($1.space.name) == .orderedAscending }
    }

    static func ungroupedChannelRooms(from channelRooms: [RoomSummary]) -> [RoomSummary] {
        channelRooms.filter { $0.primaryParentSpace == nil }
    }

    static func selectedSpaceName(
        in spaces: [SpaceSummary],
        selectedSpaceID: String?
    ) -> String? {
        guard let selectedSpaceID else {
            return nil
        }

        return spaces.first(where: { $0.id == selectedSpaceID })?.name
    }
}

enum RoomListScopeFilter {
    enum Kind: Equatable {
        case all
        case unread
        case mentions
        case agents
    }

    static func apply(_ filter: Kind, to rooms: [RoomSummary]) -> [RoomSummary] {
        switch filter {
        case .all:
            return rooms
        case .unread:
            return rooms.filter { $0.unreadCount > 0 }
        case .mentions:
            return rooms.filter(\.hasHighlight)
        case .agents:
            return rooms.filter(\.isAgentRoom)
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
        let filtered = RoomListSearchFilter.mergeInvitedRooms(invitedRooms, into: scopedRooms)

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

    func roomDisplayName(roomID: String) -> String? {
        cachedRooms.first { $0.id == roomID }?.name
    }

    func isAgentRoom(roomID: String) -> Bool {
        cachedRooms.first { $0.id == roomID }?.isAgentRoom ?? false
    }

    func hasUnreadMessages(roomID: String) -> Bool {
        cachedRooms.first { $0.id == roomID }?.unreadCount ?? 0 > 0
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

    func roomDisplayName(roomID: String) -> String? {
        guard case .loaded(let rooms) = state else {
            return nil
        }

        return rooms.first { $0.id == roomID }?.name
    }

    func isAgentRoom(roomID: String) -> Bool {
        guard case .loaded(let rooms) = state else {
            return false
        }

        return rooms.first { $0.id == roomID }?.isAgentRoom ?? false
    }

    func hasUnreadMessages(roomID: String) -> Bool {
        guard case .loaded(let rooms) = state else {
            return false
        }
        return rooms.first { $0.id == roomID }?.unreadCount ?? 0 > 0
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

    func roomDisplayName(roomID: String) -> String? {
        rooms.first { $0.id == roomID }?.name
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
                parentSpaces: room.parentSpaces,
                avatarURL: room.avatarURL
            )
        }
    }

    func rejectInvite(roomID: String) async throws {
        rejectedRoomIDs.append(roomID)
        rooms.removeAll { $0.id == roomID }
    }
}
