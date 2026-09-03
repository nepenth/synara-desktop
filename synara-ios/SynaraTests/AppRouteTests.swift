import XCTest
@testable import Synara

final class AppRouteTests: XCTestCase {
    func testTabsExposeExpectedDestinations() {
        XCTAssertEqual(AppTab.allCases.map(\.rawValue), ["rooms", "later", "notifications", "settings"])
    }

    func testSheetDestinationHasStableIdentifier() {
        XCTAssertEqual(SheetDestination.accountSwitcher.id, "accountSwitcher")
    }

    func testDeepLinkRoutesSettings() throws {
        let router = AppRouter()
        let url = try XCTUnwrap(URL(string: "synara://settings"))
        let opened = router.open(url: url, sessionIsSignedIn: true)

        XCTAssertTrue(opened)
        XCTAssertEqual(router.selectedTab, .settings)
        XCTAssertTrue(router.settingsPath.isEmpty)
    }

    func testDeepLinkRoutesRoom() throws {
        let router = AppRouter()
        let url = try XCTUnwrap(URL(string: "synara://room/!roomid:example.org"))
        let opened = router.open(url: url, sessionIsSignedIn: true)

        XCTAssertTrue(opened)
        XCTAssertEqual(router.selectedTab, .rooms)
        XCTAssertEqual(router.roomsPath, [.room(id: "!roomid:example.org", title: nil)])
    }

    func testDeepLinkRoutesRoomWithEventAnchor() throws {
        let router = AppRouter()
        let url = try XCTUnwrap(URL(string: "synara://route/%2Froom%2F!roomid%3Aexample.org%2F%24evt123"))
        let opened = router.open(url: url, sessionIsSignedIn: true)

        XCTAssertTrue(opened)
        XCTAssertEqual(router.selectedTab, .rooms)
        XCTAssertEqual(router.roomsPath, [.room(id: "!roomid:example.org", eventID: "$evt123", title: nil)])
    }

    func testUniversalLinkRoutesRoomWithEventAnchor() throws {
        let router = AppRouter()
        let url = try XCTUnwrap(URL(string: "https://synara.app/r/%2Froom%2F!roomid%3Aexample.org%2F%24evt123"))
        let opened = router.open(url: url, sessionIsSignedIn: true)

        XCTAssertTrue(opened)
        XCTAssertEqual(router.selectedTab, .rooms)
        XCTAssertEqual(router.roomsPath, [.room(id: "!roomid:example.org", eventID: "$evt123", title: nil)])
    }

    func testDeepLinkRejectsUnsafeSchemesAndHosts() throws {
        let router = AppRouter()
        let originalTab = router.selectedTab

        XCTAssertFalse(router.open(url: try XCTUnwrap(URL(string: "https://settings")), sessionIsSignedIn: true))
        XCTAssertFalse(router.open(url: try XCTUnwrap(URL(string: "https://evil.example/r/%2Fsettings")), sessionIsSignedIn: true))
        XCTAssertFalse(router.open(url: try XCTUnwrap(URL(string: "http://synara.app/r/%2Fsettings")), sessionIsSignedIn: true))
        XCTAssertEqual(router.selectedTab, originalTab)
        XCTAssertTrue(router.settingsPath.isEmpty)
    }

    func testDeepLinkRoutesNotificationsFallbackSurface() throws {
        let router = AppRouter()
        let url = try XCTUnwrap(URL(string: "synara://inbox/later"))
        let opened = router.open(url: url, sessionIsSignedIn: true)

        XCTAssertTrue(opened)
        XCTAssertEqual(router.selectedTab, .later)
        XCTAssertEqual(router.laterPath, [.later])
    }

    func testDeepLinkRoutesNotificationsSurface() throws {
        let router = AppRouter()
        let url = try XCTUnwrap(URL(string: "synara://notifications"))
        let opened = router.open(url: url, sessionIsSignedIn: true)

        XCTAssertTrue(opened)
        XCTAssertEqual(router.selectedTab, .notifications)
        XCTAssertTrue(router.notificationsPath.isEmpty)
    }

    func testRouterCanCarryRoomTitleFromRoomList() {
        let router = AppRouter()

        router.route(to: .room(id: "!roomid:example.org", title: "Alerts"))

        XCTAssertEqual(router.selectedTab, .rooms)
        XCTAssertEqual(router.roomsPath, [.room(id: "!roomid:example.org", title: "Alerts")])
    }

