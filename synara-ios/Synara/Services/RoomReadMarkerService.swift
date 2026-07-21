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
    private let writeCoordinator = RoomReadMarkerWriteCoordinator()

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
        guard case .signedIn = sessionStore.currentState else {
            return false
        }

        return await writeCoordinator.submit(roomID: roomID, eventID: eventID) { [weak self] latestEventID in
            guard let self else {
                return false
            }
            return await self.writeReadMarkers(roomID: roomID, eventID: latestEventID)
        }
    }

    private func writeReadMarkers(roomID: String, eventID: String) async -> Bool {
        guard case .signedIn(let session) = sessionStore.currentState else {
            return false
        }

        do {
            var request = URLRequest(url: readMarkersURL(session: session, roomID: roomID))
            request.httpMethod = "POST"
            request.timeoutInterval = 2
            request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
            request.httpBody = try JSONSerialization.data(withJSONObject: [
                "m.read": eventID,
                "m.fully_read": eventID
            ])

            let (_, response) = try await httpClient.data(for: request)
            guard let http = response as? HTTPURLResponse,
                  (200...299).contains(http.statusCode) else {
                logger.info("Matrix read marker update failed room=redacted status=\((response as? HTTPURLResponse)?.statusCode ?? -1)", category: .sync)
                return false
            }
            return true
        } catch {
            logger.info("Matrix read marker update failed room=redacted error=\(String(describing: error))", category: .sync)
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

    private func readMarkersURL(session: AuthenticatedSession, roomID: String) -> URL {
        var url = session.homeserverURL
        url.appendPathComponent("_matrix")
        url.appendPathComponent("client")
        url.appendPathComponent("v3")
        url.appendPathComponent("rooms")
        url.appendPathComponent(roomID)
        url.appendPathComponent("read_markers")
        return url
    }
}

private actor RoomReadMarkerWriteCoordinator {
    private struct PendingBatch {
        var eventID: String
        var coveredEventIDs: Set<String>
        var waiters: [CheckedContinuation<Bool, Never>]
    }

    private struct RoomState {
        var pending: PendingBatch?
        var isDraining = false
        var successfulEventIDs: [String] = []
        var successfulEventIDSet: Set<String> = []
    }

    private var roomStates: [String: RoomState] = [:]
    private let retainedSuccessfulEventLimit = 256

    func submit(
        roomID: String,
        eventID: String,
        operation: @escaping (String) async -> Bool
    ) async -> Bool {
        await withCheckedContinuation { continuation in
            var state = roomStates[roomID] ?? RoomState()
            if state.successfulEventIDSet.contains(eventID) {
                continuation.resume(returning: true)
                return
            }

            if var pending = state.pending {
                pending.eventID = eventID
                pending.coveredEventIDs.insert(eventID)
                pending.waiters.append(continuation)
                state.pending = pending
            } else {
                state.pending = PendingBatch(
                    eventID: eventID,
                    coveredEventIDs: [eventID],
                    waiters: [continuation]
                )
            }

            let shouldStartDrain = state.isDraining == false
            state.isDraining = true
            roomStates[roomID] = state

            if shouldStartDrain {
                Task {
                    await self.drain(roomID: roomID, operation: operation)
                }
            }
        }
    }

    private func drain(
        roomID: String,
        operation: @escaping (String) async -> Bool
    ) async {
        while let batch = takePendingBatch(roomID: roomID) {
            let succeeded = await operation(batch.eventID)
            complete(batch: batch, roomID: roomID, succeeded: succeeded)
        }
        var state = roomStates[roomID] ?? RoomState()
        state.isDraining = false
        roomStates[roomID] = state
    }

    private func takePendingBatch(roomID: String) -> PendingBatch? {
        var state = roomStates[roomID] ?? RoomState()
        let batch = state.pending
        state.pending = nil
        roomStates[roomID] = state
        return batch
    }

    private func complete(batch: PendingBatch, roomID: String, succeeded: Bool) {
        var state = roomStates[roomID] ?? RoomState()
        if succeeded {
            for eventID in batch.coveredEventIDs where state.successfulEventIDSet.insert(eventID).inserted {
                state.successfulEventIDs.append(eventID)
                if state.successfulEventIDs.count > retainedSuccessfulEventLimit {
                    let expired = state.successfulEventIDs.removeFirst()
                    state.successfulEventIDSet.remove(expired)
                }
            }
        }
        roomStates[roomID] = state
        batch.waiters.forEach { $0.resume(returning: succeeded) }
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
