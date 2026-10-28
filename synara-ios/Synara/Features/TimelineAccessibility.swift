import SwiftUI

/// What VoiceOver reads for a timeline message, and the custom actions that
/// replace its Message actions menu when the row is one element.
extension TimelineRowAccessibility {
    /// The message actions a one-element row exposes, in menu order.
    enum Action: Equatable {
        case reply
        case replyInThread
        case openThread
        case react
        /// Adds or removes the user's reaction with an existing key, as
        /// tapping its pill does.
        case toggleReaction(key: String, isOwn: Bool)
        /// Opens the Reactions sheet: who reacted with what.
        case viewReactions
        case edit
        case copy
        case selectText
        case forward
        case pinToNotes
        case declineCall
        case report
        case redact

        var title: String {
            switch self {
            case .reply:
                return String(localized: "Reply")
            case .replyInThread:
                return String(localized: "Reply in Thread")
            case .openThread:
                return String(localized: "Open Thread")
            case .react:
                return String(localized: "React")
            case let .toggleReaction(key, isOwn):
                return isOwn
                    ? String(localized: "Remove \(key) reaction")
                    : String(localized: "React with \(key)")
            case .viewReactions:
                return String(localized: "View Reactions")
            case .edit:
                return String(localized: "Edit")
            case .copy:
                return String(localized: "Copy")
            case .selectText:
                return String(localized: "Select Text")
            case .forward:
                return String(localized: "Forward")
            case .pinToNotes:
                return String(localized: "Pin to Notes")
            case .declineCall:
                return String(localized: "Decline Call")
            case .report:
                return String(localized: "Report")
            case .redact:
                return String(localized: "Redact")
            }
        }
    }

    /// Reply and React first: they are what a VoiceOver user reaches for
    /// most, and the rotor starts at the top of this list.
    static func actions(
        availability: EventActionAvailability,
        replyCount: Int,
        canCopy: Bool,
        canPinToNotes: Bool,
        reactions: [String: Int] = [:],
        ownReactions: TimelineReactionOwnership = .unknown,
        quickReactions: [String] = [],
        canViewReactions: Bool = false
    ) -> [Action] {
        var actions: [Action] = []
        if availability.canReply {
            actions.append(.reply)
        }
        let canToggle = TimelineReactionTogglePolicy.canToggle(availability: availability, ownership: ownReactions)
        // The menu's quick reactions replace the plain React action.
        if availability.canReact, canToggle == false || quickReactions.isEmpty {
            actions.append(.react)
        }
        if canToggle {
            // Existing reactions first, then quick reactions not already shown.
            let keys = reactions.keys.sorted() + quickReactions.filter { reactions[$0] == nil }
            for key in keys {
                actions.append(.toggleReaction(key: key, isOwn: ownReactions.contains(key)))
            }
        }
        if canViewReactions, reactions.isEmpty == false {
            actions.append(.viewReactions)
        }
        if availability.canReply {
            actions.append(.replyInThread)
        }
        if replyCount > 0 {
            actions.append(.openThread)
        }
        if availability.canEdit {
            actions.append(.edit)
        }
        if canCopy {
            actions.append(.copy)
            actions.append(.selectText)
        }
        if availability.canForward {
            actions.append(.forward)
        }
        if canPinToNotes {
            actions.append(.pinToNotes)
        }
        if availability.canDeclineCall {
            actions.append(.declineCall)
        }
        if availability.canReport {
            actions.append(.report)
        }
        if availability.canRedact {
            actions.append(.redact)
        }
        return actions
    }

    /// "Ada at 10:42 AM: Hello. Edited. Reactions: 👍 3, you reacted. 2 replies".
    /// VoiceOver speaks each emoji by name.
    static func summary(
        sender: String,
        time: String,
        kind: TimelineItem.Kind,
        isEdited: Bool = false,
        reactions: [String: Int] = [:],
        ownReactions: TimelineReactionOwnership = .unknown,
        replyCount: Int = 0
    ) -> String {
        let senderAndTime = String(localized: "\(sender) at \(time)")
        var parts = [body(senderAndTime: senderAndTime, kind: kind)]
        if case .systemNotice = kind {
            return parts[0]
        }
        if isEdited {
            parts.append(String(localized: "Edited"))
        }
        let reactionLabels = reactions.keys.sorted().map { key in
            reactionLabel(key: key, count: reactions[key] ?? 0, isOwn: ownReactions.contains(key))
        }
        if reactionLabels.isEmpty == false {
            let reactionList = reactionLabels.joined(separator: "; ")
            parts.append(String(localized: "Reactions: \(reactionList)"))
        }
        if replyCount > 0 {
            parts.append(replyCountLabel(replyCount))
        }
        return parts.joined(separator: ". ")
    }

    static func reactionLabel(key: String, count: Int, isOwn: Bool) -> String {
        isOwn ? String(localized: "\(key) \(count), you reacted") : String(localized: "\(key) \(count)")
    }

