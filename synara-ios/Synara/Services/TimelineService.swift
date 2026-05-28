import Foundation

struct TimelineItem: Identifiable, Equatable {
    enum Kind: Equatable {
        case text(String)
        case formattedText(body: String, html: String)
        case mediaPlaceholder(MediaResource)
        case redacted
        case encryptedPlaceholder
        case agentCard(SynaraAgentCard)
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
    let isEncrypted: Bool

    init(
        id: String,
        eventID: String,
        senderID: String,
        timestamp: Date,
        kind: Kind,
        replyToEventID: String?,
        isEdited: Bool,
        reactions: [String: Int],
        isEncrypted: Bool = false
    ) {
        self.id = id
        self.eventID = eventID
        self.senderID = senderID
        self.timestamp = timestamp
        self.kind = kind
        self.replyToEventID = replyToEventID
        self.isEdited = isEdited
        self.reactions = reactions
        self.isEncrypted = isEncrypted
    }
}

protocol LaterServicing {
    func loadItems() async -> Result<([SynaraLaterListItem], LaterInboxError?), Never>
}

enum LaterInboxError: Error, LocalizedError, Equatable {
    case noSession
    case malformedPayload
    case networkFailure

    var errorDescription: String? {
        switch self {
        case .noSession:
            return "Sign in to load your Later items."
        case .malformedPayload:
            return "Later account data was not readable."
        case .networkFailure:
            return "Could not load Later account data."
        }
    }
}

struct SynaraLaterListItem: Identifiable, Equatable {
    let id: String
    let roomID: String
    let eventID: String
    let kind: SynaraLaterItem.Kind
    let dueTs: Int?
    let completedAt: Int?
    let createdAt: Int
    let isCompleted: Bool

    var label: String {
        switch kind {
        case .saved:
            return "Saved"
        case .reminder:
            return "Reminder"
        }
    }

    var detail: String {
        if completedAt != nil {
            return "Completed"
        }

        if let dueTs {
            if dueTs < Int(Date().timeIntervalSince1970 * 1000) {
                return "Due"
            }

            return "Due soon"
        }

        return "No due date"
    }
}

final class MatrixAccountDataLaterService: LaterServicing {
    private let sessionStore: AppSessionStore
    private let httpClient: AuthHTTPClient
    private let jsonDecoder: JSONDecoder
    private let now: () -> Int

    init(
        sessionStore: AppSessionStore,
        httpClient: AuthHTTPClient = URLSession.shared,
        jsonDecoder: JSONDecoder = JSONDecoder(),
        now: @escaping () -> Int = { Int(Date().timeIntervalSince1970 * 1000) }
    ) {
        self.sessionStore = sessionStore
        self.httpClient = httpClient
        self.jsonDecoder = jsonDecoder
        self.now = now
    }

    func loadItems() async -> Result<([SynaraLaterListItem], LaterInboxError?), Never> {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return .success(([], .noSession))
        }

        do {
            let request = try accountDataRequest(for: session)
            let (data, response) = try await httpClient.data(for: request)

            guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
                return .success(([], .networkFailure))
            }

            guard let content = extractAccountDataContent(from: data),
                  let items = decode(content: content).successValue() else {
                return .success(([], .malformedPayload))
            }

