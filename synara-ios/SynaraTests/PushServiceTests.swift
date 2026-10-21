import UserNotifications
import XCTest
@testable import Synara
import SynaraCore

@MainActor
final class PushServiceTests: XCTestCase {
    private struct ApprovalActions {
        let approve: UNNotificationAction
        let deny: UNNotificationAction
        let review: UNNotificationAction
        let all: [UNNotificationAction]
    }

    private func approvalActions(approveOnceWithoutUnlock: Bool) throws -> ApprovalActions {
        let actions = SynaraNotificationActionContract.agentApprovalActions(
            approveOnceWithoutUnlock: approveOnceWithoutUnlock
        )
        return ApprovalActions(
            approve: try XCTUnwrap(
                actions.first { $0.identifier == SynaraNotificationActionContract.approveOnceIdentifier }
            ),
            deny: try XCTUnwrap(
                actions.first { $0.identifier == SynaraNotificationActionContract.denyIdentifier }
            ),
            review: try XCTUnwrap(
                actions.first { $0.identifier == SynaraNotificationActionContract.reviewIdentifier }
            ),
            all: actions
        )
    }

    func testApproveOnceRequiresUnlockByDefault() throws {
        XCTAssertFalse(SynaraSharedConstants.defaultApproveOnceWithoutUnlock)
        let actions = try approvalActions(approveOnceWithoutUnlock: false)

        // Approving runs an agent's command: a locked iPhone must be unlocked
        // first. The action still runs in the background after unlock.
        XCTAssertTrue(actions.approve.options.contains(.authenticationRequired))
        XCTAssertFalse(actions.approve.options.contains(.foreground))
        // Deny cannot run anything and stays available without an unlock.
        XCTAssertFalse(actions.deny.options.contains(.authenticationRequired))
        XCTAssertFalse(actions.deny.options.contains(.foreground))
        XCTAssertTrue(actions.deny.options.contains(.destructive))
        XCTAssertTrue(actions.review.options.contains(.foreground))
        XCTAssertFalse(
            actions.all.contains { $0.identifier == SynaraAgentApprovalNotificationActionID.approveAlways.rawValue }
        )
    }

    func testOptInLetsApproveOnceRunFromLockScreenAndWatch() throws {
        let actions = try approvalActions(approveOnceWithoutUnlock: true)

        // Foreground or authentication-required actions cannot complete from
        // a mirrored Apple Watch alert while the iPhone is locked.
        XCTAssertFalse(actions.approve.options.contains(.authenticationRequired))
        XCTAssertFalse(actions.approve.options.contains(.foreground))
        XCTAssertFalse(actions.deny.options.contains(.authenticationRequired))
        XCTAssertFalse(actions.deny.options.contains(.foreground))
        XCTAssertTrue(actions.deny.options.contains(.destructive))
        XCTAssertTrue(actions.review.options.contains(.foreground))
        XCTAssertFalse(
            actions.all.contains { $0.identifier == SynaraAgentApprovalNotificationActionID.approveAlways.rawValue }
        )
    }

    func testRegisteredCategoryFollowsTheStoredUnlockPreference() throws {
        let suite = "PushServiceTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        func approveOptions() -> UNNotificationActionOptions? {
            SynaraNotificationActionContract.agentApprovalCategory(
                approveOnceWithoutUnlock: SynaraApproveOnceWithoutUnlockPreference.isEnabled(defaults: defaults)
            )
            .actions
            .first { $0.identifier == SynaraNotificationActionContract.approveOnceIdentifier }?
            .options
        }

        XCTAssertFalse(SynaraApproveOnceWithoutUnlockPreference.isEnabled(defaults: defaults))
        XCTAssertEqual(approveOptions()?.contains(.authenticationRequired), true)

        defaults.set(true, forKey: SynaraSharedConstants.approveOnceWithoutUnlockKey)
        XCTAssertTrue(SynaraApproveOnceWithoutUnlockPreference.isEnabled(defaults: defaults))
        XCTAssertEqual(approveOptions()?.contains(.authenticationRequired), false)

