import XCTest
@testable import Synara

final class AppRouteTests: XCTestCase {
    func testTabsExposeExpectedDestinations() {
        XCTAssertEqual(AppTab.allCases.map(\.rawValue), ["rooms", "notifications", "later", "settings"])
    }

    func testSheetDestinationHasStableIdentifier() {
        XCTAssertEqual(SheetDestination.accountSwitcher.id, "accountSwitcher")
    }
}
