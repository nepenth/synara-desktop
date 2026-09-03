import XCTest
@testable import Synara
import SynaraCore

@MainActor
final class PushServiceTests: XCTestCase {
    func testAgentApprovalMutationsRequireForegroundStoreAuthority() throws {
        let actions = SynaraNotificationActionContract.agentApprovalActions()
        let approve = try XCTUnwrap(
            actions.first { $0.identifier == SynaraNotificationActionContract.approveOnceIdentifier }
        )
        let deny = try XCTUnwrap(
            actions.first { $0.identifier == SynaraNotificationActionContract.denyIdentifier }
        )

        XCTAssertTrue(approve.options.contains(.authenticationRequired))
        XCTAssertTrue(approve.options.contains(.foreground))
        XCTAssertTrue(deny.options.contains(.authenticationRequired))
        XCTAssertTrue(deny.options.contains(.destructive))
        XCTAssertTrue(deny.options.contains(.foreground))
    }

    func testForegroundApprovalPolicyRequiresActiveProcessAndLiveMatrixOwner() {
        XCTAssertTrue(
            SynaraForegroundMatrixMutationPolicy.allowsMutation(
                lifecycleActive: true,
                applicationState: .active,
                syncStatus: .connected
            )
        )
        XCTAssertFalse(
            SynaraForegroundMatrixMutationPolicy.allowsMutation(
                lifecycleActive: true,
                applicationState: .background,
                syncStatus: .connected
            )
        )
        XCTAssertFalse(
            SynaraForegroundMatrixMutationPolicy.allowsMutation(
                lifecycleActive: false,
                applicationState: .active,
                syncStatus: .connected
            )
        )
        XCTAssertFalse(
            SynaraForegroundMatrixMutationPolicy.allowsMutation(
                lifecycleActive: true,
                applicationState: .active,
                syncStatus: .stopped
            )
        )
    }

    func testSparseEventIDParserReturnsEventWhenRoomMissing() {
        XCTAssertEqual(
            NotificationPushRouteParser.sparseEventID(from: ["event_id": "$sparse:matrix.org"]),
            "$sparse:matrix.org"
        )
        XCTAssertNil(
            NotificationPushRouteParser.sparseEventID(from: [
                "room_id": "!room:matrix.org",
                "event_id": "$sparse:matrix.org"
            ])
        )
    }

    func testAlertShapeReportsPreviewAndRoutingPresenceWithoutContents() {
        let shape = NotificationPushRouteParser.alertShape(from: [
            "aps": [
                "alert": [
                    "title": "Synara",
                    "body": "New activity"
                ],
                "category": "synara.agent-approval",
                "mutable-content": 1
            ],
            "room_id": "!room:matrix.org",
            "event_id": "$event:matrix.org",
            "synara": [
                "kind": "agent-approval"
            ]
        ])

        XCTAssertEqual(shape.alertKind, "dictionary")
        XCTAssertTrue(shape.hasTitle)
        XCTAssertEqual(shape.titleLength, 6)
        XCTAssertTrue(shape.hasBody)
        XCTAssertEqual(shape.bodyLength, 12)
        XCTAssertEqual(shape.category, "synara.agent-approval")
        XCTAssertTrue(shape.hasRoomID)
        XCTAssertTrue(shape.hasEventID)
        XCTAssertEqual(shape.synaraKind, "agent-approval")
        XCTAssertFalse(shape.contentAvailable)
        XCTAssertTrue(shape.mutableContent)
        XCTAssertFalse(shape.logSummary.contains("New activity"))
    }

    func testAlertShapeReportsMissingPreviewForSilentPayload() {
        let shape = NotificationPushRouteParser.alertShape(from: [
            "aps": [
                "content-available": 1
            ],
            "event_id": "$event:matrix.org"
        ])

        XCTAssertEqual(shape.alertKind, "missing")
        XCTAssertFalse(shape.hasTitle)
        XCTAssertFalse(shape.hasBody)
        XCTAssertFalse(shape.hasRoomID)
        XCTAssertTrue(shape.hasEventID)
        XCTAssertTrue(shape.contentAvailable)
    }

    func testRouteFromPayloadUsesRoomIdAndEventId() {
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: StubPusherService()
        )

        let route = service.route(from: [
            "room_id": "!room:matrix.org",
            "event_id": "$event1:matrix.org"
        ])