        defaults.set(false, forKey: SynaraSharedConstants.approveOnceWithoutUnlockKey)
        XCTAssertEqual(approveOptions()?.contains(.authenticationRequired), true)
    }

    func testSettingsFooterDescribesTheRegisteredActionsInBothStates() {
        let locked = SynaraApprovalNotificationSettingsCopy.footer(approveOnceWithoutUnlock: false)
        XCTAssertTrue(locked.contains("Approve once requires unlocking your iPhone first."))
        XCTAssertTrue(locked.contains("Deny works without unlocking."))
        XCTAssertFalse(locked.contains("require authentication"))
        XCTAssertFalse(locked.contains("Apple Watch without unlocking"))

        let unlocked = SynaraApprovalNotificationSettingsCopy.footer(approveOnceWithoutUnlock: true)
        XCTAssertTrue(
            unlocked.contains("Approve once and Deny work from the Lock Screen and Apple Watch without unlocking your iPhone.")
        )
        XCTAssertFalse(unlocked.contains("requires unlocking"))
        for footer in [locked, unlocked] {
            XCTAssertTrue(footer.hasSuffix("Always approval remains in-app only."))
        }
    }

    func testNotificationsInboxLabelsApprovalPromptsAsApproval() {
        func room(approval: Bool, highlight: Bool, membership: RoomSummary.Membership = .joined) -> RoomSummary {
            RoomSummary(
                id: "!agent-ops:example.org",
                name: "Agent Ops",
                lastMessagePreview: "Dangerous command requires approval",
                lastMessageIsAgentApproval: approval,
                unreadCount: 1,
                hasHighlight: highlight,
                kind: .room,
                membership: membership,
                lastActivityAt: Date()
            )
        }
        XCTAssertEqual(NotificationsInboxReason(room: room(approval: true, highlight: true)), .approval)
        XCTAssertEqual(NotificationsInboxReason(room: room(approval: true, highlight: false))?.title, "Approval")
        XCTAssertEqual(NotificationsInboxReason(room: room(approval: false, highlight: true))?.title, "Mention")
        XCTAssertEqual(
            NotificationsInboxReason(room: room(approval: false, highlight: false, membership: .invited)),
            .invite
        )
        XCTAssertNil(NotificationsInboxReason(room: room(approval: false, highlight: false)))
    }

    func testOnlyOneShotDecisionsRunAsBackgroundActions() {
        XCTAssertTrue(SynaraBackgroundApprovalActionPolicy.runsInBackground(actionIdentifier: "agent-approval.approve-once"))
        XCTAssertTrue(SynaraBackgroundApprovalActionPolicy.runsInBackground(actionIdentifier: "agent-approval.deny"))
        XCTAssertFalse(SynaraBackgroundApprovalActionPolicy.runsInBackground(actionIdentifier: "agent-approval.approve-always"))
        XCTAssertFalse(SynaraBackgroundApprovalActionPolicy.runsInBackground(actionIdentifier: "agent-approval.review"))
        XCTAssertFalse(SynaraBackgroundApprovalActionPolicy.runsInBackground(actionIdentifier: UNNotificationDefaultActionIdentifier))
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

    func testApplyIncomingBadgeUsesAppIconOwnerInsteadOfClearingToZero() {
        let push = MockPushService()
        push.applyIncomingBadge(from: ["badge": 7])
        XCTAssertEqual(push.badgeApplyCallCount, 1)
        XCTAssertEqual(push.lastAppIconBadgeCount, 7)
        XCTAssertNotEqual(push.lastAppIconBadgeCount, 0)

        push.applyAppIconBadge(0)
        XCTAssertEqual(push.lastAppIconBadgeCount, 0)
        XCTAssertEqual(push.appIconBadgeApplyCallCount, 2)
    }

    func testApplyCurrentSummaryUsesRoomListLaterAndSkipsUnreadySnapshots() async {
        let push = MockPushService()
        let rooms = [
            RoomSummary(
                id: "!unread:matrix.org",
                name: "Unread",
                lastMessagePreview: "hello",
                unreadCount: 4,
                hasHighlight: false,
                kind: .room,
                membership: .joined,
                lastActivityAt: Date()
            )
        ]
        let later = MockLaterService(
            items: [
                SynaraLaterListItem(
                    id: "saved",
                    roomID: "!unread:matrix.org",
                    eventID: "$one",
                    kind: .saved,
                    dueTs: nil,
                    completedAt: nil,
                    createdAt: 1,
                    isCompleted: false
                )
            ]
        )

        await AppIconBadge.applyCurrentSummary(rooms: rooms, later: later, push: push)
        XCTAssertEqual(push.lastAppIconBadgeCount, 5)

        await AppIconBadge.applyCurrentSummary(
            roomList: MockRoomListService(state: .loading),
            later: later,
            push: push
        )
        XCTAssertEqual(push.lastAppIconBadgeCount, 5)
        XCTAssertEqual(push.appIconBadgeApplyCallCount, 1)
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

    func testPushServiceRegistersWhenTheEditPolicyIsDegradedAndSaysSo() async {
        let pusher = StubPusherService()
        pusher.editSuppressionDiagnostic = "v-pusher.edit-policy-failed"

        let service = SynaraPushService(
            logger: MockLoggingService(),
            pusherService: pusher,
            isRegistrationAvailable: true
        )
        service.configure(with: makeSession())
        service.handleDeviceToken(Data([0x7A, 0xB1, 0x3C]))

        await waitUntil {
            service.isRegistered && pusher.registerCount >= 1
        }
        XCTAssertTrue(service.isRegistered)
        XCTAssertEqual(
            service.registrationStateDescription,
            SynaraPushService.registeredWithoutEditSuppressionDescription
        )
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
            "Push cleanup unavailable; finishing local sign out"
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

    func testTokenRotatedDuringFailedRemoteCleanupStaysQuiescentUntilLocalOutcome() async {
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
        XCTAssertFalse(cleanupSucceeded)
        XCTAssertEqual(pusher.unregisterAllCount, 1)
        XCTAssertEqual(pusher.registerCount, 1)
        XCTAssertEqual(service.tokenSnippet, "7ab13c")
        service.completeRegistrationTeardown()
        XCTAssertNil(service.tokenSnippet)
        XCTAssertFalse(service.isRegistered)
        XCTAssertEqual(pusher.registerCount, 1)

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

    func testSparseRouteResolverOpensTheRoomCoreFindsForTheEvent() async {
        // I29: an event-only push resolves through Core's cached-event
        // lookup, waiting a little for sync to store the event.
        let calls = LockedCounter()
        let paused = LockedCounter()
        let resolver = SharedCoreSparsePushRouteResolver(
            lookup: { eventID in
                XCTAssertEqual(eventID, "$sparse:matrix.org")
                switch calls.increment() {
                case 1:
                    throw TimelineError.Failed(code: "p4-session-suspended", description: "suspended")
                case 2:
                    return nil
                default:
                    return TimelineEventLocationDto(
                        roomId: "!resolved:matrix.org",
                        route: TimelineEventRouteDto(kind: .thread, threadRootEventId: "$root:matrix.org")
                    )
                }
            },
            pause: { _ in paused.increment() },
            attempts: 4
        )

        let route = await resolver.resolveRoute(eventID: "$sparse:matrix.org")

        assertRoute(route, matchesRoom: "!resolved:matrix.org", eventID: "$sparse:matrix.org")
        XCTAssertEqual(calls.value, 3)
        XCTAssertEqual(paused.value, 2)
    }

    func testSparseRouteResolverGivesUpWithoutGuessing() async {
        let calls = LockedCounter()
        let resolver = SharedCoreSparsePushRouteResolver(
            lookup: { _ in
                calls.increment()
                return nil
            },
            pause: { _ in },
            attempts: 3
        )
        let missing = await resolver.resolveRoute(eventID: "$unknown:matrix.org")
        XCTAssertNil(missing)
        XCTAssertEqual(calls.value, 3)

        let notReady = SharedCoreSparsePushRouteResolver(
            lookup: { _ in
                XCTFail("no lookup before the session is ready")
                return nil
            },
            isReady: { false }
        )
        let unready = await notReady.resolveRoute(eventID: "$sparse:matrix.org")
        XCTAssertNil(unready)
        XCTAssertNil(SharedCoreSparsePushRouteResolver.route(
            eventID: "$sparse:matrix.org",
            location: TimelineEventLocationDto(
                roomId: "",
                route: TimelineEventRouteDto(kind: .room, threadRootEventId: nil)
            )
        ))
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

private final class LockedCounter: @unchecked Sendable {
    private let lock = NSLock()
    private var count = 0

    var value: Int {
        lock.lock()
        defer { lock.unlock() }
        return count
    }

    @discardableResult
    func increment() -> Int {
        lock.lock()
        defer { lock.unlock() }
        count += 1
        return count
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
        var editSuppressionDiagnostic: String?

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

    fileprivate func registerPusher(session: AuthenticatedSession, pushKey: String) async throws -> String? {
        registerCount += 1
        lastPushKey = pushKey
        registeredSessions.append(session)
        onRegister()
        if registerDelayNanoseconds > 0 {
            try await Task.sleep(nanoseconds: registerDelayNanoseconds)
        }
        return editSuppressionDiagnostic
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

    func registerPusher(pushKey: String) async throws -> String? {
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
        return PusherWriteDto(status: .ok, editSuppressionDiagnosticId: nil)
    }

    func deleteHttpPusher(pushKey: String, appId: String) async throws -> PusherWriteDto {
        _ = appId
        deletedPushKeys.append(pushKey)
        return PusherWriteDto(status: .ok, editSuppressionDiagnosticId: nil)
    }

    func deleteHttpPushersForDevice(
        appId: String,
        lastPushKey: String?
    ) async throws -> PusherWriteDto {
        _ = appId
        _ = lastPushKey
        deleteForDeviceCount += 1
        return PusherWriteDto(status: .ok, editSuppressionDiagnosticId: nil)
    }
}
