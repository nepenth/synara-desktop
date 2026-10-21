import Foundation
import SynaraCore

/// P4-S25 map of privacy-safe SharedCore room-list, invite, and space-parent
/// snapshots to product room rows. P4-S26 adds unread lookup from that
/// mapped snapshot.
///
/// Joined-room last-message text comes from the privacy-safe list DTO
/// preview. Invite preview still prefers sender name / topic / reason
/// from the invite snapshot. Avatar is an `mxc://` URL. This is not
/// iOS-on-engine and not P4 acceptance.
///
/// Core's room-list snapshot is joined-only; pending invites exist only in
/// the invite snapshot. `rooms` appends one `.invited` row per invite whose
/// room is not already a list row, so the Rooms list and the Notifications
/// invites section can show and act on it.
enum SharedCoreRoomListRows {
    static func encryptionStatus(_ status: RoomEncryptionStatus) -> SynaraRoomEncryptionStatus {
        switch status {
        case .encrypted:
            return .encrypted
        case .notEncrypted:
            return .notEncrypted
        case .unknown:
            return .unknown
        }
    }

    struct RoomRow {
        let roomId: String
        let name: String?
        let avatarUrl: String?
        let membership: RoomMembershipDto
        let isDirect: Bool
        let unreadCount: Int
        let highlightCount: Int
        let markedUnread: Bool
        let lastActivityTs: UInt64?
        let lastMessagePreview: String?
        let lastMessageIsAgentApproval: Bool
    let isFavorite: Bool
    let isCall: Bool
    let hasActiveCall: Bool
    let activeCallParticipantCount: Int
    let directUserId: String?
    let encryptionStatus: SynaraRoomEncryptionStatus
    var isSpace: Bool = false
    var isLowPriority: Bool = false
    /// Core's agent-room rule (listed agent member, or approval/workflow
    /// content), shared with desktop.
    var isAgentRoom: Bool = false
    }

    struct InviteRow {
        let roomId: String
        let roomName: String
        let roomTopic: String?
        let senderName: String
        let reason: String?
        var isDirect: Bool = false
        var isEncrypted: Bool = false
        var inviteTs: UInt64? = nil
    }

    struct SpaceParentRow {
        let roomId: String
        let parentIds: [String]
    }

    static func rooms(
        rooms: [RoomRow],
        invites: [InviteRow],
        spaceParents: [SpaceParentRow]
    ) -> [RoomSummary] {
        let namesByID = Dictionary(
            uniqueKeysWithValues: rooms.map { ($0.roomId, $0.name ?? $0.roomId) }
        )
        let invitesByID = Dictionary(uniqueKeysWithValues: invites.map { ($0.roomId, $0) })
        let parentsByID = Dictionary(uniqueKeysWithValues: spaceParents.map { ($0.roomId, $0.parentIds) })
        let listedIDs = Set(rooms.map(\.roomId))
        let joinedRows = rooms.map { room in
            let invite = invitesByID[room.roomId]
            return RoomSummary(
                id: room.roomId,
                name: room.name ?? invite?.roomName ?? room.roomId,
                lastMessagePreview: preview(
                    membership: room.membership,
                    invite: invite,
                    lastMessagePreview: room.lastMessagePreview
                ),
                lastMessageIsAgentApproval: room.lastMessageIsAgentApproval,
                unreadCount: room.unreadCount,
                hasHighlight: room.highlightCount > 0,
                isMarkedUnread: room.markedUnread,
                kind: kind(isSpace: room.isSpace, isDirect: room.isDirect),
                membership: isInvited(room.membership) ? .invited : .joined,
                lastActivityAt: activityDate(millisecondsSince1970: room.lastActivityTs),
                parentSpaces: parentSpaces(
                    parentIds: parentsByID[room.roomId] ?? [],
                    namesByID: namesByID
                ),
                avatarURL: room.avatarUrl.flatMap(URL.init(string:)),
                hasAgentActivity: room.isAgentRoom,
                isFavorite: room.isFavorite,
                isLowPriority: room.isLowPriority,
                isCall: room.isCall,
                hasActiveCall: room.hasActiveCall,
                activeCallParticipantCount: room.activeCallParticipantCount,
                directUserId: room.directUserId,
                encryptionStatus: room.encryptionStatus
            )
        }
        let inviteRows = invites
            .filter { listedIDs.contains($0.roomId) == false }
            .map { invite in
                RoomSummary(
                    id: invite.roomId,
                    name: invite.roomName.isEmpty ? invite.roomId : invite.roomName,
                    lastMessagePreview: preview(
                        membership: .invite,
                        invite: invite,
                        lastMessagePreview: nil
                    ),
                    unreadCount: 0,
                    hasHighlight: false,
                    kind: invite.isDirect ? .directMessage : .room,
                    membership: .invited,
                    lastActivityAt: activityDate(millisecondsSince1970: invite.inviteTs),
                    parentSpaces: parentSpaces(
                        parentIds: parentsByID[invite.roomId] ?? [],
                        namesByID: namesByID
                    ),
                    // Core reports `false` when it could not learn the state
                    // of a room the user has not joined; keep that unknown.
                    encryptionStatus: invite.isEncrypted ? .encrypted : .unknown
                )
            }
        return joinedRows + inviteRows
    }

