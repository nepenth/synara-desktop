import Foundation

protocol RoomReadMarkerServicing {
    func fullyReadEventID(roomID: String) async -> String?
    func markFullyRead(roomID: String, eventID: String) async -> Bool
    /// Returns the SDK-authoritative event ID that was acknowledged.
    func markRoomAsRead(roomID: String) async -> String?
}

protocol RoomReadMarkerHTTPClient {
    func data(for request: URLRequest) async throws -> (Data, URLResponse)
}

extension URLSession: RoomReadMarkerHTTPClient {}

protocol RoomReadMarkerClientStoring: AnyObject {
    func latestEventID(roomID: String, session: AuthenticatedSession) async throws -> String?
    func clearMarkedUnread(roomID: String, session: AuthenticatedSession) async throws
}

enum MatrixServerEventIDPolicy {
    static func canAcknowledge(_ eventID: String) -> Bool {
        eventID.hasPrefix("$")
            && eventID.hasPrefix("$pending-") == false
            && eventID.hasPrefix("$local-") == false
    }
}

final class RoomReadMarkerCache {
    struct Snapshot: Equatable {
        let eventID: String?
    }

    struct FetchToken: Equatable {
        fileprivate let writeRevision: UInt64
        fileprivate let fetchGeneration: UInt64
    }

    private struct Key: Hashable {
        let userID: String
        let roomID: String
    }

    private struct State {
        var eventID: String?
        var writeRevision: UInt64 = 0
        var nextFetchGeneration: UInt64 = 0
        var latestSuccessfulFetchGeneration: UInt64 = 0
    }

    private let lock = NSLock()
    private var states: [Key: State] = [:]

    func snapshot(roomID: String, userID: String) -> Snapshot {
        lock.lock()
        defer { lock.unlock() }
        return Snapshot(eventID: states[Key(userID: userID, roomID: roomID)]?.eventID)
    }

    func beginFetch(roomID: String, userID: String) -> FetchToken {
        lock.lock()
        defer { lock.unlock() }
        let cacheKey = Key(userID: userID, roomID: roomID)
        var state = states[cacheKey] ?? State()
        state.nextFetchGeneration &+= 1
        states[cacheKey] = state
        return FetchToken(
            writeRevision: state.writeRevision,
            fetchGeneration: state.nextFetchGeneration
        )
    }

    @discardableResult
    func publishFetched(
        eventID: String,
        roomID: String,
        userID: String,
        token: FetchToken
    ) -> String {
        lock.lock()
        defer { lock.unlock() }
        let cacheKey = Key(userID: userID, roomID: roomID)
        var state = states[cacheKey] ?? State()
        guard state.writeRevision == token.writeRevision,
              token.fetchGeneration > state.latestSuccessfulFetchGeneration
        else {
            return state.eventID ?? eventID
        }
        state.eventID = eventID
        state.latestSuccessfulFetchGeneration = token.fetchGeneration
        states[cacheKey] = state
        return eventID
    }

    func publishWritten(eventID: String, roomID: String, userID: String) {
        lock.lock()
        defer { lock.unlock() }
        let cacheKey = Key(userID: userID, roomID: roomID)
        var state = states[cacheKey] ?? State()
        state.writeRevision &+= 1
        state.eventID = eventID
        states[cacheKey] = state
    }
}

final class MatrixRoomReadMarkerService: RoomReadMarkerServicing {
    private let sessionStore: AppSessionStore
    private let clientStore: RoomReadMarkerClientStoring?
    private let httpClient: RoomReadMarkerHTTPClient
    private let jsonDecoder: JSONDecoder
    private let logger: LoggingServicing
    private let writeCoordinator = RoomReadMarkerWriteCoordinator()
    private let markerCache = RoomReadMarkerCache()
    private let writeRetryDelaysNanoseconds: [UInt64]

    init(
        sessionStore: AppSessionStore,
        clientStore: RoomReadMarkerClientStoring? = nil,
        httpClient: RoomReadMarkerHTTPClient = URLSession.shared,
        jsonDecoder: JSONDecoder = JSONDecoder(),
        logger: LoggingServicing = AppLogger(),
        writeRetryDelaysNanoseconds: [UInt64] = [0, 250_000_000, 750_000_000]
    ) {
        self.sessionStore = sessionStore
        self.clientStore = clientStore
        self.httpClient = httpClient
        self.jsonDecoder = jsonDecoder
        self.logger = logger
        self.writeRetryDelaysNanoseconds = writeRetryDelaysNanoseconds.isEmpty ? [0] : writeRetryDelaysNanoseconds
    }