        assertRoute(route, matchesRoom: "!room:matrix.org", eventID: "$event1:matrix.org")
    }

    func testRouteFromPayloadRouteStringParsesCustomScheme() {
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: StubPusherService()
        )

        let route = service.route(from: [
            "route": "synara://route/%2Froom%2F!room%3Amatrix.org%2F%24event2%3Amatrix.org"
        ])

        assertRoute(route, matchesRoom: "!room:matrix.org", eventID: "$event2:matrix.org")
    }

    func testRouteFromPayloadParsesUniversalLinkStyleRoute() {
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: StubPusherService()
        )

        let route = service.route(from: [
            "route": "https://synara.app/r/%2Froom%2F%21room%3Amatrix.org%2F%24event3%3Amatrix.org"
        ])

        assertRoute(route, matchesRoom: "!room:matrix.org", eventID: "$event3:matrix.org")
    }

    func testRouteFromPayloadFallsBackToSettingsForUnknownRoute() {
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: StubPusherService()
        )

        let route = service.route(from: ["route": "https://synara.app/r/invalid"])

        XCTAssertNil(route)
    }

    func testRouteFromPayloadSupportsNotificationAndLaterRoutes() {
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: StubPusherService()
        )

        let notifications = service.route(from: ["route": "synara://notifications"])
        XCTAssertEqual(notifications, .notifications)

        let later = service.route(from: ["route": "https://synara.app/r/%2Finbox%2Flater"])
        XCTAssertEqual(later, .later)
    }

    func testAgentApprovalNotificationActionParsesCoreDecisionRequest() throws {
        let request = try XCTUnwrap(
            SynaraNotificationActionContract.agentApprovalDecisionRequest(
                actionIdentifier: SynaraNotificationActionContract.approveOnceIdentifier,
                userInfo: [
                    "synara": [
                        "room_id": " !room:matrix.org ",
                        "event_id": " $approval:matrix.org "
                    ]
                ]
            )
        )

        XCTAssertEqual(
            request,
            SynaraAgentApprovalPromptDecisionRequest(
                roomID: "!room:matrix.org",
                sourceEventID: "$approval:matrix.org",
                actionIdentifier: SynaraNotificationActionContract.approveOnceIdentifier
            )
        )
    }

    func testAgentApprovalNotificationActionRejectsUnknownOrIncompletePayloads() {
        XCTAssertNil(
            SynaraNotificationActionContract.agentApprovalDecisionRequest(
                actionIdentifier: "unknown",
                userInfo: [
                    "room_id": "!room:matrix.org",
                    "event_id": "$approval:matrix.org"
                ]
            )
        )
        XCTAssertNil(
            SynaraNotificationActionContract.agentApprovalDecisionRequest(
                actionIdentifier: SynaraNotificationActionContract.denyIdentifier,
                userInfo: ["room_id": "!room:matrix.org"]
            )
        )
    }

    func testAgentApprovalNotificationActionBlocksApproveAlwaysFromNativePath() {
        let plan = SynaraNotificationActionContract.planAgentApprovalNotificationAction(
            actionIdentifier: SynaraNotificationActionContract.approveAlwaysIdentifier,
            userInfo: [
                "room_id": "!room:matrix.org",
                "event_id": "$approval:matrix.org"
            ]
        )

        XCTAssertEqual(
            plan,
            .openRoom(
                roomID: "!room:matrix.org",
                eventID: "$approval:matrix.org",
                reason: "approve-always-requires-in-app-confirmation"
            )
        )
        XCTAssertNil(
            SynaraNotificationActionContract.agentApprovalDecisionRequest(
                actionIdentifier: SynaraNotificationActionContract.approveAlwaysIdentifier,
                userInfo: [
                    "room_id": "!room:matrix.org",
                    "event_id": "$approval:matrix.org"
                ]
            )
        )
    }

    func testAgentApprovalReviewOpensExactPromptWithoutTrustingPayloadClock() {
        let plan = SynaraNotificationActionContract.planAgentApprovalNotificationAction(
            actionIdentifier: SynaraNotificationActionContract.reviewIdentifier,
            userInfo: [
                "room_id": "!room:matrix.org",
                "event_id": "$approval:matrix.org"
            ]
        )
        XCTAssertEqual(
            plan,
            .openRoom(
                roomID: "!room:matrix.org",
                eventID: "$approval:matrix.org",
                reason: "review-requested"
            )
        )

        let expiredReview = SynaraNotificationActionContract.planAgentApprovalNotificationAction(
            actionIdentifier: SynaraNotificationActionContract.reviewIdentifier,
            userInfo: [
                "room_id": "!room:matrix.org",
                "event_id": "$approval:matrix.org",
                "event_ts": 1
            ],
            now: Date(timeIntervalSince1970: 10_000),
            alreadyActed: true
        )
        XCTAssertEqual(expiredReview, plan)
    }

    func testAgentApprovalNotificationActionDefersUntrustedPayloadClockAndRejectsAlreadyActed() {
        let staleCreatedAt = Date(timeIntervalSince1970: 1)
        let expiredPlan = SynaraNotificationActionContract.planAgentApprovalNotificationAction(
            actionIdentifier: SynaraNotificationActionContract.approveOnceIdentifier,
            userInfo: [
                "room_id": "!room:matrix.org",
                "event_id": "$approval:matrix.org",
                "created_at": staleCreatedAt.timeIntervalSince1970 * 1000
            ]
        )
        guard case .submitDecision(let request) = expiredPlan else {
            return XCTFail("Payload clocks must defer to authoritative Matrix event validation")
        }
        XCTAssertEqual(request.actionIdentifier, SynaraNotificationActionContract.approveOnceIdentifier)

        let alreadyActed = SynaraNotificationActionContract.planAgentApprovalNotificationAction(
            actionIdentifier: SynaraNotificationActionContract.denyIdentifier,
            userInfo: [
                "room_id": "!room:matrix.org",
                "event_id": "$approval:matrix.org"
            ],
            alreadyActed: true
        )
        XCTAssertEqual(alreadyActed, .ignore(reason: "already-acted"))
    }

    func testAgentApprovalNotificationActionPlansDenyThroughCore() throws {
        let plan = SynaraNotificationActionContract.planAgentApprovalNotificationAction(
            actionIdentifier: SynaraNotificationActionContract.denyIdentifier,
            userInfo: [
                "room_id": "!room:matrix.org",
                "event_id": "$approval:matrix.org"
            ]
        )
        guard case .submitDecision(let request) = plan else {
            return XCTFail("Expected submitDecision plan, got \(plan)")
        }
        XCTAssertEqual(request.actionIdentifier, SynaraNotificationActionContract.denyIdentifier)
        XCTAssertEqual(request.roomID, "!room:matrix.org")
        XCTAssertEqual(request.sourceEventID, "$approval:matrix.org")
    }

    func testAgentApprovalNotificationActionDedupeStorePersistsKeys() {
        let suiteName = "SynaraAgentApprovalNotificationActionDedupeStoreTests-\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer {
            defaults.removePersistentDomain(forName: suiteName)
        }

        let storageKey = "test.agent-approval.native-action-dedupe"
        let key = SynaraAgentApprovalNotificationActionDedupeStore.key(
            roomID: "!room:matrix.org",
            eventID: "$approval:matrix.org",
            actionIdentifier: SynaraNotificationActionContract.approveOnceIdentifier
        )
        let denyKey = SynaraAgentApprovalNotificationActionDedupeStore.key(
            roomID: "!room:matrix.org",
            eventID: "$approval:matrix.org",
            actionIdentifier: SynaraNotificationActionContract.denyIdentifier
        )
        XCTAssertEqual(key, denyKey)
        let first = SynaraAgentApprovalNotificationActionDedupeStore(
            defaults: defaults,
            storageKey: storageKey
        )
        XCTAssertFalse(first.contains(key))
        first.insert(key)
        XCTAssertTrue(first.contains(key))

        let second = SynaraAgentApprovalNotificationActionDedupeStore(
            defaults: defaults,
            storageKey: storageKey
        )
        XCTAssertTrue(second.contains(key))
        second.remove(key)
        XCTAssertFalse(first.contains(key))
    }

    func testBadgeCountParsesApsBadgeAndSummaryFormats() {
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: StubPusherService()
        )

        XCTAssertEqual(service.parseBadgeCount(from: ["aps": ["badge": 3]]), 3)

        let withNumericSummary = service.parseBadgeCount(from: [
            "notification_summary": [
                "appBadgeCount": 7
            ]
        ])
        XCTAssertEqual(withNumericSummary, 7)

        let withStringSummary = service.parseBadgeCount(from: [
            "synara": [
                "notification_summary": [
                    "appBadgeCount": "9"
                ]
            ]
        ])
        XCTAssertEqual(withStringSummary, 9)
    }

    func testPushServiceRegistersAfterSessionAndToken() async {
        let pusher = StubPusherService()

        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        service.configure(with: makeSession())
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))

        await waitUntil {
            service.isRegistered
                && service.registrationStateDescription == "Pusher registration complete"
                && pusher.registerCount >= 1
        }
        XCTAssertTrue(service.isRegistered)
        XCTAssertEqual(pusher.registerCount, 1)
        XCTAssertEqual(pusher.lastPushKey, "7ab13c")
    }

    func testPushServiceCoalescesRepeatedRegistrationTriggers() async {
        let pusher = StubPusherService()
        let session = makeSession()
        let token = Data([0x7A, 0xB1, 0x3C])
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )

        service.configure(with: session)
        service.handleDeviceToken(token)
        service.configure(with: session)
        service.handleDeviceToken(token)

        await waitUntil { service.isRegistered }
        try? await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertEqual(pusher.registerCount, 1)
        XCTAssertEqual(pusher.unregisterCount, 0)
    }

    func testPushServiceCoalescesIdenticalTriggerWhileRegistrationIsInFlight() async {
        let pusher = StubPusherService(registerDelayNanoseconds: 80_000_000)
        let session = makeSession()
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )

        service.configure(with: session)
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))
        await waitUntil { pusher.registerCount == 1 }
        service.configure(with: session)

        await waitUntil { service.isRegistered }
        try? await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertEqual(pusher.registerCount, 1)
        XCTAssertEqual(pusher.unregisterCount, 0)
    }

    func testPushServiceClearsRegistrationAndUnregistersOnLogout() async {
        let pusher = StubPusherService()

        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        service.configure(with: makeSession())
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))
        await waitUntil { service.isRegistered && pusher.registerCount >= 1 }

        let cleanupSucceeded = await service.clearRegistrationState()
        await waitUntil { pusher.unregisterAllCount >= 1 && service.isRegistered == false }

        XCTAssertEqual(service.tokenSnippet, "7ab13c")
        XCTAssertFalse(service.isRegistered)
        XCTAssertEqual(pusher.unregisterAllCount, 1)
        XCTAssertTrue(cleanupSucceeded)
        service.completeRegistrationTeardown()
        XCTAssertNil(service.tokenSnippet)
    }

    func testPushServiceRetainsLogoutOwnerAndTokenUntilFailedCleanupRetriesSuccessfully() async {
        let pusher = StubPusherService()
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        service.configure(with: makeSession())
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))
        await waitUntil { service.isRegistered && pusher.registerCount == 1 }
        pusher.unregisterFailuresRemaining = 1

        let firstAttempt = await service.clearRegistrationState()

        XCTAssertFalse(firstAttempt)
        XCTAssertFalse(service.isRegistered)
        XCTAssertEqual(service.tokenSnippet, "7ab13c")
        XCTAssertEqual(pusher.unregisterAllCount, 1)

        let retry = await service.clearRegistrationState()

        XCTAssertTrue(retry)
        XCTAssertEqual(service.tokenSnippet, "7ab13c")
        XCTAssertEqual(pusher.unregisterAllCount, 2)
        XCTAssertEqual(pusher.unregisteredAllSessions, [makeSession(), makeSession()])
        service.completeRegistrationTeardown()
        XCTAssertNil(service.tokenSnippet)
    }

    func testReinstantiatedPushServiceLogsOutByEnumeratingBoundDevicePushersWithoutAPNSToken() async {
        let pusher = StubPusherService()
        let session = makeSession()
        var original: SynaraPushService? = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        original?.configure(with: session)
        original?.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))
        await waitUntil { original?.isRegistered == true }
        original = nil

        let reloaded = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        reloaded.configure(with: session)
        let cleanupSucceeded = await reloaded.clearRegistrationState()

        XCTAssertTrue(cleanupSucceeded)
        XCTAssertEqual(pusher.unregisterAllCount, 1)
        XCTAssertEqual(pusher.unregisteredAllSessions, [session])
        XCTAssertNil(reloaded.tokenSnippet)
        reloaded.completeRegistrationTeardown()
    }

    func testImmediateLogoutAfterAPNSFailureEnumeratesBoundDevicePushers() async {
        let pusher = StubPusherService()
        let session = makeSession()
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        service.configure(with: session)
        service.handleRegistrationFailure()

        let cleanupSucceeded = await service.clearRegistrationState()

        XCTAssertTrue(cleanupSucceeded)
        XCTAssertEqual(pusher.unregisterAllCount, 1)
        XCTAssertEqual(pusher.unregisteredAllSessions, [session])
        service.completeRegistrationTeardown()
    }

    func testImmediateLogoutFailsClosedWithoutAccountBoundOwner() async {
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: ThrowingPusherService(),
            isRegistrationAvailable: true
        )
        service.configure(with: makeSession())
        service.handleRegistrationFailure()

        let cleanupSucceeded = await service.clearRegistrationState()

        XCTAssertFalse(cleanupSucceeded)
        XCTAssertEqual(
            service.registrationStateDescription,
            "Pusher owner unavailable; retry sign out"
        )
    }

    func testImmediateLogoutRebindsOnceBeforeEnumeratingDevicePushers() async {
        let pusher = StubPusherService()
        pusher.bindFailuresRemaining = 1
        let session = makeSession()
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        service.configure(with: session)
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))

        let cleanupSucceeded = await service.clearRegistrationState()

        XCTAssertTrue(cleanupSucceeded)
        XCTAssertEqual(pusher.boundSessions, [session, session])
        XCTAssertEqual(pusher.unregisterAllCount, 1)
        XCTAssertEqual(pusher.unregisteredAllSessions, [session])
        XCTAssertEqual(pusher.lastUnregisterAllPushKey, "7ab13c")
    }

    func testPushCallbacksCannotRestartRegistrationWhileLogoutCleanupIsSuspended() async {
        let pusher = StubPusherService(unregisterDelayNanoseconds: 150_000_000)
        let session = makeSession()
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        service.configure(with: session)
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))
        await waitUntil { service.isRegistered && pusher.registerCount == 1 }

        let cleanupTask = Task { await service.clearRegistrationState() }
        await waitUntil { pusher.unregisterAllCount == 1 }
        service.handleDeviceToken(Data([0xAA, 0x55, 0x00]))
        service.handleRegistrationFailure()
        service.configure(with: session)

        let cleanupSucceeded = await cleanupTask.value
        XCTAssertTrue(cleanupSucceeded)
        service.handleDeviceToken(Data([0xAA, 0x55, 0x00]))
        service.configure(with: session)
        try? await Task.sleep(nanoseconds: 50_000_000)
        XCTAssertEqual(pusher.registerCount, 1)
        XCTAssertEqual(pusher.unregisterAllCount, 1)
        XCTAssertEqual(service.tokenSnippet, "7ab13c")
        XCTAssertFalse(service.isRegistered)
        service.completeRegistrationTeardown()
        XCTAssertNil(service.tokenSnippet)
    }

    func testCancelledLocalTeardownRestoresDeletedPusherForSignedInSession() async {
        let pusher = StubPusherService()
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        service.configure(with: makeSession())
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))
        await waitUntil { service.isRegistered && pusher.registerCount == 1 }

        let cleanupSucceeded = await service.clearRegistrationState()
        XCTAssertTrue(cleanupSucceeded)
        service.cancelRegistrationTeardown()
        await waitUntil { service.isRegistered && pusher.registerCount == 2 }

        XCTAssertEqual(pusher.unregisterAllCount, 1)
        XCTAssertEqual(service.tokenSnippet, "7ab13c")
    }

    func testTokenRotatedDuringTeardownIsRestoredAfterKeychainDeletionFailure() async throws {
        let authenticatedSession = makeSession()
        let secureStore = PushTestDeleteFailingSecureSessionStore(session: authenticatedSession)
        let sessionStore = AppSessionStore(
            secureStore: secureStore,
            restorePersistedSession: true
        )
        let pusher = StubPusherService(unregisterDelayNanoseconds: 150_000_000)
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        service.configure(with: authenticatedSession)
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))
        await waitUntil { service.isRegistered && pusher.registerCount == 1 }
        let wipe = AppLocalWipeService(
            session: sessionStore,
            matrix: MockMatrixClientService(syncStatus: .syncing),
            roomList: MockRoomListService(),
            timeline: MockTimelineService(),
            drafts: DraftStore(),
            push: service,
            router: AppRouter()
        )

        let logout = Task { () -> LocalWipeError? in
            do {
                try await wipe.logoutAndWipe()
                return nil
            } catch let error as LocalWipeError {
                return error
            } catch {
                XCTFail("Unexpected logout error: \(error)")
                return nil
            }
        }
        await waitUntil { pusher.unregisterAllCount == 1 }
        service.handleDeviceToken(Data([0xAA, 0x55, 0x00]))

        let logoutError = await logout.value
        await waitUntil { service.isRegistered && pusher.registerCount == 2 }

        XCTAssertEqual(logoutError, .sessionDeleteFailed)
        XCTAssertEqual(sessionStore.currentState, .signedIn(authenticatedSession))
        XCTAssertEqual(pusher.unregisterAllCount, 1)
        XCTAssertEqual(pusher.lastPushKey, "aa5500")
        XCTAssertEqual(service.tokenSnippet, "aa5500")
    }

    func testTokenRotatedDuringFailedRemoteCleanupIsAppliedAndReconciled() async {
        let pusher = StubPusherService(unregisterDelayNanoseconds: 150_000_000)
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        service.configure(with: makeSession())
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))
        await waitUntil { service.isRegistered && pusher.registerCount == 1 }
        pusher.unregisterFailuresRemaining = 1

        let cleanup = Task { await service.clearRegistrationState() }
        await waitUntil { pusher.unregisterAllCount == 1 }
        service.handleDeviceToken(Data([0xAA, 0x55, 0x00]))

        let cleanupSucceeded = await cleanup.value
        await waitUntil {
            service.isRegistered
                && pusher.unregisterCount == 1
                && pusher.registerCount == 2
        }

        XCTAssertFalse(cleanupSucceeded)
        XCTAssertEqual(pusher.unregisterAllCount, 1)
        XCTAssertEqual(pusher.lastUnregisterPushKey, "7ab13c")
        XCTAssertEqual(pusher.lastPushKey, "aa5500")
        XCTAssertEqual(service.tokenSnippet, "aa5500")
    }

    func testPushServiceLogoutEnumeratesDeviceWhenRegistrationIsInFlight() async {
        let pusher = StubPusherService(registerDelayNanoseconds: 200_000_000)
        let session = makeSession()
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )

        service.configure(with: session)
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))
        await waitUntil { pusher.registerCount == 1 }
        let cleanupSucceeded = await service.clearRegistrationState()

        XCTAssertTrue(cleanupSucceeded)
        XCTAssertFalse(service.isRegistered)
        XCTAssertEqual(service.tokenSnippet, "7ab13c")
        XCTAssertEqual(pusher.unregisteredAllSessions, [session])
        service.completeRegistrationTeardown()
        XCTAssertNil(service.tokenSnippet)
    }

    func testPushServiceReplacesRegistrationOnTokenRotation() async {
        let pusher = StubPusherService()

        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        service.configure(with: makeSession())
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))
        await waitUntil { service.isRegistered && pusher.registerCount >= 1 }
        let initialRegisterCount = pusher.registerCount

        service.handleDeviceToken(Data([0xAA, 0x55, 0x00]))
        await waitUntil {
            service.tokenSnippet == "aa5500"
                && pusher.unregisterCount >= 1
                && pusher.registerCount > initialRegisterCount
        }

        XCTAssertGreaterThan(pusher.registerCount, initialRegisterCount)
        XCTAssertEqual(pusher.unregisterCount, 1)
        XCTAssertEqual(pusher.lastUnregisterPushKey, "7ab13c")
        XCTAssertEqual(service.tokenSnippet, "aa5500")
    }

    func testPushServiceRetainsOldBindingUntilFailedRotationCleanupCanRetry() async {
        let pusher = StubPusherService()
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        service.configure(with: makeSession())
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))
        await waitUntil { service.isRegistered && pusher.registerCount == 1 }

        pusher.unregisterFailuresRemaining = 1
        service.handleDeviceToken(Data([0xAA, 0x55, 0x00]))
        await waitUntil { pusher.unregisterCount == 1 }

        XCTAssertFalse(service.isRegistered)
        XCTAssertEqual(service.registrationStateDescription, "Previous pusher cleanup failed")
        XCTAssertEqual(pusher.registerCount, 1)

        service.handleDeviceToken(Data([0xAA, 0x55, 0x00]))
        await waitUntil { service.isRegistered && pusher.registerCount == 2 }
        XCTAssertEqual(pusher.unregisterCount, 2)
        XCTAssertEqual(pusher.lastUnregisterPushKey, "7ab13c")
        XCTAssertEqual(pusher.lastPushKey, "aa5500")
    }

    func testPushServiceReplacesRegistrationUsingOriginallyBoundSession() async {
        let pusher = StubPusherService()
        let firstSession = makeSession()
        let secondSession = AuthenticatedSession(
            userID: "@bob:matrix.org",
            deviceID: "SECOND",
            homeserverURL: URL(string: "https://matrix.org")!,
            accessToken: "second-token"
        )
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )

        service.configure(with: firstSession)
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))
        await waitUntil { service.isRegistered && pusher.registerCount == 1 }

        service.configure(with: secondSession)
        await waitUntil {
            service.isRegistered
                && pusher.unregisterCount == 1
                && pusher.registerCount == 2
        }

        XCTAssertEqual(pusher.unregisteredSessions, [firstSession])
        XCTAssertEqual(pusher.registeredSessions, [firstSession, secondSession])
        XCTAssertEqual(pusher.lastUnregisterPushKey, "7ab13c")
    }

    func testProductionPusherAdapterDeletesThroughOriginallyBoundCoreOwnerOnAccountRotation() async throws {
        let firstOwner = RecordingSharedCoreHttpPusherOwner()
        let secondOwner = RecordingSharedCoreHttpPusherOwner()
        let firstSession = AuthenticatedSession(
            userID: "@first:example.org",
            deviceID: "FIRST",
            homeserverURL: try XCTUnwrap(URL(string: "https://first.example.org")),
            accessToken: ""
        )
        let secondSession = AuthenticatedSession(
            userID: "@second:example.org",
            deviceID: "SECOND",
            homeserverURL: try XCTUnwrap(URL(string: "https://second.example.org")),
            accessToken: ""
        )
        let host = SharedCoreProductHost(
            core: SharedCore(),
            storeRoot: FileManager.default.temporaryDirectory,
            sessionStore: AppSessionStore()
        )
        var boundSessionSignatures: [String] = []
        let adapter = SharedCorePusherService(
            host: host,
            gatewayURL: try XCTUnwrap(URL(string: "https://push.example.org")),
            logger: MockLoggingService(),
            ownerBinder: { session in
                boundSessionSignatures.append("\(session.userID)|\(session.deviceID)")
                return session == firstSession ? firstOwner : secondOwner
            }
        )
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: adapter,
            isRegistrationAvailable: true
        )

        service.configure(with: firstSession)
        service.handleDeviceToken(Data([0x7a, 0xb1, 0x3c]))
        await waitUntil { service.isRegistered && firstOwner.registeredPushKeys.count == 1 }

        service.configure(with: secondSession)
        await waitUntil {
            service.isRegistered
                && firstOwner.deletedPushKeys.count == 1
                && secondOwner.registeredPushKeys.count == 1
        }

        XCTAssertEqual(boundSessionSignatures, ["@first:example.org|FIRST", "@second:example.org|SECOND"])
        XCTAssertEqual(firstOwner.registeredPushKeys, ["7ab13c"])
        XCTAssertEqual(firstOwner.deletedPushKeys, ["7ab13c"])
        XCTAssertEqual(secondOwner.registeredPushKeys, ["7ab13c"])
        XCTAssertTrue(secondOwner.deletedPushKeys.isEmpty)
    }

    func testProductionPusherAdapterCleansOldOwnerWhenNewOwnerBindingFails() async throws {
        let firstOwner = RecordingSharedCoreHttpPusherOwner()
        let firstSession = AuthenticatedSession(
            userID: "@first:example.org",
            deviceID: "FIRST",
            homeserverURL: try XCTUnwrap(URL(string: "https://first.example.org")),
            accessToken: ""
        )
        let secondSession = AuthenticatedSession(
            userID: "@second:example.org",
            deviceID: "SECOND",
            homeserverURL: try XCTUnwrap(URL(string: "https://second.example.org")),
            accessToken: ""
        )
        let host = SharedCoreProductHost(
            core: SharedCore(),
            storeRoot: FileManager.default.temporaryDirectory,
            sessionStore: AppSessionStore()
        )
        let adapter = SharedCorePusherService(
            host: host,
            gatewayURL: try XCTUnwrap(URL(string: "https://push.example.org")),
            logger: MockLoggingService(),
            ownerBinder: { session in
                guard session == firstSession else {
                    throw StubPusherError.plannedFailure
                }
                return firstOwner
            }
        )
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: adapter,
            isRegistrationAvailable: true
        )

        service.configure(with: firstSession)
        service.handleDeviceToken(Data([0x7a, 0xb1, 0x3c]))
        await waitUntil { service.isRegistered && firstOwner.registeredPushKeys.count == 1 }

        service.configure(with: secondSession)
        await waitUntil { firstOwner.deletedPushKeys.count == 1 }

        XCTAssertFalse(service.isRegistered)
        XCTAssertEqual(service.registrationStateDescription, "Pusher owner unavailable")
        XCTAssertEqual(firstOwner.registeredPushKeys, ["7ab13c"])
        XCTAssertEqual(firstOwner.deletedPushKeys, ["7ab13c"])
    }

    func testProductionPusherAdapterEnumeratesCurrentDeviceForTokenlessLogout() async throws {
        let owner = RecordingSharedCoreHttpPusherOwner()
        let session = makeSession()
        let host = SharedCoreProductHost(
            core: SharedCore(),
            storeRoot: FileManager.default.temporaryDirectory,
            sessionStore: AppSessionStore()
        )
        let adapter = SharedCorePusherService(
            host: host,
            gatewayURL: try XCTUnwrap(URL(string: "https://push.example.org")),
            logger: MockLoggingService(),
            ownerBinder: { _ in owner }
        )
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: adapter,
            isRegistrationAvailable: true
        )
        service.configure(with: session)

        let cleanupSucceeded = await service.clearRegistrationState()
        XCTAssertTrue(cleanupSucceeded)
        XCTAssertEqual(owner.deleteForDeviceCount, 1)
    }

    func testPushServiceCleansStaleInFlightRegistrationBeforeBindingNewSession() async {
        let pusher = StubPusherService(registerDelayNanoseconds: 80_000_000)
        let firstSession = makeSession()
        let secondSession = AuthenticatedSession(
            userID: "@bob:matrix.org",
            deviceID: "SECOND",
            homeserverURL: URL(string: "https://matrix.org")!,
            accessToken: "second-token"
        )
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )

        service.configure(with: firstSession)
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))
        await waitUntil { pusher.registerCount == 1 }
        service.configure(with: secondSession)

        await waitUntil(timeoutNanoseconds: 2_000_000_000) {
            service.isRegistered && pusher.registerCount == 2
        }

        XCTAssertEqual(pusher.registeredSessions, [firstSession, secondSession])
        XCTAssertEqual(pusher.unregisteredSessions, [firstSession])
        XCTAssertEqual(pusher.unregisterCount, 1)
    }

    func testResolveRouteFallsBackToSparseResolver() async {
        let resolver = StubSparsePushRouteResolver(
            route: .room(id: "!resolved:matrix.org", eventID: "$sparse:matrix.org")
        )
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: StubPusherService(),
            sparseRouteResolver: resolver
        )

        let route = await service.resolveRoute(from: ["event_id": "$sparse:matrix.org"])

        assertRoute(route, matchesRoom: "!resolved:matrix.org", eventID: "$sparse:matrix.org")
        XCTAssertEqual(resolver.resolveCallCount, 1)
    }

    func testPushServiceDoesNotRegisterWithoutGateway() async {
        let pusher = StubPusherService(isGatewayConfigured: false)

        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )

        service.configure(with: makeSession())
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))

        await Task.yield()

        XCTAssertEqual(pusher.registerCount, 0)
        XCTAssertFalse(service.isRegistered)
        XCTAssertEqual(service.registrationStateDescription, "Push gateway not configured")
    }

    func testApnsRegistrationFailurePreservesAnExistingPusherBinding() async {
        let pusher = StubPusherService()
        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        service.configure(with: makeSession())
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))
        await waitUntil { service.isRegistered }

        service.handleRegistrationFailure()

        XCTAssertTrue(service.isRegistered)
        XCTAssertEqual(
            service.registrationStateDescription,
            "APNs registration failed; existing pusher retained"
        )
        XCTAssertEqual(pusher.registerCount, 1)
    }

    private func makeSession() -> AuthenticatedSession {
        AuthenticatedSession(
            userID: "@alice:matrix.org",
            deviceID: "DEVICE",
            homeserverURL: URL(string: "https://matrix.org")!,
            accessToken: "token"
        )
    }

    private func assertRoute(
        _ route: AppRoute?,
        matchesRoom roomID: String,
        eventID: String
    ) {
        guard case .room(let id, let parsedEventID, _) = route else {
            XCTFail("Expected room route")
            return
        }

        XCTAssertEqual(id, roomID)
        XCTAssertEqual(parsedEventID, eventID)
    }

    private func waitUntil(
        timeoutNanoseconds: UInt64 = 1_000_000_000,
        condition: @escaping () -> Bool
    ) async {
        let deadline = DispatchTime.now().uptimeNanoseconds + timeoutNanoseconds
        while DispatchTime.now().uptimeNanoseconds < deadline {
            if condition() {
                return
            }
            try? await Task.sleep(nanoseconds: 10_000_000)
        }
    }
}