            return .success((SynaraLaterListItem.sorted(items: items, now: now()), nil))
        } catch {
            return .success(([], .networkFailure))
        }
    }

    private func accountDataRequest(for session: AuthenticatedSession) throws -> URLRequest {
        var request = URLRequest(url: accountDataURL(for: session))
        request.httpMethod = "GET"
        request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")
        return request
    }

    private func accountDataURL(for session: AuthenticatedSession) -> URL {
        var url = session.homeserverURL
        url.appendPathComponent("_matrix")
        url.appendPathComponent("client")
        url.appendPathComponent("v3")
        url.appendPathComponent("user")
        url.appendPathComponent(session.userID.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? session.userID)
        url.appendPathComponent("account_data")
        url.appendPathComponent("in.synara.later")
        return url
    }

    private func extractAccountDataContent(from data: Data) -> [String: Any]? {
        let object: Any

        do {
            object = try JSONSerialization.jsonObject(with: data)
        } catch {
            return nil
        }

        guard let top = object as? [String: Any] else {
            return nil
        }

        if let content = top["content"] as? [String: Any] {
            return content
        }

        if top["items"] != nil || top["version"] != nil {
            return top
        }

        return nil
    }

    private func decode(content: [String: Any]) -> Result<SynaraLaterContent, LaterInboxError> {
        do {
            let data = try JSONSerialization.data(withJSONObject: content)
            let decoded = try jsonDecoder.decode(SynaraLaterContent.self, from: data)
            return .success(decoded)
        } catch {
            return .failure(.malformedPayload)
        }
    }
}

extension Result where Success == SynaraLaterContent, Failure == LaterInboxError {
    func successValue() -> SynaraLaterContent? {
        return try? self.get()
    }
}

extension SynaraLaterListItem {
    static func sorted(items: SynaraLaterContent, now: Int) -> [SynaraLaterListItem] {
        return items.items.values
            .map {
                SynaraLaterListItem(
                    id: $0.id,
                    roomID: $0.roomId,
                    eventID: $0.eventId,
                    kind: $0.kind,
                    dueTs: $0.dueTs,
                    completedAt: $0.completedAt,
                    createdAt: $0.createdAt,
                    isCompleted: $0.completedAt != nil
                )
            }
            .sorted { left, right in
                if left.completedAt != nil && right.completedAt == nil {
                    return false
                }

                if left.completedAt == nil && right.completedAt != nil {
                    return true
                }

                let leftDue = left.dueTs ?? Int.max
                let rightDue = right.dueTs ?? Int.max
                let leftDueSoon = leftDue <= now
                let rightDueSoon = rightDue <= now

                if leftDueSoon != rightDueSoon {
                    return leftDueSoon
                }

                if leftDue != rightDue {
                    return leftDue < rightDue
                }

                return left.createdAt > right.createdAt
            }
    }

    static let empty = SynaraLaterListItem(
        id: "",
        roomID: "",
        eventID: "",
        kind: .saved,
        dueTs: nil,
        completedAt: nil,
        createdAt: 0,
        isCompleted: false
    )
}

struct MockLaterService: LaterServicing {
    private let items: [SynaraLaterListItem]

    init(items: [SynaraLaterListItem] = []) {
        self.items = items
    }

    func loadItems() async -> Result<([SynaraLaterListItem], LaterInboxError?), Never> {
        .success((items, nil))
    }
}

enum SynaraAgentCardPayloadParser {
    private static let contentKeys = ["org.hermes.agent", "io.hermes.agent", "in.synara.agent", "m.custom.agent"]

    static func parse(raw: [String: Any] = [:], body: String? = nil) -> SynaraAgentCard? {
        if let directPayload = contentKeys.compactMap({ key in
            extractAgentCard(from: raw[key])
        }).first {
            return directPayload
        }

        guard let body,
              body.count <= 200_000,
              let bodyData = body.data(using: .utf8),
              let parsedBody = try? JSONSerialization.jsonObject(with: bodyData) as? [String: Any] else {
            return nil
        }

        if let directPayload = contentKeys.compactMap({ key in
            extractAgentCard(from: parsedBody[key])
        }).first {
            return directPayload
        }

        guard let hermes = parsedBody["hermes"] as? Bool, hermes else {
            return nil
        }

        return extractAgentCard(from: parsedBody["payload"]) ?? extractAgentCard(from: parsedBody["agent"])
    }

    private static func extractAgentCard(from rawValue: Any?) -> SynaraAgentCard? {
        guard let raw = rawValue as? [String: Any] else {
            return nil
        }
        do {
            return try JSONDecoder().decode(SynaraAgentCard.self, from: JSONSerialization.data(withJSONObject: raw))
        } catch {
            return nil
        }
    }
}