    func fullyReadEventID(roomID: String) async -> String? {
        guard case let .signedIn(session) = sessionStore.currentState else {
            return nil
        }
        let sessionEpoch = sessionStore.sessionEpoch
        let fetchToken = markerCache.beginFetch(roomID: roomID, userID: session.userID)

        do {
            var request = URLRequest(url: fullyReadURL(session: session, roomID: roomID))
            request.httpMethod = "GET"
            request.timeoutInterval = 5
            request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")

            let (data, response) = try await httpClient.data(for: request)
            guard sessionStore.currentState == .signedIn(session),
                  sessionStore.sessionEpoch == sessionEpoch
            else {
                return nil
            }
            guard let http = response as? HTTPURLResponse,
                  http.statusCode == 200
            else {
                return markerCache.snapshot(roomID: roomID, userID: session.userID).eventID
            }

            let eventID = try jsonDecoder.decode(RoomReadMarkerResponse.self, from: data).eventID
            return markerCache.publishFetched(
                eventID: eventID,
                roomID: roomID,
                userID: session.userID,
                token: fetchToken
            )
        } catch {
            guard sessionStore.currentState == .signedIn(session),
                  sessionStore.sessionEpoch == sessionEpoch
            else {
                return nil
            }
            return markerCache.snapshot(roomID: roomID, userID: session.userID).eventID
        }
    }

    func markFullyRead(roomID: String, eventID: String) async -> Bool {
        guard case let .signedIn(session) = sessionStore.currentState else {
            return false
        }
        guard MatrixServerEventIDPolicy.canAcknowledge(eventID) else {
            logger.info("Ignoring non-server read marker room=redacted", category: .sync)
            return false
        }
        return await submitReadMarkers(
            roomID: roomID,
            eventID: eventID,
            session: session,
            sessionEpoch: sessionStore.sessionEpoch
        )
    }

    private func submitReadMarkers(
        roomID: String,
        eventID: String,
        session: AuthenticatedSession,
        sessionEpoch: Int
    ) async -> Bool {
        return await writeCoordinator.submit(
            roomID: roomID,
            sessionEpoch: sessionEpoch,
            eventID: eventID
        ) { [weak self] latestEventID in
            guard let self else {
                return false
            }
            return await self.writeReadMarkers(roomID: roomID, eventID: latestEventID, session: session)
        }
    }

    private func writeReadMarkers(
        roomID: String,
        eventID: String,
        session: AuthenticatedSession
    ) async -> Bool {
        guard sessionStore.currentState == .signedIn(session) else {
            return false
        }

        for delay in writeRetryDelaysNanoseconds {
            if delay > 0 {
                try? await Task.sleep(nanoseconds: delay)
            }
            guard Task.isCancelled == false,
                  sessionStore.currentState == .signedIn(session)
            else {
                return false
            }
            if await writeReadMarkersOnce(roomID: roomID, eventID: eventID, session: session) {
                markerCache.publishWritten(eventID: eventID, roomID: roomID, userID: session.userID)
                return true
            }
        }
        return false
    }

    private func writeReadMarkersOnce(
        roomID: String,
        eventID: String,
        session: AuthenticatedSession
    ) async -> Bool {
        do {
            var request = URLRequest(url: readMarkersURL(session: session, roomID: roomID))
            request.httpMethod = "POST"
            request.timeoutInterval = 5
            request.setValue("Bearer \(session.accessToken)", forHTTPHeaderField: "Authorization")
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
            request.httpBody = try JSONSerialization.data(withJSONObject: [
                "m.read": eventID,
                "m.fully_read": eventID,
            ])

            let (_, response) = try await httpClient.data(for: request)
            guard let http = response as? HTTPURLResponse,
                  (200 ... 299).contains(http.statusCode)
            else {
                logger.info("Matrix read marker update failed room=redacted status=\((response as? HTTPURLResponse)?.statusCode ?? -1)", category: .sync)
                return false
            }
            return true
        } catch {
            logger.info("Matrix read marker update failed room=redacted error=\(String(describing: error))", category: .sync)
            return false
        }
    }

