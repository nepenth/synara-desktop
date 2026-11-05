import Foundation

/// A person picked from the composer's @ suggestions. The composer shows
/// `label`; the send turns it into a matrix.to pill, as desktop does.
struct ComposerMention: Equatable, Hashable {
    let userID: String
    /// "@Name", as typed into the composer.
    let label: String

    init(userID: String, displayName: String) {
        self.userID = userID
        let name = displayName.trimmingCharacters(in: .whitespacesAndNewlines)
        let shown = name.isEmpty ? userID : name
        label = shown.hasPrefix("@") ? shown : "@" + shown
    }
}

enum ComposerMentions {
    /// The "@query" the cursor is finishing: where its "@" is (UTF-16) and
    /// what follows it.
    struct Query: Equatable {
        let location: Int
        let text: String
    }

    /// What a send carries: the draft with each mention as a Markdown link,
    /// and the people it mentions (for `m.mentions`).
    struct Resolved: Equatable {
        let markdown: String
        let userIDs: [String]
    }

    static let maxQueryLength = 40

    /// The "@query" just before `cursor`, when the "@" starts a word.
    static func query(in text: String, cursor: Int) -> Query? {
        let ns = text as NSString
        guard cursor > 0, cursor <= ns.length else {
            return nil
        }
        var index = cursor - 1
        while index >= 0, cursor - index <= maxQueryLength {
            let character = ns.character(at: index)
            if character == 0x40 /* @ */ {
                guard index == 0 || startsWord(after: ns.character(at: index - 1)) else {
                    return nil
                }
                let start = index + 1
                return Query(location: index, text: ns.substring(with: NSRange(location: start, length: cursor - start)))
            }
            if isSpace(character) {
                return nil
            }
            index -= 1
        }
        return nil
    }

    /// Joined and invited members matching `query`, best first: names that
    /// start with it, then names with a word that does, then user ids.
    static func suggestions(
        for query: String,
        members: [RoomMemberSummary],
        excluding currentUserID: String,
        limit: Int = 5
    ) -> [RoomMemberSummary] {
        let needle = query.lowercased()
        var ranked: [(rank: Int, member: RoomMemberSummary)] = []
        for member in members where member.userID != currentUserID {
            guard member.membership == .join || member.membership == .invite else {
                continue
            }
            let name = member.title.lowercased()
            let localpart = member.userID.dropFirst().split(separator: ":").first.map { $0.lowercased() } ?? ""
            let rank: Int
            if needle.isEmpty || name.hasPrefix(needle) {
                rank = 0
            } else if name.split(whereSeparator: { $0 == " " || $0 == "-" || $0 == "_" })
                .contains(where: { $0.hasPrefix(needle) })
            {
                rank = 1
            } else if localpart.hasPrefix(needle) {
                rank = 2
            } else if name.contains(needle) || localpart.contains(needle) {
                rank = 3
            } else {
                continue
            }
            ranked.append((rank, member))
        }
        return ranked
            .sorted { lhs, rhs in
                lhs.rank != rhs.rank
                    ? lhs.rank < rhs.rank
                    : lhs.member.title.localizedCaseInsensitiveCompare(rhs.member.title) == .orderedAscending
            }
            .prefix(limit)
            .map(\.member)
    }

    /// Replaces the "@query" with the mention's label and a space.
    static func insert(
        _ mention: ComposerMention,
        replacing query: Query,
        cursor: Int,
        in text: String
    ) -> (text: String, cursor: Int) {
        let ns = text as NSString
        let replacement = mention.label + " "
        let range = NSRange(location: query.location, length: max(0, min(cursor, ns.length) - query.location))
        let updated = ns.replacingCharacters(in: range, with: replacement)
        return (updated, query.location + (replacement as NSString).length)
    }

    /// Turns each picked mention still in `text` into a matrix.to link.
    /// Your own id is linked but left out of `m.mentions`, as on desktop.
    static func resolve(_ text: String, mentions: [ComposerMention], currentUserID: String) -> Resolved {
        guard mentions.isEmpty == false else {
            return Resolved(markdown: text, userIDs: [])
        }
        // The longest label wins where one name starts another.
        let candidates = Set(mentions).sorted { $0.label.utf16.count > $1.label.utf16.count }
        let ns = text as NSString
        var output = ""
        var userIDs: [String] = []
        var copiedUpTo = 0
        var index = 0
        while index < ns.length {
            guard ns.character(at: index) == 0x40,
                  index == 0 || startsWord(after: ns.character(at: index - 1)),
                  let mention = candidates.first(where: { matches($0.label, in: ns, at: index) })
            else {
                index += 1
                continue
            }
            output += ns.substring(with: NSRange(location: copiedUpTo, length: index - copiedUpTo))
            output += markdownLink(for: mention)
            if mention.userID != currentUserID, userIDs.contains(mention.userID) == false {
                userIDs.append(mention.userID)
            }
            index += (mention.label as NSString).length
            copiedUpTo = index
        }
        output += ns.substring(from: copiedUpTo)
        return Resolved(markdown: output, userIDs: userIDs)
    }

    /// The mentions in a message being edited, so saving keeps its pills:
    /// each by its link text and by its user id (desktop's plain body).
    static func mentions(inHTML html: String) -> [ComposerMention] {
        guard let pattern = try? NSRegularExpression(
            pattern: #"<a href="https://matrix\.to/#/([^"]+)">([^<]*)</a>"#
        ) else {
            return []
        }
        let ns = html as NSString
        var found: [ComposerMention] = []
        for match in pattern.matches(in: html, range: NSRange(location: 0, length: ns.length)) {
            let target = ns.substring(with: match.range(at: 1))
            guard let url = URL(string: "https://matrix.to/#/" + target),
                  let userID = TimelineMentionPolicy.userID(inPermalink: url)
            else {
                continue
            }
            let text = ns.substring(with: match.range(at: 2))
            for mention in [ComposerMention(userID: userID, displayName: text), ComposerMention(userID: userID, displayName: userID)]
                where found.contains(mention) == false
            {
                found.append(mention)
            }
        }
        return found
    }

    static func markdownLink(for mention: ComposerMention) -> String {
        var escaped = ""
        for character in mention.label {
            if "\\[]*_`<>".contains(character) {
                escaped.append("\\")
            }
            escaped.append(character)
        }
        let target = mention.userID.addingPercentEncoding(withAllowedCharacters: .urlFragmentAllowed) ?? mention.userID
        return "[\(escaped)](https://matrix.to/#/\(target))"
    }

    private static func matches(_ label: String, in ns: NSString, at index: Int) -> Bool {
        let length = (label as NSString).length
        guard index + length <= ns.length,
              ns.substring(with: NSRange(location: index, length: length)) == label
        else {
            return false
        }
        // "@Al" is not a mention inside "@Alex".
        let end = index + length
        guard end < ns.length else {
            return true
        }
        guard let next = UnicodeScalar(ns.character(at: end)) else {
            return true
        }
        return CharacterSet.alphanumerics.contains(next) == false
    }

    private static func startsWord(after character: unichar) -> Bool {
        isSpace(character) || character == 0x28 /* ( */ || character == 0x22 /* " */
    }

    private static func isSpace(_ character: unichar) -> Bool {
        guard let scalar = UnicodeScalar(character) else {
            return false
        }
        return CharacterSet.whitespacesAndNewlines.contains(scalar)
    }
}
