import Foundation

protocol RoomReadMarkerServicing {
    func fullyReadEventID(roomID: String) async -> String?
    func markFullyRead(roomID: String, eventID: String) async -> Bool
    func markRoomAsRead(roomID: String) async -> Bool
}

protocol RoomReadMarkerHTTPClient {
    func data(for request: URLRequest) async throws -> (Data, URLResponse)
}

extension URLSession: RoomReadMarkerHTTPClient {}

final class MatrixRoomReadMarkerService: RoomReadMarkerServicing {
    private let sessionStore: AppSessionStore
    private let clientStore: MatrixRustSDKClientStore?
    private let httpClient: RoomReadMarkerHTTPClient
    private let jsonDecoder: JSONDecoder
    private let logger: LoggingServicing

    init(
        sessionStore: AppSessionStore,
        clientStore: MatrixRustSDKClientStore? = nil,
        httpClient: RoomReadMarkerHTTPClient = URLSession.shared,
        jsonDecoder: JSONDecoder = JSONDecoder(),
        logger: LoggingServicing = AppLogger()
    ) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
        self.httpClient = httpClient
        self.jsonDecoder = jsonDecoder
        self.logger = logger
    }

    func fullyReadEventID(roomID: String) async -> String? {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return nil
        }

        do {
            var request = URLRequest(url: fullyReadURL(session: session, roomID: roomID))
            request.httpMethod = "GET"
            request.timeoutInterval = 0.75
            request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")

            let (data, response) = try await httpClient.data(for: request)
            guard let http = response as? HTTPURLResponse,
                  http.statusCode == 200 else {
                return nil
            }

            return try jsonDecoder.decode(RoomReadMarkerResponse.self, from: data).eventID
        } catch {
            return nil
        }
    }

    func markFullyRead(roomID: String, eventID: String) async -> Bool {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return false
        }

        if let clientStore {
            do {
                try await clientStore.markRoomRead(roomID: roomID, session: session)
                return true
            } catch {
                logger.info("Matrix SDK read update failed; using Client-Server API fallback", category: .sync)
            }
        }

        let didSendReadReceipt = await sendReadReceipt(
            session: session,
            roomID: roomID,
            eventID: eventID
        )
        let didSetFullyRead = await setFullyRead(
            session: session,
            roomID: roomID,
            eventID: eventID
        )
        return didSendReadReceipt && didSetFullyRead
    }

    private func sendReadReceipt(
        session: AuthenticatedSession,
        roomID: String,
        eventID: String
    ) async -> Bool {
        do {
            var request = URLRequest(url: readReceiptURL(session: session, roomID: roomID, eventID: eventID))
            request.httpMethod = "POST"
            request.timeoutInterval = 2
            request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
            request.httpBody = Data("{}".utf8)

            let (_, response) = try await httpClient.data(for: request)
            guard let http = response as? HTTPURLResponse,
                  (200...299).contains(http.statusCode) else {
                return false
            }
            return true
        } catch {
            return false
        }
    }

    private func setFullyRead(
        session: AuthenticatedSession,
        roomID: String,
        eventID: String
    ) async -> Bool {
        do {
            var request = URLRequest(url: fullyReadURL(session: session, roomID: roomID))
            request.httpMethod = "PUT"
            request.timeoutInterval = 2
            request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
            request.httpBody = try JSONSerialization.data(withJSONObject: ["event_id": eventID])

            let (_, response) = try await httpClient.data(for: request)
            guard let http = response as? HTTPURLResponse,
                  (200...299).contains(http.statusCode) else {
                return false
            }
            return true
        } catch {
            return false
        }
    }

    func markRoomAsRead(roomID: String) async -> Bool {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return false
        }

        guard let clientStore else {
            return false
        }

        do {
            guard let eventID = try await clientStore.latestEventID(roomID: roomID, session: session) else {
                return false
            }

            return await markFullyRead(roomID: roomID, eventID: eventID)
        } catch {
            return false
        }
    }

    private func fullyReadURL(session: AuthenticatedSession, roomID: String) -> URL {
        var url = session.homeserverURL
        url.appendPathComponent("_matrix")
        url.appendPathComponent("client")
        url.appendPathComponent("v3")
        url.appendPathComponent("user")
        url.appendPathComponent(session.userID)
        url.appendPathComponent("rooms")
        url.appendPathComponent(roomID)
        url.appendPathComponent("account_data")
        url.appendPathComponent("m.fully_read")
        return url
    }

    private func readReceiptURL(session: AuthenticatedSession, roomID: String, eventID: String) -> URL {
        var url = session.homeserverURL
        url.appendPathComponent("_matrix")
        url.appendPathComponent("client")
        url.appendPathComponent("v3")
        url.appendPathComponent("rooms")
        url.appendPathComponent(roomID)
        url.appendPathComponent("receipt")
        url.appendPathComponent("m.read")
        url.appendPathComponent(eventID)
        return url
    }
}

final class MockRoomReadMarkerService: RoomReadMarkerServicing {
    var eventID: String?

    init(eventID: String? = nil) {
        self.eventID = eventID
    }

    func fullyReadEventID(roomID: String) async -> String? {
        eventID
    }

    func markFullyRead(roomID: String, eventID: String) async -> Bool {
        self.eventID = eventID
        return true
    }

    func markRoomAsRead(roomID: String) async -> Bool {
        await markFullyRead(roomID: roomID, eventID: "$latest:\(roomID)")
    }
}

private struct RoomReadMarkerResponse: Decodable {
    let eventID: String

    enum CodingKeys: String, CodingKey {
        case eventID = "event_id"
    }
}
