import Foundation
import XCTest

final class SynaraUITests: XCTestCase {
    override func setUpWithError() throws {
        try super.setUpWithError()
        continueAfterFailure = false
    }

    override func tearDownWithError() throws {
        XCUIApplication().terminate()
        try super.tearDownWithError()
    }

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

    func testRoomHeaderAccountMenuShowsSettingsAndLogout() {
        let app = launchSignedInRoomsApp()

        XCTAssertTrue(app.collectionViews["RoomList"].waitForExistence(timeout: 5))
        tap(app.buttons["RoomHeaderAccountMenuButton"])

        XCTAssertTrue(app.collectionViews["AccountMenuSheet"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["AccountMenuSettingsButton"].exists)
        XCTAssertTrue(app.buttons["AccountMenuLogoutButton"].exists)
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
        XCTAssertTrue(timelineViewport(in: app).waitForExistence(timeout: 5))
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
        tap(app.buttons["SpaceFilter-!workspace:matrix.org"])
        XCTAssertTrue(app.buttons["RoomRow-!project:matrix.org"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["SpaceFilter-all"].exists)
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
        XCTAssertTrue(timelineViewport(in: app).waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Here's the latest spec for the new permissions model."].waitForExistence(timeout: 5))
    }

    func testUnreadRoomRoutePositionsAfterSharedReadMarker() {
        let app = launchRoomApp(
            readMarkerEventID: "$synthetic-30:matrix.org",
            largeTimelineCount: 60
        )
        let viewport = timelineViewport(in: app)

        XCTAssertTrue(viewport.waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Synthetic message 31"].waitForExistence(timeout: 5))
        XCTAssertTrue(
            waitForViewportDiagnostics(viewport, containing: "topEvent=$synthetic-31:matrix.org", timeout: 5),
            "Unexpected viewport diagnostics: \(String(describing: viewport.value))"
        )
        XCTAssertTrue(waitForViewportDiagnostics(viewport, containing: "pinned=false", timeout: 5))
        XCTAssertTrue(app.buttons["JumpToLatestButton"].waitForExistence(timeout: 5))
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
        tap(app.buttons["ConfirmLeaveRoomButton"].firstMatch, timeout: 5)
        XCTAssertTrue(
            waitForAnyElement(
                [
                    app.collectionViews["RoomList"],
                    app.collectionViews["RoomListLoading"],
                    app.buttons["RoomRow-!project:matrix.org"],
                    app.staticTexts["No Rooms"],
                ],
                timeout: 10
            )
        )
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

    func testRoomPersonalNotesRendersAndAddsPrivateNote() {
        let app = launchRoomApp(roomNotes: true)

        tap(app.buttons["RoomDetailsButton"], timeout: 5)
        XCTAssertTrue(app.collectionViews["RoomDetailsScreen"].waitForExistence(timeout: 5))
        let notesLink = app.buttons["RoomPersonalNotesLink"]
        XCTAssertTrue(revealRoomDetailsElement(notesLink, app: app, timeout: 5))
        tap(notesLink)

        XCTAssertTrue(app.collectionViews["RoomNotesScreen"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Review the launch checklist"].exists)
        XCTAssertTrue(app.staticTexts["Discuss the migration privately"].exists)
        XCTAssertTrue(app.staticTexts["Private to your account. Synced across Synara clients; never posted to the room."].exists)

        let editor = app.textViews["RoomNotesBodyEditor"]
        XCTAssertTrue(editor.waitForExistence(timeout: 5))
        editor.tap()
        editor.typeText("iOS parity note")
        let keyboardDone = app.toolbars.buttons["Done"]
        XCTAssertTrue(keyboardDone.waitForExistence(timeout: 5))
        tap(keyboardDone)
        tap(app.buttons["RoomNotesAddButton"], timeout: 5)
        XCTAssertTrue(app.staticTexts["iOS parity note"].waitForExistence(timeout: 5))

        let screenshot = XCTAttachment(screenshot: app.screenshot())
        screenshot.name = "Room Personal Notes"
        screenshot.lifetime = .keepAlways
        add(screenshot)
    }

    func testRoomPersonalNotesExposesExistingItemEditor() {
        let app = launchRoomApp(roomNotes: true)

        tap(app.buttons["RoomDetailsButton"], timeout: 5)
        XCTAssertTrue(app.collectionViews["RoomDetailsScreen"].waitForExistence(timeout: 5))
        let notesLink = app.buttons["RoomPersonalNotesLink"]
        XCTAssertTrue(revealRoomDetailsElement(notesLink, app: app, timeout: 5))
        tap(notesLink)

        XCTAssertTrue(app.collectionViews["RoomNotesScreen"].waitForExistence(timeout: 5))
        let noteRow = app.staticTexts["Discuss the migration privately"]
        XCTAssertTrue(noteRow.waitForExistence(timeout: 5))
        noteRow.press(forDuration: 0.8)
        XCTAssertTrue(app.buttons["Move Down"].waitForExistence(timeout: 5))
        tap(app.buttons["pencil"], timeout: 5)

        let editBody = app.textViews["RoomNotesEditBody"]
        XCTAssertTrue(editBody.waitForExistence(timeout: 5))
        XCTAssertEqual(editBody.value as? String, "Discuss the migration privately")
        XCTAssertTrue(app.buttons["RoomNotesEditSave"].exists)
        editBody.tap()
        editBody.typeText(" updated")
        tap(app.buttons["RoomNotesEditSave"], timeout: 5)
        let updatedText = app.staticTexts.matching(
            NSPredicate(format: "label CONTAINS %@", "updated")
        ).firstMatch
        XCTAssertTrue(updatedText.waitForExistence(timeout: 5))
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

        XCTAssertTrue(app.staticTexts["Synthetic message 999"].waitForExistence(timeout: 5))

        app.swipeDown()
        XCTAssertTrue(app.buttons["TimelineSearchButton"].exists)
    }

    func testStableViewportThreeMessageScrollJumpLatestAndFiveThousandEventBoundedness() {
        let app = launchLargeTimelineApp(count: 5000)
        let viewport = timelineViewport(in: app)

        XCTAssertTrue(viewport.waitForExistence(timeout: 8))
        XCTAssertTrue(app.staticTexts["Synthetic message 4999"].waitForExistence(timeout: 8))
        XCTAssertTrue(waitForViewportDiagnostics(viewport, containing: "renderedEvents=300", timeout: 5))

        let start = viewport.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.42))
        let end = viewport.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.68))
        start.press(forDuration: 0.05, thenDragTo: end)

        XCTAssertTrue(app.buttons["JumpToLatestButton"].waitForExistence(timeout: 5))
        tap(app.buttons["JumpToLatestButton"])
        XCTAssertTrue(app.staticTexts["Synthetic message 4999"].waitForExistence(timeout: 8))
        XCTAssertTrue(waitForViewportDiagnostics(viewport, containing: "pinned=true", timeout: 5))

        let diagnostics = viewportDiagnostics(viewport)
        let visibleCells = Int(diagnostics["visibleCells"] ?? "")
        XCTAssertNotNil(visibleCells)
        XCTAssertLessThan(visibleCells ?? .max, 40)
    }

    func testStableViewportPreservesAnchorAcrossVariableHeightReplacement() {
        let app = launchLargeTimelineApp(count: 40, scenario: "height-change")
        let viewport = timelineViewport(in: app)

        XCTAssertTrue(viewport.waitForExistence(timeout: 8))
        viewport.swipeDown(velocity: .slow)
        let anchorBefore = viewportDiagnostics(viewport)["topEvent"]
        XCTAssertNotNil(anchorBefore)

        let expanded = app.staticTexts.matching(
            NSPredicate(format: "label BEGINSWITH %@", "Expanded variable-height message 137")
        ).firstMatch
        XCTAssertTrue(expanded.waitForExistence(timeout: 12))
        XCTAssertEqual(viewportDiagnostics(viewport)["topEvent"], anchorBefore)
    }

    func testStableViewportPreservesAnchorAcrossPrependSnapshot() {
        let app = launchLargeTimelineApp(count: 40, scenario: "prepend")
        let viewport = timelineViewport(in: app)

        XCTAssertTrue(viewport.waitForExistence(timeout: 8))
        viewport.swipeDown(velocity: .slow)
        let anchorBefore = viewportDiagnostics(viewport)["topEvent"]
        XCTAssertNotNil(anchorBefore)

        XCTAssertTrue(waitForViewportDiagnostics(viewport, containing: "renderedEvents=90", timeout: 12))
        XCTAssertEqual(viewportDiagnostics(viewport)["topEvent"], anchorBefore)
    }

    func testStableViewportRapidRoomSwitchingRejectsStaleRouteUpdates() {
        let app = launchSignedInRoomsApp()

        tap(app.buttons["RoomRow-!project:matrix.org"], timeout: 5)
        let initialProjectViewport = timelineViewport(in: app)
        XCTAssertTrue(waitForViewportDiagnostics(initialProjectViewport, containing: "routeID=!project:matrix.org", timeout: 5))
        XCTAssertTrue(waitForViewportDiagnostics(initialProjectViewport, containing: "newestEvent=$alex-thread:!project:matrix.org", timeout: 5))
        let initialDiagnostics = viewportDiagnostics(initialProjectViewport)
        let initialRouteID = initialDiagnostics["routeID"]
        let initialGeneration = Int(initialDiagnostics["generation"] ?? "")
        XCTAssertNotNil(initialRouteID)
        XCTAssertEqual(initialGeneration, 1)
        tap(app.buttons["Back"], timeout: 5)
        tap(app.buttons["RoomRow-!general:matrix.org"], timeout: 5)
        let generalViewport = timelineViewport(in: app)
        XCTAssertTrue(waitForViewportDiagnostics(generalViewport, containing: "routeID=!general:matrix.org", timeout: 5))
        XCTAssertTrue(waitForViewportDiagnostics(generalViewport, containing: "newestEvent=$alex-thread:!general:matrix.org", timeout: 5))
        let generalRouteID = viewportDiagnostics(generalViewport)["routeID"]
        XCTAssertNotNil(generalRouteID)
        tap(app.buttons["Back"], timeout: 5)
        tap(app.buttons["RoomRow-!project:matrix.org"], timeout: 5)

        let finalViewport = timelineViewport(in: app)
        XCTAssertTrue(waitForViewportDiagnostics(finalViewport, containing: "routeID=!project:matrix.org", timeout: 8))
        XCTAssertTrue(waitForViewportDiagnostics(finalViewport, containing: "newestEvent=$alex-thread:!project:matrix.org", timeout: 5))
        XCTAssertTrue(waitForViewportDiagnostics(finalViewport, containing: "pinned=true", timeout: 5))
        let diagnostics = viewportDiagnostics(finalViewport)
        let finalRouteID = diagnostics["routeID"]
        let finalGeneration = Int(diagnostics["generation"] ?? "")
        XCTAssertNotNil(finalRouteID)
        XCTAssertEqual(finalGeneration, 1)
        XCTAssertNotEqual(finalRouteID, initialRouteID)
        XCTAssertNotEqual(finalRouteID, generalRouteID)
        XCTAssertEqual(diagnostics["newestEvent"], "$alex-thread:!project:matrix.org")
        XCTAssertFalse(finalViewport.value.debugDescription.contains("!general:matrix.org"))
    }

    func testComposerSendsMockMessage() {
        let app = launchRoomApp()

        let composer = composerField(in: app)
        XCTAssertTrue(composer.waitForExistence(timeout: 5))
        composer.tap()
        composer.typeText("hello from ui")
        XCTAssertTrue(app.buttons["ComposerSendButton"].waitForExistence(timeout: 5))
        tap(app.buttons["ComposerSendButton"])

        let sentMessage = app.staticTexts.containing(NSPredicate(format: "label CONTAINS %@", "hello from ui")).firstMatch
        XCTAssertTrue(sentMessage.waitForExistence(timeout: 5))
    }

    func testComposerFormattingToolbarSendsRenderedMessage() {
        let app = launchRoomApp()

        tap(app.buttons["ComposerFormattingToggle"])
        XCTAssertTrue(app.scrollViews["ComposerFormattingBar"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["ComposerFormat-bold"].exists)
        XCTAssertTrue(app.buttons["ComposerFormat-bulletList"].exists)

        tap(app.buttons["ComposerFormat-bold"])
        let composer = composerField(in: app)
        XCTAssertTrue(composer.waitForExistence(timeout: 5))
        XCTAssertEqual(composer.value as? String, "**bold text**")

        tap(app.buttons["ComposerSendButton"])
        XCTAssertTrue(app.staticTexts["bold text"].waitForExistence(timeout: 5))
        XCTAssertFalse(app.staticTexts["**bold text**"].exists)
    }

    func testMediaUploadAddsAttachmentPlaceholder() {
        let app = launchRoomApp()

        tap(app.buttons["AttachmentButton"])
        XCTAssertTrue(app.otherElements["AttachmentOptionsSheet"].waitForExistence(timeout: 5))
        tap(app.buttons["AttachmentOption-Photo or Video"])

        let draftList = identifiedElement(in: app, "ComposerAttachmentDraftList")
        XCTAssertTrue(draftList.waitForExistence(timeout: 5))
        XCTAssertTrue(identifiedElement(in: app, "ComposerAttachmentDraft-synara-upload.jpg").waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["MediaPlaceholder-synara-upload.jpg"].exists)
        XCTAssertTrue(app.buttons["ComposerSendButton"].waitForExistence(timeout: 5))
        tap(app.buttons["ComposerSendButton"])

        XCTAssertTrue(app.buttons["MediaPlaceholder-synara-upload.jpg"].waitForExistence(timeout: 5))
        XCTAssertFalse(draftList.exists)
    }

    func testFileUploadAddsAttachmentPlaceholder() {
        let app = launchRoomApp()

        tap(app.buttons["AttachmentButton"])
        XCTAssertTrue(app.otherElements["AttachmentOptionsSheet"].waitForExistence(timeout: 5))
        tap(app.buttons["AttachmentOption-File"])

        let draftList = identifiedElement(in: app, "ComposerAttachmentDraftList")
        XCTAssertTrue(draftList.waitForExistence(timeout: 5))
        XCTAssertTrue(identifiedElement(in: app, "ComposerAttachmentDraft-synara-upload.pdf").waitForExistence(timeout: 5))
        XCTAssertFalse(app.buttons["MediaPlaceholder-synara-upload.pdf"].exists)
        XCTAssertTrue(app.buttons["ComposerSendButton"].waitForExistence(timeout: 5))
        tap(app.buttons["ComposerSendButton"])

        XCTAssertTrue(app.buttons["MediaPlaceholder-synara-upload.pdf"].waitForExistence(timeout: 5))
        XCTAssertFalse(draftList.exists)
    }

    func testThreadViewOpensAndRepliesFromTimeline() {
        let app = launchRoomApp()
        let threadButton = app.buttons["ThreadButton-$security:!project:matrix.org"]

        XCTAssertTrue(threadButton.waitForExistence(timeout: 5))
        tap(threadButton)
        XCTAssertTrue(app.staticTexts["ThreadTimelineTitle"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.scrollViews["ThreadTimelineList"].exists)

        let composer = composerField(in: app)
        XCTAssertTrue(composer.waitForExistence(timeout: 5))
        composer.tap()
        composer.typeText("replying in thread")
        tap(app.buttons["ComposerSendButton"])

        XCTAssertTrue(app.staticTexts["replying in thread"].waitForExistence(timeout: 5))
    }

    func testEncryptedTimelineShowsCryptoStatusRecoveryBannerAndSafePlaceholder() {
        let app = launchEncryptedRoomApp()

        XCTAssertTrue(timelineViewport(in: app).waitForExistence(timeout: 5))
        XCTAssertTrue(
            waitForTimelineElement(app.otherElements["EncryptedRecoveryBanner"], app: app, timeout: 5, preferredSwipe: .down)
        )
        XCTAssertTrue(
            waitForTimelineElement(app.staticTexts["Encrypted history needs attention"], app: app, timeout: 5, preferredSwipe: .down)
        )
        XCTAssertTrue(waitForTimelineElement(app.buttons["Retry Decryption"], app: app, timeout: 5, preferredSwipe: .down))
        XCTAssertTrue(waitForTimelineElement(app.buttons["Review Security"], app: app, timeout: 5, preferredSwipe: .down))
        XCTAssertTrue(waitForTimelineElement(app.staticTexts["Decrypted encrypted-room message"], app: app, timeout: 5))
        XCTAssertTrue(
            waitForTimelineElement(
                app.staticTexts["Encrypted content unavailable. Actions and media downloads are blocked until keys are available."],
                app: app,
                timeout: 5
            )
        )
    }

    func testNotificationsInboxShowsUnreadRooms() {
        let app = launchSignedInNotificationsApp()

        XCTAssertTrue(app.staticTexts["Notifications"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["NotificationsRow-!project:matrix.org"].waitForExistence(timeout: 10))

        let unreadRoomsDisclosure = app.buttons["Unread rooms"]
        if unreadRoomsDisclosure.waitForExistence(timeout: 2) {
            tap(unreadRoomsDisclosure)
            XCTAssertTrue(app.buttons["NotificationsRow-!general:matrix.org"].waitForExistence(timeout: 5))
        }
    }

    func testLogoutReturnsToSignedOutShell() {
        let app = launchSignedInSettingsApp()

        tapSettingsElement(app.buttons["LogoutButton"], app: app, timeout: 10)
        tap(app.buttons["ConfirmLogoutButton"].firstMatch, timeout: 5)

        XCTAssertTrue(app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5))
    }

    func testSettingsShowsNotificationSectionsAndReleaseLinks() {
        let app = launchSignedInSettingsApp()

        tapSettingsElement(app.buttons["NotificationSettingsLink"], app: app, timeout: 10)
        XCTAssertTrue(app.collectionViews["NotificationSettingsScreen"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["NotificationPermissionButton"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["PushRegistrationButton"].exists)
        app.navigationBars.buttons.element(boundBy: 0).tap()

        tapSettingsElement(app.buttons["AppearanceSettingsLink"], app: app, timeout: 10)
        XCTAssertTrue(app.collectionViews["AppearanceSettingsScreen"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Appearance"].exists)
        XCTAssertTrue(revealSettingsElement(app.staticTexts["Text Size"], app: app, timeout: 10))
        app.navigationBars.buttons.element(boundBy: 0).tap()

        tapSettingsElement(app.buttons["SecuritySettingsLink"], app: app, timeout: 10)
        XCTAssertTrue(app.collectionViews["SecuritySettingsScreen"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["Session Storage"].exists)
        XCTAssertTrue(app.staticTexts["Device Verification"].exists)
        XCTAssertTrue(app.staticTexts["Key Recovery"].exists)
        XCTAssertTrue(app.staticTexts["Key Backup"].exists)
        app.navigationBars.buttons.element(boundBy: 0).tap()

        XCTAssertTrue(app.buttons["AboutSettingsLink"].exists)
        XCTAssertTrue(app.buttons["LicensesSettingsLink"].exists)
        XCTAssertTrue(app.buttons["PrivacyPolicySettingsLink"].exists)
        XCTAssertTrue(app.buttons["SupportSettingsLink"].exists)
    }

    func testSettingsShowsEncryptedRecoveryControlsWhenNeeded() {
        let app = launchEncryptedSettingsApp()

        tapSettingsElement(app.buttons["SecuritySettingsLink"], app: app, timeout: 10)
        XCTAssertTrue(app.collectionViews["SecuritySettingsScreen"].waitForExistence(timeout: 5))
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
        XCTAssertTrue(app.collectionViews["SettingsScreen"].waitForExistence(timeout: 10))

        tapSettingsElement(app.buttons["AboutSettingsLink"], app: app, timeout: 10)
        let aboutScreen = identifiedElement(in: app, "AboutSettingsScreen")
        if aboutScreen.waitForExistence(timeout: 5) == false {
            tapSettingsElement(app.buttons["AboutSettingsLink"], app: app, timeout: 10)
        }
        XCTAssertTrue(aboutScreen.waitForExistence(timeout: 10))
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

    func testSettingsNavigationDestinationsOpenAndReturn() {
        let app = launchSignedInSettingsApp()

        tapSettingsElement(app.buttons["PrivacyPolicySettingsLink"], app: app, timeout: 10)
        XCTAssertTrue(app.collectionViews["PrivacyPolicySettingsScreen"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["PrivacyPolicyExternalLink"].exists)
        app.navigationBars.buttons.element(boundBy: 0).tap()

        tapSettingsElement(app.buttons["SupportSettingsLink"], app: app, timeout: 10)
        XCTAssertTrue(app.collectionViews["SupportSettingsScreen"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["SupportExternalLink"].exists)
        app.navigationBars.buttons.element(boundBy: 0).tap()

        XCTAssertTrue(app.collectionViews["SettingsScreen"].waitForExistence(timeout: 5))
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
        XCTAssertTrue(app.buttons["LaterRow-$text_!project_matrix.org"].exists)
        XCTAssertTrue(app.buttons["LaterRow-$done"].exists)
        XCTAssertTrue(app.descendants(matching: .any)["LaterRow-reminder-missing-destination"].exists)
        XCTAssertTrue(app.staticTexts["Completed"].exists)
        XCTAssertTrue(app.staticTexts["Destination unavailable"].exists)
    }

    func testLaterItemNavigatesToRoomAnchor() {
        let app = launchLaterApp()

        let row = app.buttons["LaterRow-$text_!project_matrix.org"]
        XCTAssertTrue(row.waitForExistence(timeout: 5))
        tap(row)

        XCTAssertTrue(app.buttons["TimelineSearchButton"].waitForExistence(timeout: 10))
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

    func testAgentApprovalPromptShowsEmojiReactionActions() {
        let app = launchAgentApprovalPromptRoomApp()

        XCTAssertTrue(app.staticTexts["Approval Required: Dangerous Command"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["AgentApprovalPromptReaction-approveOnce-$agent-approval-prompt"].exists)
        XCTAssertTrue(app.buttons["AgentApprovalPromptReaction-approveAlways-$agent-approval-prompt"].exists)
        XCTAssertTrue(app.buttons["AgentApprovalPromptReaction-deny-$agent-approval-prompt"].exists)

        tap(app.buttons["AgentApprovalPromptReaction-approveOnce-$agent-approval-prompt"])

        let alert = app.alerts["Agent Action"]
        XCTAssertTrue(alert.waitForExistence(timeout: 5))
        XCTAssertTrue(alert.staticTexts["Approval reaction sent."].exists)
    }

    func testLiveStaleCacheSmokeWhenConfigured() throws {
        let environment = ProcessInfo.processInfo.environment
        guard liveEnvironmentValue("SYNARA_LIVE_STALE_CACHE_SMOKE", in: environment) == "1" else {
            throw XCTSkip("Set SYNARA_LIVE_STALE_CACHE_SMOKE=1 for stale-cache live simulator smoke.")
        }

        guard let homeserver = liveEnvironmentValue("SYNARA_LIVE_HOMESERVER", in: environment),
              let username = liveEnvironmentValue("SYNARA_LIVE_USERNAME", in: environment),
              let password = liveEnvironmentValue("SYNARA_LIVE_PASSWORD", in: environment)
        else {
            throw XCTSkip("Stale-cache live smoke needs homeserver, username, and password environment variables.")
        }

        let roomID: String
        if let configuredRoomID = liveEnvironmentValue("SYNARA_LIVE_ROOM_ID", in: environment) {
            roomID = configuredRoomID
        } else {
            let liveClient = try MatrixLiveTestClient.login(
                homeserver: homeserver,
                username: username,
                password: password
            )
            let alias = liveEnvironmentValue("SYNARA_LIVE_ROOM_ALIAS", in: environment) ?? "#test-e2e-room:matrix.example.com"
            roomID = try liveClient.resolveRoomAlias(alias)
        }

        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_RESET_SESSION_ON_LAUNCH"] = "1"
        app.launchEnvironment["SYNARA_AUTO_OPEN_ROOM_ID"] = roomID
        launch(app)

        if app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5) {
            loginLive(app: app, homeserver: homeserver, username: username, password: password)
            dismissPasswordSavePromptIfPresent(app: app)
        }

        let composer = composerField(in: app)
        XCTAssertTrue(composer.waitForExistence(timeout: 60))

        let liveClient = try MatrixLiveTestClient.login(
            homeserver: homeserver,
            username: username,
            password: password
        )
        let message = "Synara stale-cache smoke \(Int(Date().timeIntervalSince1970))"
        _ = try liveClient.sendRoomMessage(roomID: roomID, body: message)

        XCTAssertTrue(waitForTimelineElement(app.staticTexts[message], app: app, timeout: 90))
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
        launch(app)

        if app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5) {
            guard let homeserver = liveEnvironmentValue("SYNARA_LIVE_HOMESERVER", in: environment),
                  let username = liveEnvironmentValue("SYNARA_LIVE_USERNAME", in: environment),
                  let password = liveEnvironmentValue("SYNARA_LIVE_PASSWORD", in: environment)
            else {
                throw XCTSkip("Live smoke needs an existing session or live credentials in environment variables.")
            }
            loginLive(app: app, homeserver: homeserver, username: username, password: password)
            dismissPasswordSavePromptIfPresent(app: app)
        }

        let composer = composerField(in: app)
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

    func testLiveRoomNotesSyncWhenConfigured() throws {
        let environment = ProcessInfo.processInfo.environment
        guard liveEnvironmentValue("SYNARA_LIVE_ROOM_NOTES_SMOKE", in: environment) == "1" else {
            throw XCTSkip("Set SYNARA_LIVE_ROOM_NOTES_SMOKE=1 for the live room-notes sync smoke.")
        }
        guard let homeserver = liveEnvironmentValue("SYNARA_LIVE_HOMESERVER", in: environment),
              let username = liveEnvironmentValue("SYNARA_LIVE_USERNAME", in: environment),
              let password = liveEnvironmentValue("SYNARA_LIVE_PASSWORD", in: environment)
        else {
            throw XCTSkip("Live room-notes sync needs homeserver, username, and password environment variables.")
        }

        let liveClient = try MatrixLiveTestClient.login(
            homeserver: homeserver,
            username: username,
            password: password
        )
        try liveClient.cleanupDisposableRooms(namePrefixes: ["Synara Notes Smoke "])

        let smokeID = Int(Date().timeIntervalSince1970)
        let roomID = try liveClient.createPrivateRoom(name: "Synara Notes Smoke \(smokeID)")
        let externalNoteID = "note:external-\(smokeID)"
        let externalNoteBody = "External client note \(smokeID)"
        let iosNoteBody = "iOS account-data note \(smokeID)"
        let now = Int64(Date().timeIntervalSince1970 * 1_000)

        defer {
            try? liveClient.removeRoomNotesRoom(roomID: roomID)
            try? liveClient.leaveRoom(roomID: roomID)
            try? liveClient.cleanupDisposableRooms(namePrefixes: ["Synara Notes Smoke "])
            try? liveClient.logout()
        }

        try liveClient.replaceRoomNotesRoom(
            roomID: roomID,
            items: [
                externalNoteID: [
                    "id": externalNoteID,
                    "kind": "note",
                    "roomId": roomID,
                    "createdAt": now,
                    "updatedAt": now,
                    "body": externalNoteBody,
                ],
            ]
        )

        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_RESET_SESSION_ON_LAUNCH"] = "1"
        app.launchEnvironment["SYNARA_AUTO_OPEN_ROOM_ID"] = roomID
        launch(app)

        if app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5) {
            loginLive(app: app, homeserver: homeserver, username: username, password: password)
            dismissPasswordSavePromptIfPresent(app: app)
        }

        XCTAssertTrue(composerField(in: app).waitForExistence(timeout: 90))
        tap(app.buttons["RoomDetailsButton"], timeout: 15)
        XCTAssertTrue(app.collectionViews["RoomDetailsScreen"].waitForExistence(timeout: 30))
        XCTAssertTrue(revealRoomDetailsElement(app.buttons["RoomPersonalNotesLink"], app: app, timeout: 15))
        tap(app.buttons["RoomPersonalNotesLink"], timeout: 10)

        XCTAssertTrue(app.collectionViews["RoomNotesScreen"].waitForExistence(timeout: 30))
        XCTAssertTrue(
            app.staticTexts[externalNoteBody].waitForExistence(timeout: 30),
            "A note written by another Matrix client must be visible in iOS."
        )

        let editor = app.textViews["RoomNotesBodyEditor"]
        XCTAssertTrue(editor.waitForExistence(timeout: 10))
        editor.tap()
        editor.typeText(iosNoteBody)
        if app.toolbars.buttons["Done"].waitForExistence(timeout: 5) {
            tap(app.toolbars.buttons["Done"])
        }
        tap(app.buttons["RoomNotesAddButton"], timeout: 10)
        XCTAssertTrue(app.staticTexts[iosNoteBody].waitForExistence(timeout: 30))
        XCTAssertTrue(
            liveClient.waitForRoomNote(roomID: roomID, body: iosNoteBody, timeout: 30),
            "The iOS write must be readable from Matrix global account data."
        )
    }

    func testLiveRichFormattingSmokeWhenConfigured() throws {
        let environment = ProcessInfo.processInfo.environment
        guard liveEnvironmentValue("SYNARA_LIVE_RICH_TEXT_SMOKE", in: environment) == "1" else {
            throw XCTSkip("Set SYNARA_LIVE_RICH_TEXT_SMOKE=1 for live rich-text simulator smoke.")
        }
        guard let homeserver = liveEnvironmentValue("SYNARA_LIVE_HOMESERVER", in: environment),
              let username = liveEnvironmentValue("SYNARA_LIVE_USERNAME", in: environment),
              let password = liveEnvironmentValue("SYNARA_LIVE_PASSWORD", in: environment)
        else {
            throw XCTSkip("Live rich-text smoke needs homeserver, username, and password environment variables.")
        }

        let liveClient = try MatrixLiveTestClient.login(
            homeserver: homeserver,
            username: username,
            password: password
        )
        try liveClient.cleanupDisposableRooms(namePrefixes: ["Synara Rich Text Smoke "])
        let timestamp = Int(Date().timeIntervalSince1970)
        let roomID = try liveClient.createPrivateRoom(name: "Synara Rich Text Smoke \(timestamp)")
        defer {
            try? liveClient.leaveRoom(roomID: roomID)
            try? liveClient.cleanupDisposableRooms(namePrefixes: ["Synara Rich Text Smoke "])
        }

        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_RESET_SESSION_ON_LAUNCH"] = "1"
        app.launchEnvironment["SYNARA_AUTO_OPEN_ROOM_ID"] = roomID
        launch(app)

        if app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5) {
            loginLive(app: app, homeserver: homeserver, username: username, password: password)
            dismissPasswordSavePromptIfPresent(app: app)
        }

        let composer = composerField(in: app)
        XCTAssertTrue(composer.waitForExistence(timeout: 90))
        composer.tap()
        let prefix = "Synara rich smoke \(timestamp) "
        composer.typeText(prefix)
        tap(app.buttons["ComposerFormattingToggle"], timeout: 10)
        XCTAssertTrue(app.buttons["ComposerFormat-bold"].waitForExistence(timeout: 5))
        tap(app.buttons["ComposerFormat-bold"])
        tap(app.buttons["ComposerSendButton"], timeout: 10)

        let body = "\(prefix)**bold text**"
        guard let content = liveClient.waitForMessageContent(roomID: roomID, body: body, timeout: 60) else {
            XCTFail("Formatted message did not reach the homeserver.")
            return
        }
        XCTAssertEqual(content["format"] as? String, "org.matrix.custom.html")
        XCTAssertTrue((content["formatted_body"] as? String)?.contains("<strong>bold text</strong>") == true)
        XCTAssertTrue(waitForTimelineElement(app.staticTexts.containing(NSPredicate(format: "label CONTAINS %@", "bold text")).firstMatch, app: app, timeout: 30))
        XCTAssertFalse(app.staticTexts[body].exists)
    }

    func testLiveAgentApprovalSmokeWhenConfigured() throws {
        let environment = ProcessInfo.processInfo.environment
        guard liveEnvironmentValue("SYNARA_LIVE_AGENT_SMOKE", in: environment) == "1" else {
            throw XCTSkip("Set SYNARA_LIVE_AGENT_SMOKE=1 for local live agent approval smoke.")
        }

        guard let homeserver = liveEnvironmentValue("SYNARA_LIVE_HOMESERVER", in: environment),
              let username = liveEnvironmentValue("SYNARA_LIVE_USERNAME", in: environment),
              let password = liveEnvironmentValue("SYNARA_LIVE_PASSWORD", in: environment)
        else {
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
        launch(app)

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
              let password = liveEnvironmentValue("SYNARA_LIVE_PASSWORD", in: environment)
        else {
            throw XCTSkip("Live encrypted smoke needs homeserver, username, and password environment variables.")
        }

        let roomID = try liveEncryptedRoomID(environment: environment, homeserver: homeserver, username: username, password: password)
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_RESET_SESSION_ON_LAUNCH"] = "1"
        app.launchEnvironment["SYNARA_AUTO_OPEN_ROOM_ID"] = roomID
        launch(app)

        if app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5) {
            loginLive(app: app, homeserver: homeserver, username: username, password: password)
            dismissPasswordSavePromptIfPresent(app: app)
        }

        let composer = composerField(in: app)
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
        XCTAssertFalse(
            app.staticTexts.matching(
                NSPredicate(format: "label CONTAINS %@", "m.room.encrypted")
            ).firstMatch.exists
        )
        XCTAssertFalse(
            app.staticTexts.matching(
                NSPredicate(format: "label CONTAINS %@", "\"ciphertext\"")
            ).firstMatch.exists
        )

        app.terminate()
        app.launchEnvironment.removeValue(forKey: "SYNARA_RESET_SESSION_ON_LAUNCH")
        app.launchEnvironment["SYNARA_AUTO_OPEN_ROOM_ID"] = roomID
        launch(app)

        XCTAssertTrue(composerField(in: app).waitForExistence(timeout: 60))
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
              let password = liveEnvironmentValue("SYNARA_LIVE_PASSWORD", in: environment)
        else {
            throw XCTSkip("Live room-management smoke needs homeserver, username, and password environment variables.")
        }

        let inviteUserID = liveEnvironmentValue("SYNARA_LIVE_INVITE_USER_ID", in: environment)
        let roomName = "Synara UI Room \(Int(Date().timeIntervalSince1970))"
        let liveClient = try MatrixLiveTestClient.login(
            homeserver: homeserver,
            username: username,
            password: password
        )
        try liveClient.cleanupDisposableRooms(namePrefixes: ["Synara UI Room "])
        defer {
            try? liveClient.cleanupDisposableRooms(namePrefixes: ["Synara UI Room "])
        }

        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_RESET_SESSION_ON_LAUNCH"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SELECTED_TAB"] = "settings"
        launch(app)

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

        XCTAssertTrue(composerField(in: app).waitForExistence(timeout: 90))
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
        tap(app.buttons["ConfirmLeaveRoomButton"].firstMatch, timeout: 10)
        XCTAssertTrue(
            waitForAnyElement(
                [
                    app.collectionViews["RoomList"],
                    app.collectionViews["RoomListLoading"],
                    app.staticTexts["No Rooms"],
                ],
                timeout: 60
            )
        )
        XCTAssertFalse(app.staticTexts[roomName].exists)
    }

    func testLiveVisualMockupScreenshotsWhenConfigured() throws {
        let environment = ProcessInfo.processInfo.environment
        guard liveEnvironmentValue("SYNARA_LIVE_VISUAL_SMOKE", in: environment) == "1" else {
            throw XCTSkip("Set SYNARA_LIVE_VISUAL_SMOKE=1 for local visual smoke screenshots.")
        }

        guard let homeserver = liveEnvironmentValue("SYNARA_LIVE_HOMESERVER", in: environment),
              let username = liveEnvironmentValue("SYNARA_LIVE_USERNAME", in: environment),
              let password = liveEnvironmentValue("SYNARA_LIVE_PASSWORD", in: environment),
              let screenshotDirectory = liveEnvironmentValue("SYNARA_SCREENSHOT_DIR", in: environment)
        else {
            throw XCTSkip("Live visual smoke needs homeserver, username, password, and screenshot directory.")
        }

        let roomID = liveEnvironmentValue("SYNARA_LIVE_ROOM_ID", in: environment)
        let roomName = liveEnvironmentValue("SYNARA_LIVE_ROOM_NAME", in: environment) ?? "Alerts"
        let liveClient = try MatrixLiveTestClient.login(
            homeserver: homeserver,
            username: username,
            password: password
        )
        try liveClient.cleanupDisposableRooms(
            namePrefixes: [
                "Synara Rich Text Smoke ",
                "Synara UI Room ",
                "Synara Live Smoke ",
                "Prism integration",
                "Prism dual-user test",
            ]
        )
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_RESET_SESSION_ON_LAUNCH"] = "1"
        launch(app)

        if app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5) {
            loginLive(app: app, homeserver: homeserver, username: username, password: password)
            dismissPasswordSavePromptIfPresent(app: app)
        }

        XCTAssertTrue(app.collectionViews["RoomList"].waitForExistence(timeout: 60))
        if let roomID {
            let preview = "Synara room-list preview smoke \(Int(Date().timeIntervalSince1970))"
            _ = try liveClient.sendRoomMessage(roomID: roomID, body: preview)
            XCTAssertTrue(
                app.staticTexts[preview].waitForExistence(timeout: 60),
                "A newly streamed event must update the visible room preview."
            )
        }
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "01-live-room-list")

        if let roomID {
            tap(app.buttons["RoomRow-\(roomID)"], timeout: 15)
        } else {
            tap(app.buttons.matching(NSPredicate(format: "label CONTAINS[c] %@", roomName)).firstMatch, timeout: 15)
        }

        XCTAssertTrue(composerField(in: app).waitForExistence(timeout: 60))
        XCTAssertTrue(timelineViewport(in: app).waitForExistence(timeout: 60))
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "02-live-room-timeline")

        let composer = composerField(in: app)
        composer.tap()
        composer.typeText("Visual validation draft")
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "03-live-composer-typing")

        tap(app.buttons["AttachmentButton"], timeout: 10)
        XCTAssertTrue(app.otherElements["AttachmentOptionsSheet"].waitForExistence(timeout: 5))
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "04-live-attachment-sheet")
    }

    func testLiveSettingsVisualScreenshotsWhenConfigured() throws {
        let environment = ProcessInfo.processInfo.environment
        guard liveEnvironmentValue("SYNARA_LIVE_SETTINGS_SMOKE", in: environment) == "1" else {
            throw XCTSkip("Set SYNARA_LIVE_SETTINGS_SMOKE=1 for local live Settings screenshots.")
        }
        guard let homeserver = liveEnvironmentValue("SYNARA_LIVE_HOMESERVER", in: environment),
              let username = liveEnvironmentValue("SYNARA_LIVE_USERNAME", in: environment),
              let password = liveEnvironmentValue("SYNARA_LIVE_PASSWORD", in: environment),
              let screenshotDirectory = liveEnvironmentValue("SYNARA_SCREENSHOT_DIR", in: environment)
        else {
            throw XCTSkip("Live Settings smoke needs homeserver, username, password, and screenshot directory.")
        }

        let app = XCUIApplication()
        if liveEnvironmentValue("SYNARA_LIVE_SETTINGS_REUSE_SESSION", in: environment) != "1" {
            app.launchEnvironment["SYNARA_RESET_SESSION_ON_LAUNCH"] = "1"
        }
        launch(app)

        if app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5) {
            loginLive(app: app, homeserver: homeserver, username: username, password: password)
            dismissPasswordSavePromptIfPresent(app: app)
        }

        // A reused signed-in session may reopen the last Settings subpage.
        // Route through the product URL so this visual audit always starts at
        // the top-level Settings screen without mutating the session.
        app.launchEnvironment.removeValue(forKey: "SYNARA_RESET_SESSION_ON_LAUNCH")
        if #available(iOS 16.4, *) {
            app.open(URL(string: "synara://settings")!)
        } else {
            tap(app.buttons["SettingsTab"], timeout: 20)
        }

        XCTAssertTrue(app.collectionViews["SettingsScreen"].waitForExistence(timeout: 20))
        XCTAssertTrue(app.buttons["AccountSettingsLink"].waitForExistence(timeout: 10))
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "08-live-settings-top")

        tapSettingsElement(app.buttons["AccountSettingsLink"], app: app, timeout: 10)
        XCTAssertTrue(app.collectionViews["AccountSettingsScreen"].waitForExistence(timeout: 10))
        XCTAssertTrue(app.staticTexts["User"].exists)
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "09-live-settings-account")
        navigateBack(app: app)

        tapSettingsElement(app.buttons["NotificationSettingsLink"], app: app, timeout: 10)
        XCTAssertTrue(app.collectionViews["NotificationSettingsScreen"].waitForExistence(timeout: 10))
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "10-live-settings-notifications")
        navigateBack(app: app)

        tapSettingsElement(app.buttons["AppearanceSettingsLink"], app: app, timeout: 10)
        XCTAssertTrue(app.collectionViews["AppearanceSettingsScreen"].waitForExistence(timeout: 10))
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "11-live-settings-appearance")
        navigateBack(app: app)

        tapSettingsElement(app.buttons["SecuritySettingsLink"], app: app, timeout: 10)
        XCTAssertTrue(app.collectionViews["SecuritySettingsScreen"].waitForExistence(timeout: 10))
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "12-live-settings-security")
        navigateBack(app: app)