private final class StubSparsePushRouteResolver: SparsePushRouteResolving {
    let route: AppRoute?
    private(set) var resolveCallCount = 0

    init(route: AppRoute?) {
        self.route = route
    }

    func resolveRoute(eventID: String) async -> AppRoute? {
        resolveCallCount += 1
        return route
    }
}

private final class StubPusherService: MatrixPusherServicing {
        var isGatewayConfigured: Bool
        var configuredGatewayURL: URL? {
            isGatewayConfigured ? URL(string: "https://push.example.internal") : nil
        }
        private(set) var registerCount = 0
        private(set) var unregisterCount = 0
        private(set) var unregisterAllCount = 0
        private(set) var lastPushKey: String?
        private(set) var lastUnregisterPushKey: String?
        private(set) var lastUnregisterAllPushKey: String?
        private(set) var registeredSessions: [AuthenticatedSession] = []
        private(set) var unregisteredSessions: [AuthenticatedSession] = []
        private(set) var unregisteredAllSessions: [AuthenticatedSession] = []
        private(set) var boundSessions: [AuthenticatedSession] = []
        let registerDelayNanoseconds: UInt64
        let unregisterDelayNanoseconds: UInt64
        var unregisterFailuresRemaining = 0
        var bindFailuresRemaining = 0
        var onRegister: () -> Void = {}
        var onUnregister: () -> Void = {}

