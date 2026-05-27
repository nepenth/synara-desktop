import Foundation

struct MessageSendRequest: Equatable {
    let roomID: String
    let body: String
    let replyToEventID: String?
    let editEventID: String?
}

enum MessageSendError: LocalizedError, Equatable {
    case emptyMessage
    case failed

    var errorDescription: String? {
        switch self {
        case .emptyMessage:
            return "Enter a message before sending."
        case .failed:
            return "Message could not be sent. Try again."
        }
    }
}

protocol MessageSending {
    func send(_ request: MessageSendRequest) async throws -> TimelineItem
}

final class DraftStore {
    private var drafts: [String: String] = [:]

    func draft(roomID: String) -> String {
        drafts[roomID] ?? ""
    }

    func setDraft(_ draft: String, roomID: String) {
        drafts[roomID] = draft
    }

    func clearDraft(roomID: String) {
        drafts.removeValue(forKey: roomID)
    }
}

struct MockMessageSendService: MessageSending {
    func send(_ request: MessageSendRequest) async throws -> TimelineItem {
        let body = request.body.trimmingCharacters(in: .whitespacesAndNewlines)
        guard body.isEmpty == false else {
            throw MessageSendError.emptyMessage
        }

        let eventID = "$local-\(UUID().uuidString)"
        return TimelineItem(
            id: eventID,
            eventID: eventID,
            senderID: "@local:matrix.org",
            timestamp: Date(),
            kind: .text(body),
            replyToEventID: request.replyToEventID,
            isEdited: request.editEventID != nil,
            reactions: [:]
        )
    }
}

final class MatrixMessageSendService: MessageSending {
    private let sessionStore: AppSessionStore
    private let httpClient: AuthHTTPClient
    private let jsonEncoder: JSONEncoder
    private let jsonDecoder: JSONDecoder

    init(
        sessionStore: AppSessionStore,
        httpClient: AuthHTTPClient = URLSession.shared,
        jsonEncoder: JSONEncoder = JSONEncoder(),
        jsonDecoder: JSONDecoder = JSONDecoder()
    ) {
        self.sessionStore = sessionStore
        self.httpClient = httpClient
        self.jsonEncoder = jsonEncoder
        self.jsonDecoder = jsonDecoder
    }

    func send(_ request: MessageSendRequest) async throws -> TimelineItem {
        let body = request.body.trimmingCharacters(in: .whitespacesAndNewlines)
        guard body.isEmpty == false else {
            throw MessageSendError.emptyMessage
        }

        guard case .signedIn(let session) = sessionStore.currentState else {
            throw MessageSendError.failed
        }

        var urlRequest = URLRequest(
            url: sendURL(
                homeserverURL: session.homeserverURL,
                roomID: request.roomID,
                transactionID: UUID().uuidString
            )
        )
        urlRequest.httpMethod = "PUT"
        urlRequest.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")
        urlRequest.setValue("application/json", forHTTPHeaderField: "Content-Type")
        urlRequest.httpBody = try jsonEncoder.encode(
            MatrixSendMessageRequest(body: body, replyToEventID: request.replyToEventID)
        )

        do {
            let (data, response) = try await httpClient.data(for: urlRequest)
            guard let httpResponse = response as? HTTPURLResponse,
                  httpResponse.statusCode == 200 else {
                throw MessageSendError.failed
            }

            let sendResponse = try jsonDecoder.decode(MatrixSendMessageResponse.self, from: data)
            return TimelineItem(
                id: sendResponse.eventID,
                eventID: sendResponse.eventID,
                senderID: session.userID,
                timestamp: Date(),
                kind: .text(body),
                replyToEventID: request.replyToEventID,
                isEdited: request.editEventID != nil,
                reactions: [:]
            )
        } catch let error as MessageSendError {
            throw error
        } catch {
            throw MessageSendError.failed
        }
    }

    private func sendURL(homeserverURL: URL, roomID: String, transactionID: String) -> URL {
        var url = homeserverURL
        url.appendPathComponent("_matrix")
        url.appendPathComponent("client")
        url.appendPathComponent("v3")
        url.appendPathComponent("rooms")
        url.appendPathComponent(roomID)
        url.appendPathComponent("send")
        url.appendPathComponent("m.room.message")
        url.appendPathComponent(transactionID)
        return url
    }
}

private struct MatrixSendMessageRequest: Encodable {
    let msgtype = "m.text"
    let body: String
    let relatesTo: MatrixSendRelatesTo?

    init(body: String, replyToEventID: String?) {
        self.body = body
        if let replyToEventID {
            relatesTo = MatrixSendRelatesTo(inReplyTo: MatrixSendInReplyTo(eventID: replyToEventID))
        } else {
            relatesTo = nil
        }
    }

    enum CodingKeys: String, CodingKey {
        case msgtype
        case body
        case relatesTo = "m.relates_to"
    }
}

private struct MatrixSendRelatesTo: Encodable {
    let inReplyTo: MatrixSendInReplyTo

    enum CodingKeys: String, CodingKey {
        case inReplyTo = "m.in_reply_to"
    }
}

private struct MatrixSendInReplyTo: Encodable {
    let eventID: String

    enum CodingKeys: String, CodingKey {
        case eventID = "event_id"
    }
}

private struct MatrixSendMessageResponse: Decodable {
    let eventID: String

    enum CodingKeys: String, CodingKey {
        case eventID = "event_id"
    }
}