        XCTAssertTrue(revealSettingsElement(app.buttons["AboutSettingsLink"], app: app, timeout: 15))
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "13-live-settings-lower")

        tap(app.buttons["AboutSettingsLink"])
        XCTAssertTrue(identifiedElement(in: app, "AboutSettingsScreen").waitForExistence(timeout: 10))
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "14-live-settings-about")
        navigateBack(app: app)

        XCTAssertTrue(revealSettingsElement(app.buttons["LogoutButton"], app: app, timeout: 15))
        XCTAssertTrue(app.buttons["LogoutButton"].isHittable)
        let settingsTab = app.buttons["SettingsTab"]
        XCTAssertTrue(settingsTab.exists)
        XCTAssertLessThanOrEqual(
            app.buttons["LogoutButton"].frame.maxY + 8,
            settingsTab.frame.minY,
            "The floating tab bar must not obscure the final Settings action."
        )
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "15-live-settings-logout")
    }

    func testLiveNotificationPreviewOptInWhenConfigured() throws {
        let environment = ProcessInfo.processInfo.environment
        guard liveEnvironmentValue("SYNARA_LIVE_NOTIFICATION_PREVIEW_SMOKE", in: environment) == "1" else {
            throw XCTSkip("Set SYNARA_LIVE_NOTIFICATION_PREVIEW_SMOKE=1 for the local notification-preview smoke.")
        }

        let app = XCUIApplication()
        launch(app)
        XCTAssertTrue(app.buttons["SettingsTab"].waitForExistence(timeout: 60))

        if #available(iOS 16.4, *) {
            app.open(URL(string: "synara://settings")!)
        } else {
            tap(app.buttons["SettingsTab"], timeout: 20)
        }

        XCTAssertTrue(app.collectionViews["SettingsScreen"].waitForExistence(timeout: 20))
        tapSettingsElement(app.buttons["NotificationSettingsLink"], app: app, timeout: 10)
        XCTAssertTrue(app.collectionViews["NotificationSettingsScreen"].waitForExistence(timeout: 10))

        let permissionButton = app.buttons["NotificationPermissionButton"]
        XCTAssertTrue(permissionButton.waitForExistence(timeout: 5))
        if permissionButton.label == "Enable Notifications" {
            permissionButton.tap()
            let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")
            let alert = springboard.alerts.firstMatch
            if alert.waitForExistence(timeout: 5) {
                let allow = alert.buttons["Allow"]
                XCTAssertTrue(allow.waitForExistence(timeout: 2))
                allow.tap()
            }
        }

        let toggle = app.switches["LockScreenMessagePreviewsToggle"]
        XCTAssertTrue(toggle.waitForExistence(timeout: 10))
        if (toggle.value as? String) != "1" {
            XCTAssertTrue(toggle.isHittable)
            toggle.coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5)).tap()
        }
        expectation(
            for: NSPredicate(format: "value == %@", "1"),
            evaluatedWith: toggle
        )
        waitForExpectations(timeout: 5)
    }

    func testLiveDeviceVerificationAcrossSimulatorsWhenConfigured() throws {
        let environment = ProcessInfo.processInfo.environment
        guard liveEnvironmentValue("SYNARA_LIVE_VERIFICATION_SMOKE", in: environment) == "1" else {
            throw XCTSkip("Set SYNARA_LIVE_VERIFICATION_SMOKE=1 for the paired live verification smoke.")
        }
        guard let homeserver = liveEnvironmentValue("SYNARA_LIVE_HOMESERVER", in: environment),
              let username = liveEnvironmentValue("SYNARA_LIVE_USERNAME", in: environment),
              let password = liveEnvironmentValue("SYNARA_LIVE_PASSWORD", in: environment),
              let role = liveEnvironmentValue("SYNARA_LIVE_VERIFICATION_ROLE", in: environment),
              ["initiator", "responder"].contains(role)
        else {
            throw XCTSkip("Paired verification needs credentials and an initiator or responder role.")
        }

        let app = XCUIApplication()
        if let screenshotDirectory = liveEnvironmentValue("SYNARA_SCREENSHOT_DIR", in: environment),
           let proofID = liveEnvironmentValue("SYNARA_LIVE_VERIFICATION_PROOF_ID", in: environment)
        {
            let directory = URL(fileURLWithPath: screenshotDirectory, isDirectory: true)
            for suffix in ["sas", "device"] {
                try? FileManager.default.removeItem(
                    at: directory.appendingPathComponent("\(proofID)-\(role)-\(suffix).txt")
                )
            }
        }
        if liveEnvironmentValue("SYNARA_LIVE_VERIFICATION_REUSE_SESSION", in: environment) != "1" {
            app.launchEnvironment["SYNARA_RESET_SESSION_ON_LAUNCH"] = "1"
        }
        launch(app)
        if app.textFields["HomeserverAddressField"].waitForExistence(timeout: 5) {
            loginLive(app: app, homeserver: homeserver, username: username, password: password)
            // Login starts the native crypto restore before the signed-in shell
            // appears. Opening the Settings URL while that work is in flight
            // relaunches the app too early and can return to Choose Server.
            XCTAssertTrue(app.buttons["SettingsTab"].waitForExistence(timeout: 240))
            dismissPasswordSavePromptIfPresent(app: app)
        }

        // Exercise the production URL router to reach Settings. This keeps the
        // paired smoke independent of floating-tab accessibility hit testing.
        // `open` may relaunch the process, so do not carry the one-shot session
        // reset into that second launch.
        app.launchEnvironment.removeValue(forKey: "SYNARA_RESET_SESSION_ON_LAUNCH")
        if #available(iOS 16.4, *) {
            app.open(URL(string: "synara://settings")!)
        } else {
            let settingsTab = app.buttons["SettingsTab"]
            XCTAssertTrue(settingsTab.waitForExistence(timeout: 60))
            tap(settingsTab)
        }
        // Restoring the native crypto store can make the first signed-in task
        // materially slower on a freshly provisioned simulator. The router
        // replays this pending deep link after that production startup work.
        XCTAssertTrue(app.collectionViews["SettingsScreen"].waitForExistence(timeout: 90))

        let coordinatedPeerDeviceID = try coordinateVerificationDevices(
            app: app,
            role: role,
            environment: environment
        )

        if role == "initiator" {
            let targetDeviceId = liveEnvironmentValue(
                "SYNARA_LIVE_VERIFICATION_TARGET_DEVICE_ID",
                in: environment
            ) ?? coordinatedPeerDeviceID
            // The peer can finish login after this screen's initial snapshot.
            // Pull to refresh exercises the production homeserver-backed reload
            // before locating the exact coordinated session.
            let accountScreen = app.collectionViews["AccountSettingsScreen"]
            accountScreen.swipeDown()
            let actionsButton = identifiedElement(in: app, "SessionActionsButton-\(targetDeviceId)")
            XCTAssertTrue(revealSettingsElement(actionsButton, app: app, timeout: 180))
            tap(actionsButton, timeout: 1)
            let verifyButton = app.buttons["VerifySessionButton-\(targetDeviceId)"]
            if verifyButton.waitForExistence(timeout: 3) {
                tap(verifyButton, timeout: 1)
            } else {
                tap(app.buttons["Verify"], timeout: 1)
            }
            XCTAssertTrue(app.staticTexts["Request sent"].waitForExistence(timeout: 30))
        } else {
            navigateBack(app: app)
            tapSettingsElement(identifiedElement(in: app, "SecuritySettingsLink"), app: app, timeout: 20)
            XCTAssertTrue(app.collectionViews["SecuritySettingsScreen"].waitForExistence(timeout: 20))
            let acceptButton = app.buttons["AcceptDeviceVerificationButton"]
            XCTAssertTrue(acceptButton.waitForExistence(timeout: 300))
            tap(acceptButton, timeout: 1)
        }

        // Keep SAS ownership deterministic across real devices. The requester
        // starts the comparison after Ready; the recipient accepts the SAS
        // after Started. RootShell deliberately does not duplicate this action
        // from a state-observer callback.
        let startButton = app.buttons["StartDeviceVerificationSasButton"]
        XCTAssertTrue(startButton.waitForExistence(timeout: 120))
        tap(startButton, timeout: 1)

        let confirmButton = app.buttons["ConfirmDeviceVerificationButton"]
        XCTAssertTrue(confirmButton.waitForExistence(timeout: 120))
        let signature = try verificationSasSignature(in: app)
        XCTAssertFalse(signature.isEmpty, "The comparison must expose non-empty user-readable values.")
        if let screenshotDirectory = liveEnvironmentValue("SYNARA_SCREENSHOT_DIR", in: environment),
           let proofID = liveEnvironmentValue("SYNARA_LIVE_VERIFICATION_PROOF_ID", in: environment)
        {
            let directory = URL(fileURLWithPath: screenshotDirectory, isDirectory: true)
            let ownSignature = directory.appendingPathComponent("\(proofID)-\(role)-sas.txt")
            let peerRole = role == "initiator" ? "responder" : "initiator"
            let peerSignature = directory.appendingPathComponent("\(proofID)-\(peerRole)-sas.txt")
            try signature.write(to: ownSignature, atomically: true, encoding: .utf8)
            let deadline = Date().addingTimeInterval(30)
            while FileManager.default.fileExists(atPath: peerSignature.path) == false, Date() < deadline {
                RunLoop.current.run(until: Date().addingTimeInterval(0.2))
            }
            XCTAssertTrue(FileManager.default.fileExists(atPath: peerSignature.path))
            XCTAssertEqual(try String(contentsOf: peerSignature, encoding: .utf8), signature)
        }
        // Let the large-detent transition and text rendering settle before the
        // visual proof is captured and the comparison is confirmed.
        RunLoop.current.run(until: Date().addingTimeInterval(0.75))
        if let screenshotDirectory = liveEnvironmentValue("SYNARA_SCREENSHOT_DIR", in: environment) {
            try saveScreenshot(
                app: app,
                directory: screenshotDirectory,
                name: "16-live-verification-\(role)-sas"
            )
        }
        tap(confirmButton, timeout: 1)
        XCTAssertTrue(app.staticTexts["Device verified"].waitForExistence(timeout: 90))
        if let screenshotDirectory = liveEnvironmentValue("SYNARA_SCREENSHOT_DIR", in: environment) {
            try saveScreenshot(
                app: app,
                directory: screenshotDirectory,
                name: "17-live-verification-\(role)-done"
            )
        }
        try assertPeerSessionVerified(
            app: app,
            peerDeviceID: coordinatedPeerDeviceID
        )
    }

    private func assertPeerSessionVerified(
        app: XCUIApplication,
        peerDeviceID: String
    ) throws {
        guard let settingsURL = URL(string: "synara://settings") else {
            XCTFail("The Settings deep link must be valid.")
            return
        }
        app.terminate()
        if #available(iOS 16.4, *) {
            app.open(settingsURL)
        } else {
            throw XCTSkip("The durable verification readback needs Settings deep-link support.")
        }
        XCTAssertTrue(app.collectionViews["SettingsScreen"].waitForExistence(timeout: 30))
        tapSettingsElement(identifiedElement(in: app, "AccountSettingsLink"), app: app, timeout: 20)
        let accountScreen = app.collectionViews["AccountSettingsScreen"]
        XCTAssertTrue(accountScreen.waitForExistence(timeout: 20))
        accountScreen.swipeDown()
        let peerTrust = identifiedElement(in: app, "SettingsSessionTrust-\(peerDeviceID)")
        XCTAssertTrue(revealSettingsElement(peerTrust, app: app, timeout: 30))
        XCTAssertEqual(
            peerTrust.label,
            "Verified",
            "The SDK-backed Account snapshot must durably report the paired device verified."
        )
    }

    private func coordinateVerificationDevices(
        app: XCUIApplication,
        role: String,
        environment: [String: String]
    ) throws -> String {
        tapSettingsElement(identifiedElement(in: app, "AccountSettingsLink"), app: app, timeout: 20)
        XCTAssertTrue(app.collectionViews["AccountSettingsScreen"].waitForExistence(timeout: 20))
        let device = identifiedElement(in: app, "SettingsAccountDevice")
        XCTAssertTrue(device.waitForExistence(timeout: 20))
        let prefix = "Device, "
        let label = device.label.trimmingCharacters(in: .whitespacesAndNewlines)
        XCTAssertTrue(label.hasPrefix(prefix), "The account device row must expose its current device ID.")
        let ownDeviceID = String(label.dropFirst(prefix.count))
            .trimmingCharacters(in: .whitespacesAndNewlines)
        XCTAssertFalse(ownDeviceID.isEmpty)

        guard let screenshotDirectory = liveEnvironmentValue("SYNARA_SCREENSHOT_DIR", in: environment),
              let proofID = liveEnvironmentValue("SYNARA_LIVE_VERIFICATION_PROOF_ID", in: environment)
        else {
            throw XCTSkip("Paired verification device coordination needs a proof directory and ID.")
        }
        let directory = URL(fileURLWithPath: screenshotDirectory, isDirectory: true)
        let ownDevice = directory.appendingPathComponent("\(proofID)-\(role)-device.txt")
        let peerRole = role == "initiator" ? "responder" : "initiator"
        let peerDevice = directory.appendingPathComponent("\(proofID)-\(peerRole)-device.txt")
        try ownDeviceID.write(to: ownDevice, atomically: true, encoding: .utf8)

        let deadline = Date().addingTimeInterval(120)
        while FileManager.default.fileExists(atPath: peerDevice.path) == false, Date() < deadline {
            RunLoop.current.run(until: Date().addingTimeInterval(0.2))
        }
        XCTAssertTrue(
            FileManager.default.fileExists(atPath: peerDevice.path),
            "The paired simulator did not publish its device ID."
        )
        let peerDeviceID = try String(contentsOf: peerDevice, encoding: .utf8)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        XCTAssertFalse(peerDeviceID.isEmpty)
        XCTAssertNotEqual(
            peerDeviceID,
            ownDeviceID,
            "Paired simulators must use distinct Matrix device sessions."
        )
        return peerDeviceID
    }

    private func verificationSasSignature(in app: XCUIApplication) throws -> String {
        let firstEmoji = identifiedElement(in: app, "VerificationEmoji-0")
        if firstEmoji.waitForExistence(timeout: 5) {
            let values = (0..<7).map { identifiedElement(in: app, "VerificationEmoji-\($0)") }
            for value in values {
                XCTAssertTrue(value.exists)
                XCTAssertFalse(value.label.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
            }
            return values.map(\.label).joined(separator: "|")
        }

        let firstDecimal = identifiedElement(in: app, "VerificationDecimal-0")
        XCTAssertTrue(firstDecimal.waitForExistence(timeout: 5))
        let values = (0..<3).map { identifiedElement(in: app, "VerificationDecimal-\($0)") }
        for value in values {
            XCTAssertTrue(value.exists)
            XCTAssertFalse(value.label.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
        }
        return values.map(\.label).joined(separator: "|")
    }

    func testMockThreadVisualScreenshotWhenConfigured() throws {
        let environment = ProcessInfo.processInfo.environment
        guard liveEnvironmentValue("SYNARA_MOCK_THREAD_VISUAL_SMOKE", in: environment) == "1" else {
            throw XCTSkip("Set SYNARA_MOCK_THREAD_VISUAL_SMOKE=1 for thread visual smoke screenshots.")
        }
        guard let screenshotDirectory = liveEnvironmentValue("SYNARA_SCREENSHOT_DIR", in: environment) else {
            throw XCTSkip("Thread visual smoke needs screenshot directory.")
        }

        let app = launchRoomApp()
        let threadButton = app.buttons.matching(NSPredicate(format: "label CONTAINS[c] %@", "repl")).firstMatch
        XCTAssertTrue(threadButton.waitForExistence(timeout: 5))
        tap(threadButton)
        XCTAssertTrue(app.staticTexts["ThreadTimelineTitle"].waitForExistence(timeout: 5))
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "05-mock-thread")

        let composer = composerField(in: app)
        XCTAssertTrue(composer.waitForExistence(timeout: 5))
        composer.tap()
        composer.typeText("Thread validation draft")
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "06-mock-thread-typing")
    }

    func testMockAgentVisualScreenshotWhenConfigured() throws {
        let environment = ProcessInfo.processInfo.environment
        guard liveEnvironmentValue("SYNARA_MOCK_AGENT_VISUAL_SMOKE", in: environment) == "1" else {
            throw XCTSkip("Set SYNARA_MOCK_AGENT_VISUAL_SMOKE=1 for agent visual smoke screenshots.")
        }
        guard let screenshotDirectory = liveEnvironmentValue("SYNARA_SCREENSHOT_DIR", in: environment) else {
            throw XCTSkip("Agent visual smoke needs screenshot directory.")
        }

        let app = launchAgentCardRoomApp()
        XCTAssertTrue(app.staticTexts["Deploy to Production"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["AgentCardAction-approve-deploy"].exists)
        XCTAssertTrue(app.buttons["AgentCardAction-reject-deploy"].exists)
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "07-mock-agent-approval")
    }

    func testMockRoomsVisualScreenshotsWhenConfigured() throws {
        let environment = ProcessInfo.processInfo.environment
        guard liveEnvironmentValue("SYNARA_MOCK_ROOMS_VISUAL_SMOKE", in: environment) == "1" else {
            throw XCTSkip("Set SYNARA_MOCK_ROOMS_VISUAL_SMOKE=1 for room visual smoke screenshots.")
        }
        guard let screenshotDirectory = liveEnvironmentValue("SYNARA_SCREENSHOT_DIR", in: environment) else {
            throw XCTSkip("Room visual smoke needs screenshot directory.")
        }

        let app = launchSignedInRoomsApp()
        XCTAssertTrue(app.collectionViews["RoomList"].waitForExistence(timeout: 5))
        XCTAssertTrue(app.buttons["RoomRow-!project:matrix.org"].waitForExistence(timeout: 5))
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "01-mock-room-list")

        tap(app.buttons["RoomRow-!project:matrix.org"], timeout: 5)
        XCTAssertTrue(timelineViewport(in: app).waitForExistence(timeout: 5))
        XCTAssertTrue(composerField(in: app).waitForExistence(timeout: 5))
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "02-mock-room-timeline")

        let composer = composerField(in: app)
        composer.tap()
        composer.typeText("Sounds good. I'll prep some notes before our sync.")
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "03-mock-composer-typing")

        tap(app.buttons["AttachmentButton"], timeout: 5)
        XCTAssertTrue(app.otherElements["AttachmentOptionsSheet"].waitForExistence(timeout: 5))
        try saveScreenshot(app: app, directory: screenshotDirectory, name: "04-mock-attachment-sheet")
    }

    private func launchApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        launch(app)
        return app
    }

    private func launchRoomApp(
        readMarkerEventID: String? = nil,
        largeTimelineCount: Int? = nil,
        roomNotes: Bool = false
    ) -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_ID"] = "!project:matrix.org"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_TITLE"] = "Project"
        if let readMarkerEventID {
            app.launchEnvironment["SYNARA_UI_TEST_READ_MARKER_EVENT_ID"] = readMarkerEventID
        }
        if let largeTimelineCount {
            app.launchEnvironment["SYNARA_UI_TEST_LARGE_TIMELINE"] = "1"
            app.launchEnvironment["SYNARA_UI_TEST_LARGE_TIMELINE_COUNT"] = "\(largeTimelineCount)"
        }
        if roomNotes {
            app.launchEnvironment["SYNARA_UI_TEST_ROOM_NOTES"] = "1"
        }
        launch(app)
        return app
    }

    private func launchSignedInRoomsApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SIGNED_IN"] = "1"
        launch(app)
        return app
    }

    private func launchLargeRoomsApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SIGNED_IN"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_LARGE_ROOMS"] = "1"
        launch(app)
        return app
    }

    private func launchFilteredRoomsApp(query: String) -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SIGNED_IN"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_SEARCH"] = query
        launch(app)
        return app
    }

    private func launchLargeTimelineApp(count: Int = 1000, scenario: String? = nil) -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_ID"] = "!large:matrix.org"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_TITLE"] = "Large Timeline"
        app.launchEnvironment["SYNARA_UI_TEST_LARGE_TIMELINE"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_LARGE_TIMELINE_COUNT"] = "\(count)"
        if let scenario {
            app.launchEnvironment["SYNARA_UI_TEST_VIEWPORT_SCENARIO"] = scenario
        }
        launch(app)
        return app
    }

    private func launchSignedInNotificationsApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SIGNED_IN"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SELECTED_TAB"] = "notifications"
        launch(app)
        return app
    }

    private func launchSignedInSettingsApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SIGNED_IN"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SELECTED_TAB"] = "settings"
        launch(app)
        return app
    }

    private func launchEncryptedSettingsApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SIGNED_IN"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SELECTED_TAB"] = "settings"
        app.launchEnvironment["SYNARA_UI_TEST_ENCRYPTED_TIMELINE"] = "1"
        launch(app)
        return app
    }

    private func launchEncryptedRoomApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_ID"] = "!encrypted:matrix.org"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_TITLE"] = "Secret"
        app.launchEnvironment["SYNARA_UI_TEST_ENCRYPTED_TIMELINE"] = "1"
        launch(app)
        return app
    }

    private func launchInviteApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SIGNED_IN"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_INVITE"] = "1"
        launch(app)
        return app
    }

    private func launchRoomManagementSheetApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SIGNED_IN"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_MANAGEMENT_SHEET"] = "1"
        launch(app)
        return app
    }

    private func launchLaterApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SIGNED_IN"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_SELECTED_TAB"] = "later"
        app.launchEnvironment["SYNARA_UI_TEST_LATER_ITEMS"] = "1"
        launch(app)
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
        launch(app)
        return app
    }

    private func launchAgentApprovalPromptRoomApp() -> XCUIApplication {
        let app = XCUIApplication()
        app.launchEnvironment["SYNARA_UI_TESTS"] = "1"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_ID"] = "!agent:matrix.org"
        app.launchEnvironment["SYNARA_UI_TEST_ROOM_TITLE"] = "Agent"
        app.launchEnvironment["SYNARA_UI_TEST_AGENT_APPROVAL_PROMPT"] = "1"
        launch(app)
        return app
    }

    private func launch(_ app: XCUIApplication) {
        app.launchEnvironment["SYNARA_DISABLE_ANIMATIONS"] = "1"
        let stableArguments = [
            "-ApplePersistenceIgnoreState",
            "YES",
            "-UIPreferredContentSizeCategoryName",
            "UICTContentSizeCategoryM",
        ]
        for argument in stableArguments {
            if app.launchArguments.contains(argument) == false {
                app.launchArguments.append(argument)
            }
        }
        app.launch()
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
            ?? liveEnvironmentValue("SYNARA_LIVE_ROOM_ID", in: environment)
        {
            return roomID
        }

        let alias = liveEnvironmentValue("SYNARA_LIVE_AGENT_ROOM_ALIAS", in: environment)
            ?? liveEnvironmentValue("SYNARA_LIVE_ROOM_ALIAS", in: environment)
            ?? "#test-e2e-room:matrix.example.com"
        return try client.resolveRoomAlias(alias)
    }

    private func liveEncryptedRoomID(environment: [String: String], homeserver: String, username: String, password: String) throws -> String {
        if let roomID = liveEnvironmentValue("SYNARA_LIVE_E2EE_ROOM_ID", in: environment)
            ?? liveEnvironmentValue("SYNARA_LIVE_ROOM_ID", in: environment)
        {
            return roomID
        }

        let liveClient = try MatrixLiveTestClient.login(
            homeserver: homeserver,
            username: username,
            password: password
        )
        let alias = liveEnvironmentValue("SYNARA_LIVE_E2EE_ROOM_ALIAS", in: environment)
            ?? "#test-e2e-room:matrix.example.com"
        return try liveClient.resolveRoomAlias(alias)
    }

    private func composerField(in app: XCUIApplication) -> XCUIElement {
        identifiedElement(in: app, "ComposerTextField")
    }

    private func identifiedElement(in app: XCUIApplication, _ identifier: String) -> XCUIElement {
        app.descendants(matching: .any)
            .matching(identifier: identifier)
            .firstMatch
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
        let settingsList: XCUIElement
        if app.collectionViews["AccountSettingsScreen"].exists {
            settingsList = app.collectionViews["AccountSettingsScreen"]
        } else if app.collectionViews["SettingsScreen"].exists {
            settingsList = app.collectionViews["SettingsScreen"]
        } else {
            settingsList = app.collectionViews.firstMatch
        }

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

    private func navigateBack(app: XCUIApplication) {
        let backButton = app.navigationBars.buttons.element(boundBy: 0)
        XCTAssertTrue(backButton.waitForExistence(timeout: 5))
        backButton.tap()
        XCTAssertTrue(app.buttons["AccountSettingsLink"].waitForExistence(timeout: 5))
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
        guard notNow.waitForExistence(timeout: 3) else {
            return
        }

        let deadline = Date().addingTimeInterval(10)
        while notNow.exists && Date() < deadline {
            if notNow.isHittable {
                notNow.tap()
            } else {
                notNow.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.5))
        }

        XCTAssertFalse(
            notNow.exists,
            "The iOS password-save prompt must be fully dismissed before app interactions continue."
        )
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

    private func timelineViewport(in app: XCUIApplication) -> XCUIElement {
        app.descendants(matching: .any).matching(identifier: "TimelineList").firstMatch
    }

    private func viewportDiagnostics(_ viewport: XCUIElement) -> [String: String] {
        guard let value = viewport.value as? String else {
            return [:]
        }
        return Dictionary(uniqueKeysWithValues: value.split(separator: ";").compactMap { field in
            let parts = field.split(separator: "=", maxSplits: 1).map(String.init)
            guard parts.count == 2 else {
                return nil
            }
            return (parts[0], parts[1])
        })
    }

    private func waitForViewportDiagnostics(
        _ viewport: XCUIElement,
        containing expectedValue: String,
        timeout: TimeInterval
    ) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if (viewport.value as? String)?.contains(expectedValue) == true {
                return true
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.2))
        }
        return (viewport.value as? String)?.contains(expectedValue) == true
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

    private enum TimelineSwipeDirection {
        case up
        case down
    }

    private func waitForTimelineElement(
        _ element: XCUIElement,
        app: XCUIApplication,
        timeout: TimeInterval,
        preferredSwipe: TimelineSwipeDirection = .up
    ) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        let timeline = timelineViewport(in: app)
        var nextSwipe = preferredSwipe

        while Date() < deadline {
            if element.exists {
                return true
            }
            if timeline.exists {
                switch nextSwipe {
                case .up:
                    timeline.swipeUp()
                    nextSwipe = .down
                case .down:
                    timeline.swipeDown()
                    nextSwipe = .up
                }
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.5))
        }

        return element.exists
    }

    private func waitForAnyElement(_ elements: [XCUIElement], timeout: TimeInterval) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)

        while Date() < deadline {
            if elements.contains(where: \.exists) {
                return true
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.25))
        }

        return elements.contains(where: \.exists)
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
               value.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
            {
                return true
            }
            RunLoop.current.run(until: Date().addingTimeInterval(0.25))
        }
        return (element.value as? String)?.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
    }

    private func saveScreenshot(app: XCUIApplication, directory: String, name: String) throws {
        let directoryURL = URL(fileURLWithPath: directory, isDirectory: true)
        try FileManager.default.createDirectory(at: directoryURL, withIntermediateDirectories: true)
        try app.screenshot().pngRepresentation.write(to: directoryURL.appendingPathComponent("\(name).png"))
    }
}

