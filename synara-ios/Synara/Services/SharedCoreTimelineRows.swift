import Foundation
import SynaraCore

/// P4-S16/S29 map of privacy-safe SharedCore timeline rows to product items.
///
/// Skips virtual separators/markers. Poll / membership / state / call /
/// other rows use the existing body text. Does not load media bytes or
/// invent mxc URLs. This is not iOS-on-engine and not P4 acceptance.
enum SharedCoreTimelineRows {
    static func items(from rows: [TimelineViewRowDto]) -> [TimelineItem] {
        rows.compactMap(item(from:))
    }

    static func outcome(from rows: [TimelineViewRowDto]) -> TimelineLoadOutcome {
        let items = items(from: rows)
        return items.isEmpty ? .empty : .loaded(items)
    }

    static func item(from row: TimelineViewRowDto) -> TimelineItem? {
        guard let kind = displayKind(
            rowKind: row.kind,
            body: row.body,
            formattedBody: row.formattedBody,
            agentCardJSON: row.agentCardJson,
            messageType: row.messageType,
            mediaHandleId: row.mediaHandleId,
            mediaMimeType: row.mediaMimeType
        ) else {
            return nil
        }
        let eventID = row.eventId.isEmpty ? row.itemId : row.eventId
        let timestamp = Date(timeIntervalSince1970: TimeInterval(row.originServerTs) / 1000)
        return TimelineItem(
            id: row.itemId,
            eventID: eventID,
            senderID: row.sender,
            senderAvatarURL: senderAvatarURL(row.senderAvatarUrl),
            timestamp: timestamp,
            kind: kind,
            replyToEventID: row.replyToEventId,
            isEdited: row.edited,
            reactions: reactions(from: row.reactions),
            isEncrypted: row.kind == "encrypted"
        )
    }

    /// Timeline avatars are metadata-only Matrix content URIs. Reject every
    /// other scheme at the Swift boundary before it can reach the media owner.
    static func senderAvatarURL(_ rawValue: String?) -> URL? {
        guard let rawValue,
              rawValue.hasPrefix("mxc://"),
              let url = URL(string: rawValue),
              url.scheme == "mxc",
              url.host?.isEmpty == false,
              url.pathComponents.count > 1
        else {
            return nil
        }
        return url
    }

    static func displayKind(
        rowKind: String,
        body: String,
        formattedBody: String?,
        agentCardJSON: String? = nil,
        messageType: String? = nil,
        mediaHandleId: String? = nil,
        mediaMimeType: String? = nil
    ) -> TimelineItem.Kind? {
        if let agentCard = SynaraAgentCardPayloadParser.parse(payloadJSON: agentCardJSON) {
            return .agentCard(agentCard)
        }
        if let media = mediaPlaceholder(
            rowKind: rowKind,
            body: body,
            messageType: messageType,
            mediaHandleId: mediaHandleId,
            mediaMimeType: mediaMimeType
        ) {
            return .mediaPlaceholder(media)
        }
        switch rowKind {
        case "date_separator", "read_marker", "unread_marker", "timeline_start", "pagination":
            return nil
        case "redacted":
            return .redacted
        case "encrypted":
            return .encryptedPlaceholder
        case "message":
            if let html = formattedBody, html.isEmpty == false {
                return .formattedText(body: body, html: html)
            }
            return .text(body)
        case "poll", "membership", "state", "call", "other", "sticker":
            let trimmed = body.trimmingCharacters(in: .whitespacesAndNewlines)
            if trimmed.isEmpty == false {
                return .text(trimmed)
            }
            return .unknown(type: rowKind)
        default:
            return .unknown(type: rowKind)
        }
    }

    static func reactions(from rows: [TimelineViewReactionDto]) -> [String: Int] {
        reactionCounts(rows.map { ($0.key, $0.count) })
    }

    static func reactionCounts(_ rows: [(key: String, count: UInt32)]) -> [String: Int] {
        var counts: [String: Int] = [:]
        for row in rows where row.key.isEmpty == false {
            counts[row.key] = Int(row.count)
        }
        return counts
    }

    static func mediaPlaceholder(
        rowKind: String,
        body: String,
        messageType: String?,
        mediaHandleId: String?,
        mediaMimeType: String?
    ) -> MediaResource? {
        guard let handle = mediaHandleId?.trimmingCharacters(in: .whitespacesAndNewlines),
              handle.isEmpty == false,
              let url = URL(string: "synara-timeline-media://\(handle)") else {
            return nil
        }
        let filename = body.trimmingCharacters(in: .whitespacesAndNewlines)
        return MediaResource(
            id: handle,
            filename: filename.isEmpty ? (messageType ?? rowKind) : filename,
            authenticatedURL: url,
            requiresAuthentication: true,
            isEncrypted: false,
            mimeType: mediaMimeType
        )
    }
}
