import XCTest
@testable import Synara

final class NotificationPreviewSupportTests: XCTestCase {
    func testPreviewPayloadParserReadsFlatAndNestedRouteFields() throws {
        let payload = try XCTUnwrap(
            SynaraNotificationPreviewPayloadParser.payload(
                from: [
                    "aps": [
                        "category": "synara.agent-approval"
                    ],
                    "synara": [
                        "kind": "agent-approval",
                        "room_id": " !room:matrix.example.com ",
                        "event_id": " $event:matrix.example.com "
                    ]
                ]
            )
        )

        XCTAssertEqual(payload.roomID, "!room:matrix.example.com")
        XCTAssertEqual(payload.eventID, "$event:matrix.example.com")
        XCTAssertTrue(payload.isAgentApproval)
    }

    func testPreviewPreferenceDefaultsOffAndPersistsOptIn() {
        let suiteName = "synara.notification-preview.test.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer {
            defaults.removePersistentDomain(forName: suiteName)
        }

        XCTAssertFalse(SynaraNotificationPreviewPreference.isEnabled(defaults: defaults))

        defaults.set(true, forKey: SynaraSharedConstants.lockScreenMessagePreviewsKey)

        XCTAssertTrue(SynaraNotificationPreviewPreference.isEnabled(defaults: defaults))
    }

    func testPreviewComposerBuildsBoundedCleartextPreview() throws {
        let body = String(repeating: "message ", count: 80)
        let preview = try XCTUnwrap(
            SynaraMatrixEventPreviewComposer.preview(
                from: SynaraMatrixEventPreviewInput(
                    senderID: "@alice:matrix.example.com",
                    body: body,
                    messageType: "m.text"
                )
            )
        )

        XCTAssertEqual(preview.title, "alice")
        XCTAssertLessThanOrEqual(preview.body.count, 240)
        XCTAssertTrue(preview.body.hasSuffix("..."))
    }

    func testPreviewComposerLeavesEncryptedEventsGeneric() {
        XCTAssertNil(
            SynaraMatrixEventPreviewComposer.preview(
                from: SynaraMatrixEventPreviewInput(
                    eventType: "m.room.encrypted",
                    senderID: "@alice:matrix.example.com",
                    body: "ciphertext",
                    messageType: nil
                )
            )
        )
    }
}