    /// A pill's label: "👍, 3 reactions: Alice, Bob and you". Without any
    /// named sender it falls back to the count and "you reacted".
    static func reactionPillLabel(key: String, count: Int, includesYou: Bool, otherNames: [String]) -> String {
        let total = max(count, otherNames.count + (includesYou ? 1 : 0))
        let countText = String(localized: "\(total) reactions")
        guard otherNames.isEmpty == false || includesYou,
              let people = reactorList(otherNames: otherNames, includesYou: includesYou, total: total)
        else {
            return String(localized: "\(key), \(countText)")
        }
        return String(localized: "\(key), \(countText): \(people)")
    }

    /// "Alice", "Alice and you", "Alice, Bob and you"; past three people,
    /// the first two others (and you), then "and 2 others". People Core
    /// counted without naming join the "others".
    static func reactorList(otherNames: [String], includesYou: Bool, total: Int) -> String? {
        let you = String(localized: "you")
        let everyone = otherNames + (includesYou ? [you] : [])
        let total = max(total, everyone.count)
        guard everyone.isEmpty == false else {
            return nil
        }
        if everyone.count == total, total <= 3 {
            return joinedNames(everyone)
        }
        let named = Array(otherNames.prefix(2)) + (includesYou ? [you] : [])
        let remaining = total - named.count
        guard remaining > 0 else {
            return joinedNames(named)
        }
        let list = named.joined(separator: String(localized: ", "))
        return String(localized: "\(list) and \(remaining) others")
    }

    private static func joinedNames(_ names: [String]) -> String {
        switch names.count {
        case 0:
            return ""
        case 1:
            return names[0]
        case 2:
            return String(localized: "\(names[0]) and \(names[1])")
        default:
            let leading = names.dropLast().joined(separator: String(localized: ", "))
            return String(localized: "\(leading) and \(names[names.count - 1])")
        }
    }

    /// The pill's custom action; tapping it still toggles the reaction.
    static var showReactorsActionTitle: String {
        String(localized: "Show who reacted")
    }

    static func reactionToggleHint(isOwn: Bool) -> String {
        isOwn ? String(localized: "Removes your reaction") : String(localized: "Adds your reaction")
    }

    static func replyCountLabel(_ count: Int) -> String {
        String(localized: "\(count) replies")
    }

    private static func body(senderAndTime: String, kind: TimelineItem.Kind) -> String {
        switch kind {
        case let .text(body):
            return String(localized: "\(senderAndTime): \(body)")
        case let .formattedText(body, _):
            return String(localized: "\(senderAndTime): \(body)")
        case let .mediaPlaceholder(resource):
            if resource.isEncrypted {
                return String(localized: "\(senderAndTime) sent encrypted media that cannot be opened until keys are available")
            }
            if let caption = resource.caption {
                return String(localized: "\(senderAndTime) sent \(resource.safeDescription): \(caption)")
            }
            return String(localized: "\(senderAndTime) sent \(resource.safeDescription)")
        case .redacted:
            return String(localized: "\(senderAndTime): message deleted")
        case .encryptedPlaceholder:
            return String(localized: "\(senderAndTime): encrypted message unavailable")
        case .unknown:
            return String(localized: "\(senderAndTime): \(TimelineUnavailableMessageCopy.text)")
        case let .systemNotice(text):
            return text
        case let .agentCard(card):
            let status = card.status.map { String(localized: ", status \($0)") } ?? ""
            let primaryAction = card.actions.first(where: SynaraAgentCardActionResolver.shouldRender)
                .map { String(localized: ", primary action \($0.title)") } ?? ""
            return String(localized: "\(senderAndTime): agent card: \(card.title)\(status)\(primaryAction)")
        }
    }
}

/// Whether tapping a reaction pill can add or remove the user's reaction.
/// Core's toggle needs to know which keys are the user's own, so an unknown
/// ownership leaves the pills display-only.
enum TimelineReactionTogglePolicy {
    static func canToggle(availability: EventActionAvailability, ownership: TimelineReactionOwnership) -> Bool {
        guard availability.canReact, case .known = ownership else {
            return false
        }
        return true
    }
}

/// Layout switches for the accessibility Dynamic Type sizes (AX1–AX5), where
/// one-line chrome truncates ("# Thread…", "Unr…") or floats over messages.
enum SynaraDynamicTypeLayout {
    /// Room titles in the room list and the conversation header.
    static func titleLineLimit(_ size: DynamicTypeSize) -> Int {
        size.isAccessibilitySize ? 3 : 1
    }

    /// Last-message previews in the room list.
    static func previewLineLimit(_ size: DynamicTypeSize) -> Int {
        size.isAccessibilitySize ? 3 : 1
    }

    /// Room rows put time, badges and status icons on their own line, so the
    /// preview keeps the full width.
    static func stacksRoomRowMetadata(_ size: DynamicTypeSize) -> Bool {
        size.isAccessibilitySize
    }

    /// "Jump to Last Read" becomes an icon button like Jump to latest; the
    /// spoken label stays.
    static func jumpToLastReadShowsTitle(_ size: DynamicTypeSize) -> Bool {
        size.isAccessibilitySize == false
    }

    /// Divider and day-header labels wrap instead of truncating.
    static func ruleLabelLineLimit(_ size: DynamicTypeSize) -> Int? {
        size.isAccessibilitySize ? nil : 1
    }
}
