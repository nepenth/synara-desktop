import Foundation

enum EventActionType: Equatable {
    case reply
    case edit
    case redact
    case react(String)
}

struct EventActionAvailability: Equatable {
    let canReply: Bool
    let canEdit: Bool
    let canRedact: Bool
    let canReact: Bool
}

protocol EventActionServicing {
    func availability(for item: TimelineItem, currentUserID: String) -> EventActionAvailability
    func apply(_ action: EventActionType, to item: TimelineItem, currentUserID: String, roomID: String) async -> TimelineItem
}

struct MockEventActionService: EventActionServicing {
    func availability(for item: TimelineItem, currentUserID: String) -> EventActionAvailability {
        switch item.kind {
        case .mediaPlaceholder(let resource) where resource.isEncrypted:
            return EventActionAvailability(canReply: false, canEdit: false, canRedact: false, canReact: false)
        case .redacted, .encryptedPlaceholder:
            return EventActionAvailability(canReply: false, canEdit: false, canRedact: false, canReact: false)
        default:
            return EventActionAvailability(
                canReply: true,
                canEdit: item.senderID == currentUserID,
                canRedact: item.senderID == currentUserID,
                canReact: true
            )
        }
    }

    func apply(_ action: EventActionType, to item: TimelineItem, currentUserID: String, roomID: String) async -> TimelineItem {
        switch action {
        case .reply:
            return item
        case .edit:
            return TimelineItem(
                id: item.id,
                eventID: item.eventID,
                senderID: item.senderID,
                timestamp: item.timestamp,
                kind: item.kind,
                replyToEventID: item.replyToEventID,
                isEdited: true,
                reactions: item.reactions,
                isEncrypted: item.isEncrypted
            )
        case .redact:
            return TimelineItem(
                id: item.id,
                eventID: item.eventID,
                senderID: item.senderID,
                timestamp: item.timestamp,
                kind: .redacted,
                replyToEventID: item.replyToEventID,
                isEdited: item.isEdited,
                reactions: [:],
                isEncrypted: item.isEncrypted
            )
        case .react(let reaction):
            var reactions = item.reactions
            reactions[reaction, default: 0] += 1
            return TimelineItem(
                id: item.id,
                eventID: item.eventID,
                senderID: item.senderID,
                timestamp: item.timestamp,
                kind: item.kind,
                replyToEventID: item.replyToEventID,
                isEdited: item.isEdited,
                reactions: reactions,
                isEncrypted: item.isEncrypted
            )
        }
    }
}

final class MatrixEventActionService: EventActionServicing {
    private let sessionStore: AppSessionStore
    private let httpClient: AuthHTTPClient
    private let jsonEncoder: JSONEncoder

    init(
        sessionStore: AppSessionStore,
        httpClient: AuthHTTPClient = URLSession.shared,
        jsonEncoder: JSONEncoder = JSONEncoder()
    ) {
        self.sessionStore = sessionStore
        self.httpClient = httpClient
        self.jsonEncoder = jsonEncoder
    }

    func availability(for item: TimelineItem, currentUserID: String) -> EventActionAvailability {
        MockEventActionService().availability(for: item, currentUserID: currentUserID)
    }

    func apply(_ action: EventActionType, to item: TimelineItem, currentUserID: String, roomID: String) async -> TimelineItem {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return item
        }

        do {
            switch action {
            case .reply, .edit:
                return item
            case .redact:
                try await sendRedaction(session: session, roomID: roomID, eventID: item.eventID)
                return await MockEventActionService().apply(action, to: item, currentUserID: currentUserID, roomID: roomID)
            case .react(let reaction):
                try await sendReaction(session: session, roomID: roomID, eventID: item.eventID, reaction: reaction)
                return await MockEventActionService().apply(action, to: item, currentUserID: currentUserID, roomID: roomID)
            }
        } catch {
            return item
        }
    }

    private func sendRedaction(session: AuthenticatedSession, roomID: String, eventID: String) async throws {
        var request = URLRequest(
            url: roomEventURL(
                homeserverURL: session.homeserverURL,
                roomID: roomID,
                pathComponents: ["redact", eventID, UUID().uuidString]
            )
        )
        request.httpMethod = "PUT"
        request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = Data("{}".utf8)

        try await send(request)
    }

    private func sendReaction(session: AuthenticatedSession, roomID: String, eventID: String, reaction: String) async throws {
        var request = URLRequest(
            url: roomEventURL(
                homeserverURL: session.homeserverURL,
                roomID: roomID,
                pathComponents: ["send", "m.reaction", UUID().uuidString]
            )
        )
        request.httpMethod = "PUT"
        request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try jsonEncoder.encode(MatrixReactionRequest(eventID: eventID, reaction: reaction))

        try await send(request)
    }

    private func send(_ request: URLRequest) async throws {
        let (_, response) = try await httpClient.data(for: request)
        guard let httpResponse = response as? HTTPURLResponse,
              (200...299).contains(httpResponse.statusCode) else {
            throw MessageSendError.failed
        }
    }

    private func roomEventURL(homeserverURL: URL, roomID: String, pathComponents: [String]) -> URL {
        var url = homeserverURL
        url.appendPathComponent("_matrix")
        url.appendPathComponent("client")
        url.appendPathComponent("v3")
        url.appendPathComponent("rooms")
        url.appendPathComponent(roomID)
        for pathComponent in pathComponents {
            url.appendPathComponent(pathComponent)
        }
        return url
    }
}

private struct MatrixReactionRequest: Encodable {
    let relatesTo: MatrixReactionRelatesTo

    init(eventID: String, reaction: String) {
        relatesTo = MatrixReactionRelatesTo(eventID: eventID, reaction: reaction)
    }

    enum CodingKeys: String, CodingKey {
        case relatesTo = "m.relates_to"
    }
}

private struct MatrixReactionRelatesTo: Encodable {
    let relType = "m.annotation"
    let eventID: String
    let reaction: String

    enum CodingKeys: String, CodingKey {
        case relType = "rel_type"
        case eventID = "event_id"
        case reaction = "key"
    }
}
