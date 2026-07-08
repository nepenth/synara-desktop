import SwiftUI
import UIKit
import UserNotifications

@main
struct SynaraApp: App {
    @UIApplicationDelegateAdaptor(SynaraAppDelegate.self) private var appDelegate
    @Environment(\.scenePhase) private var scenePhase

    private let environment: AppEnvironment = {
        PerformanceTrace.event("AppEnvironmentCreate")
        if ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1" {
            return .uiTest()
        }
        return .live()
    }()

    init() {
        PerformanceTrace.event("AppInit")
        let environment = ProcessInfo.processInfo.environment
        if environment["SYNARA_UI_TESTS"] == "1" || environment["SYNARA_DISABLE_ANIMATIONS"] == "1" {
            UIView.setAnimationsEnabled(false)
        }
    }

    private func handleScenePhaseChange(_ phase: ScenePhase) async {
        switch phase {
        case .active:
            PerformanceTrace.event("SceneActive")
            if case .signedIn(let session) = environment.session.currentState {
                await environment.matrix.resumeFromForeground(session: session)
            }
        case .background:
            PerformanceTrace.event("SceneBackground")
            await environment.matrix.pauseForBackground()
        case .inactive:
            PerformanceTrace.event("SceneInactive")
        @unknown default:
            PerformanceTrace.event("SceneUnknown")
        }
    }

    var body: some Scene {
        WindowGroup {
            RootShellView(environment: environment)
                .onAppear {
                    PerformanceTrace.event("RootShellAppear")
                    appDelegate.bind(to: environment)
                }
                .onChange(of: scenePhase) { phase in
                    Task {
                        await handleScenePhaseChange(phase)
                    }
                }
        }
    }
}

final class SynaraAppDelegate: NSObject, UIApplicationDelegate, UNUserNotificationCenterDelegate {
    private var push: PushServicing?
    private var matrix: MatrixClientServicing?
    private var session: AppSessionStore?
    private var router: AppRouter?
    private var logger: LoggingServicing?
    private var agentApprovalReactions: AgentApprovalReactionServicing?
    private var pendingRoute: AppRoute?
    private var pendingNotificationPayload: [AnyHashable: Any]?

    func bind(to environment: AppEnvironment) {
        push = environment.push
        matrix = environment.matrix
        session = environment.session
        router = environment.router
        logger = environment.logger
        agentApprovalReactions = environment.agentApprovalReactions
        UNUserNotificationCenter.current().delegate = self

        if let pendingRoute {
            routeToDestination(pendingRoute)
            self.pendingRoute = nil
        }

        if let pendingNotificationPayload {
            let payload = pendingNotificationPayload
            self.pendingNotificationPayload = nil
            Task { @MainActor in
                await self.resolveNotificationRoute(from: payload)
            }
        }
    }

    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        SynaraNotificationActionContract.registerCategories()

        if let remotePayload = launchOptions?[.remoteNotification] as? [AnyHashable: Any] {
            if let route = NotificationPushRouteParser.route(from: remotePayload) {
                routeToDestination(route)
            } else if NotificationPushRouteParser.sparseEventID(from: remotePayload) != nil {
                pendingNotificationPayload = remotePayload
            } else {
                routeToFallback()
            }
        }

