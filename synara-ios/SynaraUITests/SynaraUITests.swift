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
        XCTAssertTrue(app.buttons["LoadOlderTimelineButton"].exists)
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

    func testAcceptInviteTransitionsRowToJoinedRoom() {
        let app = launchInviteApp()

        XCTAssertTrue(app.buttons["AcceptInvite-!alerts:matrix.org"].waitForExistence(timeout: 5))
        tap(app.buttons["AcceptInvite-!alerts:matrix.org"])

        XCTAssertTrue(app.buttons["RoomRow-!alerts:matrix.org"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["AcceptInvite-!alerts:matrix.org"].exists)
    }

    func testRejectInviteRemovesInviteRow() {
        let app = launchInviteApp()

        XCTAssertTrue(app.buttons["RejectInvite-!alerts:matrix.org"].waitForExistence(timeout: 5))
        tap(app.buttons["RejectInvite-!alerts:matrix.org"])

        XCTAssertTrue(app.staticTexts["No Rooms"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["RejectInvite-!alerts:matrix.org"].exists)
    }

    func testLiveSmokeWhenConfigured() throws {
        let environment = ProcessInfo.processInfo.environment
        guard liveEnvironmentValue("SYNARA_LIVE_SMOKE", in: environment) == "1" else {
            throw XCTSkip("Set SYNARA_LIVE_SMOKE=1 for local live simulator smoke.")
        }

        let app = XCUIApplication()
        app.launch()

        if app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5) {
            guard let homeserver = liveEnvironmentValue("SYNARA_LIVE_HOMESERVER", in: environment),
                  let username = liveEnvironmentValue("SYNARA_LIVE_USERNAME", in: environment),
                  let password = liveEnvironmentValue("SYNARA_LIVE_PASSWORD", in: environment) else {
                throw XCTSkip("Live smoke needs an existing session or live credentials in environment variables.")
            }
            loginLive(app: app, homeserver: homeserver, username: username, password: password)
        }

        XCTAssertTrue(app.tabBars.buttons["Rooms"].waitForExistence(timeout: 10))

        let roomName = liveEnvironmentValue("SYNARA_LIVE_ROOM_NAME", in: environment) ?? "Alerts"
        let room: XCUIElement
        if let roomID = liveEnvironmentValue("SYNARA_LIVE_ROOM_ID", in: environment) {
            room = app.buttons["RoomRow-\(roomID)"]
        } else {
            room = app.buttons.containing(NSPredicate(format: "label BEGINSWITH %@", roomName)).firstMatch
        }
        XCTAssertTrue(room.waitForExistence(timeout: 20))
        tap(room, timeout: 20)

        if liveEnvironmentValue("SYNARA_LIVE_ROOM_ID", in: environment) == nil {
            XCTAssertTrue(app.navigationBars[roomName].waitForExistence(timeout: 10))
        }
        XCTAssertTrue(app.textFields["ComposerTextField"].waitForExistence(timeout: 10))

        let message = "Synara live smoke \(Int(Date().timeIntervalSince1970))"
        app.textFields["ComposerTextField"].tap()
        app.textFields["ComposerTextField"].typeText(message)
        tap(app.buttons["ComposerSendButton"], timeout: 10)

        XCTAssertTrue(app.staticTexts[message].waitForExistence(timeout: 20))
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

    private func launchInviteApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SIGNED_IN"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_INVITE"] = "1"
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

    private func loginLive(app: XCUIApplication, homeserver: String, username: String, password: String) {
        let addressField = app.textFields["HomeserverAddressField"]
        XCTAssertTrue(addressField.waitForExistence(timeout: 10))
        addressField.tap()
        addressField.typeText(homeserver)
        app.buttons["HomeserverContinueButton"].tap()

        waitForLogin(app: app)
        app.textFields["LoginUsernameField"].tap()
        app.textFields["LoginUsernameField"].typeText(username)
        app.secureTextFields["LoginPasswordField"].tap()
        app.secureTextFields["LoginPasswordField"].typeText(password)
        app.buttons["LoginSubmitButton"].tap()
    }

    private func waitForLogin(app: XCUIApplication) {
        XCTAssertTrue(app.textFields["LoginUsernameField"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.secureTextFields["LoginPasswordField"].exists)
        XCTAssertTrue(app.buttons["LoginSubmitButton"].exists)
    }

    private func liveEnvironmentValue(_ key: String, in environment: [String: String]) -> String? {
        environment[key] ?? environment["TEST_RUNNER_\(key)"]
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
