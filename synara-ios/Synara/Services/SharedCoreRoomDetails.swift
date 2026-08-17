import Foundation

/// P4-S22 map of privacy-safe SharedCore room snapshots to product details.
///
/// Uses existing list / members / power-level / join-rule / invite reads
/// only. Topic and encryption come from the invite snapshot when the
/// room is an invite; joined-room topic is not on the list DTO. Avatar
/// is an `mxc://` string. Notification mode stays leftover default.
/// This is not iOS-on-engine and not P4 acceptance.
enum SharedCoreRoomDetails {
    struct RoomRow {
        let roomId: String
        let name: String?
        let canonicalAlias: String?
        let avatarUrl: String?
    }

    struct MemberRow {
        let userId: String
        let membership: String
        let powerLevel: Int
    }

    static func details(
        roomID: String,
        ownUserID: String?,
        room: RoomRow?,
        members: [MemberRow],
        powerLevelsJSON: String?,
        joinRule: String?,
        topic: String?,
        isEncrypted: Bool,
        notificationMode: SynaraRoomNotificationMode = .allMessages
    ) -> RoomDetails {
        let power = powerSummary(
            ownUserID: ownUserID,
            members: members,
            powerLevelsJSON: powerLevelsJSON
        )
        let aliases = [room?.canonicalAlias]
            .compactMap { $0 }
            .filter { $0.isEmpty == false }
        return RoomDetails(
            roomID: roomID,
            name: room?.name ?? room?.canonicalAlias ?? roomID,
            topic: topic,
            aliases: aliases,
            isEncrypted: isEncrypted,
            isPublic: joinRule.map { $0 == "public" },
            memberCount: members.filter { $0.membership == "join" }.count,
            canInvite: power?.canInvite ?? false,
            canEditName: power?.canEditName ?? false,
            canEditTopic: power?.canEditTopic ?? false,
            canEditAvatar: power?.canEditAvatar ?? false,
            canEditAliases: canEditAliases(
                ownUserID: ownUserID,
                members: members,
                powerLevelsJSON: powerLevelsJSON
            ),
            powerLevels: power,
            notificationMode: notificationMode,
            avatarURL: room?.avatarUrl
        )
    }

    static func notificationMode(_ raw: String?) -> SynaraRoomNotificationMode {
        switch raw {
        case "mentions":
            return .mentionsOnly
        case "mute":
            return .mute
        default:
            return .allMessages
        }
    }

    static func powerSummary(
        ownUserID: String?,
        members: [MemberRow],
        powerLevelsJSON: String?
    ) -> RoomPowerLevelSummary? {
        guard let parsed = parsePowerLevels(powerLevelsJSON) else {
            return nil
        }
        let ownUserLevel = ownPowerLevel(
            ownUserID: ownUserID,
            members: members,
            users: parsed.users,
            usersDefault: parsed.usersDefault
        )
        let roomName = parsed.event("m.room.name") ?? parsed.stateDefault
        let roomTopic = parsed.event("m.room.topic") ?? parsed.stateDefault
        let roomAvatar = parsed.event("m.room.avatar") ?? parsed.stateDefault
        return RoomPowerLevelSummary(
            ownUserLevel: ownUserLevel,
            usersDefault: parsed.usersDefault,
            eventsDefault: parsed.eventsDefault,
            stateDefault: parsed.stateDefault,
            invite: parsed.invite,
            kick: parsed.kick,
            ban: parsed.ban,
            redact: parsed.redact,
            roomName: roomName,
            roomTopic: roomTopic,
            roomAvatar: roomAvatar,
            canInvite: ownUserLevel >= parsed.invite,
            canKick: ownUserLevel >= parsed.kick,
            canBan: ownUserLevel >= parsed.ban,
            canRedactOther: ownUserLevel >= parsed.redact,
            canEditName: ownUserLevel >= roomName,
            canEditTopic: ownUserLevel >= roomTopic,
            canEditAvatar: ownUserLevel >= roomAvatar
        )
    }

    static func canEditAliases(
        ownUserID: String?,
        members: [MemberRow],
        powerLevelsJSON: String?
    ) -> Bool {
        guard let parsed = parsePowerLevels(powerLevelsJSON) else {
            return false
        }
        let ownUserLevel = ownPowerLevel(
            ownUserID: ownUserID,
            members: members,
            users: parsed.users,
            usersDefault: parsed.usersDefault
        )
        let threshold = parsed.event("m.room.canonical_alias") ?? parsed.stateDefault
        return ownUserLevel >= threshold
    }

    private struct ParsedPowerLevels {
        var usersDefault: Int64 = 0
        var eventsDefault: Int64 = 0
        var stateDefault: Int64 = 50
        var invite: Int64 = 0
        var kick: Int64 = 50
        var ban: Int64 = 50
        var redact: Int64 = 50
        var events: [String: Int64] = [:]
        var users: [String: Int64] = [:]

        func event(_ type: String) -> Int64? {
            events[type]
        }
    }

    private static func parsePowerLevels(_ json: String?) -> ParsedPowerLevels? {
        guard let json, json.isEmpty == false,
              let data = json.data(using: .utf8),
              let object = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any]
        else {
            return nil
        }
        var parsed = ParsedPowerLevels()
        parsed.usersDefault = int64(object["users_default"]) ?? parsed.usersDefault
        parsed.eventsDefault = int64(object["events_default"]) ?? parsed.eventsDefault
        parsed.stateDefault = int64(object["state_default"]) ?? parsed.stateDefault
        parsed.invite = int64(object["invite"]) ?? parsed.invite
        parsed.kick = int64(object["kick"]) ?? parsed.kick
        parsed.ban = int64(object["ban"]) ?? parsed.ban
        parsed.redact = int64(object["redact"]) ?? parsed.redact
        if let events = object["events"] as? [String: Any] {
            parsed.events = events.compactMapValues(int64)
        }
        if let users = object["users"] as? [String: Any] {
            parsed.users = users.compactMapValues(int64)
        }
        return parsed
    }

    private static func ownPowerLevel(
        ownUserID: String?,
        members: [MemberRow],
        users: [String: Int64],
        usersDefault: Int64
    ) -> Int64 {
        if let ownUserID,
           let member = members.first(where: { $0.userId == ownUserID })
        {
            return Int64(member.powerLevel)
        }
        if let ownUserID, let override = users[ownUserID] {
            return override
        }
        return usersDefault
    }

    private static func int64(_ value: Any?) -> Int64? {
        if let number = value as? NSNumber {
            return number.int64Value
        }
        if let int = value as? Int {
            return Int64(int)
        }
        if let int = value as? Int64 {
            return int
        }
        return nil
    }
}
