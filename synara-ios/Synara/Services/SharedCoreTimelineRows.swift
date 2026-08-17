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
            formattedBody: row.formattedBody
        ) else {
            return nil
        }
        let eventID = row.eventId.isEmpty ? row.itemId : row.eventId
        let timestamp = Date(timeIntervalSince1970: TimeInterval(row.originServerTs) / 1000)
        return TimelineItem(
            id: row.itemId,
            eventID: eventID,
            senderID: row.sender,
            timestamp: timestamp,
            kind: kind,
            replyToEventID: row.replyToEventId,
            isEdited: row.edited,
            reactions: [:],
            isEncrypted: row.kind == "encrypted"
        )
    }

    static func displayKind(
        rowKind: String,
        body: String,
        formattedBody: String?
    ) -> TimelineItem.Kind? {
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
}