    func testRouterRoutesRoomOnActiveLaterTabStack() {
        let router = AppRouter()
        router.selectedTab = .later

        router.route(to: .room(id: "!roomid:example.org", eventID: "$evt", title: "Product"))

        XCTAssertEqual(router.selectedTab, .later)
        XCTAssertEqual(
            router.laterPath,
            [.room(id: "!roomid:example.org", eventID: "$evt", title: "Product")]
        )
        XCTAssertTrue(router.roomsPath.isEmpty)
    }

    func testRouterRoutesRoomOnActiveNotificationsTabStack() {
        let router = AppRouter()
        router.selectedTab = .notifications

        router.route(to: .room(id: "!roomid:example.org", title: "Alerts"))

        XCTAssertEqual(router.selectedTab, .notifications)
        XCTAssertEqual(router.notificationsPath, [.room(id: "!roomid:example.org", title: "Alerts")])
        XCTAssertTrue(router.roomsPath.isEmpty)
    }

    func testRouterPopsSelectedTabToRoot() {
        let router = AppRouter()
        router.selectedTab = .notifications
        router.route(to: .room(id: "!roomid:example.org", title: "Alerts"))
        router.roomsPath = [.room(id: "!other:example.org", title: "Other")]

        router.popSelectedTabToRoot()

        XCTAssertTrue(router.notificationsPath.isEmpty)
        XCTAssertEqual(router.roomsPath, [.room(id: "!other:example.org", title: "Other")])
    }

    func testRouterCanRouteToThreadSurface() {
        let router = AppRouter()

        router.route(
            to: .thread(
                roomID: "!roomid:example.org",
                rootEventID: "$event:example.org",
                roomTitle: "Alerts",
                rootTitle: "Root message"
            )
        )

        XCTAssertEqual(router.selectedTab, .rooms)
        XCTAssertEqual(
            router.roomsPath,
            [
                .thread(
                    roomID: "!roomid:example.org",
                    rootEventID: "$event:example.org",
                    roomTitle: "Alerts",
                    rootTitle: "Root message"
                )
            ]
        )
    }

    func testRouterAppendsThreadWhenRoomAlreadyInPath() {
        let router = AppRouter()
        router.route(to: .room(id: "!roomid:example.org", title: "Alerts"))
        router.route(
            to: .thread(
                roomID: "!roomid:example.org",
                rootEventID: "$event:example.org",
                roomTitle: "Alerts",
                rootTitle: "Root message"
            )
        )

        XCTAssertEqual(
            router.roomsPath,
            [
                .room(id: "!roomid:example.org", title: "Alerts"),
                .thread(
                    roomID: "!roomid:example.org",
                    rootEventID: "$event:example.org",
                    roomTitle: "Alerts",
                    rootTitle: "Root message"
                )
            ]
        )
    }

    func testDeepLinkDefersRouteWhileSignedOut() throws {
        let router = AppRouter()
        let url = try XCTUnwrap(URL(string: "synara://room/!roomid:example.org"))

        XCTAssertTrue(router.open(url: url, sessionIsSignedIn: false))
        XCTAssertEqual(router.pendingDeepLink, .room(id: "!roomid:example.org", title: nil))
        XCTAssertTrue(router.roomsPath.isEmpty)
        XCTAssertEqual(router.selectedTab, .rooms)
    }

    func testReplayPendingDeepLinkRoutesAfterSignIn() throws {
        let router = AppRouter()
        let url = try XCTUnwrap(URL(string: "synara://room/!roomid:example.org"))

        XCTAssertTrue(router.open(url: url, sessionIsSignedIn: false))
        router.replayPendingDeepLinkIfNeeded(sessionIsSignedIn: true)

        XCTAssertNil(router.pendingDeepLink)
        XCTAssertEqual(router.selectedTab, .rooms)
        XCTAssertEqual(router.roomsPath, [.room(id: "!roomid:example.org", title: nil)])
    }

    @MainActor
    func testRouterResetClearsNavigationAndSheets() {
        let router = AppRouter()
        router.route(to: .settings)
        router.present(.accountSwitcher)

        router.resetNavigationPathsForAccountChange()

        XCTAssertEqual(router.selectedTab, .rooms)
        XCTAssertTrue(router.roomsPath.isEmpty)
        XCTAssertTrue(router.settingsPath.isEmpty)
        XCTAssertNil(router.sheetDestination)
    }
}
