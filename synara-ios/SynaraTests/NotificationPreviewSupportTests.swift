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

    func testTimeSensitiveApprovalPreferenceDefaultsOffAndPersistsOptIn() {
        let suiteName = "synara.agent-approval-alert.test.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }

        XCTAssertFalse(SynaraTimeSensitiveAgentApprovalPreference.isEnabled(defaults: defaults))
        defaults.set(true, forKey: SynaraSharedConstants.timeSensitiveAgentApprovalsKey)
        XCTAssertTrue(SynaraTimeSensitiveAgentApprovalPreference.isEnabled(defaults: defaults))
    }

    func testAgentApprovalFreshnessFailsClosedAtFiveMinuteBoundary() {
        let now = Date(timeIntervalSince1970: 2_000_000)
        let nowMS = UInt64(now.timeIntervalSince1970 * 1_000)

        XCTAssertTrue(
            SynaraAgentApprovalFreshness.isFresh(
                originServerTimestampMS: nowMS - (5 * 60 * 1_000 - 1),
                now: now
            )
        )
        XCTAssertFalse(
            SynaraAgentApprovalFreshness.isFresh(
                originServerTimestampMS: nowMS - 5 * 60 * 1_000,
                now: now
            )
        )
        XCTAssertFalse(
            SynaraAgentApprovalFreshness.isFresh(originServerTimestampMS: 0, now: now)
        )
    }

    func testAgentApprovalFreshnessRejectsImplausibleFutureTimestamp() {
        let now = Date(timeIntervalSince1970: 2_000_000)
        let nowMS = UInt64(now.timeIntervalSince1970 * 1_000)

        XCTAssertFalse(
            SynaraAgentApprovalFreshness.isFresh(
                originServerTimestampMS: nowMS + 60_001,
                now: now
            )
        )
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

    func testAppGroupUnavailableStageIsAFixedDecodableCode() throws {
        XCTAssertEqual(
            SynaraNotificationDiagnostics.Stage.appGroupUnavailable.rawValue,
            "app-group-unavailable"
        )
        let suiteName = "synara.notification-diagnostics.app-group.test.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let runID = UUID()

        SynaraNotificationDiagnostics.record(.appGroupUnavailable, runID: runID, defaults: defaults)

        let entries = SynaraNotificationDiagnostics.entries(defaults: defaults)
        XCTAssertEqual(entries.count, 1)
        XCTAssertEqual(entries.first?.runID, runID)
        XCTAssertEqual(entries.first?.stage, "app-group-unavailable")
        XCTAssertEqual(
            SynaraNotificationDiagnostics.Stage(rawValue: entries.first?.stage ?? ""),
            .appGroupUnavailable
        )
    }

    func testNotificationDiagnosticsAreBoundedStageOnlyAndClearable() throws {
        let suiteName = "synara.notification-diagnostics.test.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let start = Date(timeIntervalSince1970: 2_000_000)
        let runID = UUID()

        for offset in 0 ..< SynaraNotificationDiagnostics.maximumEntries + 5 {
            SynaraNotificationDiagnostics.record(
                offset.isMultiple(of: 2) ? .received : .payloadInvalid,
                runID: runID,
                now: start.addingTimeInterval(TimeInterval(offset)),
                defaults: defaults
            )
        }

        let entries = SynaraNotificationDiagnostics.entries(defaults: defaults)
        XCTAssertEqual(entries.count, SynaraNotificationDiagnostics.maximumEntries)
        XCTAssertEqual(entries.first?.timestamp, start.addingTimeInterval(5))
        XCTAssertEqual(entries.last?.stage, SynaraNotificationDiagnostics.Stage.received.rawValue)
        XCTAssertTrue(entries.allSatisfy { $0.runID == runID })
        XCTAssertTrue(entries.allSatisfy { SynaraNotificationDiagnostics.Stage(rawValue: $0.stage) != nil })

        let encoded = try XCTUnwrap(
            defaults.data(forKey: SynaraSharedConstants.notificationDiagnosticsKey)
        )
        let storedText = String(decoding: encoded, as: UTF8.self)
        XCTAssertFalse(storedText.contains("room_id"))
        XCTAssertFalse(storedText.contains("event_id"))
        XCTAssertFalse(storedText.contains("body"))
        XCTAssertFalse(storedText.contains("token"))
        XCTAssertFalse(storedText.contains("matrix.org"))

        SynaraNotificationDiagnostics.clear(defaults: defaults)
        XCTAssertTrue(SynaraNotificationDiagnostics.entries(defaults: defaults).isEmpty)
    }

    func testNotificationDiagnosticsDecodeRecordsFromBeforeCorrelationIDs() throws {
        let suiteName = "synara.notification-diagnostics.legacy.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let legacyEntry = LegacyNotificationDiagnosticEntry(
            id: UUID(),
            timestamp: Date(timeIntervalSince1970: 2_000_000),
            stage: SynaraNotificationDiagnostics.Stage.received.rawValue
        )
        defaults.set(
            try JSONEncoder().encode([legacyEntry]),
            forKey: SynaraSharedConstants.notificationDiagnosticsKey
        )

        let decoded = try XCTUnwrap(SynaraNotificationDiagnostics.entries(defaults: defaults).first)
        XCTAssertEqual(decoded.id, legacyEntry.id)
        XCTAssertNil(decoded.runID)
        XCTAssertEqual(decoded.stage, legacyEntry.stage)
    }

    func testEmptyDeadlineExpirationDoesNotCreateUncorrelatedDiagnostic() throws {
        let suiteName = "synara.notification-diagnostics.empty-deadline.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }

        SynaraNotificationDiagnostics.recordDeadlineDeliveries(for: [], defaults: defaults)

        XCTAssertTrue(SynaraNotificationDiagnostics.entries(defaults: defaults).isEmpty)
    }

    func testDeadlineDiagnosticsAreCorrelatedOnlyToWinningRequestIDs() throws {
        let suiteName = "synara.notification-diagnostics.deadline.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        defer { defaults.removePersistentDomain(forName: suiteName) }
        let requestID = UUID()

        SynaraNotificationDiagnostics.recordDeadlineDeliveries(
            for: [requestID],
            defaults: defaults
        )

        let entries = SynaraNotificationDiagnostics.entries(defaults: defaults)
        XCTAssertEqual(entries.map(\.runID), [requestID, requestID])
        XCTAssertEqual(
            entries.map(\.stage),
            [
                SynaraNotificationDiagnostics.Stage.systemDeadline.rawValue,
                SynaraNotificationDiagnostics.Stage.delivered.rawValue
            ]
        )
    }
}

private struct LegacyNotificationDiagnosticEntry: Codable {
    let id: UUID
    let timestamp: Date
    let stage: String
}
