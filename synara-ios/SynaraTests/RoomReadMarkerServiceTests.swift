import Foundation
@testable import Synara
import XCTest

final class RoomReadMarkerServiceTests: XCTestCase {
    func testReadMarkerReturnsNilWhenSignedOut() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: #"{"event_id":"$event"}"#)
        let service = MatrixRoomReadMarkerService(sessionStore: AppSessionStore(), httpClient: http)

        let eventID = await service.fullyReadEventID(roomID: "!room:matrix.example")

        XCTAssertNil(eventID)
        XCTAssertNil(http.lastRequest)
    }

    func testReadMarkerReadsFullyReadAccountData() async throws {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: #"{"event_id":"$event:matrix.example"}"#)
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(
            sessionStore: sessionStore,
            httpClient: http,
            writeRetryDelaysNanoseconds: [0]
        )

        let eventID = await service.fullyReadEventID(roomID: "!room:matrix.example")

        XCTAssertEqual(eventID, "$event:matrix.example")
        let request = try XCTUnwrap(http.lastRequest)
        XCTAssertEqual(request.httpMethod, "GET")
        XCTAssertEqual(request.value(forHTTPHeaderField: "Authorization"), "Bearer token")
        XCTAssertTrue(try XCTUnwrap(request.url?.absoluteString).contains("/_matrix/client/v3/user/"))
        XCTAssertTrue(try XCTUnwrap(request.url?.absoluteString).contains("/account_data/m.fully_read"))
    }

    func testReadMarkerReturnsNilForNonSuccessStatus() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 404, body: #"{"errcode":"M_NOT_FOUND"}"#)
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(sessionStore: sessionStore, httpClient: http)

        let eventID = await service.fullyReadEventID(roomID: "!room:matrix.example")

        XCTAssertNil(eventID)
    }

    func testReadMarkerReturnsNilForMalformedPayload() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: #"{"event_id":42}"#)
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(sessionStore: sessionStore, httpClient: http)

        let eventID = await service.fullyReadEventID(roomID: "!room:matrix.example")

        XCTAssertNil(eventID)
    }

    func testReadMarkerFallsBackToLastSuccessfulValueAfterTransportFailure() async {
        let http = SequencedReadMarkerHTTPClient(
            statusCodes: [200, 503],
            bodies: [#"{"event_id":"$cached:matrix.example"}"#, "{}"]
        )
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(sessionStore: sessionStore, httpClient: http)

        let fetched = await service.fullyReadEventID(roomID: "!room:matrix.example")
        let fallback = await service.fullyReadEventID(roomID: "!room:matrix.example")

        XCTAssertEqual(fetched, "$cached:matrix.example")
        XCTAssertEqual(fallback, fetched)
    }

    func testStaleFetchCannotOverwriteMarkerWrittenWhileRequestWasInFlight() {
        let cache = RoomReadMarkerCache()
        let token = cache.beginFetch(roomID: "!room:matrix.example", userID: "@alice:matrix.example")
        cache.publishWritten(
            eventID: "$written",
            roomID: "!room:matrix.example",
            userID: "@alice:matrix.example"
        )

        let effectiveEventID = cache.publishFetched(
            eventID: "$stale-fetch",
            roomID: "!room:matrix.example",
            userID: "@alice:matrix.example",
            token: token
        )

        XCTAssertEqual(effectiveEventID, "$written")
        XCTAssertEqual(
            cache.snapshot(roomID: "!room:matrix.example", userID: "@alice:matrix.example").eventID,
            "$written"
        )
    }

    func testLatestIssuedFetchWinsRegardlessOfCompletionOrder() {
        let olderCompletesFirst = RoomReadMarkerCache()
        let olderToken = olderCompletesFirst.beginFetch(roomID: "!room", userID: "@alice")
        let newerToken = olderCompletesFirst.beginFetch(roomID: "!room", userID: "@alice")
        _ = olderCompletesFirst.publishFetched(
            eventID: "$older",
            roomID: "!room",
            userID: "@alice",
            token: olderToken
        )
        _ = olderCompletesFirst.publishFetched(
            eventID: "$newer",
            roomID: "!room",
            userID: "@alice",
            token: newerToken
        )
        XCTAssertEqual(olderCompletesFirst.snapshot(roomID: "!room", userID: "@alice").eventID, "$newer")

        let newerCompletesFirst = RoomReadMarkerCache()
        let delayedOlderToken = newerCompletesFirst.beginFetch(roomID: "!room", userID: "@alice")
        let fastNewerToken = newerCompletesFirst.beginFetch(roomID: "!room", userID: "@alice")
        _ = newerCompletesFirst.publishFetched(
            eventID: "$newer",
            roomID: "!room",
            userID: "@alice",
            token: fastNewerToken
        )
        let delayedResult = newerCompletesFirst.publishFetched(
            eventID: "$older",
            roomID: "!room",
            userID: "@alice",
            token: delayedOlderToken
        )
        XCTAssertEqual(delayedResult, "$newer")
        XCTAssertEqual(newerCompletesFirst.snapshot(roomID: "!room", userID: "@alice").eventID, "$newer")

        let newerFetchFails = RoomReadMarkerCache()
        let viableOlderToken = newerFetchFails.beginFetch(roomID: "!room", userID: "@alice")
        _ = newerFetchFails.beginFetch(roomID: "!room", userID: "@alice")
        let viableResult = newerFetchFails.publishFetched(
            eventID: "$older-but-successful",
            roomID: "!room",
            userID: "@alice",
            token: viableOlderToken
        )
        XCTAssertEqual(viableResult, "$older-but-successful")
        XCTAssertEqual(
            newerFetchFails.snapshot(roomID: "!room", userID: "@alice").eventID,
            "$older-but-successful"
        )
    }

    func testCacheUsesStructuredUserAndRoomIdentity() {
        let cache = RoomReadMarkerCache()
        cache.publishWritten(eventID: "$first", roomID: "c", userID: "a|b")
        cache.publishWritten(eventID: "$second", roomID: "b|c", userID: "a")

        XCTAssertEqual(cache.snapshot(roomID: "c", userID: "a|b").eventID, "$first")
        XCTAssertEqual(cache.snapshot(roomID: "b|c", userID: "a").eventID, "$second")
    }

    func testMarkFullyReadReturnsFalseWhenSignedOut() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: #"{"event_id":"$event"}"#)
        let service = MatrixRoomReadMarkerService(sessionStore: AppSessionStore(), httpClient: http)

        let didMark = await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$event:matrix.example")

        XCTAssertFalse(didMark)
        XCTAssertNil(http.lastRequest)
    }

    func testLocalAndTransactionIdentifiersNeverReachReadMarkerWriter() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: "{}")
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(sessionStore: sessionStore, httpClient: http)

        let pendingResult = await service.markFullyRead(
            roomID: "!room:matrix.example",
            eventID: "$pending-local"
        )
        let transactionResult = await service.markFullyRead(
            roomID: "!room:matrix.example",
            eventID: "transaction-123"
        )

        XCTAssertFalse(pendingResult)
        XCTAssertFalse(transactionResult)
        XCTAssertTrue(http.requests.isEmpty)
    }

    func testMarkFullyReadUsesExactEventReadMarkersEndpoint() async throws {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: #"{"event_id":"$event:matrix.example"}"#)
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(sessionStore: sessionStore, httpClient: http)

        let didMark = await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$event:matrix.example")

        XCTAssertTrue(didMark)
        XCTAssertEqual(http.requests.count, 1)
        let request = try XCTUnwrap(http.lastRequest)
        XCTAssertEqual(request.httpMethod, "POST")
        XCTAssertEqual(request.value(forHTTPHeaderField: "Authorization"), "Bearer token")
        XCTAssertEqual(request.value(forHTTPHeaderField: "Content-Type"), "application/json")
        XCTAssertTrue(try XCTUnwrap(request.url?.absoluteString).contains("/rooms/!room:matrix.example/read_markers"))
        let body = try XCTUnwrap(request.httpBody)
        let payload = try XCTUnwrap(JSONSerialization.jsonObject(with: body) as? [String: String])
        XCTAssertEqual(payload["m.read"], "$event:matrix.example")
        XCTAssertEqual(payload["m.fully_read"], "$event:matrix.example")
        XCTAssertNil(payload["event_id"])
    }

    func testMarkFullyReadResendsSuccessfulEventWithoutCrossSessionMemoization() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: "{}")
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(sessionStore: sessionStore, httpClient: http)

        let firstResult = await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$event:matrix.example")
        let duplicateResult = await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$event:matrix.example")

        XCTAssertTrue(firstResult)
        XCTAssertTrue(duplicateResult)
        XCTAssertEqual(http.requests.count, 2)
    }

    func testConcurrentDifferentMarkersRemainSerialAndSettleExactWaiters() async throws {
        let http = GatedReadMarkerHTTPClient()
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(
            sessionStore: sessionStore,
            httpClient: http,
            writeRetryDelaysNanoseconds: [0]
        )

        let first = Task { await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$first") }
        await http.waitUntilRequestCount(1)
        let second = Task { await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$second") }
        await Task.yield()
        let third = Task { await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$third") }
        await Task.yield()

        let activeRequestCount = await http.requestCount()
        XCTAssertEqual(activeRequestCount, 1)
        await http.completeNext(statusCode: 200)
        await http.waitUntilRequestCount(2)
        let secondNetworkEventID = try await http.nextPendingEventID()
        await http.completeNext(statusCode: secondNetworkEventID == "$second" ? 500 : 200)
        await http.waitUntilRequestCount(3)
        let thirdNetworkEventID = try await http.nextPendingEventID()
        await http.completeNext(statusCode: thirdNetworkEventID == "$second" ? 500 : 200)

        let firstResult = await first.value
        let secondResult = await second.value
        let thirdResult = await third.value
        let submittedEventIDs = try await http.submittedEventIDs()
        XCTAssertTrue(firstResult)
        XCTAssertFalse(secondResult)
        XCTAssertTrue(thirdResult)
        XCTAssertEqual(submittedEventIDs.first, "$first")
        XCTAssertEqual(Set(submittedEventIDs.dropFirst()), Set(["$second", "$third"]))
        XCTAssertEqual(submittedEventIDs.count, 3)
    }

    func testIdenticalMarkerRequestsCoalesceOnlyWhileAdjacentAndInFlight() async throws {
        let http = GatedReadMarkerHTTPClient()
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(sessionStore: sessionStore, httpClient: http)

        let first = Task { await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$same") }
        await http.waitUntilRequestCount(1)
        let duplicate = Task { await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$same") }
        await Task.yield()
        await http.completeNext(statusCode: 200)

        let firstResult = await first.value
        let duplicateResult = await duplicate.value
        let submittedEventIDs = try await http.submittedEventIDs()
        XCTAssertTrue(firstResult)
        XCTAssertTrue(duplicateResult)
        XCTAssertEqual(submittedEventIDs, ["$same"])
    }

    func testActiveExactMarkerDeduplicatesAcrossDifferentPendingMarker() async throws {
        let http = GatedReadMarkerHTTPClient()
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(
            sessionStore: sessionStore,
            httpClient: http,
            writeRetryDelaysNanoseconds: [0]
        )

        let active = Task { await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$older") }
        await http.waitUntilRequestCount(1)
        let pending = Task { await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$newer") }
        await Task.yield()
        let repeatedActive = Task {
            await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$older")
        }
        try? await Task.sleep(nanoseconds: 10_000_000)

        await http.completeNext(statusCode: 200)
        let repeatedResult = await repeatedActive.value
        await http.waitUntilRequestCount(2)
        await http.completeNext(statusCode: 200)

        let results = await [active.value, pending.value]
        let submittedEventIDs = try await http.submittedEventIDs()
        XCTAssertTrue(repeatedResult)
        XCTAssertEqual(results, [true, true])
        XCTAssertEqual(submittedEventIDs, ["$older", "$newer"])
    }

    func testCoalescedWaitersShareFailureAndLaterRetryStillWrites() async throws {
        let http = GatedReadMarkerHTTPClient()
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(
            sessionStore: sessionStore,
            httpClient: http,
            writeRetryDelaysNanoseconds: [0]
        )

        let first = Task { await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$same") }
        await http.waitUntilRequestCount(1)
        let duplicate = Task { await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$same") }
        await Task.yield()
        await http.completeNext(statusCode: 500)
        let firstResult = await first.value
        let duplicateResult = await duplicate.value

        let retry = Task { await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$same") }
        await http.waitUntilRequestCount(2)
        await http.completeNext(statusCode: 200)
        let retryResult = await retry.value

        let submittedEventIDs = try await http.submittedEventIDs()
        XCTAssertFalse(firstResult)
        XCTAssertFalse(duplicateResult)
        XCTAssertTrue(retryResult)
        XCTAssertEqual(submittedEventIDs, ["$same", "$same"])
    }

    func testLiveBottomAcknowledgementNeverReusesStaleUICandidateAfterSDKTailAdvances() async throws {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: "{}")
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let clientStore = RecordingReadMarkerClientStore(latestEventID: "$older")
        let service = MatrixRoomReadMarkerService(
            sessionStore: sessionStore,
            clientStore: clientStore,
            httpClient: http
        )

        let first = await service.markRoomAsRead(roomID: "!room:matrix.example")
        await clientStore.setLatestEventID("$newer")
        let newer = await service.markRoomAsRead(roomID: "!room:matrix.example")
        // This represents a delayed live-bottom task whose UI candidate was old.
        // markRoomAsRead resolves the SDK tail again, so it cannot submit $older.
        let staleTask = await service.markRoomAsRead(roomID: "!room:matrix.example")

        let submittedEventIDs = try http.requests.map { request in
            let body = try XCTUnwrap(request.httpBody)
            let payload = try XCTUnwrap(JSONSerialization.jsonObject(with: body) as? [String: String])
            return try XCTUnwrap(payload["m.fully_read"])
        }
        XCTAssertEqual([first, newer, staleTask], ["$older", "$newer", "$newer"])
        XCTAssertEqual(submittedEventIDs, ["$older", "$newer", "$newer"])
    }

    func testFailedMarkerCanRetryTheSameExactEvent() async throws {
        let http = GatedReadMarkerHTTPClient()
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(
            sessionStore: sessionStore,
            httpClient: http,
            writeRetryDelaysNanoseconds: [0]
        )

        let first = Task { await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$retry") }
        await http.waitUntilRequestCount(1)
        await http.completeNext(statusCode: 503)
        let firstResult = await first.value
        let retry = Task { await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$retry") }
        await http.waitUntilRequestCount(2)
        await http.completeNext(statusCode: 200)
        let retryResult = await retry.value

        let submittedEventIDs = try await http.submittedEventIDs()
        XCTAssertFalse(firstResult)
        XCTAssertTrue(retryResult)
        XCTAssertEqual(submittedEventIDs, ["$retry", "$retry"])
    }

    func testMarkerWriteRetriesTransientFailureWithinBoundedAttemptBudget() async {
        let http = SequencedReadMarkerHTTPClient(statusCodes: [503, 200])
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(
            sessionStore: sessionStore,
            httpClient: http,
            writeRetryDelaysNanoseconds: [0, 0]
        )

        let didMark = await service.markFullyRead(
            roomID: "!room:matrix.example",
            eventID: "$retry-once"
        )

        let requestCount = await http.requestCount()
        XCTAssertTrue(didMark)
        XCTAssertEqual(requestCount, 2)
    }

    func testLogoutAndLoginResendsSameMarkerWithNewSession() async throws {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: "{}")
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession(accessToken: "first-token")))
        let service = MatrixRoomReadMarkerService(sessionStore: sessionStore, httpClient: http)

        let firstResult = await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$event")
        try sessionStore.signOut()
        try sessionStore.completeLogin(makeSession(accessToken: "second-token"))
        let secondResult = await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$event")

        XCTAssertTrue(firstResult)
        XCTAssertTrue(secondResult)
        XCTAssertEqual(http.requests.count, 2)
        XCTAssertEqual(http.requests[0].value(forHTTPHeaderField: "Authorization"), "Bearer first-token")
        XCTAssertEqual(http.requests[1].value(forHTTPHeaderField: "Authorization"), "Bearer second-token")
    }

    func testNewSessionMarkerDoesNotCoalesceWithSameEventInFlightFromOldSession() async throws {
        let http = GatedReadMarkerHTTPClient()
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession(accessToken: "first-token")))
        let service = MatrixRoomReadMarkerService(sessionStore: sessionStore, httpClient: http)

        let oldSessionWrite = Task {
            await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$same-event")
        }
        await http.waitUntilRequestCount(1)
        try sessionStore.signOut()
        try sessionStore.completeLogin(makeSession(accessToken: "second-token"))
        let newSessionWrite = Task {
            await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$same-event")
        }
        await Task.yield()

        let requestCountWhileOldWriteIsActive = await http.requestCount()
        XCTAssertEqual(requestCountWhileOldWriteIsActive, 1)
        await http.completeNext(statusCode: 200)
        await http.waitUntilRequestCount(2)
        await http.completeNext(statusCode: 200)

        let oldResult = await oldSessionWrite.value
        let newResult = await newSessionWrite.value
        let authorizationHeaders = await http.authorizationHeaders()
        XCTAssertTrue(oldResult)
        XCTAssertTrue(newResult)
        XCTAssertEqual(authorizationHeaders, ["Bearer first-token", "Bearer second-token"])
    }

    func testMarkFullyReadReturnsFalseForNonSuccessStatus() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 500, body: #"{"errcode":"M_UNKNOWN"}"#)
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let service = MatrixRoomReadMarkerService(
            sessionStore: sessionStore,
            httpClient: http,
            writeRetryDelaysNanoseconds: [0]
        )

        let didMark = await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$event:matrix.example")

        XCTAssertFalse(didMark)
    }

    func testMarkRoomAsReadClearsExplicitUnreadOnlyAfterMarkersSucceed() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: "{}")
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let clientStore = RecordingReadMarkerClientStore(latestEventID: "$latest")
        let service = MatrixRoomReadMarkerService(
            sessionStore: sessionStore,
            clientStore: clientStore,
            httpClient: http,
            writeRetryDelaysNanoseconds: [0]
        )

        let acknowledgedEventID = await service.markRoomAsRead(roomID: "!room:matrix.example")

        let clearedRoomIDs = await clientStore.clearedRoomIDs
        XCTAssertEqual(acknowledgedEventID, "$latest")
        XCTAssertEqual(clearedRoomIDs, ["!room:matrix.example"])
    }

    func testMidTimelineMarkNeverClearsExplicitUnread() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: "{}")
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let clientStore = RecordingReadMarkerClientStore(latestEventID: "$latest")
        let service = MatrixRoomReadMarkerService(
            sessionStore: sessionStore,
            clientStore: clientStore,
            httpClient: http,
            writeRetryDelaysNanoseconds: [0]
        )

        let didMark = await service.markFullyRead(roomID: "!room:matrix.example", eventID: "$middle")

        let clearedRoomIDs = await clientStore.clearedRoomIDs
        XCTAssertTrue(didMark)
        XCTAssertTrue(clearedRoomIDs.isEmpty)
    }

    func testMarkRoomAsReadDoesNotClearExplicitUnreadWhenMarkerFails() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 500, body: "{}")
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let clientStore = RecordingReadMarkerClientStore(latestEventID: "$latest")
        let service = MatrixRoomReadMarkerService(
            sessionStore: sessionStore,
            clientStore: clientStore,
            httpClient: http,
            writeRetryDelaysNanoseconds: [0]
        )

        let acknowledgedEventID = await service.markRoomAsRead(roomID: "!room:matrix.example")

        let clearedRoomIDs = await clientStore.clearedRoomIDs
        XCTAssertNil(acknowledgedEventID)
        XCTAssertTrue(clearedRoomIDs.isEmpty)
    }

    func testMarkRoomAsReadDoesNotClearExplicitUnreadWithoutLatestEvent() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: "{}")
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let clientStore = RecordingReadMarkerClientStore(latestEventID: nil)
        let service = MatrixRoomReadMarkerService(
            sessionStore: sessionStore,
            clientStore: clientStore,
            httpClient: http
        )

        let acknowledgedEventID = await service.markRoomAsRead(roomID: "!room:matrix.example")

        let clearedRoomIDs = await clientStore.clearedRoomIDs
        XCTAssertNil(acknowledgedEventID)
        XCTAssertTrue(clearedRoomIDs.isEmpty)
        XCTAssertTrue(http.requests.isEmpty)
    }

    func testMarkRoomAsReadReportsFailureWhenExplicitUnreadCannotBeCleared() async {
        let http = RecordingReadMarkerHTTPClient(statusCode: 200, body: "{}")
        let sessionStore = AppSessionStore(currentState: .signedIn(makeSession()))
        let clientStore = RecordingReadMarkerClientStore(latestEventID: "$latest", failsClear: true)
        let service = MatrixRoomReadMarkerService(
            sessionStore: sessionStore,
            clientStore: clientStore,
            httpClient: http
        )

        let acknowledgedEventID = await service.markRoomAsRead(roomID: "!room:matrix.example")

        let clearAttempts = await clientStore.clearAttempts
        XCTAssertNil(acknowledgedEventID)
        XCTAssertEqual(clearAttempts, 1)
    }

    func testMockMarkRoomAsReadUsesLatestEventMarker() async {
        let service = MockRoomReadMarkerService()

        let acknowledgedEventID = await service.markRoomAsRead(roomID: "!room:matrix.example")

        XCTAssertEqual(acknowledgedEventID, "$latest:!room:matrix.example")
        XCTAssertEqual(service.eventID, "$latest:!room:matrix.example")
    }

    private func makeSession(accessToken: String = "token") -> AuthenticatedSession {
        AuthenticatedSession(
            userID: "@test:matrix.example",
            deviceID: "DEVICE",
            homeserverURL: URL(string: "https://matrix.example")!,
            accessToken: accessToken
        )
    }
}

