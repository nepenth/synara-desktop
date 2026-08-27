import SwiftUI
import UIKit
import UserNotifications

@main
struct SynaraApp: App {
    @UIApplicationDelegateAdaptor(SynaraAppDelegate.self) private var appDelegate

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

    var body: some Scene {
        WindowGroup {
            SynaraRootHost(environment: environment, appDelegate: appDelegate)
        }
    }
}

/// Structurally binds lifecycle authority before constructing RootShellView.
/// This removes any scheduler race between the shell's signed-in `.task` and
/// the app delegate's foreground/background gate.
private struct SynaraRootHost: View {
    let environment: AppEnvironment
    let appDelegate: SynaraAppDelegate
    @Environment(\.scenePhase) private var scenePhase
    @State private var delegateBound = false
    @State private var shellReady = false

    var body: some View {
        Group {
            if shellReady {
                RootShellView(environment: environment)
            } else {
                Color.clear
            }
        }
        .onAppear {
            apply(phase: scenePhase)
        }
        .onChange(of: scenePhase) { phase in
            apply(phase: phase)
        }
    }

    private func apply(phase: ScenePhase) {
        // Bind services in every launch mode so push routing is available, but
        // do not construct RootShellView (and therefore its Matrix `.task`)
        // until both SwiftUI and UIKit say the process is foreground-active.
        appDelegate.updateScenePhase(phase)
        if delegateBound == false {
            appDelegate.bind(to: environment)
            delegateBound = true
        }
        guard shellReady == false,
              phase == .active,
              UIApplication.shared.applicationState == .active
        else {
            return
        }
        PerformanceTrace.event("RootShellAppear")
        shellReady = true
    }
}

@MainActor
protocol SynaraBackgroundTaskManaging: AnyObject {
    func beginBackgroundTask(
        withName taskName: String?,
        expirationHandler handler: (@Sendable () -> Void)?
    ) -> UIBackgroundTaskIdentifier
    func endBackgroundTask(_ identifier: UIBackgroundTaskIdentifier)
}

extension UIApplication: SynaraBackgroundTaskManaging {}

enum SynaraForegroundMatrixMutationPolicy {
    static func hasAuthority(
        lifecycleActive: Bool,
        applicationState: UIApplication.State
    ) -> Bool {
        lifecycleActive && applicationState == .active
    }

    static func allowsMutation(
        lifecycleActive: Bool,
        applicationState: UIApplication.State,
        syncStatus: MatrixSyncStatus
    ) -> Bool {
        guard hasAuthority(
            lifecycleActive: lifecycleActive,
            applicationState: applicationState
        ) else {
            return false
        }
        switch syncStatus {
        case .starting, .syncing, .connected, .reconnecting:
            return true
        case .stopped, .disconnected, .restoreFailed, .failed:
            return false
        }
    }
}

/// Owns the sole iOS suspension transition. Quiescence begins at
/// `applicationWillResignActive`, before UIKit can suspend the process. The
/// background assertion remains held until the native lifecycle transaction
/// has stopped SyncService, drained all in-flight store work, and released
/// every SQLite connection. Foreground activation serially reopens the stores
/// before restarting the same session owner.
@MainActor
final class SynaraBackgroundSyncCoordinator {
    private var backgroundTaskIdentifier: UIBackgroundTaskIdentifier = .invalid
    private var pauseTask: Task<Void, Never>?
    private var needsForegroundResume = false
    private var isForeground = true

    var hasPendingPause: Bool {
        pauseTask != nil
    }

    func bind(
        application: SynaraBackgroundTaskManaging,
        matrix: MatrixClientServicing,
        foregroundActive: Bool
    ) {
        guard foregroundActive == false else {
            isForeground = true
            matrix.setForegroundActive(true)
            return
        }
        enterBackground(application: application, matrix: matrix)
    }