        return true
    }

    func application(
        _ application: UIApplication,
        didRegisterForRemoteNotificationsWithDeviceToken deviceToken: Data
    ) {
        push?.handleDeviceToken(deviceToken)
    }

    func application(
        _ application: UIApplication,
        didFailToRegisterForRemoteNotificationsWithError error: Error
    ) {
        logger?.error("APNs registration failed: \(error.localizedDescription)", category: .push)
    }

    func application(
        _ application: UIApplication,
        didReceiveRemoteNotification userInfo: [AnyHashable: Any],
        fetchCompletionHandler completionHandler: @escaping (UIBackgroundFetchResult) -> Void
    ) {
        Task {
            let result = await handleBackgroundRemoteNotification(userInfo)
            completionHandler(result)
        }
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse
    ) async {
        guard let push else {
            return
        }

        let userInfo = response.notification.request.content.userInfo
        if let request = SynaraNotificationActionContract.agentApprovalReactionRequest(
            actionIdentifier: response.actionIdentifier,
            userInfo: userInfo
        ) {
            await handleAgentApprovalNotificationAction(request, userInfo: userInfo)
            return
        }

        if let route = await push.resolveRoute(from: userInfo) {
            routeToDestination(route)
            push.applyIncomingBadge(from: userInfo)
        } else {
            routeToFallback()
        }
    }

    func applicationDidBecomeActive(_ application: UIApplication) {
        Task { @MainActor in
            clearBadgeToZero()
        }
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification
    ) async -> UNNotificationPresentationOptions {
        await MainActor.run {
            push?.applyIncomingBadge(from: notification.request.content.userInfo)
        }

        return [.banner, .sound, .badge, .list]
    }

    private func handleBackgroundRemoteNotification(_ userInfo: [AnyHashable: Any]) async -> UIBackgroundFetchResult {
        await MainActor.run {
            push?.applyIncomingBadge(from: userInfo)
        }

        if let badge = push?.parseBadgeCount(from: userInfo) {
            logger?.info("Background push updated badge to \(badge)", category: .push)
        }

        guard let matrix, let session, case .signedIn(let authenticatedSession) = session.currentState else {
            return push?.parseBadgeCount(from: userInfo) == nil ? .noData : .newData
        }

        if await matrix.syncForBackgroundNotification(session: authenticatedSession) {
            logger?.info("Background push sync completed", category: .push)
            return .newData
        }

        return push?.parseBadgeCount(from: userInfo) == nil ? .noData : .newData
    }

    private func resolveNotificationRoute(from payload: [AnyHashable: Any]) async {
        if let route = await push?.resolveRoute(from: payload)
            ?? NotificationPushRouteParser.route(from: payload) {
            routeToDestination(route)
        } else {
            routeToFallback()
        }
    }

    private func handleAgentApprovalNotificationAction(
        _ request: SynaraAgentApprovalReactionRequest,
        userInfo: [AnyHashable: Any]
    ) async {
        guard let agentApprovalReactions else {
            routeToFallback()
            return
        }

        do {
            try await agentApprovalReactions.submitReaction(request)
            await MainActor.run {
                push?.applyIncomingBadge(from: userInfo)
            }
        } catch {
            logger?.error("Agent approval notification action failed: \(error.localizedDescription)", category: .push)
            await resolveNotificationRoute(from: userInfo)
        }
    }

    private func routeToDestination(_ route: AppRoute) {
        guard let router else {
            pendingRoute = route
            return
        }

        Task { @MainActor in
            router.route(to: route)
            clearBadgeToZero()
        }
    }

    private func routeToFallback() {
        guard let router else {
            pendingRoute = .notifications
            return
        }

        Task { @MainActor in
            router.routeToNotificationFallback()
            clearBadgeToZero()
        }
    }

    private func clearBadgeToZero() {
        UNUserNotificationCenter.current().setBadgeCount(0) { error in
            if let error {
                self.logger?.error("Failed to clear badge: \(error.localizedDescription)", category: .push)
            }
        }
    }
}

