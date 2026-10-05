import XCTest
@testable import Synara

final class NotificationDiagnosticReportTests: XCTestCase {
    func testReportRetainsSafeEvidenceAndDropsUnknownArbitraryStageText() throws {
        let runID = UUID()
        let now = Date(timeIntervalSince1970: 2_000_000)
        let text = SynaraNotificationDiagnosticReport.text(
            context: context,
            entries: [
                .init(id: UUID(), runID: runID, timestamp: now, stage: "received"),
                .init(id: UUID(), runID: runID, timestamp: now, stage: "core-decryption-unavailable"),
                .init(id: UUID(), runID: nil, timestamp: now, stage: "token-secret @private:example.org")
            ], now: now
        )
        let json = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(text.utf8)) as? [String: Any])
        XCTAssertEqual(json["schemaVersion"] as? Int, 1)
        XCTAssertEqual(json["extensionReceiptCount"] as? Int, 1)
        XCTAssertEqual((json["records"] as? [[String: Any]])?.count, 2)
        XCTAssertTrue(text.contains(runID.uuidString))
        XCTAssertTrue(text.contains("core-decryption-unavailable"))
        XCTAssertFalse(text.contains("token-secret"))
        XCTAssertFalse(text.contains("example.org"))
        XCTAssertFalse(text.contains("homeserver"))
        XCTAssertFalse(text.contains("storeRoot"))
    }

    func testReportBoundsRecordsAndIncludesContextWithoutExtensionActivity() throws {
        let entries = (0..<300).map { _ in
            SynaraNotificationDiagnosticEntry(id: UUID(), runID: nil, timestamp: Date(), stage: "foreground-received")
        }
        let text = SynaraNotificationDiagnosticReport.text(context: context, entries: entries)
        let json = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(text.utf8)) as? [String: Any])
        XCTAssertEqual(json["extensionReceiptCount"] as? Int, 0)
        XCTAssertEqual((json["records"] as? [[String: Any]])?.count, 256)
        let snapshot = try XCTUnwrap(json["context"] as? [String: Any])
        XCTAssertEqual(snapshot["sharedStoreReady"] as? Bool, false)
        XCTAssertEqual(snapshot["pushRegistered"] as? Bool, true)
        XCTAssertEqual(snapshot["showPreviewsSetting"] as? String, "when-authenticated")
    }

    func testPublicVersionRejectsUnexpectedPrivateText() {
        XCTAssertEqual(SynaraNotificationDiagnosticReport.publicVersion("2.1.43"), "2.1.43")
        XCTAssertNil(SynaraNotificationDiagnosticReport.publicVersion("https://private.example.org"))
        XCTAssertNil(SynaraNotificationDiagnosticReport.publicVersion(String(repeating: "a", count: 41)))
        XCTAssertNil(SynaraNotificationDiagnosticReport.publicVersion(123))
    }

    private var context: SynaraNotificationDiagnosticReport.Context {
        .init(
            appVersion: "2.1.43", appBuild: "2.1.43",
            messagePreviewsEnabled: true, urgentApprovalsEnabled: true,
            appGroupAvailable: true, sharedStoreReady: false,
            pushRegistrationAvailable: true, pushRegistered: true, pushGatewayConfigured: true,
            notificationAuthorization: "authorized", alertSetting: "enabled", soundSetting: "enabled",
            lockScreenSetting: "enabled", notificationCenterSetting: "enabled",
            criticalAlertSetting: "not-supported", timeSensitiveSetting: "enabled",
            showPreviewsSetting: "when-authenticated",
            criticalAlertsBuildEnabled: false, notificationFilteringBuildEnabled: false
        )
    }
}
