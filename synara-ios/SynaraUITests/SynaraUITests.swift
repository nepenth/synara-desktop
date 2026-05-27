import XCTest
import Foundation

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

    func testLaterListRendersStatesAndUnavailableDestinations() {
        let app = launchLaterApp()

        XCTAssertTrue(app.collectionViews["LaterList"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["LaterRow-$hello"].exists)
        XCTAssertTrue(app.buttons["LaterRow-$done"].exists)
        XCTAssertTrue(app.buttons["LaterRow-reminder-missing-destination"].exists)
        XCTAssertTrue(app.staticTexts["Completed"].exists)
        XCTAssertTrue(app.staticTexts["Destination unavailable"].exists)
    }

    func testLaterItemNavigatesToRoomAnchor() {
        let app = launchLaterApp()

        let row = app.buttons["LaterRow-$hello"]
        XCTAssertTrue(row.waitForExistence(timeout: 5))
        tap(row)

        XCTAssertTrue(app.navigationBars["!project:matrix.org"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.scrollViews["TimelineList"].waitForExistence(timeout: 5))
    }

    func testAgentCardApproveActionShowsSubmittedState() {
        let app = launchAgentCardRoomApp()

        XCTAssertTrue(app.staticTexts["Deployment Approval"].waitForExistence(timeout: 5))
        tap(app.buttons["AgentCardAction-approve-deploy"])

        let alert = app.alerts["Agent Action"]
        XCTAssertTrue(alert.waitForExistence(timeout: 5))
        XCTAssertTrue(alert.staticTexts["Agent action approved"].exists)
    }

    func testAgentCardApprovalFailureIsVisibleAndRetryable() {
        let app = launchAgentCardRoomApp(approvalError: "failed")

        XCTAssertTrue(app.staticTexts["Deployment Approval"].waitForExistence(timeout: 5))
        tap(app.buttons["AgentCardAction-reject-deploy"])

        let alert = app.alerts["Agent Action"]
        XCTAssertTrue(alert.waitForExistence(timeout: 5))
        XCTAssertTrue(alert.staticTexts["Agent action could not be submitted. Try again."].exists)
    }

    func testLiveSmokeWhenConfigured() throws {
        let environment = ProcessInfo.processInfo.environment
        guard liveEnvironmentValue("SYNARA_LIVE_SMOKE", in: environment) == "1" else {
            throw XCTSkip("Set SYNARA_LIVE_SMOKE=1 for local live simulator smoke.")
        }

        let roomName = liveEnvironmentValue("SYNARA_LIVE_ROOM_NAME", in: environment) ?? "Alerts"
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_RESET_SESSION_ON_LAUNCH"] = "1"
        if let roomID = liveEnvironmentValue("SYNARA_LIVE_ROOM_ID", in: environment) {
            app.launchEnvironment["SYNARA_AUTO_OPEN_ROOM_ID"] = roomID
        } else {
            app.launchEnvironment["SYNARA_AUTO_OPEN_ROOM_NAME"] = roomName
        }
        app.launch()

        if app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5) {
            guard let homeserver = liveEnvironmentValue("SYNARA_LIVE_HOMESERVER", in: environment),
                  let username = liveEnvironmentValue("SYNARA_LIVE_USERNAME", in: environment),
                  let password = liveEnvironmentValue("SYNARA_LIVE_PASSWORD", in: environment) else {
                throw XCTSkip("Live smoke needs an existing session or live credentials in environment variables.")
            }
            loginLive(app: app, homeserver: homeserver, username: username, password: password)
            dismissPasswordSavePromptIfPresent(app: app)
        }

        XCTAssertTrue(app.tabBars.buttons["Rooms"].waitForExistence(timeout: 60))

        let composer = app.textFields["ComposerTextField"]
        guard composer.waitForExistence(timeout: 30) else {
            XCTFail("Expected encrypted room timeline composer to appear.")
            return
        }

        let message = "Synara live smoke \(Int(Date().timeIntervalSince1970))"
        composer.tap()
        composer.typeText(message)
        tap(app.buttons["ComposerSendButton"], timeout: 10)

        XCTAssertTrue(app.staticTexts[message].waitForExistence(timeout: 20))
    }

    func testLiveAgentApprovalSmokeWhenConfigured() throws {
        let environment = ProcessInfo.processInfo.environment
        guard liveEnvironmentValue("SYNARA_LIVE_AGENT_SMOKE", in: environment) == "1" else {
            throw XCTSkip("Set SYNARA_LIVE_AGENT_SMOKE=1 for local live agent approval smoke.")
        }

        guard let homeserver = liveEnvironmentValue("SYNARA_LIVE_HOMESERVER", in: environment),
              let username = liveEnvironmentValue("SYNARA_LIVE_USERNAME", in: environment),
              let password = liveEnvironmentValue("SYNARA_LIVE_PASSWORD", in: environment) else {
            throw XCTSkip("Live agent smoke needs homeserver, username, and password environment variables.")
        }

        let liveClient = try MatrixLiveTestClient.login(
            homeserver: homeserver,
            username: username,
            password: password
        )
        let roomID = try liveAgentRoomID(environment: environment, client: liveClient)
        let smokeID = Int(Date().timeIntervalSince1970)
        let title = "Synara approval smoke \(smokeID)"
        let seededEventID = try liveClient.seedAgentApprovalCard(
            roomID: roomID,
            title: title,
            actionID: "live-approve-\(smokeID)"
        )

        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_RESET_SESSION_ON_LAUNCH"] = "1"
        app.launchEnvironment["SYNARA_AUTO_OPEN_ROOM_ID"] = roomID
        app.launch()

        if app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5) {
            loginLive(app: app, homeserver: homeserver, username: username, password: password)
            dismissPasswordSavePromptIfPresent(app: app)
        }

        XCTAssertTrue(app.tabBars.buttons["Rooms"].waitForExistence(timeout: 60))
        XCTAssertTrue(waitForTimelineElement(app.staticTexts[title], app: app, timeout: 30))
        XCTAssertTrue(waitForTimelineElement(app.buttons["AgentCardAction-live-approve-\(smokeID)"], app: app, timeout: 10))
        tap(app.buttons["AgentCardAction-live-approve-\(smokeID)"], timeout: 1)

        let alert = app.alerts["Agent Action"]
        XCTAssertTrue(alert.waitForExistence(timeout: 15))
        XCTAssertTrue(alert.staticTexts["Agent action approved"].exists)

        XCTAssertTrue(
            liveClient.waitForApprovalEvent(
                roomID: roomID,
                sourceEventID: seededEventID,
                actionID: "live-approve-\(smokeID)",
                decision: "approve",
                timeout: 20
            )
        )
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

    private func launchLaterApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SIGNED_IN"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SELECTED_TAB"] = "later"
        app.launchEnvironment["SYNARA_UI_TEST_LATER_ITEMS"] = "1"
        app.launch()
        return app
    }

    private func launchAgentCardRoomApp(approvalError: String? = nil) -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_ID"] = "!agent:matrix.org"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_TITLE"] = "Agent"
        app.launchEnvironment["SYNARA_UI_TEST_AGENT_CARD"] = "1"
        if let approvalError {
            app.launchEnvironment["SYNARA_UI_TEST_AGENT_APPROVAL_ERROR"] = approvalError
        }
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

    private func liveAgentRoomID(environment: [String: String], client: MatrixLiveTestClient) throws -> String {
        if let roomID = liveEnvironmentValue("SYNARA_LIVE_AGENT_ROOM_ID", in: environment)
            ?? liveEnvironmentValue("SYNARA_LIVE_ROOM_ID", in: environment) {
            return roomID
        }

        let alias = liveEnvironmentValue("SYNARA_LIVE_AGENT_ROOM_ALIAS", in: environment)
            ?? liveEnvironmentValue("SYNARA_LIVE_ROOM_ALIAS", in: environment)
            ?? "#test-e2e-room:whyland.com"
        return try client.resolveRoomAlias(alias)
    }

    private func tap(_ element: XCUIElement, timeout: TimeInterval = 5) {
        XCTAssertTrue(element.waitForExistence(timeout: timeout))
        if element.isHittable {
            element.tap()
        } else {
            element.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
        }
    }

    private func dismissPasswordSavePromptIfPresent(app: XCUIApplication) {
        let notNow = app.buttons["Not Now"]
        if notNow.waitForExistence(timeout: 3) {
            if notNow.isHittable {
                notNow.tap()
            } else {
                notNow.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
            }
        }
    }

    private func waitForTimelineElement(_ element: XCUIElement, app: XCUIApplication, timeout: TimeInterval) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        let timeline = app.scrollViews["TimelineList"]

        while Date() < deadline {
            if element.exists {
                return true
            }
            if timeline.exists {
                timeline.swipeUp()
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.5))
        }

        return element.exists
    }

}