        init(
            isGatewayConfigured: Bool = true,
            registerDelayNanoseconds: UInt64 = 0,
            unregisterDelayNanoseconds: UInt64 = 0
        ) {
            self.isGatewayConfigured = isGatewayConfigured
            self.registerDelayNanoseconds = registerDelayNanoseconds
            self.unregisterDelayNanoseconds = unregisterDelayNanoseconds
        }

    func bindPusher(to session: AuthenticatedSession) throws -> MatrixPusherAccountServicing {
        boundSessions.append(session)
        if bindFailuresRemaining > 0 {
            bindFailuresRemaining -= 1
            throw StubPusherError.plannedFailure
        }
        return StubPusherAccountService(service: self, session: session)
    }

    fileprivate func registerPusher(session: AuthenticatedSession, pushKey: String) async throws {
        registerCount += 1
        lastPushKey = pushKey
        registeredSessions.append(session)
        onRegister()
        if registerDelayNanoseconds > 0 {
            try await Task.sleep(nanoseconds: registerDelayNanoseconds)
        }
    }

    fileprivate func unregisterPusher(session: AuthenticatedSession, pushKey: String) async throws {
        unregisterCount += 1
        lastUnregisterPushKey = pushKey
        unregisteredSessions.append(session)
        onUnregister()
        if unregisterFailuresRemaining > 0 {
            unregisterFailuresRemaining -= 1
            throw StubPusherError.plannedFailure
        }
        if unregisterDelayNanoseconds > 0 {
            try await Task.sleep(nanoseconds: unregisterDelayNanoseconds)
        }
    }

