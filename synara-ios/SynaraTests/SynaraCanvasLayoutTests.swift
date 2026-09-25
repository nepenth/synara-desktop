import SwiftUI
import XCTest
@testable import Synara

final class SynaraCanvasLayoutTests: XCTestCase {
    func testCompactWidthUsesStackedCanvas() {
        XCTAssertEqual(SynaraCanvasLayoutPolicy.layout(horizontalSizeClass: .compact), .stacked)
    }

    func testRegularWidthUsesSplitCanvas() {
        XCTAssertEqual(SynaraCanvasLayoutPolicy.layout(horizontalSizeClass: .regular), .split)
    }

    func testMissingSizeClassDefaultsToStackedPhoneCanvas() {
        XCTAssertEqual(SynaraCanvasLayoutPolicy.layout(horizontalSizeClass: nil), .stacked)
    }

    func testStackedConversationKeepsBackButtonAndHidesTabBar() {
        XCTAssertTrue(SynaraCanvasLayout.stacked.showsConversationBackButton)
        XCTAssertTrue(SynaraCanvasLayout.stacked.hidesTabBarInConversation)
    }

    func testSplitConversationKeepsTabsAndHidesBackToList() {
        XCTAssertFalse(SynaraCanvasLayout.split.showsConversationBackButton)
        XCTAssertFalse(SynaraCanvasLayout.split.hidesTabBarInConversation)
    }

    func testConversationTabsUseSplitCanvasAndSettingsDoesNot() {
        XCTAssertTrue(AppTab.rooms.usesConversationCanvas)
        XCTAssertTrue(AppTab.notifications.usesConversationCanvas)
        XCTAssertTrue(AppTab.later.usesConversationCanvas)
        XCTAssertFalse(AppTab.settings.usesConversationCanvas)
    }

    func testSplitRootAndTailKeepRoomThenThreadWithoutRewritingIdentity() {
        let path: [AppRoute] = [
            .room(id: "!room:example.org", title: "Alerts"),
            .thread(
                roomID: "!room:example.org",
                rootEventID: "$root",
                roomTitle: "Alerts",
                rootTitle: "Root"
            )
        ]

        XCTAssertEqual(SynaraConversationPath.splitRoot(in: path), path[0])
        XCTAssertEqual(SynaraConversationPath.splitTail(in: path), [path[1]])
        XCTAssertEqual(SynaraConversationPath.selectedConversationID(in: path), "!room:example.org")
        XCTAssertEqual(
            SynaraConversationPath.applyingSplitTail([], to: path),
            [.room(id: "!room:example.org", title: "Alerts")]
        )
    }

    func testEmptyPathHasNoSplitDetailAndSamePathWorksAfterFold() {
        XCTAssertNil(SynaraConversationPath.splitRoot(in: []))
        XCTAssertEqual(SynaraConversationPath.splitTail(in: []), [])
        XCTAssertNil(SynaraConversationPath.selectedConversationID(in: []))

        let openRoom: [AppRoute] = [.room(id: "!room:example.org", title: "Alerts")]
        XCTAssertEqual(SynaraConversationPath.splitRoot(in: openRoom), openRoom[0])
        XCTAssertEqual(SynaraConversationPath.selectedConversationID(in: openRoom), "!room:example.org")
    }

    func testLaterSurfaceRouteIsNotAConversationColumn() {
        let path: [AppRoute] = [.later]
        XCTAssertNil(SynaraConversationPath.splitRoot(in: path))
        XCTAssertNil(SynaraConversationPath.selectedConversationID(in: path))
        XCTAssertFalse(AppRoute.later.isConversation)
        XCTAssertTrue(AppRoute.room(id: "!r").isConversation)
        XCTAssertTrue(AppRoute.thread(roomID: "!r", rootEventID: "$e").isConversation)
    }

    func testRouterStillStoresRoomOnTheTabPathUsedByBothCanvases() {
        let router = AppRouter()
        router.route(to: .room(id: "!room:example.org", title: "Alerts"))

        XCTAssertEqual(router.selectedTab, .rooms)
        XCTAssertEqual(
            SynaraConversationPath.selectedConversationID(in: router.roomsPath),
            "!room:example.org"
        )
        XCTAssertEqual(SynaraConversationPath.splitRoot(in: router.roomsPath), router.roomsPath[0])
    }
}
