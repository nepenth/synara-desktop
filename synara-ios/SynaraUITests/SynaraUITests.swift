import XCTest

final class SynaraUITests: XCTestCase {
    func testShellShowsHomeserverSelectionWhenSignedOut() {
        let app = XCUIApplication()
        app.launch()

        XCTAssertTrue(app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["HomeserverContinueButton"].exists)
    }

    func testInvalidHomeserverShowsErrorBeforeNavigation() {
        let app = XCUIApplication()
        app.launch()

        let addressField = app.textFields["HomeserverAddressField"]
        XCTAssertTrue(addressField.waitForExistence(timeout: 5))
        addressField.tap()
        addressField.typeText("http://example.org")
        app.buttons["HomeserverContinueButton"].tap()

        XCTAssertTrue(app.staticTexts["HomeserverErrorText"].waitForExistence(timeout: 5))
    }

    func testValidHomeserverNavigatesToLoginPlaceholder() {
        let app = XCUIApplication()
        app.launch()

        let addressField = app.textFields["HomeserverAddressField"]
        XCTAssertTrue(addressField.waitForExistence(timeout: 5))
        addressField.tap()
        addressField.typeText("matrix.org")
        app.buttons["HomeserverContinueButton"].tap()

        XCTAssertTrue(app.otherElements["LoginScreen"].waitForExistence(timeout: 5))
    }

    func testLoginValidationShowsNonSensitiveError() {
        let app = XCUIApplication()
        app.launch()

        let addressField = app.textFields["HomeserverAddressField"]
        XCTAssertTrue(addressField.waitForExistence(timeout: 5))
        addressField.tap()
        addressField.typeText("matrix.org")
        app.buttons["HomeserverContinueButton"].tap()

        XCTAssertTrue(app.otherElements["LoginScreen"].waitForExistence(timeout: 5))
        app.buttons["LoginSubmitButton"].tap()

        XCTAssertTrue(app.staticTexts["LoginErrorText"].waitForExistence(timeout: 5))
    }

    func testSuccessfulMockLoginShowsSignedInShell() {
        let app = XCUIApplication()
        app.launch()

        login(app: app)

        XCTAssertTrue(app.tabBars.buttons["Rooms"].waitForExistence(timeout: 5))
    }

    func testRoomListOpensTimeline() {
        let app = XCUIApplication()
        app.launch()

        login(app: app)

        XCTAssertTrue(app.buttons["RoomRow-!project:matrix.org"].waitForExistence(timeout: 5))
        app.buttons["RoomRow-!project:matrix.org"].tap()

        XCTAssertTrue(app.otherElements["RoomTimelineScreen-!project:matrix.org"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.otherElements["TimelineItem-$text:!project:matrix.org"].waitForExistence(timeout: 5))
    }

    func testComposerSendsMockMessage() {
        let app = XCUIApplication()
        app.launch()

        login(app: app)

        XCTAssertTrue(app.buttons["RoomRow-!project:matrix.org"].waitForExistence(timeout: 5))
        app.buttons["RoomRow-!project:matrix.org"].tap()
        XCTAssertTrue(app.textViews["ComposerTextEditor"].waitForExistence(timeout: 5))
        app.textViews["ComposerTextEditor"].tap()
        app.textViews["ComposerTextEditor"].typeText("hello from ui")
        app.buttons["ComposerSendButton"].tap()

        XCTAssertTrue(app.staticTexts["hello from ui"].waitForExistence(timeout: 5))
    }

    func testMediaUploadAddsAttachmentPlaceholder() {
        let app = XCUIApplication()
        app.launch()

        login(app: app)

        XCTAssertTrue(app.buttons["RoomRow-!project:matrix.org"].waitForExistence(timeout: 5))
        app.buttons["RoomRow-!project:matrix.org"].tap()
        XCTAssertTrue(app.buttons["AttachmentButton"].waitForExistence(timeout: 5))
        app.buttons["AttachmentButton"].tap()

        XCTAssertTrue(app.buttons["MediaPlaceholder-synara-upload.jpg"].waitForExistence(timeout: 5))
    }

    func testLogoutReturnsToSignedOutShell() {
        let app = XCUIApplication()
        app.launch()

        login(app: app)

        XCTAssertTrue(app.tabBars.buttons["Settings"].waitForExistence(timeout: 5))
        app.tabBars.buttons["Settings"].tap()
        XCTAssertTrue(app.buttons["LogoutButton"].waitForExistence(timeout: 5))
        app.buttons["LogoutButton"].tap()

        XCTAssertTrue(app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5))
    }

    private func login(app: XCUIApplication) {
        let addressField = app.textFields["HomeserverAddressField"]
        XCTAssertTrue(addressField.waitForExistence(timeout: 5))
        addressField.tap()
        addressField.typeText("matrix.org")
        app.buttons["HomeserverContinueButton"].tap()

        XCTAssertTrue(app.otherElements["LoginScreen"].waitForExistence(timeout: 5))
        app.textFields["LoginUsernameField"].tap()
        app.textFields["LoginUsernameField"].typeText("alice")
        app.secureTextFields["LoginPasswordField"].tap()
        app.secureTextFields["LoginPasswordField"].typeText("password")
        app.buttons["LoginSubmitButton"].tap()
    }
}
