import Foundation

struct TimelineItem: Identifiable, Equatable {
    enum Kind: Equatable {
        case text(String)
        case mediaPlaceholder(MediaResource)
        case redacted
        case encryptedPlaceholder
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
            kind = .encryptedPlaceholder
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

final class MatrixTimelineService: TimelineServicing {
    private let sessionStore: AppSessionStore
    private let httpClient: AuthHTTPClient
    private let jsonDecoder: JSONDecoder
    private var paginationTokensByRoom: [String: String] = [:]

    init(
        sessionStore: AppSessionStore,
        httpClient: AuthHTTPClient = URLSession.shared,
        jsonDecoder: JSONDecoder = JSONDecoder()
    ) {
        self.sessionStore = sessionStore
        self.httpClient = httpClient
        self.jsonDecoder = jsonDecoder
    }

    func loadInitialTimeline(roomID: String) async -> [TimelineItem] {
        await loadTimeline(roomID: roomID, from: nil)
    }

    func loadOlderTimeline(roomID: String, before eventID: String) async -> [TimelineItem] {
        await loadTimeline(roomID: roomID, from: eventID)
    }

    private func loadTimeline(roomID: String, from: String?) async -> [TimelineItem] {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return []
        }

        do {
            var request = URLRequest(url: messagesURL(homeserverURL: session.homeserverURL, roomID: roomID, from: from))
            request.httpMethod = "GET"
            request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")

            let (data, response) = try await httpClient.data(for: request)
            guard let httpResponse = response as? HTTPURLResponse,
                  httpResponse.statusCode == 200 else {
                return []
            }

            let messages = try jsonDecoder.decode(MatrixMessagesResponse.self, from: data)
            if let end = messages.end {
                paginationTokensByRoom[roomID] = end
            }
            return messages.chunk
                .reversed()
                .compactMap(mapEvent)
                .map(TimelineMapper.map)
        } catch {
            return []
        }
    }

    private func messagesURL(homeserverURL: URL, roomID: String, from: String?) -> URL {
        var url = homeserverURL
        url.appendPathComponent("_matrix")
        url.appendPathComponent("client")
        url.appendPathComponent("v3")
        url.appendPathComponent("rooms")
        url.appendPathComponent(roomID)
        url.appendPathComponent("messages")

        var queryItems = [
            URLQueryItem(name: "dir", value: "b"),
            URLQueryItem(name: "limit", value: "50")
        ]

        if let from {
            queryItems.append(URLQueryItem(name: "from", value: paginationTokensByRoom[roomID] ?? from))
        }

        var components = URLComponents(url: url, resolvingAgainstBaseURL: false)
        components?.queryItems = queryItems
        return components?.url ?? url
    }

    private func mapEvent(_ event: MatrixTimelineEvent) -> RawTimelineEvent? {
        guard let eventID = event.eventID,
              let sender = event.sender,
              shouldShow(eventType: event.type) else {
            return nil
        }

        let content = event.content ?? MatrixTimelineEventContent()
        let eventType: String
        let body: String?
        let mediaURL: URL?

        if event.type == "m.room.message" {
            if let msgtype = content.msgtype,
               ["m.image", "m.file", "m.audio", "m.video"].contains(msgtype) {
                eventType = "m.room.media"
                body = content.body
                mediaURL = content.url.flatMap(URL.init(string:))
            } else {
                eventType = event.type
                body = content.body
                mediaURL = nil
            }
        } else {
            eventType = event.type
            body = content.body
            mediaURL = content.url.flatMap(URL.init(string:))
        }

        return RawTimelineEvent(
            eventID: eventID,
            senderID: sender,
            timestamp: Date(timeIntervalSince1970: TimeInterval(event.originServerTimestamp ?? 0) / 1_000),
            type: eventType,
            body: body,
            replyToEventID: content.relatesTo?.inReplyTo?.eventID,
            isEdited: content.relatesTo?.relType == "m.replace",
            mediaURL: mediaURL
        )
    }

    private func shouldShow(eventType: String) -> Bool {
        if ["m.room.message", "m.room.encrypted", "m.room.redaction"].contains(eventType) {
            return true
        }

        return eventType.hasPrefix("synara.")
    }
}

private struct MatrixMessagesResponse: Decodable {
    let chunk: [MatrixTimelineEvent]
    let end: String?
}

private struct MatrixTimelineEvent: Decodable {
    let eventID: String?
    let sender: String?
    let originServerTimestamp: Int?
    let type: String
    let content: MatrixTimelineEventContent?

    enum CodingKeys: String, CodingKey {
        case eventID = "event_id"
        case sender
        case originServerTimestamp = "origin_server_ts"
        case type
        case content
    }
}

private struct MatrixTimelineEventContent: Decodable {
    let body: String?
    let msgtype: String?
    let url: String?
    let relatesTo: MatrixRelatesTo?

    init(
        body: String? = nil,
        msgtype: String? = nil,
        url: String? = nil,
        relatesTo: MatrixRelatesTo? = nil
    ) {
        self.body = body
        self.msgtype = msgtype
        self.url = url
        self.relatesTo = relatesTo
    }

    enum CodingKeys: String, CodingKey {
        case body
        case msgtype
        case url
        case relatesTo = "m.relates_to"
    }
}

private struct MatrixRelatesTo: Decodable {
    let relType: String?
    let inReplyTo: MatrixInReplyTo?

    enum CodingKeys: String, CodingKey {
        case relType = "rel_type"
        case inReplyTo = "m.in_reply_to"
    }
}

private struct MatrixInReplyTo: Decodable {
    let eventID: String?

    enum CodingKeys: String, CodingKey {
        case eventID = "event_id"
    }
}