    func enterBackground(
        application: SynaraBackgroundTaskManaging,
        matrix: MatrixClientServicing
    ) {
        let wasForeground = isForeground
        isForeground = false
        needsForegroundResume = true
        matrix.setForegroundActive(false)

        // SwiftUI delivers `.inactive` before `.background`, and UIKit may
        // independently report the same transition. Once quiescence has
        // completed, later duplicate callbacks must not checkpoint closed
        // stores a second time.
        guard wasForeground || pauseTask != nil else {
            return
        }

        if backgroundTaskIdentifier == .invalid {
            backgroundTaskIdentifier = application.beginBackgroundTask(
                withName: "Quiesce Matrix stores before suspension"
            ) { [weak self, weak application] in
                Task { @MainActor [weak self, weak application] in
                    guard let self, let application else {
                        return
                    }
                    PerformanceTrace.event("BackgroundSyncAssertionExpired")
                    self.finishBackgroundTask(application: application)
                }
            }
        }

        guard pauseTask == nil else {
            return
        }
        PerformanceTrace.event("BackgroundSyncPauseBegin")

        pauseTask = Task { [weak self, weak application] in
            await matrix.pauseForBackground()
            guard let self, let application else {
                return
            }
            self.pauseTask = nil
            PerformanceTrace.event("BackgroundSyncPauseComplete")
            self.finishBackgroundTask(application: application)
        }
    }

    func enterForeground(
        matrix: MatrixClientServicing,
        session: AuthenticatedSession
    ) {
        isForeground = true
        matrix.setForegroundActive(true)
        guard needsForegroundResume else {
            return
        }
        needsForegroundResume = false
        let pendingPause = pauseTask
        Task { [weak self] in
            await pendingPause?.value
            guard let self, self.isForeground else {
                PerformanceTrace.event("ForegroundSyncResumeSuppressed")
                return
            }
            PerformanceTrace.event("ForegroundSyncResumeBegin")
            await matrix.resumeFromForeground(session: session)
        }
    }

