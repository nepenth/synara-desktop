import Foundation

/// P4-S25 map of privacy-safe SharedCore room-list, invite, and space-parent
/// snapshots to product room rows.
///
/// Joined-room last-message text is not on the list DTO and stays empty.
/// Invite preview uses sender name / topic / reason from the invite
/// snapshot. Avatar is an `mxc://` URL. This is not iOS-on-engine and
/// not P4 acceptance.
enum SharedCoreRoomListRows {
    struct RoomRow {
        let roomId: String
        let name: String?
        let avatarUrl: String?
        let membership: String
        let isDirect: Bool
        let unreadCount: Int
        let highlightCount: Int
        let markedUnread: Bool
        let lastActivityTs: UInt64?
    }

    struct InviteRow {
        let roomId: String
        let roomName: String
        let roomTopic: String?
        let senderName: String
        let reason: String?
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
        return rooms.map { room in
            let invite = invitesByID[room.roomId]
            return RoomSummary(
                id: room.roomId,
                name: room.name ?? invite?.roomName ?? room.roomId,
                lastMessagePreview: preview(membership: room.membership, invite: invite),
                unreadCount: room.unreadCount,
                hasHighlight: room.highlightCount > 0 || room.markedUnread,
                kind: room.isDirect ? .directMessage : .room,
                membership: isInvited(room.membership) ? .invited : .joined,
                lastActivityAt: room.lastActivityTs.map {
                    Date(timeIntervalSince1970: TimeInterval($0) / 1000)
                } ?? Date(timeIntervalSince1970: 0),
                parentSpaces: parentSpaces(
                    parentIds: parentsByID[room.roomId] ?? [],
                    namesByID: namesByID
                ),
                avatarURL: room.avatarUrl.flatMap(URL.init(string:))
            )
        }
    }

    static func preview(membership: String, invite: InviteRow?) -> String {
        guard isInvited(membership), let invite else {
            return ""
        }
        let reason = invite.reason?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if reason.isEmpty == false {
            return reason
        }
        let sender = invite.senderName.trimmingCharacters(in: .whitespacesAndNewlines)
        if sender.isEmpty == false {
            return "Invited by \(sender)"
        }
        return invite.roomTopic?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
    }

    static func isInvited(_ membership: String) -> Bool {
        membership == "invite" || membership == "invited"
    }

    static func parentSpaces(parentIds: [String], namesByID: [String: String]) -> [SpaceSummary] {
        parentIds.filter { $0.isEmpty == false }.map { parentID in
            SpaceSummary(id: parentID, name: namesByID[parentID] ?? parentID)
        }
    }
}
