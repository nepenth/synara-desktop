import Foundation

struct TimelineItem: Identifiable, Equatable {
    enum Kind: Equatable {
        case text(String)
        case mediaPlaceholder(MediaResource)
        case redacted
        case unknown(type: String)
    }

    let id: String
    let eventID: String
    let senderID: String
    let timestamp: Date
    let kind: Kind
    let replyToEventID: String?
    let isEdited: Bool
    let reactions: [String: Int]
}

struct RawTimelineEvent: Equatable {
    let eventID: String
    let senderID: String
    let timestamp: Date
    let type: String
    let body: String?
    let replyToEventID: String?
    let isEdited: Bool
    let mediaURL: URL?
}

protocol TimelineServicing {
    func loadInitialTimeline(roomID: String) async -> [TimelineItem]
    func loadOlderTimeline(roomID: String, before eventID: String) async -> [TimelineItem]
}

enum TimelineMapper {
    static func map(_ event: RawTimelineEvent) -> TimelineItem {
        let kind: TimelineItem.Kind

        switch event.type {
        case "m.room.message":
            kind = .text(event.body ?? "")
        case "m.room.encrypted":
            kind = .unknown(type: event.type)
        case "m.room.redaction":
            kind = .redacted
        case "m.room.media":
            kind = .mediaPlaceholder(
                MediaResource(
                    id: event.eventID,
                    filename: event.body ?? "Attachment",
                    authenticatedURL: event.mediaURL,
                    requiresAuthentication: true
                )
            )
        default:
            kind = .unknown(type: event.type)
        }

        return TimelineItem(
            id: event.eventID,
            eventID: event.eventID,
            senderID: event.senderID,
            timestamp: event.timestamp,
            kind: kind,
            replyToEventID: event.replyToEventID,
            isEdited: event.isEdited,
            reactions: [:]
        )
    }
}

enum TimelineFixtures {
    static let baseDate = Date(timeIntervalSince1970: 1_700_000_000)

    static func commonEvents(roomID: String = "!project:matrix.org") -> [RawTimelineEvent] {
        [
            RawTimelineEvent(
                eventID: "$text:\(roomID)",
                senderID: "@alice:matrix.org",
                timestamp: baseDate,
                type: "m.room.message",
                body: "Hello from iOS",
                replyToEventID: nil,
                isEdited: false,
                mediaURL: nil
            ),
            RawTimelineEvent(
                eventID: "$reply:\(roomID)",
                senderID: "@bob:matrix.org",
                timestamp: baseDate.addingTimeInterval(30),
                type: "m.room.message",
                body: "Reply body",
                replyToEventID: "$text:\(roomID)",
                isEdited: true,
                mediaURL: nil
            ),
            RawTimelineEvent(
                eventID: "$media:\(roomID)",
                senderID: "@alice:matrix.org",
                timestamp: baseDate.addingTimeInterval(45),
                type: "m.room.media",
                body: "photo.jpg",
                replyToEventID: nil,
                isEdited: false,
                mediaURL: URL(string: "mxc://matrix.org/media-id")
            ),
            RawTimelineEvent(
                eventID: "$unknown:\(roomID)",
                senderID: "@agent:matrix.org",
                timestamp: baseDate.addingTimeInterval(60),
                type: "synara.agent.card",
                body: nil,
                replyToEventID: nil,
                isEdited: false,
                mediaURL: nil
            )
        ]
    }

    static func largeTimeline(count: Int = 10_000) -> [TimelineItem] {
        var items: [TimelineItem] = []
        items.reserveCapacity(count)

        for index in 0..<count {
            let item = TimelineItem(
                id: "$synthetic-\(index):matrix.org",
                eventID: "$synthetic-\(index):matrix.org",
                senderID: index % 2 == 0 ? "@alice:matrix.org" : "@bob:matrix.org",
                timestamp: baseDate.addingTimeInterval(TimeInterval(index)),
                kind: .text("Synthetic message \(index)"),
                replyToEventID: nil,
                isEdited: false,
                reactions: [:]
            )
            items.append(item)
        }

        return items
    }
}

struct MockTimelineService: TimelineServicing {
    var events: [RawTimelineEvent] = TimelineFixtures.commonEvents()

    func loadInitialTimeline(roomID: String) async -> [TimelineItem] {
        events.map(TimelineMapper.map)
    }

    func loadOlderTimeline(roomID: String, before eventID: String) async -> [TimelineItem] {
        events.filter { $0.eventID != eventID }.map(TimelineMapper.map)
    }
}