    func markRoomAsRead(roomID: String) async -> String? {
        guard case let .signedIn(session) = sessionStore.currentState else {
            return nil
        }
        let sessionEpoch = sessionStore.sessionEpoch

        guard let clientStore else {
            return nil
        }

        do {
            guard let eventID = try await clientStore.latestEventID(roomID: roomID, session: session) else {
                return nil
            }
            guard MatrixServerEventIDPolicy.canAcknowledge(eventID) else {
                return nil
            }

            guard sessionStore.currentState == .signedIn(session),
                  sessionStore.sessionEpoch == sessionEpoch
            else {
                return nil
            }
            guard await submitReadMarkers(
                roomID: roomID,
                eventID: eventID,
                session: session,
                sessionEpoch: sessionEpoch
            ) else {
                return nil
            }
            guard sessionStore.currentState == .signedIn(session),
                  sessionStore.sessionEpoch == sessionEpoch
            else {
                return nil
            }
            try await clientStore.clearMarkedUnread(roomID: roomID, session: session)
            return eventID
        } catch {
            logger.info("Matrix mark-room-read completion failed room=redacted", category: .sync)
            return nil
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
    private struct PendingWrite {
        let id = UUID()
        let sessionEpoch: Int
        var eventID: String
        var waiters: [CheckedContinuation<Bool, Never>]
        let operation: (String) async -> Bool
    }

    private struct RoomState {
        var active: PendingWrite?
        var pending: [PendingWrite] = []
        var isDraining = false
    }

    private var roomStates: [String: RoomState] = [:]

    func submit(
        roomID: String,
        sessionEpoch: Int,
        eventID: String,
        operation: @escaping (String) async -> Bool
    ) async -> Bool {
        await withCheckedContinuation { continuation in
            var state = roomStates[roomID] ?? RoomState()

            if var active = state.active,
               active.sessionEpoch == sessionEpoch,
               active.eventID == eventID
            {
                active.waiters.append(continuation)
                state.active = active
            } else if let matchingIndex = state.pending.firstIndex(where: {
                $0.sessionEpoch == sessionEpoch && $0.eventID == eventID
            }) {
                state.pending[matchingIndex].waiters.append(continuation)
            } else {
                state.pending.append(PendingWrite(
                    sessionEpoch: sessionEpoch,
                    eventID: eventID,
                    waiters: [continuation],
                    operation: operation
                ))
            }

            let shouldStartDrain = state.isDraining == false
            state.isDraining = true
            roomStates[roomID] = state

            if shouldStartDrain {
                Task {
                    await self.drain(roomID: roomID)
                }
            }
        }
    }

    private func drain(
        roomID: String
    ) async {
        while let write = takeNextWrite(roomID: roomID) {
            let succeeded = await write.operation(write.eventID)
            complete(writeID: write.id, roomID: roomID, succeeded: succeeded)
        }
        var state = roomStates[roomID] ?? RoomState()
        state.isDraining = false
        if state.active == nil, state.pending.isEmpty {
            roomStates.removeValue(forKey: roomID)
        } else {
            roomStates[roomID] = state
        }
    }

    private func takeNextWrite(roomID: String) -> PendingWrite? {
        var state = roomStates[roomID] ?? RoomState()
        guard state.active == nil, state.pending.isEmpty == false else {
            return nil
        }
        let write = state.pending.removeFirst()
        state.active = write
        roomStates[roomID] = state
        return write
    }

    private func complete(writeID: UUID, roomID: String, succeeded: Bool) {
        var state = roomStates[roomID] ?? RoomState()
        guard let active = state.active, active.id == writeID else {
            return
        }
        state.active = nil
        roomStates[roomID] = state
        active.waiters.forEach { $0.resume(returning: succeeded) }
    }
}

final class MockRoomReadMarkerService: RoomReadMarkerServicing {
    var eventID: String?

    init(eventID: String? = nil) {
        self.eventID = eventID
    }

    func fullyReadEventID(roomID _: String) async -> String? {
        eventID
    }

    func markFullyRead(roomID _: String, eventID: String) async -> Bool {
        self.eventID = eventID
        return true
    }

    func markRoomAsRead(roomID: String) async -> String? {
        let eventID = "$latest:\(roomID)"
        return await markFullyRead(roomID: roomID, eventID: eventID) ? eventID : nil
    }
}

private struct RoomReadMarkerResponse: Decodable {
    let eventID: String

    enum CodingKeys: String, CodingKey {
        case eventID = "event_id"
    }
}