    fileprivate func unregisterAllPushers(
        session: AuthenticatedSession,
        lastPushKey: String?
    ) async throws {
        unregisterAllCount += 1
        unregisteredAllSessions.append(session)
        lastUnregisterAllPushKey = lastPushKey
        if unregisterDelayNanoseconds > 0 {
            try await Task.sleep(nanoseconds: unregisterDelayNanoseconds)
        }
        if unregisterFailuresRemaining > 0 {
            unregisterFailuresRemaining -= 1
            throw StubPusherError.plannedFailure
        }
    }
}

private final class StubPusherAccountService: MatrixPusherAccountServicing {
    private let service: StubPusherService
    private let session: AuthenticatedSession

    init(service: StubPusherService, session: AuthenticatedSession) {
        self.service = service
        self.session = session
    }

    func registerPusher(pushKey: String) async throws {
        try await service.registerPusher(session: session, pushKey: pushKey)
    }

    func unregisterPusher(pushKey: String) async throws {
        try await service.unregisterPusher(session: session, pushKey: pushKey)
    }

    func unregisterAllPushersForDevice(lastPushKey: String?) async throws {
        try await service.unregisterAllPushers(
            session: session,
            lastPushKey: lastPushKey
        )
    }
}

private enum StubPusherError: Error {
    case plannedFailure
}