private final class MatrixLiveTestClient {
    private let homeserverURL: URL
    private let accessToken: String
    private let userID: String

    private init(homeserverURL: URL, accessToken: String, userID: String) {
        self.homeserverURL = homeserverURL
        self.accessToken = accessToken
        self.userID = userID
    }

    static func login(homeserver: String, username: String, password: String) throws -> MatrixLiveTestClient {
        guard let homeserverURL = URL(string: homeserver.hasPrefix("http") ? homeserver : "https://\(homeserver)") else {
            throw LiveMatrixError.invalidHomeserver
        }

        let requestBody: [String: Any] = [
            "type": "m.login.password",
            "identifier": [
                "type": "m.id.user",
                "user": username,
            ],
            "password": password,
            "initial_device_display_name": "Synara iOS UI smoke",
        ]

        var request = URLRequest(url: homeserverURL.appendingMatrixPath(["client", "v3", "login"]))
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONSerialization.data(withJSONObject: requestBody)

        let data = try perform(request).data
        guard let object = try JSONSerialization.jsonObject(with: data) as? [String: Any],
              let token = object["access_token"] as? String,
              let userID = object["user_id"] as? String
        else {
            throw LiveMatrixError.invalidResponse
        }

        return MatrixLiveTestClient(homeserverURL: homeserverURL, accessToken: token, userID: userID)
    }