struct RawTimelineEvent: Equatable {
    let eventID: String
    let senderID: String
    let timestamp: Date
    let type: String
    let body: String?
    let formattedBody: String?
    let replyToEventID: String?
    let isEdited: Bool
    let mediaURL: URL?
    let isEncrypted: Bool
    let agentCard: SynaraAgentCard?

    init(
        eventID: String,
        senderID: String,
        timestamp: Date,
        type: String,
        body: String?,
        formattedBody: String? = nil,
        replyToEventID: String?,
        isEdited: Bool,
        mediaURL: URL?,
        isEncrypted: Bool = false,
        agentCard: SynaraAgentCard? = nil
    ) {
        self.eventID = eventID
        self.senderID = senderID
        self.timestamp = timestamp
        self.type = type
        self.body = body
        self.formattedBody = formattedBody
        self.replyToEventID = replyToEventID
        self.isEdited = isEdited
        self.mediaURL = mediaURL
        self.isEncrypted = isEncrypted
        self.agentCard = agentCard
    }
}

protocol TimelineServicing {
    func loadInitialTimeline(roomID: String) async -> [TimelineItem]
    func loadOlderTimeline(roomID: String, before eventID: String) async -> [TimelineItem]
}

enum TimelineMapper {
    static func map(_ event: RawTimelineEvent) -> TimelineItem {
        let kind: TimelineItem.Kind

        if let agentCard = event.agentCard {
            kind = .agentCard(agentCard)
        } else {
            switch event.type {
        case "m.room.message":
            if let formattedBody = event.formattedBody, formattedBody.isEmpty == false {
                kind = .formattedText(body: event.body ?? "", html: formattedBody)
            } else {
                kind = .text(event.body ?? "")
            }
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
                    requiresAuthentication: true,
                    isEncrypted: event.isEncrypted
                )
            )
        default:
            kind = .unknown(type: event.type)
        }
        }

        return TimelineItem(
            id: event.eventID,
            eventID: event.eventID,
            senderID: event.senderID,
            timestamp: event.timestamp,
            kind: kind,
            replyToEventID: event.replyToEventID,
            isEdited: event.isEdited,
            reactions: [:],
            isEncrypted: event.type == "m.room.encrypted" || event.isEncrypted
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
                body: "Here's the latest spec for the new permissions model. Hello from iOS",
                replyToEventID: nil,
                isEdited: false,
                mediaURL: nil
            ),
            RawTimelineEvent(
                eventID: "$reply:\(roomID)",
                senderID: "@bob:matrix.org",
                timestamp: baseDate.addingTimeInterval(30),
                type: "m.room.message",
                body: "Thanks. A couple of questions inline.",
                replyToEventID: "$text:\(roomID)",
                isEdited: true,
                mediaURL: nil
            ),
            RawTimelineEvent(
                eventID: "$media:\(roomID)",
                senderID: "@alice:matrix.org",
                timestamp: baseDate.addingTimeInterval(45),
                type: "m.room.media",
                body: "permissions-v2.pdf",
                replyToEventID: nil,
                isEdited: false,
                mediaURL: URL(string: "mxc://matrix.org/media-id")
            ),
            RawTimelineEvent(
                eventID: "$security:\(roomID)",
                senderID: "@ravi:matrix.org",
                timestamp: baseDate.addingTimeInterval(60),
                type: "m.room.message",
                body: "From a security perspective this looks good.",
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
    var events: [RawTimelineEvent]
    var itemFixture: [TimelineItem]?

    init(events: [RawTimelineEvent] = TimelineFixtures.commonEvents()) {
        self.events = events
        self.itemFixture = nil
    }

    init(items: [TimelineItem]) {
        self.events = []
        self.itemFixture = items
    }

    func loadInitialTimeline(roomID: String) async -> [TimelineItem] {
        if let itemFixture {
            return itemFixture
        }
        return events.map(TimelineMapper.map)
    }

    func loadOlderTimeline(roomID: String, before eventID: String) async -> [TimelineItem] {
        if let itemFixture {
            return Array(itemFixture.prefix(50))
        }
        return events.filter { $0.eventID != eventID }.map(TimelineMapper.map)
    }
}

