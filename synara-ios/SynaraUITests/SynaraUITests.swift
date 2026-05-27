import XCTest

final class SynaraUITests: XCTestCase {
    func testShellShowsHomeserverSelectionWhenSignedOut() {
        let app = launchApp()

        XCTAssertTrue(app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["HomeserverContinueButton"].exists)
    }

    func testInvalidHomeserverShowsErrorBeforeNavigation() {
        let app = launchApp()

        let addressField = app.textFields["HomeserverAddressField"]
        XCTAssertTrue(addressField.waitForExistence(timeout: 5))
        addressField.tap()
        addressField.typeText("http://example.org")
        app.buttons["HomeserverContinueButton"].tap()

        XCTAssertTrue(app.staticTexts["HomeserverErrorText"].waitForExistence(timeout: 5))
    }

    func testValidHomeserverNavigatesToLoginPlaceholder() {
        let app = launchApp()

        let addressField = app.textFields["HomeserverAddressField"]
        XCTAssertTrue(addressField.waitForExistence(timeout: 5))
        addressField.tap()
        addressField.typeText("matrix.org")
        app.buttons["HomeserverContinueButton"].tap()

        waitForLogin(app: app)
    }

    func testLoginValidationShowsNonSensitiveError() {
        let app = launchApp()

        let addressField = app.textFields["HomeserverAddressField"]
        XCTAssertTrue(addressField.waitForExistence(timeout: 5))
        addressField.tap()
        addressField.typeText("matrix.org")
        app.buttons["HomeserverContinueButton"].tap()

        waitForLogin(app: app)
        app.buttons["LoginSubmitButton"].tap()

        XCTAssertTrue(app.staticTexts["LoginErrorText"].waitForExistence(timeout: 5))
    }

    func testSuccessfulMockLoginShowsSignedInShell() {
        let app = launchApp()

        login(app: app)

        XCTAssertTrue(app.tabBars.buttons["Rooms"].waitForExistence(timeout: 5))
    }

    func testRoomListShowsStableRoomRows() {
        let app = launchApp()

        login(app: app)

        XCTAssertTrue(app.buttons["RoomRow-!project:matrix.org"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["RoomRow-!alice:matrix.org"].exists)
    }

    func testRoomRouteShowsTimeline() {
        let app = launchRoomApp()

        XCTAssertTrue(app.navigationBars["Project"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.scrollViews["TimelineList"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Hello from iOS"].waitForExistence(timeout: 5))
    }

    func testComposerSendsMockMessage() {
        let app = launchRoomApp()

        XCTAssertTrue(app.textFields["ComposerTextField"].waitForExistence(timeout: 5))
        app.textFields["ComposerTextField"].tap()
        app.textFields["ComposerTextField"].typeText("hello from ui")
        tap(app.buttons["ComposerSendButton"])

        XCTAssertTrue(app.staticTexts["hello from ui"].waitForExistence(timeout: 5))
    }

    func testMediaUploadAddsAttachmentPlaceholder() {
        let app = launchRoomApp()

        tap(app.buttons["AttachmentButton"])

        XCTAssertTrue(app.buttons["MediaPlaceholder-synara-upload.jpg"].waitForExistence(timeout: 5))
    }

    func testLogoutReturnsToSignedOutShell() {
        let app = launchSignedInSettingsApp()

        XCTAssertTrue(app.buttons["LogoutButton"].waitForExistence(timeout: 5))
        tap(app.buttons["LogoutButton"])

        XCTAssertTrue(app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5))
    }

    private func launchApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launch()
        return app
    }

    private func launchRoomApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_ID"] = "!project:matrix.org"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_TITLE"] = "Project"
        app.launch()
        return app
    }

    private func launchSignedInSettingsApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SIGNED_IN"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SELECTED_TAB"] = "settings"
        app.launch()
        return app
    }

    private func login(app: XCUIApplication) {
        let addressField = app.textFields["HomeserverAddressField"]
        XCTAssertTrue(addressField.waitForExistence(timeout: 5))
        addressField.tap()
        addressField.typeText("matrix.org")
        app.buttons["HomeserverContinueButton"].tap()

        waitForLogin(app: app)
        app.textFields["LoginUsernameField"].tap()
        app.textFields["LoginUsernameField"].typeText("alice")
        app.secureTextFields["LoginPasswordField"].tap()
        app.secureTextFields["LoginPasswordField"].typeText("password")
        app.buttons["LoginSubmitButton"].tap()
    }

    private func waitForLogin(app: XCUIApplication) {
        XCTAssertTrue(app.textFields["LoginUsernameField"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.secureTextFields["LoginPasswordField"].exists)
        XCTAssertTrue(app.buttons["LoginSubmitButton"].exists)
    }

    private func tap(_ element: XCUIElement, timeout: TimeInterval = 5) {
        XCTAssertTrue(element.waitForExistence(timeout: timeout))
        if element.isHittable {
            element.tap()
        } else {
            element.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
        }
    }
}