    private func finishBackgroundTask(application: SynaraBackgroundTaskManaging) {
        guard backgroundTaskIdentifier != .invalid else {
            return
        }
        let identifier = backgroundTaskIdentifier
        backgroundTaskIdentifier = .invalid
        application.endBackgroundTask(identifier)
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
    private var pendingNotificationResponse: UNNotificationResponse?
    /// UIKit lifecycle callbacks can precede SwiftUI's `.onAppear`, where the
    /// shared environment is bound. Retain foreground authority independently
    /// so a callback is never lost merely because Matrix is not attached yet.
    private var foregroundActive = false
    private let agentApprovalActionDedupe = SynaraAgentApprovalNotificationActionDedupeStore()
    private let backgroundSyncCoordinator = SynaraBackgroundSyncCoordinator()

    func bind(to environment: AppEnvironment) {
        push = environment.push
        matrix = environment.matrix
        session = environment.session
        router = environment.router
        logger = environment.logger
        agentApprovalReactions = environment.agentApprovalReactions
        UNUserNotificationCenter.current().delegate = self
        backgroundSyncCoordinator.bind(
            application: UIApplication.shared,
            matrix: environment.matrix,
            foregroundActive: foregroundActive
        )

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
        drainPendingNotificationResponseIfReady()
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
        logNotificationPayloadShape(userInfo, context: "background")
        Task {
            let result = await handleBackgroundRemoteNotification(userInfo)
            completionHandler(result)
        }
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse
    ) async {
        guard push != nil, foregroundActive else {
            pendingNotificationResponse = response
            return
        }

        await handleNotificationResponse(response)
    }

    private func handleNotificationResponse(_ response: UNNotificationResponse) async {
        guard let push else {
            pendingNotificationResponse = response
            return
        }
        let userInfo = response.notification.request.content.userInfo
        logNotificationPayloadShape(userInfo, context: "response")
        if SynaraAgentApprovalNotificationActionID(rawValue: response.actionIdentifier) != nil {
            await handleAgentApprovalNotificationAction(
                actionIdentifier: response.actionIdentifier,
                userInfo: userInfo
            )
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
        updateForegroundActive(true)
        Task { @MainActor in
            clearBadgeToZero()
        }
    }

    func applicationWillResignActive(_ application: UIApplication) {
        updateForegroundActive(false)
    }

    func applicationDidEnterBackground(_ application: UIApplication) {
        updateForegroundActive(false)
    }

    /// SwiftUI is the authoritative lifecycle source for the window scene.
    /// UIApplicationDelegate callbacks remain as a redundant early signal,
    /// while this bridge closes pre-bind and scene-only delivery gaps.
    func updateScenePhase(_ phase: ScenePhase) {
        switch phase {
        case .active:
            // A SwiftUI scene sample cannot overrule process-level UIKit
            // background state during notification or restoration launches.
            updateForegroundActive(UIApplication.shared.applicationState == .active)
        case .inactive, .background:
            updateForegroundActive(false)
        @unknown default:
            updateForegroundActive(false)
        }
    }

    private func updateForegroundActive(_ active: Bool) {
        foregroundActive = active
        guard let matrix else {
            return
        }
        if active,
           let session,
           case .signedIn(let authenticatedSession) = session.currentState {
            PerformanceTrace.event("SceneActive")
            backgroundSyncCoordinator.enterForeground(
                matrix: matrix,
                session: authenticatedSession
            )
            drainPendingNotificationResponseIfReady()
            return
        }

        guard active == false else {
            matrix.setForegroundActive(true)
            drainPendingNotificationResponseIfReady()
            return
        }
        PerformanceTrace.event("SceneBackground")
        // Start at inactive, before background: a sync response can be
        // committing an event-cache transaction while UIKit advances the app
        // toward suspension. The coordinator is idempotent across duplicate
        // SwiftUI and UIApplication lifecycle delivery.
        backgroundSyncCoordinator.enterBackground(
            application: UIApplication.shared,
            matrix: matrix
        )
    }

    private func drainPendingNotificationResponseIfReady() {
        guard foregroundActive, push != nil, let pendingNotificationResponse else {
            return
        }
        self.pendingNotificationResponse = nil
        Task { await self.handleNotificationResponse(pendingNotificationResponse) }
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification
    ) async -> UNNotificationPresentationOptions {
        logNotificationPayloadShape(notification.request.content.userInfo, context: "foreground")
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

    private func logNotificationPayloadShape(_ userInfo: [AnyHashable: Any], context: String) {
        let shape = NotificationPushRouteParser.alertShape(from: userInfo)
        logger?.info("Remote notification payload shape (\(context)): \(shape.logSummary)", category: .push)
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
        actionIdentifier: String,
        userInfo: [AnyHashable: Any]
    ) async {
        let plan = SynaraNotificationActionContract.planAgentApprovalNotificationAction(
            actionIdentifier: actionIdentifier,
            userInfo: userInfo
        )

        switch plan {
        case .ignore(let reason):
            logger?.info("Agent approval notification action ignored: \(reason)", category: .push)
            await resolveNotificationRoute(from: userInfo)
        case .openRoom(let roomID, let eventID, let reason):
            // Approve-always (and other open-only dispositions) require in-app confirmation.
            logger?.info("Agent approval notification action opening room: \(reason)", category: .push)
            routeToDestination(.room(id: roomID, eventID: eventID))
            await MainActor.run {
                push?.applyIncomingBadge(from: userInfo)
            }
        case .submitReaction(let request):
            let dedupeKey = SynaraAgentApprovalNotificationActionDedupeStore.key(
                roomID: request.roomID,
                eventID: request.sourceEventID,
                actionIdentifier: actionIdentifier
            )
            guard agentApprovalActionDedupe.contains(dedupeKey) == false else {
                logger?.info("Agent approval notification action ignored: already-acted", category: .push)
                routeToDestination(.room(id: request.roomID, eventID: request.sourceEventID))
                return
            }

            // A foreground notification action may cold-launch the process,
            // and its delegate callback can beat asynchronous store resume.
            // Join the serialized lifecycle chain before touching Matrix.
            guard await prepareForegroundApprovalSessionIfNeeded() else {
                logger?.error("Agent approval notification action failed: signed-in Matrix owner unavailable", category: .push)
                await resolveNotificationRoute(from: userInfo)
                return
            }

            guard let agentApprovalReactions else {
                routeToDestination(.room(id: request.roomID, eventID: request.sourceEventID))
                return
            }

            agentApprovalActionDedupe.insert(dedupeKey)
            do {
                try await agentApprovalReactions.submitNativeDecision(
                    roomID: request.roomID,
                    eventID: request.sourceEventID,
                    actionIdentifier: actionIdentifier
                )
                await MainActor.run {
                    push?.applyIncomingBadge(from: userInfo)
                }
            } catch {
                agentApprovalActionDedupe.remove(dedupeKey)
                logger?.error("Agent approval notification action failed: \(error.localizedDescription)", category: .push)
                await resolveNotificationRoute(from: userInfo)
            }
        }
    }

    private func prepareForegroundApprovalSessionIfNeeded() async -> Bool {
        let applicationState = UIApplication.shared.applicationState
        guard SynaraForegroundMatrixMutationPolicy.hasAuthority(
            lifecycleActive: foregroundActive,
            applicationState: applicationState
        ),
              let matrix,
              let session,
              case let .signedIn(authenticatedSession) = session.currentState
        else {
            return false
        }

        await matrix.resumeFromForeground(session: authenticatedSession)
        return SynaraForegroundMatrixMutationPolicy.allowsMutation(
            lifecycleActive: foregroundActive,
            applicationState: UIApplication.shared.applicationState,
            syncStatus: matrix.syncStatus
        )
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
            switch processEnvironment["SYNARA_UI_TEST_VIEWPORT_SCENARIO"] {
            case "busy-composer":
                let service = MockTimelineService(
                    items: TimelineFixtures.largeTimeline(indices: 0..<300, usesFormattedHTML: true)
                )
                service.updateIntervalNanoseconds = 25_000_000
                service.updateOutcomes = (300..<360).map { newestIndex in
                    .loaded(
                        TimelineFixtures.largeTimeline(
                            indices: (newestIndex - 299)..<(newestIndex + 1),
                            usesFormattedHTML: true
                        )
                    )
                }
                timeline = service
            case "height-change":
                let service = MockTimelineService(items: TimelineFixtures.largeTimeline(indices: 100..<140))
                service.updateDelayNanoseconds = 5_000_000_000
                service.updateOutcomes = [
                    .loaded(
                        TimelineFixtures.largeTimeline(
                            indices: 100..<140,
                            expandedMessageIndex: 137,
                            expandedLineCount: 180
                        )
                    )
                ]
                timeline = service
            case "prepend":
                let service = MockTimelineService(items: TimelineFixtures.largeTimeline(indices: 100..<140))
                service.updateDelayNanoseconds = 5_000_000_000
                service.updateOutcomes = [.loaded(TimelineFixtures.largeTimeline(indices: 50..<140))]
                timeline = service
            default:
                timeline = MockTimelineService(items: TimelineFixtures.largeTimeline(count: count))
            }
        } else {
            timeline = MockTimelineService()
        }

        let roomList = processEnvironment["SYNARA_UI_TEST_LARGE_ROOMS"] == "1"
            ? MockRoomListService(state: .loaded(RoomListFixtures.large()))
            : MockRoomListService()
        let later = processEnvironment["SYNARA_UI_TEST_LATER_ITEMS"] == "1"
            ? MockLaterService(items: uiTestLaterItems())
            : MockLaterService()
        let roomNotes = processEnvironment["SYNARA_UI_TEST_ROOM_NOTES"] == "1"
            ? MockRoomNotesService(items: uiTestRoomNotesItems())
            : MockRoomNotesService()
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
                roomNotes: roomNotes,
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
            roomNotes: roomNotes,
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

    static func uiTestRoomNotesItems() -> [SynaraRoomNoteItem] {
        let base = Date(timeIntervalSince1970: 1_770_000_000)
        return [
            SynaraRoomNoteItem(
                id: "todo-active",
                kind: .todo,
                roomID: "!project:matrix.org",
                createdAt: base,
                updatedAt: base.addingTimeInterval(60),
                body: "Review the launch checklist",
                completedAt: nil,
                order: 1_770_000_060_000,
                eventID: nil,
                eventTimestamp: nil,
                senderID: nil
            ),
            SynaraRoomNoteItem(
                id: "note-private",
                kind: .note,
                roomID: "!project:matrix.org",
                createdAt: base,
                updatedAt: base,
                body: "Discuss the migration privately",
                completedAt: nil,
                order: 1_770_000_000_000,
                eventID: nil,
                eventTimestamp: nil,
                senderID: nil
            ),
            SynaraRoomNoteItem(
                id: "note-follow-up",
                kind: .note,
                roomID: "!project:matrix.org",
                createdAt: base.addingTimeInterval(-120),
                updatedAt: base.addingTimeInterval(-120),
                body: "Capture the private follow-up",
                completedAt: nil,
                order: 1_769_999_880_000,
                eventID: nil,
                eventTimestamp: nil,
                senderID: nil
            ),
            SynaraRoomNoteItem(
                id: "message-anchor",
                kind: .message,
                roomID: "!project:matrix.org",
                createdAt: base,
                updatedAt: base.addingTimeInterval(-60),
                body: "Here's the latest spec for the new permissions model.",
                completedAt: nil,
                order: nil,
                eventID: "$text:!project:matrix.org",
                eventTimestamp: base,
                senderID: "@alice:matrix.org"
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
                senderID: "@agent:matrix.example.com",
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
        curl -fsS http://browser-control.example.com:9377/openapi.json -o /tmp/browser_openapi.json

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
