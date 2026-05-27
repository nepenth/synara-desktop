import SwiftUI
import UIKit
import UserNotifications

@main
struct SynaraApp: App {
    @UIApplicationDelegateAdaptor(SynaraAppDelegate.self) private var appDelegate

    private let environment: AppEnvironment = {
        if ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1" {
            return .uiTest()
        }
        return .live()
    }()

    var body: some Scene {
        WindowGroup {
            RootShellView(environment: environment)
                .onAppear {
                    appDelegate.bind(to: environment)
                }
        }
    }
}

final class SynaraAppDelegate: NSObject, UIApplicationDelegate, UNUserNotificationCenterDelegate {
    private var push: PushServicing?
    private var router: AppRouter?
    private var logger: LoggingServicing?
    private var pendingRoute: AppRoute?

    func bind(to environment: AppEnvironment) {
        push = environment.push
        router = environment.router
        logger = environment.logger
        UNUserNotificationCenter.current().delegate = self

        if let pendingRoute {
            routeToDestination(pendingRoute)
            self.pendingRoute = nil
        }
    }

    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        if let remotePayload = launchOptions?[.remoteNotification] as? [AnyHashable: Any],
           let push {
            if let route = push.route(from: remotePayload) {
                routeToDestination(route)
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

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse
    ) async {
        guard let push else {
            return
        }

        if let route = push.route(from: response.notification.request.content.userInfo) {
            routeToDestination(route)
            push.applyIncomingBadge(from: response.notification.request.content.userInfo)
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
        }

        if let inviteTransitionService {
            return .mock(
                router: router,
                session: session,
                roomList: inviteTransitionService,
                roomMembership: inviteTransitionService
            )
        }

        return .mock(router: router, session: session)
    }
}
