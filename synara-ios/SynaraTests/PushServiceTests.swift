import XCTest
@testable import Synara

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
        XCTAssertGreaterThanOrEqual(pusher.registerCount, 1)
        XCTAssertEqual(pusher.lastPushKey, "7ab13c")
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

        await service.clearRegistrationState()
        await waitUntil { pusher.unregisterCount >= 1 && service.isRegistered == false }

        XCTAssertEqual(service.tokenSnippet, nil)
        XCTAssertFalse(service.isRegistered)
        XCTAssertEqual(pusher.unregisterCount, 1)
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
        private(set) var lastPushKey: String?
        private(set) var lastUnregisterPushKey: String?
        var onRegister: () -> Void = {}
        var onUnregister: () -> Void = {}

        init(isGatewayConfigured: Bool = true) {
            self.isGatewayConfigured = isGatewayConfigured
        }

    func registerPusher(session: AuthenticatedSession, pushKey: String) async throws {
        registerCount += 1
        lastPushKey = pushKey
        onRegister()
    }

    func unregisterPusher(session: AuthenticatedSession, pushKey: String) async throws {
        unregisterCount += 1
        lastUnregisterPushKey = pushKey
        onUnregister()
    }
}
