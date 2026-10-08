import XCTest
import UserNotifications
@testable import Synara

final class NotificationPreviewSupportTests: XCTestCase {
    func testPreviewReferenceRejectsConflictsWithoutFlatteningThemAway() throws {
        XCTAssertEqual(
            try SynaraNotificationPreviewPayloadParser.parse(["room_id": "!Nhcu5BS-UMnFX7hBVfVSoXiD7OgH6iRT-xyIuqDnpYQ", "event_id": "$event"]).get().roomID,
            "!Nhcu5BS-UMnFX7hBVfVSoXiD7OgH6iRT-xyIuqDnpYQ"
        )
        let same: [AnyHashable: Any] = [
            "room_id": "!room:example.org", "event_id": "$event",
            "synara.room_id": "!room:example.org",
            "synara": ["room_id": " !room:example.org ", "event_id": "$event"]
        ]
        XCTAssertEqual(try SynaraNotificationPreviewPayloadParser.parse(same).get().roomID, "!room:example.org")
        for payload: [AnyHashable: Any] in [
            ["room_id": "!one:example.org", "event_id": "$event", "synara": ["room_id": "!two:example.org"]],
            ["room_id": "!room:example.org", "synara.event_id": "$one", "synara": ["event_id": "$two"]],
            ["room_id": "!room:example.org", "event_id": 42, "synara": ["event_id": "$event"]]
        ] {
            XCTAssertNil(SynaraNotificationPreviewPayloadParser.payload(from: payload))
        }
    }

    func testPayloadFailuresExposeOnlyFixedReasonsAndBoundTraversal() {
        func stage(_ payload: [AnyHashable: Any]) -> SynaraNotificationDiagnostics.Stage? {
            guard case .failure(let failure) = SynaraNotificationPreviewPayloadParser.parse(payload) else { return nil }
            return failure.stage
        }
        XCTAssertEqual(stage(["aps": ["badge": 3]]), .payloadNoEventReference)
        XCTAssertEqual(stage(["event_id": "$event"]), .payloadMissingRoom)
        XCTAssertEqual(stage(["room_id": "!room:example.org"]), .payloadMissingEvent)
        XCTAssertEqual(stage(["room_id": "!room:example.org", "event_id": 42]), .payloadIdentifierTypeInvalid)
        XCTAssertEqual(stage(["room_id": "!room:example.org", "event_id": "event"]), .payloadIdentifiersInvalid)
        XCTAssertEqual(stage(["room_id": "!room:example.org", "event_id": "$event\u{202e}"]), .payloadIdentifiersInvalid)
        var nested: [AnyHashable: Any] = ["room_id": "!room:example.org", "event_id": "$event"]
        for _ in 0..<14 { nested = ["wrapper": nested] }
        XCTAssertEqual(stage(nested), .payloadComplexityExceeded)
        var wide: [AnyHashable: Any] = ["room_id": "!room:example.org", "event_id": "$event"]
        for index in 0..<513 { wide["field\(index)"] = false }
        XCTAssertEqual(stage(wide), .payloadComplexityExceeded)
    }

    func testTypedCoreFailureStagesKeepInitializationAndFetchCausesDistinct() {
        for (code, stage): (String, SynaraNotificationDiagnostics.Stage) in [
            ("p4-s11-nse-client-init-failed", .coreClientInitFailed),
            ("p4-s11-nse-event-fetch-failed", .coreFetchFailed),
            ("p4-s11-nse-session-rejected", .coreSessionRejected),
            ("p4-s11-nse-network-timeout", .coreNetworkTimeout),
            ("p4-s11-nse-store-lock-failed", .coreStoreLockFailed),
            ("p4-s11-nse-sliding-sync-version-missing", .coreSlidingSyncVersionMissing),
            ("p4-s11-nse-invalid-response", .coreInvalidResponse)
        ] {
            XCTAssertEqual(SynaraNotificationDiagnostics.previewFailureStage(coreCode: code), stage)
        }
        XCTAssertEqual(SynaraNotificationDiagnostics.previewFailureStage(coreCode: "token-secret"), .coreResolutionFailed)
    }

    func testSuppressionRequiresExplicitPolicyResultAndSupportedBuild() {
        XCTAssertNil(SynaraAgentNotificationSuppressionPolicy.content(for: .agentPolicyFiltered, filteringEnabled: false))
        for stage in [SynaraNotificationDiagnostics.Stage.coreFetchFailed, .coreDecryptionUnavailable, .coreEventFiltered, .resolvedApproval] {
            XCTAssertNil(SynaraAgentNotificationSuppressionPolicy.content(for: stage, filteringEnabled: true))
        }
        let content = SynaraAgentNotificationSuppressionPolicy.content(for: .agentPolicyFiltered, filteringEnabled: true)
        XCTAssertEqual(content?.title, "")
        XCTAssertEqual(content?.body, "")
        XCTAssertNil(content?.sound)
        XCTAssertEqual(SynaraNotificationDiagnostics.previewFailureStage(coreCode: "p4-s11-nse-agent-policy-filtered"), .agentPolicyFiltered)
    }