    func replaceRoomNotesRoom(roomID: String, items: [String: [String: Any]]) throws {
        var content = try roomNotesContent()
        var rooms = content["rooms"] as? [String: Any] ?? [:]
        rooms[roomID] = ["items": items]
        content["version"] = 1
        content["rooms"] = rooms
        try setRoomNotesContent(content)
    }

    func removeRoomNotesRoom(roomID: String) throws {
        var content = try roomNotesContent()
        var rooms = content["rooms"] as? [String: Any] ?? [:]
        rooms.removeValue(forKey: roomID)
        content["version"] = 1
        content["rooms"] = rooms
        try setRoomNotesContent(content)
    }

    func waitForRoomNote(roomID: String, body: String, timeout: TimeInterval) -> Bool {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if (try? hasRoomNote(roomID: roomID, body: body)) == true {
                return true
            }
            RunLoop.current.run(until: Date().addingTimeInterval(1))
        }
        return (try? hasRoomNote(roomID: roomID, body: body)) == true
    }

    private func hasRoomNote(roomID: String, body: String) throws -> Bool {
        let content = try roomNotesContent()
        guard let rooms = content["rooms"] as? [String: Any],
              let room = rooms[roomID] as? [String: Any],
              let items = room["items"] as? [String: Any]
        else {
            return false
        }
        return items.values.contains { value in
            (value as? [String: Any])?["body"] as? String == body
        }
    }

    private func roomNotesContent() throws -> [String: Any] {
        do {
            let response = try authenticatedRequest(
                method: "GET",
                path: ["client", "v3", "user", userID, "account_data", "in.synara.room_notes"],
                body: nil
            )
            guard let content = try JSONSerialization.jsonObject(with: response.data) as? [String: Any] else {
                throw LiveMatrixError.invalidResponse
            }
            return content
        } catch LiveMatrixError.httpStatus(404) {
            return ["version": 1, "rooms": [String: Any]()]
        }
    }

    private func setRoomNotesContent(_ content: [String: Any]) throws {
        _ = try authenticatedRequest(
            method: "PUT",
            path: ["client", "v3", "user", userID, "account_data", "in.synara.room_notes"],
            body: content
        )
    }

    func resolveRoomAlias(_ alias: String) throws -> String {
        let response = try authenticatedRequest(
            method: "GET",
            path: ["client", "v3", "directory", "room", alias],
            body: nil
        )
        guard let object = try JSONSerialization.jsonObject(with: response.data) as? [String: Any],
              let roomID = object["room_id"] as? String
        else {
            throw LiveMatrixError.invalidResponse
        }
        return roomID
    }

    func logout() throws {
        _ = try authenticatedRequest(
            method: "POST",
            path: ["client", "v3", "logout"],
            body: [:]
        )
    }

    func sendRoomMessage(roomID: String, body: String) throws -> String {
        let content: [String: Any] = [
            "msgtype": "m.text",
            "body": body,
        ]

        let response = try authenticatedRequest(
            method: "PUT",
            path: ["client", "v3", "rooms", roomID, "send", "m.room.message", UUID().uuidString],
            body: content
        )
        guard let object = try JSONSerialization.jsonObject(with: response.data) as? [String: Any],
              let eventID = object["event_id"] as? String
        else {
            throw LiveMatrixError.invalidResponse
        }
        return eventID
    }

    func createPrivateRoom(name: String) throws -> String {
        let response = try authenticatedRequest(
            method: "POST",
            path: ["client", "v3", "createRoom"],
            body: [
                "name": name,
                "preset": "private_chat",
                "visibility": "private",
            ]
        )
        guard let object = try JSONSerialization.jsonObject(with: response.data) as? [String: Any],
              let roomID = object["room_id"] as? String
        else {
            throw LiveMatrixError.invalidResponse
        }
        return roomID
    }

    func leaveRoom(roomID: String) throws {
        _ = try authenticatedRequest(
            method: "POST",
            path: ["client", "v3", "rooms", roomID, "leave"],
            body: [:]
        )
    }

    func cleanupDisposableRooms(namePrefixes: [String]) throws {
        guard namePrefixes.isEmpty == false else {
            return
        }

        let response = try authenticatedRequest(
            method: "GET",
            path: ["client", "v3", "joined_rooms"],
            body: nil
        )
        guard let object = try JSONSerialization.jsonObject(with: response.data) as? [String: Any],
              let roomIDs = object["joined_rooms"] as? [String]
        else {
            throw LiveMatrixError.invalidResponse
        }

        for roomID in roomIDs {
            guard let roomName = try? joinedRoomName(roomID: roomID),
                  namePrefixes.contains(where: roomName.hasPrefix)
            else {
                continue
            }
            try leaveRoom(roomID: roomID)
        }
    }

    private func joinedRoomName(roomID: String) throws -> String {
        let response = try authenticatedRequest(
            method: "GET",
            path: ["client", "v3", "rooms", roomID, "state", "m.room.name"],
            body: nil
        )
        guard let object = try JSONSerialization.jsonObject(with: response.data) as? [String: Any],
              let roomName = object["name"] as? String
        else {
            throw LiveMatrixError.invalidResponse
        }
        return roomName
    }

    func waitForMessageContent(roomID: String, body: String, timeout: TimeInterval) -> [String: Any]? {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if let content = try? messageContent(roomID: roomID, body: body) {
                return content
            }
            RunLoop.current.run(until: Date().addingTimeInterval(1))
        }
        return try? messageContent(roomID: roomID, body: body)
    }

    private func messageContent(roomID: String, body: String) throws -> [String: Any]? {
        let response = try authenticatedRequest(
            method: "GET",
            path: ["client", "v3", "rooms", roomID, "messages"],
            queryItems: [
                URLQueryItem(name: "dir", value: "b"),
                URLQueryItem(name: "limit", value: "40"),
            ],
            body: nil
        )
        guard let object = try JSONSerialization.jsonObject(with: response.data) as? [String: Any],
              let chunk = object["chunk"] as? [[String: Any]]
        else {
            throw LiveMatrixError.invalidResponse
        }
        return chunk.compactMap { $0["content"] as? [String: Any] }.first { content in
            content["body"] as? String == body
        }
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
                    "prompt": "approve live smoke",
                ],
            ],
        ]
        let bodyData = try JSONSerialization.data(withJSONObject: [
            "hermes": true,
            "payload": agentPayload,
        ])
        let body = String(data: bodyData, encoding: .utf8) ?? title

        let content: [String: Any] = [
            "msgtype": "m.notice",
            "body": body,
            "in.synara.agent": agentPayload,
        ]

        let response = try authenticatedRequest(
            method: "PUT",
            path: ["client", "v3", "rooms", roomID, "send", "m.room.message", UUID().uuidString],
            body: content
        )
        guard let object = try JSONSerialization.jsonObject(with: response.data) as? [String: Any],
              let eventID = object["event_id"] as? String
        else {
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
                URLQueryItem(name: "limit", value: "40"),
            ],
            body: nil
        )
        guard let object = try JSONSerialization.jsonObject(with: response.data) as? [String: Any],
              let chunk = object["chunk"] as? [[String: Any]]
        else {
            throw LiveMatrixError.invalidResponse
        }

        return chunk.contains { event in
            guard let content = event["content"] as? [String: Any],
                  let action = content["in.synara.agent.action"] as? [String: Any]
            else {
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
                  let data
            else {
                result = .failure(LiveMatrixError.invalidResponse)
                return
            }
            guard (200 ... 299).contains(http.statusCode) else {
                result = .failure(LiveMatrixError.httpStatus(http.statusCode))
                return
            }
            result = .success((data, http.statusCode))
        }.resume()

        guard semaphore.wait(timeout: .now() + 30) == .success else {
            throw LiveMatrixError.timeout
        }

        switch result {
        case let .success(value):
            return value
        case let .failure(error):
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
