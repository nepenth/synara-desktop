import XCTest
@testable import Synara

final class AppRouteTests: XCTestCase {
    func testTabsExposeExpectedDestinations() {
        XCTAssertEqual(AppTab.allCases.map(\.rawValue), ["rooms", "notifications", "later", "settings"])
    }

    func testSheetDestinationHasStableIdentifier() {
        XCTAssertEqual(SheetDestination.accountSwitcher.id, "accountSwitcher")
    }

    func testDeepLinkRoutesSettings() throws {
        let router = AppRouter()
        let url = try XCTUnwrap(URL(string: "synara://settings"))
        let opened = router.open(url: url)

        XCTAssertTrue(opened)
        XCTAssertEqual(router.selectedTab, .settings)
        XCTAssertEqual(router.settingsPath, [.settings])
    }

    func testDeepLinkRoutesRoom() throws {
        let router = AppRouter()
        let url = try XCTUnwrap(URL(string: "synara://room/!roomid:example.org"))
        let opened = router.open(url: url)

        XCTAssertTrue(opened)
        XCTAssertEqual(router.selectedTab, .rooms)
        XCTAssertEqual(router.roomsPath, [.room(id: "!roomid:example.org", title: nil)])
    }

    func testDeepLinkRoutesRoomWithEventAnchor() throws {
        let router = AppRouter()
        let url = try XCTUnwrap(URL(string: "synara://route/%2Froom%2F!roomid%3Aexample.org%2F%24evt123"))
        let opened = router.open(url: url)

        XCTAssertTrue(opened)
        XCTAssertEqual(router.selectedTab, .rooms)
        XCTAssertEqual(router.roomsPath, [.room(id: "!roomid:example.org", eventID: "$evt123", title: nil)])
    }

    func testDeepLinkRoutesNotificationsFallbackSurface() throws {
        let router = AppRouter()
        let url = try XCTUnwrap(URL(string: "synara://inbox/later"))
        let opened = router.open(url: url)

        XCTAssertTrue(opened)
        XCTAssertEqual(router.selectedTab, .later)
        XCTAssertEqual(router.laterPath, [.later])
    }

    func testDeepLinkRoutesNotificationsSurface() throws {
        let router = AppRouter()
        let url = try XCTUnwrap(URL(string: "synara://notifications"))
        let opened = router.open(url: url)

        XCTAssertTrue(opened)
        XCTAssertEqual(router.selectedTab, .notifications)
        XCTAssertEqual(router.notificationsPath, [.notifications])
    }

    func testRouterCanCarryRoomTitleFromRoomList() {
        let router = AppRouter()

        router.route(to: .room(id: "!roomid:example.org", title: "Alerts"))

        XCTAssertEqual(router.selectedTab, .rooms)
        XCTAssertEqual(router.roomsPath, [.room(id: "!roomid:example.org", title: "Alerts")])
    }

    func testRouterResetClearsNavigationAndSheets() {
        let router = AppRouter()
        router.route(to: .settings)
        router.present(.accountSwitcher)

        router.resetForAccountChange()

        XCTAssertEqual(router.selectedTab, .rooms)
        XCTAssertTrue(router.roomsPath.isEmpty)
        XCTAssertTrue(router.settingsPath.isEmpty)
        XCTAssertNil(router.sheetDestination)
    }
}
