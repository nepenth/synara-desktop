import XCTest

final class SynaraUITests: XCTestCase {
    func testShellShowsPrimaryTabs() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.tabBars.buttons["Rooms"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.tabBars.buttons["Notifications"].exists)
        XCTAssertTrue(app.tabBars.buttons["Later"].exists)
        XCTAssertTrue(app.tabBars.buttons["Settings"].exists)
    }
}