private final class PushTestDeleteFailingSecureSessionStore: SecureSessionStoring {
    private let session: AuthenticatedSession

    init(session: AuthenticatedSession) {
        self.session = session
    }

    func save(_: AuthenticatedSession) throws {}
    func load() throws -> AuthenticatedSession? { session }
    func delete() throws { throw SecureSessionStoreError.keychainFailure(status: -1) }
    func migrateIfNeeded() throws -> SessionMigrationResult { .notNeeded }
}

private final class ThrowingPusherService: MatrixPusherServicing {
    var isGatewayConfigured: Bool { true }
    var configuredGatewayURL: URL? { URL(string: "https://push.example.internal") }

    func bindPusher(to session: AuthenticatedSession) throws -> MatrixPusherAccountServicing {
        _ = session
        throw StubPusherError.plannedFailure
    }
}

private final class RecordingSharedCoreHttpPusherOwner: SharedCoreHttpPusherOwning {
    private(set) var registeredPushKeys: [String] = []
    private(set) var deletedPushKeys: [String] = []
    private(set) var deleteForDeviceCount = 0

    func registerHttpPusher(
        pushKey: String,
        appId: String,
        gatewayUrl: String,
        appDisplayName: String,
        lang: String
    ) async throws -> PusherWriteDto {
        _ = appId
        _ = gatewayUrl
        _ = appDisplayName
        _ = lang
        registeredPushKeys.append(pushKey)
        return PusherWriteDto(status: "ok")
    }

    func deleteHttpPusher(pushKey: String, appId: String) async throws -> PusherWriteDto {
        _ = appId
        deletedPushKeys.append(pushKey)
        return PusherWriteDto(status: "ok")
    }

    func deleteHttpPushersForDevice(
        appId: String,
        lastPushKey: String?
    ) async throws -> PusherWriteDto {
        _ = appId
        _ = lastPushKey
        deleteForDeviceCount += 1
        return PusherWriteDto(status: "ok")
    }
}