private final class MatrixLiveTestClient {
    private let homeserverURL: URL
    private let accessToken: String

    private init(homeserverURL: URL, accessToken: String) {
        self.homeserverURL = homeserverURL
        self.accessToken = accessToken
    }

    static func login(homeserver: String, username: String, password: String) throws -> MatrixLiveTestClient {
        guard let homeserverURL = URL(string: homeserver.hasPrefix("http") ? homeserver : "https://\(homeserver)") else {
            throw LiveMatrixError.invalidHomeserver
        }

        let requestBody: [String: Any] = [
            "type": "m.login.password",
            "identifier": [
                "type": "m.id.user",
                "user": username
            ],
            "password": password,
            "initial_device_display_name": "Synara iOS UI smoke"
        ]

        var request = URLRequest(url: homeserverURL.appendingMatrixPath(["client", "v3", "login"]))
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: requestBody)

        let data = try perform(request).data
        guard let object = try JSONSerialization.jsonObject(with: data) as? [String: Any],
              let token = object["access_token"] as? String else {
            throw LiveMatrixError.invalidResponse
        }

        return MatrixLiveTestClient(homeserverURL: homeserverURL, accessToken: token)
    }

    func resolveRoomAlias(_ alias: String) throws -> String {
        let response = try authenticatedRequest(
            method: "GET",
            path: ["client", "v3", "directory", "room", alias],
            body: nil
        )
        guard let object = try JSONSerialization.jsonObject(with: response.data) as? [String: Any],
              let roomID = object["room_id"] as? String else {
            throw LiveMatrixError.invalidResponse
        }
        return roomID
    }

    func seedAgentApprovalCard(roomID: String, title: String, actionID: String) throws -> String {
        let agentPayload: [String: Any] = [
            "title": title,
            "status": "pending",
            "summary": "Live approval smoke test card.",
            "actions": [
                [
                    "id": actionID,
                    "title": "Approve",
                    "kind": "approve",
                    "prompt": "approve live smoke"
                ]
            ]
        ]
        let bodyData = try JSONSerialization.data(withJSONObject: [
            "hermes": true,
            "payload": agentPayload
        ])
        let body = String(data: bodyData, encoding: .utf8) ?? title

        let content: [String: Any] = [
            "msgtype": "m.notice",
            "body": body,
            "in.synara.agent": agentPayload
        ]

        let response = try authenticatedRequest(
            method: "PUT",
            path: ["client", "v3", "rooms", roomID, "send", "m.room.message", UUID().uuidString],
            body: content
        )
        guard let object = try JSONSerialization.jsonObject(with: response.data) as? [String: Any],
              let eventID = object["event_id"] as? String else {
            throw LiveMatrixError.invalidResponse
        }
        return eventID
    }

    func waitForApprovalEvent(
        roomID: String,
        sourceEventID: String,
        actionID: String,
        decision: String,
        timeout: TimeInterval
    ) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if (try? hasApprovalEvent(
                roomID: roomID,
                sourceEventID: sourceEventID,
                actionID: actionID,
                decision: decision
            )) == true {
                return true
            }
            RunLoop.current.run(until: Date().addingTimeInterval(1))
        }

        return false
    }

    private func hasApprovalEvent(
        roomID: String,
        sourceEventID: String,
        actionID: String,
        decision: String
    ) throws -> Bool {
        let response = try authenticatedRequest(
            method: "GET",
            path: ["client", "v3", "rooms", roomID, "messages"],
            queryItems: [
                URLQueryItem(name: "dir", value: "b"),
                URLQueryItem(name: "limit", value: "40")
            ],
            body: nil
        )
        guard let object = try JSONSerialization.jsonObject(with: response.data) as? [String: Any],
              let chunk = object["chunk"] as? [[String: Any]] else {
            throw LiveMatrixError.invalidResponse
        }

        return chunk.contains { event in
            guard let content = event["content"] as? [String: Any],
                  let action = content["in.synara.agent.action"] as? [String: Any] else {
                return false
            }
            return action["source_event_id"] as? String == sourceEventID
                && action["action_id"] as? String == actionID
                && action["decision"] as? String == decision
        }
    }

    private func authenticatedRequest(
        method: String,
        path: [String],
        queryItems: [URLQueryItem] = [],
        body: [String: Any]?
    ) throws -> (data: Data, statusCode: Int) {
        var components = URLComponents(url: homeserverURL.appendingMatrixPath(path), resolvingAgainstBaseURL: false)
        if queryItems.isEmpty == false {
            components?.queryItems = queryItems
        }

        guard let url = components?.url else {
            throw LiveMatrixError.invalidHomeserver
        }

        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue("Bearer \(accessToken)", forHTTPHeaderField: "Authorization")
        if let body {
            request.setValue("application/json", forHTTPHeaderField: "Content-Type")
            request.httpBody = try JSONSerialization.data(withJSONObject: body)
        }

        return try Self.perform(request)
    }

    private static func perform(_ request: URLRequest) throws -> (data: Data, statusCode: Int) {
        let semaphore = DispatchSemaphore(value: 0)
        var result: Result<(Data, Int), Error>?

        URLSession.shared.dataTask(with: request) { data, response, error in
            defer { semaphore.signal() }
            if let error {
                result = .failure(error)
                return
            }
            guard let http = response as? HTTPURLResponse,
                  let data else {
                result = .failure(LiveMatrixError.invalidResponse)
                return
            }
            guard (200...299).contains(http.statusCode) else {
                result = .failure(LiveMatrixError.httpStatus(http.statusCode))
                return
            }
            result = .success((data, http.statusCode))
        }.resume()

        guard semaphore.wait(timeout: .now() + 30) == .success else {
            throw LiveMatrixError.timeout
        }

        switch result {
        case .success(let value):
            return value
        case .failure(let error):
            throw error
        case nil:
            throw LiveMatrixError.invalidResponse
        }
    }
}

private enum LiveMatrixError: Error {
    case invalidHomeserver
    case invalidResponse
    case httpStatus(Int)
    case timeout
}

private extension URL {
    func appendingMatrixPath(_ components: [String]) -> URL {
        var url = self
        url.appendPathComponent("_matrix")
        for component in components {
            url.appendPathComponent(component)
        }
        return url
    }
}
