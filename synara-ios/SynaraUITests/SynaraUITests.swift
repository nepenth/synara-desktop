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

    func testRoomListOpensTimeline() {
        let app = launchApp()

        login(app: app)

        XCTAssertTrue(app.buttons["RoomRow-!project:matrix.org"].waitForExistence(timeout: 5))
        app.buttons["RoomRow-!project:matrix.org"].tap()

        XCTAssertTrue(app.scrollViews["TimelineList"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Hello from iOS"].waitForExistence(timeout: 5))
    }

    func testComposerSendsMockMessage() {
        let app = launchApp()

        login(app: app)

        XCTAssertTrue(app.buttons["RoomRow-!project:matrix.org"].waitForExistence(timeout: 5))
        app.buttons["RoomRow-!project:matrix.org"].tap()
        XCTAssertTrue(app.textFields["ComposerTextField"].waitForExistence(timeout: 5))
        app.textFields["ComposerTextField"].tap()
        app.textFields["ComposerTextField"].typeText("hello from ui")
        app.buttons["ComposerSendButton"].tap()

        XCTAssertTrue(app.staticTexts["hello from ui"].waitForExistence(timeout: 5))
    }

    func testMediaUploadAddsAttachmentPlaceholder() {
        let app = launchApp()

        login(app: app)

        XCTAssertTrue(app.buttons["RoomRow-!project:matrix.org"].waitForExistence(timeout: 5))
        app.buttons["RoomRow-!project:matrix.org"].tap()
        XCTAssertTrue(app.buttons["AttachmentButton"].waitForExistence(timeout: 5))
        app.buttons["AttachmentButton"].tap()

        XCTAssertTrue(app.buttons["MediaPlaceholder-synara-upload.jpg"].waitForExistence(timeout: 5))
    }

    func testLogoutReturnsToSignedOutShell() {
        let app = launchApp()

        login(app: app)

        XCTAssertTrue(app.tabBars.buttons["Settings"].waitForExistence(timeout: 5))
        app.tabBars.buttons["Settings"].tap()
        XCTAssertTrue(app.buttons["LogoutButton"].waitForExistence(timeout: 5))
        app.buttons["LogoutButton"].tap()

        XCTAssertTrue(app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5))
    }

    private func launchApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
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
}
