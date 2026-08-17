import Foundation
import SynaraCore

/// P4-S16 map of privacy-safe SharedCore timeline rows to product items.
///
/// Skips virtual separators/markers. Does not load media bytes. This is
/// not iOS-on-engine and not P4 acceptance.
enum SharedCoreTimelineRows {
    static func items(from rows: [TimelineViewRowDto]) -> [TimelineItem] {
        rows.compactMap(item(from:))
    }

    static func outcome(from rows: [TimelineViewRowDto]) -> TimelineLoadOutcome {
        let items = items(from: rows)
        return items.isEmpty ? .empty : .loaded(items)
    }

    static func item(from row: TimelineViewRowDto) -> TimelineItem? {
        switch row.kind {
        case "date_separator", "read_marker", "unread_marker", "timeline_start", "pagination":
            return nil
        default:
            break
        }
        let eventID = row.eventId.isEmpty ? row.itemId : row.eventId
        let timestamp = Date(timeIntervalSince1970: TimeInterval(row.originServerTs) / 1000)
        return TimelineItem(
            id: row.itemId,
            eventID: eventID,
            senderID: row.sender,
            timestamp: timestamp,
            kind: kind(from: row),
            replyToEventID: row.replyToEventId,
            isEdited: row.edited,
            reactions: [:],
            isEncrypted: row.kind == "encrypted"
        )
    }

    private static func kind(from row: TimelineViewRowDto) -> TimelineItem.Kind {
        switch row.kind {
        case "redacted":
            return .redacted
        case "encrypted":
            return .encryptedPlaceholder
        case "message":
            if let html = row.formattedBody, html.isEmpty == false {
                return .formattedText(body: row.body, html: html)
            }
            return .text(row.body)
        default:
            return .unknown(type: row.kind)
        }
    }
}
