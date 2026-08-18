import Foundation
import Security
import UserNotifications

final class NotificationService: UNNotificationServiceExtension {
    private var contentHandler: ((UNNotificationContent) -> Void)?
    private var bestAttemptContent: UNMutableNotificationContent?
    private var enrichmentTask: Task<Void, Never>?
    private let deliveryQueue = DispatchQueue(label: "com.whylandcreative.synara.notification-service.delivery")
    private var didDeliver = false

    override func didReceive(
        _ request: UNNotificationRequest,
        withContentHandler contentHandler: @escaping (UNNotificationContent) -> Void
    ) {
        self.contentHandler = contentHandler

        guard let content = request.content.mutableCopy() as? UNMutableNotificationContent else {
            contentHandler(request.content)
            return
        }

        bestAttemptContent = content

        guard SynaraNotificationPreviewPreference.isEnabled(),
              let payload = SynaraNotificationPreviewPayloadParser.payload(from: request.content.userInfo),
              payload.isAgentApproval == false else {
            deliver(content)
            return
        }

        let resolver = MatrixNotificationPreviewResolver()
        enrichmentTask = Task { [weak self] in
            guard let self else { return }
            if let preview = await resolver.preview(for: payload) {
                content.title = preview.title
                content.body = preview.body
            }
            self.deliver(content)
        }
    }

    override func serviceExtensionTimeWillExpire() {
        enrichmentTask?.cancel()
        if let bestAttemptContent {
            deliver(bestAttemptContent)
        }
    }

    private func deliver(_ content: UNNotificationContent) {
        deliveryQueue.sync {
            guard didDeliver == false else { return }
            didDeliver = true
            let handler = contentHandler
            contentHandler = nil
            handler?(content)
        }
    }
}

private struct StoredSynaraSession: Decodable {
    let homeserverURL: URL
    let accessToken: String
}

private struct StoredSynaraSessionEnvelope: Decodable {
    let version: Int
    let session: StoredSynaraSession
}

private struct NotificationSessionStore {
    private let service = "com.whylandcreative.synara.session"
    private let account = "current"

    func load() -> StoredSynaraSession? {
        guard let accessGroup = Self.sharedAccessGroup() else {
            return nil
        }

        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecAttrAccessGroup as String: accessGroup,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]

        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        guard status == errSecSuccess, let data = item as? Data else {
            return nil
        }

        if let envelope = try? JSONDecoder().decode(StoredSynaraSessionEnvelope.self, from: data),
           envelope.version == 1 {
            return envelope.session
        }

        return try? JSONDecoder().decode(StoredSynaraSession.self, from: data)
    }

    private static func sharedAccessGroup(bundle: Bundle = .main) -> String? {
        guard let value = bundle.object(forInfoDictionaryKey: SynaraSharedConstants.keychainAccessGroupInfoKey) as? String else {
            return nil
        }

        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.isEmpty == false,
              trimmed.contains("$(") == false else {
            return nil
        }

        return trimmed
    }
}

private struct MatrixNotificationPreviewResolver {
    private static let maximumEventResponseBytes = 256 * 1024
    private let sessionStore: NotificationSessionStore
    private let urlSession: URLSession

    init(
        sessionStore: NotificationSessionStore = NotificationSessionStore(),
        urlSession: URLSession = .synaraNotificationPreview
    ) {
        self.sessionStore = sessionStore
        self.urlSession = urlSession
    }

    func preview(for payload: SynaraNotificationPreviewPayload) async -> SynaraNotificationPreview? {
        guard Task.isCancelled == false,
              let session = sessionStore.load(),
              let url = eventURL(
                homeserverURL: session.homeserverURL,
                roomID: payload.roomID,
                eventID: payload.eventID
              ) else {
            return nil
        }

        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")
        request.timeoutInterval = 12

        do {
            let (bytes, response) = try await urlSession.bytes(for: request)
            guard Task.isCancelled == false,
                  let httpResponse = response as? HTTPURLResponse,
                  (200..<300).contains(httpResponse.statusCode),
                  httpResponse.expectedContentLength < 0 ||
                    httpResponse.expectedContentLength <= Int64(Self.maximumEventResponseBytes) else {
                return nil
            }

            var data = Data()
            data.reserveCapacity(
                min(max(0, Int(httpResponse.expectedContentLength)), Self.maximumEventResponseBytes)
            )
            for try await byte in bytes {
                guard Task.isCancelled == false,
                      data.count < Self.maximumEventResponseBytes else {
                    return nil
                }
                data.append(byte)
            }

            let event = try JSONDecoder().decode(MatrixEventResponse.self, from: data)
            return SynaraMatrixEventPreviewComposer.preview(
                from: SynaraMatrixEventPreviewInput(
                    eventType: event.type,
                    senderID: event.sender,
                    body: event.content.body,
                    messageType: event.content.msgtype
                )
            )
        } catch {
            return nil
        }
    }

    private func eventURL(homeserverURL: URL, roomID: String, eventID: String) -> URL? {
        guard var components = URLComponents(url: homeserverURL, resolvingAgainstBaseURL: false) else {
            return nil
        }

        let basePath = components.percentEncodedPath.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        let encodedRoomID = Self.encodePathSegment(roomID)
        let encodedEventID = Self.encodePathSegment(eventID)
        components.percentEncodedPath = "/" + ([basePath, "_matrix/client/v3/rooms", encodedRoomID, "event", encodedEventID]
            .filter { $0.isEmpty == false }
            .joined(separator: "/"))
        components.query = nil
        components.fragment = nil
        return components.url
    }

    private static func encodePathSegment(_ value: String) -> String {
        let allowed = CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "-._~"))
        return value.addingPercentEncoding(withAllowedCharacters: allowed) ?? value
    }
}

private struct MatrixEventResponse: Decodable {
    let type: String
    let sender: String?
    let content: MatrixEventContent
}

private struct MatrixEventContent: Decodable {
    let body: String?
    let msgtype: String?
}

private extension URLSession {
    static let synaraNotificationPreview: URLSession = {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = 12
        configuration.timeoutIntervalForResource = 18
        configuration.waitsForConnectivity = false
        return URLSession(configuration: configuration)
    }()
}
