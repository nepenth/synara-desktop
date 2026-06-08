import Foundation

protocol RoomReadMarkerServicing {
    func fullyReadEventID(roomID: String) async -> String?
    func markFullyRead(roomID: String, eventID: String) async -> Bool
}

protocol RoomReadMarkerHTTPClient {
    func data(for request: URLRequest) async throws -> (Data, URLResponse)
}

extension URLSession: RoomReadMarkerHTTPClient {}

final class MatrixRoomReadMarkerService: RoomReadMarkerServicing {
    private let sessionStore: AppSessionStore
    private let httpClient: RoomReadMarkerHTTPClient
    private let jsonDecoder: JSONDecoder

    init(
        sessionStore: AppSessionStore,
        httpClient: RoomReadMarkerHTTPClient = URLSession.shared,
        jsonDecoder: JSONDecoder = JSONDecoder()
    ) {
        self.sessionStore = sessionStore
        self.httpClient = httpClient
        self.jsonDecoder = jsonDecoder
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
}

private struct RoomReadMarkerResponse: Decodable {
    let eventID: String

    enum CodingKeys: String, CodingKey {
        case eventID = "event_id"
    }
}