private extension AppEnvironment {
    static func uiTest() -> AppEnvironment {
        let processEnvironment = ProcessInfo.processInfo.environment
        let shouldStartSignedIn = processEnvironment["SYNARA_UI_TEST_SIGNED_IN"] == "1"
            || processEnvironment["SYNARA_UI_TEST_ROOM_ID"] != nil

        guard shouldStartSignedIn else {
            return .mock()
        }

        let router = AppRouter()
        let session = AppSessionStore(
            currentState: .signedIn(
                AuthenticatedSession(
                    userID: "@alice:matrix.org",
                    deviceID: "UITEST",
                    homeserverURL: URL(string: "https://matrix.org")!,
                    accessToken: "ui-test-token"
                )
            )
        )

        let inviteTransitionService: MockInviteTransitionService?
        if processEnvironment["SYNARA_UI_TEST_INVITE"] == "1" {
            inviteTransitionService = MockInviteTransitionService()
        } else {
            inviteTransitionService = nil
        }

        if let roomID = processEnvironment["SYNARA_UI_TEST_ROOM_ID"] {
            let title = processEnvironment["SYNARA_UI_TEST_ROOM_TITLE"]
            router.route(to: .room(id: roomID, title: title))
        } else if processEnvironment["SYNARA_UI_TEST_SELECTED_TAB"] == "settings" {
            router.selectedTab = .settings
        } else if processEnvironment["SYNARA_UI_TEST_SELECTED_TAB"] == "later" {
            router.selectedTab = .later
        } else if processEnvironment["SYNARA_UI_TEST_SELECTED_TAB"] == "notifications" {
            router.selectedTab = .notifications
        }

        let timeline: TimelineServicing
        if processEnvironment["SYNARA_UI_TEST_AGENT_CARD"] == "1" {
            timeline = MockTimelineService(events: uiTestAgentCardEvents())
        } else if processEnvironment["SYNARA_UI_TEST_AGENT_APPROVAL_PROMPT"] == "1" {
            timeline = MockTimelineService(events: uiTestAgentApprovalPromptEvents())
        } else if processEnvironment["SYNARA_UI_TEST_ENCRYPTED_TIMELINE"] == "1" {
            timeline = MockTimelineService(items: uiTestEncryptedTimelineItems())
        } else if processEnvironment["SYNARA_UI_TEST_LARGE_TIMELINE"] == "1" {
            let count = processEnvironment["SYNARA_UI_TEST_LARGE_TIMELINE_COUNT"]
                .flatMap(Int.init) ?? 10_000
            timeline = MockTimelineService(items: TimelineFixtures.largeTimeline(count: count))
        } else {
            timeline = MockTimelineService()
        }

        let roomList = processEnvironment["SYNARA_UI_TEST_LARGE_ROOMS"] == "1"
            ? MockRoomListService(state: .loaded(RoomListFixtures.large()))
            : MockRoomListService()
        let later = processEnvironment["SYNARA_UI_TEST_LATER_ITEMS"] == "1"
            ? MockLaterService(items: uiTestLaterItems())
            : MockLaterService()
        let approvalError: SynaraAgentApprovalError? = processEnvironment["SYNARA_UI_TEST_AGENT_APPROVAL_ERROR"] == "failed"
            ? .failed
            : nil
        let agentApprovals = MockAgentApprovalService(error: approvalError)
        let agentApprovalReactions = MockAgentApprovalReactionService(error: approvalError)
        let readMarkers = MockRoomReadMarkerService(eventID: processEnvironment["SYNARA_UI_TEST_READ_MARKER_EVENT_ID"])
        let crypto = processEnvironment["SYNARA_UI_TEST_ENCRYPTED_TIMELINE"] == "1"
            ? MockCryptoStatusService(
                roomCryptoStatus: RoomCryptoStatus(
                    encryption: .encrypted,
                    verification: .unverified,
                    recovery: .incomplete,
                    backup: .unavailable,
                    unableToDecryptCount: 1
                ),
                sessionCryptoStatus: SessionCryptoStatus(
                    verification: .unverified,
                    recovery: .incomplete,
                    backup: .unavailable,
                    hasDevicesToVerifyAgainst: true,
                    isLastDevice: false,
                    unableToDecryptCount: 1
                )
            )
            : MockCryptoStatusService()

        if let inviteTransitionService {
            return .mock(
                router: router,
                session: session,
                roomList: inviteTransitionService,
                roomMembership: inviteTransitionService,
                timeline: timeline,
                later: later,
                agentApprovals: agentApprovals,
                agentApprovalReactions: agentApprovalReactions,
                readMarkers: readMarkers,
                crypto: crypto
            )
        }

        return .mock(
            router: router,
            session: session,
            roomList: roomList,
            timeline: timeline,
            later: later,
            agentApprovals: agentApprovals,
            agentApprovalReactions: agentApprovalReactions,
            readMarkers: readMarkers,
            crypto: crypto
        )
    }

    static func uiTestLaterItems() -> [SynaraLaterListItem] {
        [
            SynaraLaterListItem(
                id: "saved-active",
                roomID: "!project:matrix.org",
                eventID: "$text:!project:matrix.org",
                kind: .saved,
                dueTs: nil,
                completedAt: nil,
                createdAt: 1_770_000_000_000,
                isCompleted: false
            ),
            SynaraLaterListItem(
                id: "reminder-missing-destination",
                roomID: "",
                eventID: "",
                kind: .reminder,
                dueTs: 1_700_000_000_000,
                completedAt: nil,
                createdAt: 1_769_999_000_000,
                isCompleted: false
            ),
            SynaraLaterListItem(
                id: "saved-completed",
                roomID: "!project:matrix.org",
                eventID: "$done",
                kind: .saved,
                dueTs: nil,
                completedAt: 1_770_000_100_000,
                createdAt: 1_769_998_000_000,
                isCompleted: true
            )
        ]
    }