private actor RecordingReadMarkerClientStore: RoomReadMarkerClientStoring {
    private var latestEventIDValue: String?
    let failsClear: Bool
    private(set) var clearedRoomIDs: [String] = []
    private(set) var clearAttempts = 0

    init(latestEventID: String?, failsClear: Bool = false) {
        latestEventIDValue = latestEventID
        self.failsClear = failsClear
    }

    func latestEventID(roomID _: String, session _: AuthenticatedSession) async throws -> String? {
        latestEventIDValue
    }

    func setLatestEventID(_ eventID: String?) {
        latestEventIDValue = eventID
    }

    func clearMarkedUnread(roomID: String, session _: AuthenticatedSession) async throws {
        clearAttempts += 1
        if failsClear {
            throw URLError(.cannotWriteToFile)
        }
        clearedRoomIDs.append(roomID)
    }
}

private actor GatedReadMarkerHTTPClient: RoomReadMarkerHTTPClient {
    private struct PendingRequest {
        let request: URLRequest
        let continuation: CheckedContinuation<(Data, URLResponse), Error>
    }

    private struct CountWaiter {
        let count: Int
        let continuation: CheckedContinuation<Void, Never>
    }

    private var requests: [URLRequest] = []
    private var pendingRequests: [PendingRequest] = []
    private var countWaiters: [CountWaiter] = []

    func data(for request: URLRequest) async throws -> (Data, URLResponse) {
        try await withCheckedThrowingContinuation { continuation in
            requests.append(request)
            pendingRequests.append(PendingRequest(request: request, continuation: continuation))
            settleCountWaiters()
        }
    }

    func waitUntilRequestCount(_ count: Int) async {
        guard requests.count < count else {
            return
        }
        await withCheckedContinuation { continuation in
            countWaiters.append(CountWaiter(count: count, continuation: continuation))
        }
    }

    func requestCount() -> Int {
        requests.count
    }

    func completeNext(statusCode: Int, body: String = "{}") {
        guard pendingRequests.isEmpty == false else {
            return
        }
        let pending = pendingRequests.removeFirst()
        let url = pending.request.url ?? URL(string: "https://matrix.example")!
        let response = HTTPURLResponse(
            url: url,
            statusCode: statusCode,
            httpVersion: nil,
            headerFields: nil
        )!
        pending.continuation.resume(returning: (Data(body.utf8), response))
    }

    func nextPendingEventID() throws -> String {
        let request = try XCTUnwrap(pendingRequests.first?.request)
        let body = try XCTUnwrap(request.httpBody)
        let payload = try XCTUnwrap(JSONSerialization.jsonObject(with: body) as? [String: String])
        return try XCTUnwrap(payload["m.fully_read"])
    }

    func submittedEventIDs() throws -> [String] {
        try requests.map { request in
            let body = try XCTUnwrap(request.httpBody)
            let payload = try XCTUnwrap(JSONSerialization.jsonObject(with: body) as? [String: String])
            return try XCTUnwrap(payload["m.fully_read"])
        }
    }

    func authorizationHeaders() -> [String] {
        requests.compactMap { $0.value(forHTTPHeaderField: "Authorization") }
    }

    private func settleCountWaiters() {
        var unsettled: [CountWaiter] = []
        for waiter in countWaiters {
            if requests.count >= waiter.count {
                waiter.continuation.resume()
            } else {
                unsettled.append(waiter)
            }
        }
        countWaiters = unsettled
    }
}