    func testCriticalPermissionIsRequestedOnlyWhenAvailableToTheApp() {
        let unsupported = SynaraNotificationAuthorizationPolicy.options(criticalAlertsSupported: false)
        XCTAssertTrue(unsupported.contains([.alert, .badge, .sound]))
        XCTAssertFalse(unsupported.contains(.criticalAlert))
        XCTAssertTrue(SynaraNotificationAuthorizationPolicy.options(criticalAlertsSupported: true)
            .contains(.criticalAlert))
    }

    func testGatewayHintsCannotGrantActionsUrgencyOrPreviewDisclosure() {
        let gateway = UNMutableNotificationContent()
        gateway.title = "Private sender"
        gateway.body = "Private command"
        gateway.categoryIdentifier = "synara.agent-approval"
        gateway.interruptionLevel = .critical
        gateway.sound = .defaultCritical
        gateway.userInfo = ["room_id": "!room:example.org", "event_id": "$event"]
        gateway.badge = 3
        let fallback = SynaraNotificationPresentationPolicy.fallback(from: gateway)
        XCTAssertEqual(fallback.title, "Synara")
        XCTAssertEqual(fallback.body, "New activity")
        XCTAssertEqual(fallback.categoryIdentifier, "")
        XCTAssertEqual(fallback.interruptionLevel, .active)
        XCTAssertEqual(fallback.userInfo["event_id"] as? String, "$event")
        XCTAssertEqual(fallback.badge, 3)
    }

    func testVerifiedFreshApprovalUsesCriticalOnlyWithOSAuthorization() {
        let now = Date(timeIntervalSince1970: 2_000_000)
        for authorized in [false, true] {
            let content = SynaraNotificationPresentationPolicy.fallback(from: UNMutableNotificationContent())
            let stage = SynaraNotificationPresentationPolicy.applyResolvedEvent(
                to: content, preview: nil, showPreview: false,
                approvalAlertsEnabled: true, isAgentApproval: true,
                originServerTimestampMS: 2_000_000_000,
                criticalAlertsAuthorized: authorized, now: now
            )
            XCTAssertEqual(content.categoryIdentifier, "synara.agent-approval")
            XCTAssertEqual(content.interruptionLevel, authorized ? .critical : .timeSensitive)
            XCTAssertEqual(content.title, "Agent approval needed")
            XCTAssertEqual(stage, authorized ? .resolvedCriticalApproval : .resolvedApproval)
        }
    }

    func testSilentEventPreviewAndExpiredApprovalPreserveZeroBadge() {
        let now = Date(timeIntervalSince1970: 2_000_000)
        for (approval, timestamp) in [
            (false, UInt64(2_000_000_000)),
            (true, UInt64(1_999_700_000))
        ] {
            let gateway = UNMutableNotificationContent()
            gateway.badge = 0
            gateway.sound = nil
            let content = SynaraNotificationPresentationPolicy.fallback(from: gateway)
            XCTAssertEqual(content.badge, 0)
            XCTAssertNil(content.sound)

            let stage = SynaraNotificationPresentationPolicy.applyResolvedEvent(
                to: content, preview: .init(title: "Sender", body: "Message"), showPreview: true,
                approvalAlertsEnabled: true, isAgentApproval: approval,
                originServerTimestampMS: timestamp, criticalAlertsAuthorized: true, now: now
            )
            XCTAssertEqual(stage, .resolvedPreview)
            XCTAssertEqual(content.title, "Sender")
            XCTAssertEqual(content.body, "Message")
            XCTAssertEqual(content.badge, 0)
            XCTAssertNil(content.sound)
            XCTAssertEqual(content.categoryIdentifier, "")
            XCTAssertEqual(content.interruptionLevel, .active)
        }
    }

    func testOrdinaryExpiredAndDisabledApprovalAlertsNeverEscalate() {
        let now = Date(timeIntervalSince1970: 2_000_000)
        for (enabled, approval, timestamp) in [
            (true, false, UInt64(2_000_000_000)),
            (false, true, UInt64(2_000_000_000)),
            (true, true, UInt64(1_999_700_000))
        ] {
            let content = SynaraNotificationPresentationPolicy.fallback(from: UNMutableNotificationContent())
            let stage = SynaraNotificationPresentationPolicy.applyResolvedEvent(
                to: content, preview: .init(title: "Sender", body: "Message"), showPreview: true,
                approvalAlertsEnabled: enabled, isAgentApproval: approval,
                originServerTimestampMS: timestamp, criticalAlertsAuthorized: true, now: now
            )
            XCTAssertEqual(content.interruptionLevel, .active)
            XCTAssertEqual(content.categoryIdentifier, "")
            XCTAssertEqual(content.body, "Message")
            XCTAssertEqual(stage, .resolvedPreview)
        }
    }

