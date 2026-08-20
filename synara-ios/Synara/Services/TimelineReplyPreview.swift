import Foundation

struct ComposerRelationTarget: Equatable {
    enum Kind: Equatable {
        case reply
        case edit
    }

    let eventID: String
    let senderName: String
    let snippet: String
    let kind: Kind
    let isLocalPending: Bool

    init(item: TimelineItem, kind: Kind, currentUserID: String) {
        let preview = TimelineReplyPreview.from(item: item, currentUserID: currentUserID)
        eventID = item.eventID
        senderName = preview.senderName
        snippet = preview.snippet
        self.kind = kind
        isLocalPending = item.isLocalPending
    }

    var bannerTitle: String {
        switch kind {
        case .reply:
            return "Replying to \(senderName)"
        case .edit:
            return isLocalPending ? "Editing unsent message" : "Editing your message"
        }
    }
}

struct TimelineReplyPreview: Equatable {
    static let maxSnippetLength = 80

    let senderName: String
    let snippet: String

    static func from(item: TimelineItem, currentUserID: String) -> TimelineReplyPreview {
        TimelineReplyPreview(
            senderName: item.resolvedSenderDisplayName(currentUserID: currentUserID),
            snippet: snippet(for: item.kind)
        )
    }

    static func snippet(for kind: TimelineItem.Kind) -> String {
        let raw: String
        switch kind {
        case .text(let body):
            raw = body
        case .formattedText(let body, let html):
            let markdown = MatrixHTMLRenderer.markdownExcludingDetails(body: body, html: html)
            raw = markdown.isEmpty ? body : markdown
        case .mediaPlaceholder(let resource):
            if resource.isEncrypted {
                raw = "Encrypted attachment"
            } else {
                raw = resource.safeDescription
            }
        case .redacted:
            raw = "Message deleted"
        case .encryptedPlaceholder:
            raw = "Encrypted message"
        case .agentCard(let card):
            if let summary = card.summary?.trimmingCharacters(in: .whitespacesAndNewlines),
               summary.isEmpty == false {
                raw = summary
            } else {
                raw = card.title
            }
        case .unknown(let type):
            raw = "Unsupported message (\(type))"
        }
        return truncatedSnippet(raw)
    }

    static func previewsByEventID(in items: [TimelineItem], currentUserID: String) -> [String: TimelineReplyPreview] {
        var previews: [String: TimelineReplyPreview] = [:]
        previews.reserveCapacity(items.count)
        for item in items {
            previews[item.eventID] = from(item: item, currentUserID: currentUserID)
        }
        return previews
    }

    static func truncatedSnippet(_ text: String) -> String {
        let collapsed = text
            .replacingOccurrences(of: "\n", with: " ")
            .replacingOccurrences(of: "\r", with: " ")
            .replacingOccurrences(of: "\\s+", with: " ", options: .regularExpression)
            .trimmingCharacters(in: .whitespacesAndNewlines)

        guard collapsed.count > maxSnippetLength else {
            return collapsed
        }

        let endIndex = collapsed.index(collapsed.startIndex, offsetBy: maxSnippetLength - 1)
        return String(collapsed[..<endIndex]) + "…"
    }
}

extension TimelineItem {
    var senderDisplayName: String {
        resolvedSenderDisplayName(currentUserID: nil)
    }

    func resolvedSenderDisplayName(currentUserID: String) -> String {
        resolvedSenderDisplayName(currentUserID: currentUserID as String?)
    }

    private func resolvedSenderDisplayName(currentUserID: String?) -> String {
        if let currentUserID, senderID == currentUserID {
            return "You"
        }

        switch senderID.lowercased() {
        case "@mina:matrix.org":
            return "Mina"
        case "@alex:matrix.org":
            return "Alex"
        case "@ravi:matrix.org":
            return "Ravi"
        case "@local:matrix.org", "@you:matrix.org":
            return "You"
        default:
            break
        }

        guard senderID.hasPrefix("@") else {
            return senderID
        }

        return senderID
            .dropFirst()
            .split(separator: ":")
            .first
            .map(String.init) ?? senderID
    }
}