    /// Core reports `is_space` from the room's create event; a space is never
    /// a channel row.
    static func kind(isSpace: Bool, isDirect: Bool) -> RoomSummary.RoomKind {
        if isSpace {
            return .space
        }
        return isDirect ? .directMessage : .room
    }

    private static func activityDate(millisecondsSince1970 timestamp: UInt64?) -> Date {
        guard let timestamp, timestamp > 0 else {
            return .distantPast
        }
        return Date(timeIntervalSince1970: TimeInterval(timestamp) / 1000)
    }

    static func preview(
        membership: RoomMembershipDto,
        invite: InviteRow?,
        lastMessagePreview: String?
    ) -> String {
        if isInvited(membership), let invite {
            let reason = invite.reason?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            if reason.isEmpty == false {
                return reason
            }
            let sender = invite.senderName.trimmingCharacters(in: .whitespacesAndNewlines)
            if sender.isEmpty == false {
                return "Invited by \(sender)"
            }
            let topic = invite.roomTopic?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            if topic.isEmpty == false {
                return topic
            }
        }
        return lastMessagePreview?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
    }

    static func preview(membership: RoomMembershipDto, invite: InviteRow?) -> String {
        preview(membership: membership, invite: invite, lastMessagePreview: nil)
    }

    static func isInvited(_ membership: RoomMembershipDto) -> Bool {
        membership == .invite
    }

    static func parentSpaces(parentIds: [String], namesByID: [String: String]) -> [SpaceSummary] {
        parentIds.filter { $0.isEmpty == false }.map { parentID in
            SpaceSummary(id: parentID, name: namesByID[parentID] ?? parentID)
        }
    }

    static func hasUnreadMessages(
        unreadCount: Int,
        hasHighlight: Bool,
        isMarkedUnread: Bool = false
    ) -> Bool {
        unreadCount > 0 || hasHighlight || isMarkedUnread
    }
}

extension RoomMembershipDto {
    /// User-facing membership text.
    var displayName: String {
        switch self {
        case .join: return "Joined"
        case .invite: return "Invited"
        case .ban: return "Banned"
        case .leave: return "Left"
        case .knock: return "Knocking"
        }
    }

    /// Core's stable wire label (`join`, `invite`, ...), for Core inputs that
    /// still take the Matrix spelling.
    var wireLabel: String {
        switch self {
        case .invite: return "invite"
        case .join: return "join"
        case .knock: return "knock"
        case .leave: return "leave"
        case .ban: return "ban"
        }
    }
}