    static func uiTestEncryptedTimelineItems() -> [TimelineItem] {
        [
            TimelineItem(
                id: "$decrypted:matrix.org",
                eventID: "$decrypted:matrix.org",
                senderID: "@alice:matrix.org",
                timestamp: TimelineFixtures.baseDate,
                kind: .text("Decrypted encrypted-room message"),
                replyToEventID: nil,
                isEdited: false,
                reactions: [:],
                isEncrypted: true
            ),
            TimelineItem(
                id: "$utd:matrix.org",
                eventID: "$utd:matrix.org",
                senderID: "@bob:matrix.org",
                timestamp: TimelineFixtures.baseDate.addingTimeInterval(30),
                kind: .encryptedPlaceholder,
                replyToEventID: nil,
                isEdited: false,
                reactions: [:],
                isEncrypted: true
            )
        ]
    }

    static func uiTestAgentCardEvents() -> [RawTimelineEvent] {
        [
            RawTimelineEvent(
                eventID: "$agent-ui",
                senderID: "@agent:matrix.org",
                timestamp: Date(timeIntervalSince1970: 1_770_000_000),
                type: "in.synara.agent",
                body: nil,
                replyToEventID: nil,
                isEdited: false,
                mediaURL: nil,
                agentCard: try? uiTestAgentCard()
            )
        ]
    }

    static func uiTestAgentApprovalPromptEvents() -> [RawTimelineEvent] {
        [
            RawTimelineEvent(
                eventID: "$agent-approval-prompt",
                senderID: "@automation:matrix.example.com",
                timestamp: Date(timeIntervalSince1970: 1_770_000_120),
                type: "m.room.message",
                body: uiTestAgentApprovalPromptBody(),
                replyToEventID: nil,
                isEdited: false,
                mediaURL: nil
            )
        ]
    }

    static func uiTestAgentApprovalPromptBody() -> String {
        """
        ⚠️ Dangerous command requires approval

        Code

        Copy
        set -euo pipefail
        curl -fsS http://browser-control.example.com:9377/openapi.json -o /tmp/camofox_openapi.json

        Reason: Security scan - [HIGH] Plain HTTP URL in execution context.

        Reply !approve to execute, !approve session to approve this pattern for the session, !approve always to approve permanently, or !deny to cancel.

        You can also react to this prompt:
        ✅ = approve once
        ♾️ = approve always
        ❌ = deny
        """
    }

    static func uiTestAgentCard() throws -> SynaraAgentCard {
        let actions = [
            try? SynaraAgentCardAction(
                id: "approve-deploy",
                title: "Approve",
                kind: "approve",
                prompt: "approve deployment"
            ),
            try? SynaraAgentCardAction(
                id: "reject-deploy",
                title: "Reject",
                kind: "reject",
                prompt: "reject deployment"
            ),
            try? SynaraAgentCardAction(
                id: "copy-prompt",
                title: "Copy Prompt",
                kind: "copy_prompt",
                prompt: "deploy only service api"
            ),
            try? SynaraAgentCardAction(
                id: "view-changes",
                title: "View changes",
                kind: "open_url",
                url: "https://staging.synara.app/review/a1b2c3d"
            )
        ].compactMap { $0 }

        return try SynaraAgentCard(
            title: "Deploy to Production",
            status: "Pending approval",
            summary: "Includes user permissions update and audit log improvements.",
            actions: actions,
            artifacts: [
                try SynaraAgentCardArtifact(title: "api.synara.app", type: "deployment", summary: "Production target")
            ],
            logs: [
                try SynaraAgentCardCodeBlock(id: "build", title: "Build", language: "text", code: "Passed"),
                try SynaraAgentCardCodeBlock(id: "tests", title: "Tests", language: "text", code: "Passed"),
                try SynaraAgentCardCodeBlock(id: "security", title: "Security scan", language: "text", code: "Passed")
            ]
        )
    }
}
