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
        XCTAssertTrue(app.buttons["RoomRow-!general:matrix.org"].exists)
        XCTAssertTrue(app.buttons["RoomRow-!agent-workflows:matrix.org"].exists)
    }

    func testRoomManagementCreatesPrivateEncryptedRoom() {
        let app = launchRoomManagementSheetApp()

        XCTAssertTrue(app.staticTexts["Create Room"].waitForExistence(timeout: 5))
        app.textFields["Name"].tap()
        app.textFields["Name"].typeText("Incident Room")
        app.textFields["Topic"].tap()
        app.textFields["Topic"].typeText("Operational response")
        tap(app.buttons["RoomManagementSubmitButton"])

        XCTAssertTrue(app.staticTexts["Incident Room"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.scrollViews["TimelineList"].waitForExistence(timeout: 5))
    }

    func testRoomSearchFiltersByName() {
        let app = launchFilteredRoomsApp(query: "Alice")

        XCTAssertTrue(app.collectionViews["RoomList"].waitForExistence(timeout: 5))
        let searchField = app.textFields["RoomSearchField"]
        XCTAssertTrue(searchField.waitForExistence(timeout: 5))
        XCTAssertEqual(searchField.value as? String, "Alice")

        XCTAssertTrue(app.buttons["RoomRow-!alice:matrix.org"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["RoomRow-!project:matrix.org"].exists)
    }

    func testSpaceFilterScopesRoomList() {
        let app = launchApp()

        login(app: app)

        XCTAssertTrue(app.scrollViews["SpaceFilterStrip"].waitForExistence(timeout: 5))
        tap(app.buttons["Workspace"])
        XCTAssertTrue(app.buttons["RoomRow-!project:matrix.org"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["All spaces"].exists)
    }

    func testRoomManagementPublicDirectorySearchMockFlow() {
        let app = launchRoomManagementSheetApp()

        tap(app.buttons["Join"])
        let searchField = app.textFields["PublicRoomSearchField"]
        XCTAssertTrue(searchField.waitForExistence(timeout: 5))
        searchField.tap()
        searchField.typeText("alerts")
        tap(app.buttons["PublicRoomSearchButton"])
        XCTAssertTrue(app.buttons["PublicRoomResult-!public-alerts:matrix.org"].waitForExistence(timeout: 5))
    }

    func testRoomRouteShowsTimeline() {
        let app = launchRoomApp()

        XCTAssertTrue(app.staticTexts["Project"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.scrollViews["TimelineList"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["LoadOlderTimelineButton"].exists)
        XCTAssertTrue(app.staticTexts["Here's the latest spec for the new permissions model. Hello from iOS"].waitForExistence(timeout: 5))
    }

    func testRoomDetailsInviteAndLeaveMockFlow() {
        let app = launchRoomApp()

        XCTAssertTrue(app.buttons["RoomDetailsButton"].waitForExistence(timeout: 5))
        tap(app.buttons["RoomDetailsButton"])
        XCTAssertTrue(app.collectionViews["RoomDetailsScreen"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Room ID"].exists)
        XCTAssertTrue(app.staticTexts["Encryption"].exists)
        XCTAssertTrue(app.staticTexts["Members"].exists)
        XCTAssertTrue(revealRoomDetailsElement(app.staticTexts["Your level"], app: app, timeout: 5))
        XCTAssertTrue(revealRoomDetailsElement(app.staticTexts["Change topic"], app: app, timeout: 5))

        let inviteField = app.textFields["RoomInviteUserField"]
        XCTAssertTrue(revealRoomDetailsElement(inviteField, app: app, timeout: 12))
        inviteField.tap()
        inviteField.typeText("@newuser:matrix.org")
        dismissKeyboardIfPresent(app: app)
        XCTAssertTrue(revealRoomDetailsElement(app.buttons["RoomInviteUserButton"], app: app, timeout: 5))
        XCTAssertTrue(waitForEnabled(app.buttons["RoomInviteUserButton"], timeout: 5))

        XCTAssertTrue(revealRoomDetailsElement(app.buttons["LeaveRoomButton"], app: app, timeout: 8))
        tap(app.buttons["LeaveRoomButton"])
        tap(app.buttons["Leave Room"].firstMatch)
        XCTAssertTrue(app.collectionViews["RoomList"].waitForExistence(timeout: 5))
    }

    func testRoomDetailsProfileEditMockFlow() {
        let app = launchRoomApp()

        XCTAssertTrue(app.buttons["RoomDetailsButton"].waitForExistence(timeout: 5))
        tap(app.buttons["RoomDetailsButton"])
        XCTAssertTrue(app.collectionViews["RoomDetailsScreen"].waitForExistence(timeout: 5))

        let nameField = app.textFields["RoomProfileNameField"]
        let topicField = app.textFields["RoomProfileTopicField"]
        XCTAssertTrue(waitForNonEmptyValue(nameField, timeout: 5))
        XCTAssertTrue(waitForNonEmptyValue(topicField, timeout: 5))
        nameField.tap()
        nameField.typeText(" Updated")
        dismissKeyboardIfPresent(app: app)
        let aliasField = app.textFields["RoomCanonicalAliasField"]
        XCTAssertTrue(revealRoomDetailsElement(aliasField, app: app, timeout: 10))
        XCTAssertTrue(app.buttons["RoomProfileSaveButton"].isEnabled)
        tap(app.buttons["Save"])

        let profileMessage = app.staticTexts["RoomDetailsMessage"]
        XCTAssertTrue(revealRoomDetailsElement(profileMessage, app: app, timeout: 10, direction: .down))
        XCTAssertEqual(profileMessage.label, "Profile updated.")
    }

    func testLargeRoomFixtureRendersAndScrolls() {
        let app = launchLargeRoomsApp()

        let roomList = app.collectionViews["RoomList"]
        XCTAssertTrue(roomList.waitForExistence(timeout: 5))
        XCTAssertTrue(app.cells.firstMatch.waitForExistence(timeout: 5))

        roomList.swipeUp()
        XCTAssertTrue(app.cells.firstMatch.exists)
    }

    func testLargeTimelineFixtureRendersAndScrolls() {
        let app = launchLargeTimelineApp()

        let timeline = app.scrollViews["TimelineList"]
        XCTAssertTrue(timeline.waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Synthetic message 0"].waitForExistence(timeout: 5))

        timeline.swipeUp()
        XCTAssertTrue(timeline.exists)
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

    func testEncryptedTimelineShowsCryptoStatusRecoveryBannerAndSafePlaceholder() {
        let app = launchEncryptedRoomApp()

        XCTAssertTrue(app.scrollViews["TimelineList"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Recovery Needed"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Encrypted history needs attention"].waitForExistence(timeout: 5))
        XCTAssertTrue(waitForTimelineElement(app.buttons["Retry Decryption"], app: app, timeout: 5))
        XCTAssertTrue(waitForTimelineElement(app.buttons["Review Security"], app: app, timeout: 5))
        XCTAssertTrue(app.staticTexts["Decrypted encrypted-room message"].exists)
        XCTAssertTrue(app.staticTexts["Encrypted content unavailable. Actions and media downloads are blocked until keys are available."].exists)
    }

    func testLogoutReturnsToSignedOutShell() {
        let app = launchSignedInSettingsApp()

        tapSettingsElement(app.buttons["LogoutButton"], app: app, timeout: 10)
        tap(app.buttons["ConfirmLogoutButton"].firstMatch, timeout: 5)

        XCTAssertTrue(app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5))
    }

    func testSettingsShowsNotificationSectionsAndReleaseLinks() {
        let app = launchSignedInSettingsApp()

        XCTAssertTrue(app.buttons["NotificationPermissionButton"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["PushRegistrationButton"].exists)
        XCTAssertTrue(revealSettingsElement(app.staticTexts["Theme"], app: app, timeout: 10))
        XCTAssertTrue(revealSettingsElement(app.staticTexts["Session Storage"], app: app, timeout: 10))
        XCTAssertTrue(revealSettingsElement(app.staticTexts["Device Verification"], app: app, timeout: 10))
        XCTAssertTrue(revealSettingsElement(app.staticTexts["Key Recovery"], app: app, timeout: 10))
        XCTAssertTrue(revealSettingsElement(app.staticTexts["Key Backup"], app: app, timeout: 10))
        XCTAssertTrue(revealSettingsElement(app.buttons["AboutSettingsLink"], app: app, timeout: 10))
        XCTAssertTrue(revealSettingsElement(app.buttons["LicensesSettingsLink"], app: app, timeout: 10))
        XCTAssertTrue(revealSettingsElement(app.buttons["PrivacyPolicySettingsLink"], app: app, timeout: 10))
        XCTAssertTrue(revealSettingsElement(app.buttons["SupportSettingsLink"], app: app, timeout: 10))
    }

    func testSettingsShowsEncryptedRecoveryControlsWhenNeeded() {
        let app = launchEncryptedSettingsApp()

        XCTAssertTrue(revealSettingsElement(app.staticTexts["Unverified"], app: app, timeout: 10))
        XCTAssertTrue(revealSettingsElement(app.staticTexts["Needs Recovery"], app: app, timeout: 10))
        XCTAssertTrue(revealSettingsElement(app.staticTexts["Unavailable"], app: app, timeout: 10))
        XCTAssertTrue(revealSettingsElement(app.buttons["RequestDeviceVerificationButton"], app: app, timeout: 10))
        XCTAssertTrue(revealSettingsElement(app.secureTextFields["RecoveryKeyField"], app: app, timeout: 10))
        app.secureTextFields["RecoveryKeyField"].tap()
        app.secureTextFields["RecoveryKeyField"].typeText("mock-recovery-key")
        XCTAssertTrue(revealSettingsElement(app.buttons["RecoverKeysButton"], app: app, timeout: 10))
    }

    func testAboutScreenShowsVersionBuildLicenseSupportAndPrivacyLinks() {
        let app = launchSignedInSettingsApp()

        tapSettingsElement(app.buttons["AboutSettingsLink"], app: app, timeout: 10)

        XCTAssertTrue(app.collectionViews["AboutSettingsScreen"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Synara"].exists)
        XCTAssertTrue(app.staticTexts["Version"].exists)
        XCTAssertTrue(app.staticTexts["Build"].exists)
        XCTAssertTrue(app.buttons["AboutPrivacyLink"].exists)
        XCTAssertTrue(app.buttons["AboutSupportLink"].exists)

        app.navigationBars.buttons.element(boundBy: 0).tap()
        tapSettingsElement(app.buttons["LicensesSettingsLink"], app: app, timeout: 10)
        XCTAssertTrue(app.collectionViews["LicensesSettingsScreen"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["AGPL-3.0-only"].exists)
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

        XCTAssertTrue(app.staticTexts["!project:matrix.org"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.scrollViews["TimelineList"].waitForExistence(timeout: 5))
    }

    func testAgentCardApproveActionShowsSubmittedState() {
        let app = launchAgentCardRoomApp()

        XCTAssertTrue(app.staticTexts["Deploy to Production"].waitForExistence(timeout: 5))
        tap(app.buttons["AgentCardAction-approve-deploy"])

        let alert = app.alerts["Agent Action"]
        XCTAssertTrue(alert.waitForExistence(timeout: 5))
        XCTAssertTrue(alert.staticTexts["Agent action approved"].exists)
    }

    func testAgentCardApprovalFailureIsVisibleAndRetryable() {
        let app = launchAgentCardRoomApp(approvalError: "failed")

        XCTAssertTrue(app.staticTexts["Deploy to Production"].waitForExistence(timeout: 5))
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

        let composer = app.textFields["ComposerTextField"]
        if composer.waitForExistence(timeout: 5) == false {
            XCTAssertTrue(app.tabBars.buttons["Rooms"].waitForExistence(timeout: 60))
        }
        guard composer.waitForExistence(timeout: 30) else {
            XCTFail("Expected encrypted room timeline composer to appear.")
            return
        }

        let message = "Synara live smoke \(Int(Date().timeIntervalSince1970))"
        composer.tap()
        composer.typeText(message)
        tap(app.buttons["ComposerSendButton"], timeout: 10)

        XCTAssertTrue(waitForTimelineElement(app.staticTexts[message], app: app, timeout: 60))
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

        XCTAssertTrue(waitForTimelineElement(app.staticTexts[title], app: app, timeout: 60))
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

    func testLiveEncryptedRoomSmokeWhenConfigured() throws {
        let environment = ProcessInfo.processInfo.environment
        guard liveEnvironmentValue("SYNARA_LIVE_E2EE_SMOKE", in: environment) == "1" else {
            throw XCTSkip("Set SYNARA_LIVE_E2EE_SMOKE=1 for local encrypted-room simulator smoke.")
        }

        guard let homeserver = liveEnvironmentValue("SYNARA_LIVE_HOMESERVER", in: environment),
              let username = liveEnvironmentValue("SYNARA_LIVE_USERNAME", in: environment),
              let password = liveEnvironmentValue("SYNARA_LIVE_PASSWORD", in: environment) else {
            throw XCTSkip("Live encrypted smoke needs homeserver, username, and password environment variables.")
        }

        let roomID = try liveEncryptedRoomID(environment: environment, homeserver: homeserver, username: username, password: password)
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_RESET_SESSION_ON_LAUNCH"] = "1"
        app.launchEnvironment["SYNARA_AUTO_OPEN_ROOM_ID"] = roomID
        app.launch()

        if app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5) {
            loginLive(app: app, homeserver: homeserver, username: username, password: password)
            dismissPasswordSavePromptIfPresent(app: app)
        }

        let composer = app.textFields["ComposerTextField"]
        XCTAssertTrue(composer.waitForExistence(timeout: 60))
        XCTAssertTrue(
            waitForAnyStaticText(
                ["Encrypted", "Recovery Needed", "No Key Backup", "Unverified", "Encryption Unknown"],
                app: app,
                timeout: 30
            )
        )

        let message = "Synara encrypted smoke \(Int(Date().timeIntervalSince1970))"
        composer.tap()
        composer.typeText(message)
        tap(app.buttons["ComposerSendButton"], timeout: 10)

        XCTAssertTrue(waitForTimelineElement(app.staticTexts[message], app: app, timeout: 90))
        XCTAssertFalse(app.staticTexts["Encrypted content unavailable. Actions and media downloads are blocked until keys are available."].exists)

        app.terminate()
        app.launchEnvironment.removeValue(forKey: "SYNARA_RESET_SESSION_ON_LAUNCH")
        app.launchEnvironment["SYNARA_AUTO_OPEN_ROOM_ID"] = roomID
        app.launch()

        XCTAssertTrue(app.textFields["ComposerTextField"].waitForExistence(timeout: 60))
        XCTAssertTrue(waitForTimelineElement(app.staticTexts[message], app: app, timeout: 90))
        XCTAssertTrue(
            waitForAnyStaticText(
                ["Encrypted", "Recovery Needed", "No Key Backup", "Unverified", "Encryption Unknown"],
                app: app,
                timeout: 30
            )
        )
    }

    func testLiveRoomManagementSmokeWhenConfigured() throws {
        let environment = ProcessInfo.processInfo.environment
        guard liveEnvironmentValue("SYNARA_LIVE_ROOM_MANAGEMENT_SMOKE", in: environment) == "1" else {
            throw XCTSkip("Set SYNARA_LIVE_ROOM_MANAGEMENT_SMOKE=1 for local room-management simulator smoke.")
        }

        guard let homeserver = liveEnvironmentValue("SYNARA_LIVE_HOMESERVER", in: environment),
              let username = liveEnvironmentValue("SYNARA_LIVE_USERNAME", in: environment),
              let password = liveEnvironmentValue("SYNARA_LIVE_PASSWORD", in: environment) else {
            throw XCTSkip("Live room-management smoke needs homeserver, username, and password environment variables.")
        }

        let inviteUserID = liveEnvironmentValue("SYNARA_LIVE_INVITE_USER_ID", in: environment)
        let roomName = "Synara UI Room \(Int(Date().timeIntervalSince1970))"

        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_RESET_SESSION_ON_LAUNCH"] = "1"
        app.launch()

        if app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5) {
            loginLive(app: app, homeserver: homeserver, username: username, password: password)
            dismissPasswordSavePromptIfPresent(app: app)
        }

        XCTAssertTrue(app.collectionViews["RoomList"].waitForExistence(timeout: 60))
        XCTAssertTrue(openRoomManagementSheet(app: app, timeout: 20))
        app.textFields["CreateRoomNameField"].tap()
        app.textFields["CreateRoomNameField"].typeText(roomName)
        app.textFields["CreateRoomTopicField"].tap()
        app.textFields["CreateRoomTopicField"].typeText("Disposable live room-management smoke")
        tap(app.buttons["RoomManagementSubmitButton"], timeout: 10)

        XCTAssertTrue(app.textFields["ComposerTextField"].waitForExistence(timeout: 90))
        XCTAssertTrue(app.buttons["RoomDetailsButton"].waitForExistence(timeout: 10))
        tap(app.buttons["RoomDetailsButton"], timeout: 10)

        XCTAssertTrue(app.collectionViews["RoomDetailsScreen"].waitForExistence(timeout: 30))
        XCTAssertTrue(app.staticTexts["Room ID"].exists)
        XCTAssertTrue(app.staticTexts["Encryption"].exists)
        XCTAssertTrue(app.staticTexts["Members"].exists)

        if let inviteUserID, inviteUserID.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false {
            let inviteField = app.textFields["RoomInviteUserField"]
            XCTAssertTrue(revealRoomDetailsElement(inviteField, app: app, timeout: 15))
            inviteField.tap()
            inviteField.typeText(inviteUserID)
            dismissKeyboardIfPresent(app: app)
            XCTAssertTrue(revealRoomDetailsElement(app.buttons["RoomInviteUserButton"], app: app, timeout: 10))
            XCTAssertTrue(waitForEnabled(app.buttons["RoomInviteUserButton"], timeout: 10))
            tap(app.buttons["RoomInviteUserButton"], timeout: 1)
            XCTAssertTrue(revealRoomDetailsElement(app.staticTexts["Invitation sent."], app: app, timeout: 30, direction: .down))
        }

        XCTAssertTrue(revealRoomDetailsElement(app.buttons["LeaveRoomButton"], app: app, timeout: 10))
        tap(app.buttons["LeaveRoomButton"], timeout: 1)
        tap(app.buttons["Leave Room"].firstMatch, timeout: 10)
        XCTAssertTrue(app.collectionViews["RoomList"].waitForExistence(timeout: 60))
        XCTAssertFalse(app.staticTexts[roomName].exists)
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

    private func launchLargeRoomsApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SIGNED_IN"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_LARGE_ROOMS"] = "1"
        app.launch()
        return app
    }

    private func launchFilteredRoomsApp(query: String) -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SIGNED_IN"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_SEARCH"] = query
        app.launch()
        return app
    }

    private func launchLargeTimelineApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_ID"] = "!large:matrix.org"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_TITLE"] = "Large Timeline"
        app.launchEnvironment["SYNARA_UI_TEST_LARGE_TIMELINE"] = "1"
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

    private func launchEncryptedSettingsApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SIGNED_IN"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SELECTED_TAB"] = "settings"
        app.launchEnvironment["SYNARA_UI_TEST_ENCRYPTED_TIMELINE"] = "1"
        app.launch()
        return app
    }

    private func launchEncryptedRoomApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_ID"] = "!encrypted:matrix.org"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_TITLE"] = "Secret"
        app.launchEnvironment["SYNARA_UI_TEST_ENCRYPTED_TIMELINE"] = "1"
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

    private func launchRoomManagementSheetApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SIGNED_IN"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_MANAGEMENT_SHEET"] = "1"
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
        app.swipeUp()
        tap(app.buttons["LoginSubmitButton"])
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
        app.swipeUp()
        tap(app.buttons["LoginSubmitButton"])
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

    private func liveEncryptedRoomID(environment: [String: String], homeserver: String, username: String, password: String) throws -> String {
        if let roomID = liveEnvironmentValue("SYNARA_LIVE_E2EE_ROOM_ID", in: environment)
            ?? liveEnvironmentValue("SYNARA_LIVE_ROOM_ID", in: environment) {
            return roomID
        }

        let liveClient = try MatrixLiveTestClient.login(
            homeserver: homeserver,
            username: username,
            password: password
        )
        let alias = liveEnvironmentValue("SYNARA_LIVE_E2EE_ROOM_ALIAS", in: environment)
            ?? "#test-e2e-room:matrix.whyland.com"
        return try liveClient.resolveRoomAlias(alias)
    }

    private func tap(_ element: XCUIElement, timeout: TimeInterval = 5) {
        XCTAssertTrue(element.waitForExistence(timeout: timeout))
        if element.isHittable {
            element.tap()
        } else {
            element.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
        }
    }

    private func revealSettingsElement(_ element: XCUIElement, app: XCUIApplication, timeout: TimeInterval) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        let settingsList = app.collectionViews["SettingsScreen"].exists
            ? app.collectionViews["SettingsScreen"]
            : app.collectionViews.firstMatch

        while Date() < deadline {
            if element.exists && element.isHittable {
                return true
            }
            if settingsList.exists {
                settingsList.swipeUp()
            } else {
                app.swipeUp()
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.25))
        }

        return element.exists && element.isHittable
    }

    private func tapSettingsElement(_ element: XCUIElement, app: XCUIApplication, timeout: TimeInterval) {
        XCTAssertTrue(revealSettingsElement(element, app: app, timeout: timeout))
        tap(element, timeout: 1)
    }

    private enum ScrollDirection {
        case up
        case down
    }

    private func revealRoomDetailsElement(
        _ element: XCUIElement,
        app: XCUIApplication,
        timeout: TimeInterval,
        direction: ScrollDirection = .up
    ) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        let detailsList = app.collectionViews["RoomDetailsScreen"].exists
            ? app.collectionViews["RoomDetailsScreen"]
            : app.collectionViews.firstMatch

        while Date() < deadline {
            if element.exists && element.isHittable {
                return true
            }
            if detailsList.exists {
                switch direction {
                case .up:
                    detailsList.swipeUp()
                case .down:
                    detailsList.swipeDown()
                }
            } else {
                switch direction {
                case .up:
                    app.swipeUp()
                case .down:
                    app.swipeDown()
                }
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.25))
        }

        return element.exists && element.isHittable
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

    private func dismissKeyboardIfPresent(app: XCUIApplication) {
        guard app.keyboards.firstMatch.exists else {
            return
        }
        if app.keyboards.buttons["Done"].exists {
            app.keyboards.buttons["Done"].tap()
        } else if app.keyboards.buttons["Return"].exists {
            app.keyboards.buttons["Return"].tap()
        } else {
            app.swipeDown()
        }
    }

    private func waitForEnabled(_ element: XCUIElement, timeout: TimeInterval) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if element.exists && element.isEnabled {
                return true
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.25))
        }
        return element.exists && element.isEnabled
    }

    private func openRoomManagementSheet(app: XCUIApplication, timeout: TimeInterval) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if app.staticTexts["Create Room"].exists {
                return true
            }
            let button = app.buttons["NewRoomButton"]
            if button.waitForExistence(timeout: 2) {
                tap(button, timeout: 1)
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.5))
        }
        return app.staticTexts["Create Room"].exists
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

    private func waitForAnyStaticText(_ values: [String], app: XCUIApplication, timeout: TimeInterval) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if values.contains(where: { app.staticTexts[$0].exists }) {
                return true
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.5))
        }
        return values.contains(where: { app.staticTexts[$0].exists })
    }

    private func waitForNonEmptyValue(_ element: XCUIElement, timeout: TimeInterval) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if let value = element.value as? String,
               value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false {
                return true
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.25))
        }
        return (element.value as? String)?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
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