enum MatrixHTMLRenderer {
    static func attributedString(body: String, html: String) -> AttributedString {
        let markdown = sanitizedMarkdown(body: body, html: html)
        if let attributed = try? AttributedString(
            markdown: markdown,
            options: AttributedString.MarkdownParsingOptions(interpretedSyntax: .inlineOnlyPreservingWhitespace)
        ) {
            return attributed
        }

        return AttributedString(body)
    }

    static func sanitizedMarkdown(body: String, html: String) -> String {
        var output = html
            .removingHTMLBlocks(named: "script")
            .removingHTMLBlocks(named: "style")

        output = output.replacingAnchorTags()
        output = output.replacingTag("strong", with: "**")
        output = output.replacingTag("b", with: "**")
        output = output.replacingTag("em", with: "*")
        output = output.replacingTag("i", with: "*")
        output = output.replacingTag("code", with: "`")
        output = output.replacingTag("del", with: "~~")
        output = output.replacingHTMLPattern(#"<br\s*/?>"#, with: "\n")
        output = output.replacingHTMLPattern(#"</p\s*>"#, with: "\n\n")
        output = output.replacingHTMLPattern(#"<p(?:\s+[^>]*)?>"#, with: "")
        output = output.replacingHTMLPattern(#"<li(?:\s+[^>]*)?>"#, with: "\n- ")
        output = output.replacingHTMLPattern(#"</li\s*>"#, with: "")
        output = output.replacingHTMLPattern(#"</?(ul|ol)(?:\s+[^>]*)?>"#, with: "\n")
        output = output.replacingHTMLPattern(#"<blockquote(?:\s+[^>]*)?>"#, with: "\n> ")
        output = output.replacingHTMLPattern(#"</blockquote\s*>"#, with: "\n")
        output = output.replacingHTMLPattern(#"<span[^>]*data-mx-spoiler[^>]*>"#, with: "")
        output = output.replacingHTMLPattern(#"</?span(?:\s+[^>]*)?>"#, with: "")
        output = output.replacingHTMLPattern(#"</?[^>]+>"#, with: "")
        output = output.decodingBasicHTMLEntities()
        output = output.replacingOccurrences(of: "\n\n\n", with: "\n\n")
            .trimmingCharacters(in: .whitespacesAndNewlines)

        return output.isEmpty ? body : output
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
        let formattedBody: String?
        let mediaURL: URL?
        let isEncrypted: Bool
        let agentCard = parseAgentCard(from: content)

        if event.type == "m.room.message" {
            if let msgtype = content.msgtype,
               ["m.image", "m.file", "m.audio", "m.video"].contains(msgtype) {
                eventType = "m.room.media"
                body = content.body
                formattedBody = nil
                mediaURL = (content.url ?? content.encryptedFileURL).flatMap(URL.init(string:))
                isEncrypted = content.encryptedFileURL != nil
            } else {
                eventType = event.type
                body = content.body
                formattedBody = content.formattedBody
                mediaURL = nil
                isEncrypted = false
            }
        } else {
            eventType = event.type
            body = content.body
            formattedBody = content.formattedBody
            mediaURL = content.url.flatMap(URL.init(string:))
            isEncrypted = event.type == "m.room.encrypted"
            if eventType.hasPrefix("m.room.") && body?.isEmpty == false {
                // Keep existing behavior for plain message-like room events.
            }
        }

        return RawTimelineEvent(
            eventID: eventID,
            senderID: sender,
            timestamp: Date(timeIntervalSince1970: TimeInterval(event.originServerTimestamp ?? 0) / 1_000),
            type: eventType,
            body: body,
            formattedBody: formattedBody,
            replyToEventID: content.relatesTo?.inReplyTo?.eventID,
            isEdited: content.relatesTo?.relType == "m.replace",
            mediaURL: mediaURL,
            isEncrypted: isEncrypted,
            agentCard: agentCard
        )
    }

    private func shouldShow(eventType: String) -> Bool {
        if ["m.room.message", "m.room.encrypted", "m.room.redaction"].contains(eventType) {
            return true
        }

        if ["org.hermes.agent", "io.hermes.agent", "in.synara.agent", "m.custom.agent"].contains(eventType) {
            return true
        }

        return eventType.hasPrefix("synara.")
    }

    private func parseAgentCard(from content: MatrixTimelineEventContent) -> SynaraAgentCard? {
        SynaraAgentCardPayloadParser.parse(raw: content.raw, body: content.body)
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
    let format: String?
    let formattedBody: String?
    let msgtype: String?
    let url: String?
    let encryptedFileURL: String?
    let relatesTo: MatrixRelatesTo?
    let raw: [String: Any]

    init(
        body: String? = nil,
        format: String? = nil,
        formattedBody: String? = nil,
        msgtype: String? = nil,
        url: String? = nil,
        encryptedFileURL: String? = nil,
        relatesTo: MatrixRelatesTo? = nil,
        raw: [String: Any] = [:]
    ) {
        self.body = body
        self.format = format
        self.formattedBody = formattedBody
        self.msgtype = msgtype
        self.url = url
        self.encryptedFileURL = encryptedFileURL
        self.relatesTo = relatesTo
        self.raw = raw
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        let raw = try container.decode([String: JSONAny].self)
        self.raw = raw.toAnyDictionary()
        self.body = raw["body"]?.string
        self.format = raw["format"]?.string
        let formattedBody = raw["formatted_body"]?.string
        self.formattedBody = raw["format"]?.string == "org.matrix.custom.html" ? formattedBody : nil
        self.msgtype = raw["msgtype"]?.string
        self.url = raw["url"]?.string
        self.encryptedFileURL = raw["file"]?.dictionary?["url"]?.string

        if let relatesToObject = raw["m.relates_to"]?.dictionary {
            let replyEventID = relatesToObject["m.in_reply_to"]?.dictionary?["event_id"]?.string
            self.relatesTo = MatrixRelatesTo(
                relType: relatesToObject["rel_type"]?.string,
                inReplyTo: MatrixInReplyTo(eventID: replyEventID)
            )
        } else {
            self.relatesTo = nil
        }
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

    init(eventID: String?) {
        self.eventID = eventID
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.eventID = try container.decodeIfPresent(String.self, forKey: .eventID)
    }
}

private enum JSONAny: Decodable {
    case dictionary([String: JSONAny])
    case array([JSONAny])
    case string(String)
    case number(Double)
    case bool(Bool)
    case null

    init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()

        if container.decodeNil() {
            self = .null
            return
        }
        if let boolValue = try? container.decode(Bool.self) {
            self = .bool(boolValue)
            return
        }
        if let stringValue = try? container.decode(String.self) {
            self = .string(stringValue)
            return
        }
        if let intValue = try? container.decode(Int.self) {
            self = .number(Double(intValue))
            return
        }
        if let doubleValue = try? container.decode(Double.self) {
            self = .number(doubleValue)
            return
        }
        if let arrayValue = try? container.decode([JSONAny].self) {
            self = .array(arrayValue)
            return
        }
        if let dictionaryValue = try? container.decode([String: JSONAny].self) {
            self = .dictionary(dictionaryValue)
            return
        }

        throw DecodingError.dataCorruptedError(in: container, debugDescription: "Unsupported JSON value")
    }

    var string: String? {
        if case .string(let value) = self { return value }
        return nil
    }

    var bool: Bool? {
        if case .bool(let value) = self { return value }
        return nil
    }

    var dictionary: [String: JSONAny]? {
        if case .dictionary(let value) = self { return value }
        return nil
    }

    var toAny: Any {
        switch self {
        case .string(let value):
            return value
        case .number(let value):
            return value
        case .bool(let value):
            return value
        case .null:
            return NSNull()
        case .array(let array):
            return array.map(\.toAny)
        case .dictionary(let dictionary):
            return dictionary.toAnyDictionary()
        }
    }

    var toAnyDictionary: [String: Any] {
        return toAny as? [String: Any] ?? [:]
    }
}

private extension Dictionary where Key == String, Value == JSONAny {
    func toAnyDictionary() -> [String: Any] {
        return reduce(into: [:]) { object, pair in
            object[pair.key] = pair.value.toAny
        }
    }
}

private extension String {
    func removingHTMLBlocks(named tagName: String) -> String {
        replacingHTMLPattern(#"<\#(tagName)(?:\s+[^>]*)?>[\s\S]*?</\#(tagName)\s*>"#, with: "")
    }

    func replacingTag(_ tagName: String, with marker: String) -> String {
        replacingHTMLPattern(#"<\#(tagName)(?:\s+[^>]*)?>"#, with: marker)
            .replacingHTMLPattern(#"</\#(tagName)\s*>"#, with: marker)
    }

    func replacingAnchorTags() -> String {
        let pattern = #"<a\s+[^>]*href\s*=\s*['"]([^'"]+)['"][^>]*>([\s\S]*?)</a\s*>"#
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return self
        }

        let nsRange = NSRange(startIndex..<endIndex, in: self)
        let matches = regex.matches(in: self, range: nsRange).reversed()
        let source = self as NSString
        let output = NSMutableString(string: self)

        for match in matches {
            guard match.numberOfRanges == 3 else {
                continue
            }

            let href = source.substring(with: match.range(at: 1)).decodingBasicHTMLEntities()
            let label = source.substring(with: match.range(at: 2))
                .replacingHTMLPattern(#"</?[^>]+>"#, with: "")
                .decodingBasicHTMLEntities()
            if href.isSafeMatrixHTMLLink {
                output.replaceCharacters(in: match.range(at: 0), with: "[\(label)](\(href))")
            } else {
                output.replaceCharacters(in: match.range(at: 0), with: label)
            }
        }

        return output as String
    }

    func replacingHTMLPattern(_ pattern: String, with replacement: String) -> String {
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return self
        }
        let nsRange = NSRange(startIndex..<endIndex, in: self)
        return regex.stringByReplacingMatches(in: self, range: nsRange, withTemplate: replacement)
    }

    func decodingBasicHTMLEntities() -> String {
        replacingOccurrences(of: "&amp;", with: "&")
            .replacingOccurrences(of: "&lt;", with: "<")
            .replacingOccurrences(of: "&gt;", with: ">")
            .replacingOccurrences(of: "&quot;", with: "\"")
            .replacingOccurrences(of: "&#39;", with: "'")
            .replacingOccurrences(of: "&apos;", with: "'")
            .decodingNumericHTMLEntities()
    }

    func decodingNumericHTMLEntities() -> String {
        let pattern = #"&#(x?[0-9A-Fa-f]+);"#
        guard let regex = try? NSRegularExpression(pattern: pattern) else {
            return self
        }

        let nsRange = NSRange(startIndex..<endIndex, in: self)
        let matches = regex.matches(in: self, range: nsRange).reversed()
        let source = self as NSString
        let output = NSMutableString(string: self)

        for match in matches {
            guard match.numberOfRanges == 2 else {
                continue
            }

            let rawValue = source.substring(with: match.range(at: 1))
            let radix = rawValue.hasPrefix("x") || rawValue.hasPrefix("X") ? 16 : 10
            let digits = radix == 16 ? String(rawValue.dropFirst()) : rawValue
            guard let scalarValue = UInt32(digits, radix: radix),
                  let scalar = UnicodeScalar(scalarValue) else {
                continue
            }

            output.replaceCharacters(in: match.range(at: 0), with: String(Character(scalar)))
        }

        return output as String
    }

    var isSafeMatrixHTMLLink: Bool {
        guard let components = URLComponents(string: self),
              let scheme = components.scheme?.lowercased() else {
            return false
        }

        return ["https", "http", "matrix"].contains(scheme)
    }
}
