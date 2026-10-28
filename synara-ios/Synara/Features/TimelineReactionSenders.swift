import Foundation

/// Names and avatars for the people who reacted to a message. Core sends only
/// user ids with a reaction, so names come from what the app already knows:
/// the profiles Core resolved on loaded timeline rows, then the room's member
/// list, then the Matrix ID's localpart.
struct TimelineReactorDirectory: Equatable {
    struct Profile: Equatable {
        var displayName: String?
        var avatarURL: URL?
    }

    private(set) var profiles: [String: Profile] = [:]

    init() {}

    /// Later rows win, so a renamed sender reads by their newest name; a row
    /// without a profile name never replaces one that has it.
    init(items: [TimelineItem]) {
        for item in items where item.senderID.hasPrefix("@") {
            let profileName = Self.nonEmpty(item.senderProfileDisplayName)
            var profile = profiles[item.senderID] ?? Profile()
            if let profileName {
                profile.displayName = profileName
            } else if profile.displayName == nil {
                // The fixture names and localpart fallback the row header uses.
                profile.displayName = item.senderDisplayName
            }
            if let avatarURL = item.senderAvatarURL {
                profile.avatarURL = avatarURL
            }
            profiles[item.senderID] = profile
        }
    }

    /// Fills in members who have not posted in the loaded window, and
    /// replaces a localpart fallback with the member's display name.
    mutating func merge(members: [RoomMemberSummary]) {
        for member in members {
            guard let name = Self.nonEmpty(member.displayName) else { continue }
            var profile = profiles[member.userID] ?? Profile()
            if profile.displayName == nil || profile.displayName == Self.localpart(of: member.userID) {
                profile.displayName = name
            }
            profiles[member.userID] = profile
        }
    }

    func displayName(for userID: String) -> String {
        profiles[userID]?.displayName ?? Self.localpart(of: userID)
    }

    func avatarURL(for userID: String) -> URL? {
        profiles[userID]?.avatarURL
    }

    /// Display names for this item's reaction senders only, so a timeline
    /// row carries just what its pills read.
    func names(for item: TimelineItem) -> [String: String] {
        var names: [String: String] = [:]
        for userID in item.reactionSenders.values.joined() where names[userID] == nil {
            names[userID] = displayName(for: userID)
        }
        return names
    }

    static func localpart(of userID: String) -> String {
        RoomTypingPresentation.displayName(for: userID)
    }

    private static func nonEmpty(_ value: String?) -> String? {
        guard let trimmed = value?.trimmingCharacters(in: .whitespacesAndNewlines), trimmed.isEmpty == false else {
            return nil
        }
        return trimmed
    }
}

/// One person in the Reactions sheet.
struct TimelineReactor: Identifiable, Equatable {
    let userID: String
    let displayName: String
    let avatarURL: URL?
    let isCurrentUser: Bool

    var id: String { userID }

    /// "You" for the signed-in user, else their display name.
    var title: String {
        isCurrentUser ? String(localized: "You") : displayName
    }
}

/// One emoji tab in the Reactions sheet.
struct TimelineReactionGroup: Identifiable, Equatable {
    let key: String
    let count: Int
    let reactors: [TimelineReactor]

    var id: String { key }

    /// People Core counted but did not name, shown as "and N more".
    var unnamedCount: Int {
        max(0, count - reactors.count)
    }
}

enum TimelineReactionSenders {
    /// One group per reaction, in the pills' order. The user's own reaction
    /// leads its list; everyone else keeps Core's order.
    static func groups(
        for item: TimelineItem,
        currentUserID: String,
        directory: TimelineReactorDirectory
    ) -> [TimelineReactionGroup] {
        item.reactions.keys.sorted().compactMap { key in
            let count = item.reactions[key] ?? 0
            let senders = item.reactionSenders[key] ?? []
            var reactors = senders.map { userID in
                TimelineReactor(
                    userID: userID,
                    displayName: directory.displayName(for: userID),
                    avatarURL: directory.avatarURL(for: userID),
                    isCurrentUser: userID == currentUserID
                )
            }
            if let ownIndex = reactors.firstIndex(where: \.isCurrentUser), ownIndex > 0 {
                reactors.insert(reactors.remove(at: ownIndex), at: 0)
            }
            guard count > 0 || reactors.isEmpty == false else { return nil }
            return TimelineReactionGroup(key: key, count: max(count, reactors.count), reactors: reactors)
        }
    }

    /// The other people who reacted with `key`, by name, in Core's order.
    static func otherNames(
        key: String,
        in item: TimelineItem,
        currentUserID: String,
        names: [String: String]
    ) -> [String] {
        (item.reactionSenders[key] ?? [])
            .filter { $0 != currentUserID }
            .map { names[$0] ?? TimelineReactorDirectory.localpart(of: $0) }
    }

    /// Whether the signed-in user reacted with `key`, from either Core's
    /// ownership flag or the sender list.
    static func includesCurrentUser(key: String, in item: TimelineItem, currentUserID: String) -> Bool {
        item.reactionOwnership.contains(key) || (item.reactionSenders[key] ?? []).contains(currentUserID)
    }
}

/// The sheet's selected tab, opened from a pill (that emoji) or from the
/// message menu (the first reaction).
struct TimelineReactionsSheetTarget: Identifiable, Equatable {
    let item: TimelineItem
    let initialKey: String?

    var id: String { item.eventID }

    func selectedKey(in groups: [TimelineReactionGroup]) -> String? {
        if let initialKey, groups.contains(where: { $0.key == initialKey }) {
            return initialKey
        }
        return groups.first?.key
    }
}