private actor SequencedReadMarkerHTTPClient: RoomReadMarkerHTTPClient {
    private var statusCodes: [Int]
    private var bodies: [String]
    private var requests: [URLRequest] = []

    init(statusCodes: [Int], bodies: [String] = []) {
        self.statusCodes = statusCodes
        self.bodies = bodies
    }

    func data(for request: URLRequest) async throws -> (Data, URLResponse) {
        requests.append(request)
        let statusCode = statusCodes.isEmpty ? 500 : statusCodes.removeFirst()
        let url = request.url ?? URL(string: "https://matrix.example")!
        let response = HTTPURLResponse(
            url: url,
            statusCode: statusCode,
            httpVersion: nil,
            headerFields: nil
        )!
        let body = bodies.isEmpty ? "{}" : bodies.removeFirst()
        return (Data(body.utf8), response)
    }

    func requestCount() -> Int {
        requests.count
    }
}

private final class RecordingReadMarkerHTTPClient: RoomReadMarkerHTTPClient {
    private let statusCode: Int
    private let body: String
    private(set) var lastRequest: URLRequest?
    private(set) var requests: [URLRequest] = []

    init(statusCode: Int, body: String) {
        self.statusCode = statusCode
        self.body = body
    }

    func data(for request: URLRequest) async throws -> (Data, URLResponse) {
        lastRequest = request
        requests.append(request)
        let url = request.url ?? URL(string: "https://matrix.example")!
        let response = HTTPURLResponse(
            url: url,
            statusCode: statusCode,
            httpVersion: nil,
            headerFields: nil
        )!
        return (Data(body.utf8), response)
    }
}
