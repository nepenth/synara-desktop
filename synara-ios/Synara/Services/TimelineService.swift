import Foundation

struct TimelineItem: Identifiable, Equatable {
    enum Kind: Equatable {
        case text(String)
        case mediaPlaceholder(filename: String?)
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
}

struct RawTimelineEvent: Equatable {
    let eventID: String
    let senderID: String
    let timestamp: Date
    let type: String
    let body: String?
    let replyToEventID: String?
    let isEdited: Bool
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
            kind = .mediaPlaceholder(filename: event.body)
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
            isEdited: event.isEdited
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
                isEdited: false
            ),
            RawTimelineEvent(
                eventID: "$reply:\(roomID)",
                senderID: "@bob:matrix.org",
                timestamp: baseDate.addingTimeInterval(30),
                type: "m.room.message",
                body: "Reply body",
                replyToEventID: "$text:\(roomID)",
                isEdited: true
            ),
            RawTimelineEvent(
                eventID: "$unknown:\(roomID)",
                senderID: "@agent:matrix.org",
                timestamp: baseDate.addingTimeInterval(60),
                type: "synara.agent.card",
                body: nil,
                replyToEventID: nil,
                isEdited: false
            )
        ]
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