    func testPreviewOptOutRetainsGenericContentAfterResolution() {
        let content = SynaraNotificationPresentationPolicy.fallback(from: UNMutableNotificationContent())
        let stage = SynaraNotificationPresentationPolicy.applyResolvedEvent(
            to: content, preview: .init(title: "Sender", body: "Secret message"), showPreview: false,
            approvalAlertsEnabled: true, isAgentApproval: false,
            originServerTimestampMS: 0, criticalAlertsAuthorized: true
        )
        XCTAssertEqual(content.title, "Synara")
        XCTAssertEqual(content.body, "New activity")
        XCTAssertEqual(stage, .resolvedWithoutPreview)
    }

    func testNotificationServiceExtensionDoesNotWriteAppIconBadge() throws {
        let url = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("SynaraNotificationService/NotificationService.swift")
        let source = try String(contentsOf: url, encoding: .utf8)
        XCTAssertFalse(source.contains("setBadgeCount"))
        XCTAssertFalse(source.contains("applyAppIconBadge"))
    }

    func testPreviewFailureDiagnosticsKeepOnlyAllowlistedReasons() {
        XCTAssertEqual(SynaraNotificationDiagnostics.previewFailureStage(coreCode: "p4-s11-nse-decryption-unavailable"), .coreDecryptionUnavailable)
        XCTAssertEqual(SynaraNotificationDiagnostics.previewFailureStage(coreCode: "p4-s11-nse-event-filtered"), .coreEventFiltered)
        XCTAssertEqual(SynaraNotificationDiagnostics.previewFailureStage(coreCode: "p4-s11-nse-resolution-timeout"), .coreResolutionTimedOut)
        XCTAssertEqual(SynaraNotificationDiagnostics.previewFailureStage(coreCode: "secret-token @user:example.org event-body"), .coreResolutionFailed)
    }

    func testPreviewFailureDiagnosticsRecognizeNseCoreVaultAdapterCode() {
        // NseSecretVault translates the Swift vault error before it crosses
        // the Core FFI boundary; classify the code that actually arrives.
        XCTAssertEqual(
            SynaraNotificationDiagnostics.previewFailureStage(coreCode: "nse-secret-vault-unavailable"),
            .coreStoreUnavailable
        )
    }

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

    func testTimeSensitiveApprovalPreferenceDefaultsOnAndPersistsOptOut() {
        let suiteName = "synara.agent-approval-alert.test.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }

        XCTAssertTrue(SynaraTimeSensitiveAgentApprovalPreference.isEnabled(defaults: defaults))
        defaults.set(false, forKey: SynaraSharedConstants.timeSensitiveAgentApprovalsKey)
        XCTAssertFalse(SynaraTimeSensitiveAgentApprovalPreference.isEnabled(defaults: defaults))
    }

    /// Core owns these rules but the NSE mirrors them in Swift; the Rust tests
    /// in agent_approvals.rs and nse_error.rs read the same vector file.
    private func sharedNotificationPolicyVectors() throws -> [String: Any] {
        let url = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("crates/synara-core/tests/support/notification-policy-vectors.json")
        let data = try Data(contentsOf: url)
        return try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
    }

    func testAgentApprovalFreshnessMatchesCoreVectors() throws {
        let vectors = try sharedNotificationPolicyVectors()
        let freshness = try XCTUnwrap(vectors["agentApprovalFreshness"] as? [String: Any])
        XCTAssertEqual(
            (freshness["ttlMs"] as? NSNumber)?.uint64Value,
            SynaraAgentApprovalFreshness.ttlMilliseconds
        )
        XCTAssertEqual(
            (freshness["futureToleranceMs"] as? NSNumber)?.uint64Value,
            SynaraAgentApprovalFreshness.futureToleranceMilliseconds
        )
        let nowMS = try XCTUnwrap((freshness["nowMs"] as? NSNumber)?.uint64Value)
        let now = Date(timeIntervalSince1970: Double(nowMS) / 1_000)
        let cases = try XCTUnwrap(freshness["cases"] as? [[String: Any]])
        XCTAssertFalse(cases.isEmpty)
        for vector in cases {
            let name = try XCTUnwrap(vector["name"] as? String)
            let origin = try XCTUnwrap((vector["originServerTs"] as? NSNumber)?.uint64Value)
            let fresh = try XCTUnwrap(vector["fresh"] as? Bool)
            XCTAssertEqual(
                SynaraAgentApprovalFreshness.isFresh(originServerTimestampMS: origin, now: now),
                fresh,
                name
            )
        }
    }

    func testPreviewFailureStagesMatchCoreVectors() throws {
        let vectors = try sharedNotificationPolicyVectors()
        let stages = try XCTUnwrap(vectors["nsePreviewFailureStages"] as? [String: Any])
        let fallback = try XCTUnwrap(stages["fallbackStage"] as? String)
        let codes = try XCTUnwrap(stages["codes"] as? [String: String])
        XCTAssertFalse(codes.isEmpty)
        for (code, stage) in codes {
            XCTAssertEqual(
                SynaraNotificationDiagnostics.previewFailureStage(coreCode: code).rawValue,
                stage,
                code
            )
        }
        XCTAssertEqual(
            SynaraNotificationDiagnostics.previewFailureStage(coreCode: "p4-s11-nse-not-a-real-code").rawValue,
            fallback
        )
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
