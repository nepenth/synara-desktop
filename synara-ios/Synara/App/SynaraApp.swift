import SwiftUI

@main
struct SynaraApp: App {
    private let environment: AppEnvironment = {
        if ProcessInfo.processInfo.environment["SYNARA_UI_TESTS"] == "1" {
            return .uiTest()
        }
        return .live()
    }()

    var body: some Scene {
        WindowGroup {
            RootShellView(environment: environment)
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
