import Foundation
import UserNotifications

/// These flags opt signed release configurations into restricted Apple
/// capabilities. Actual Critical delivery additionally requires OS permission.
enum SynaraNotificationCapabilities {
    static func criticalAlertsEnabled(bundle: Bundle = .main) -> Bool {
        enabled("SynaraCriticalAlertsEnabled", bundle: bundle)
    }

    static func filteringEnabled(bundle: Bundle = .main) -> Bool {
        enabled("SynaraNotificationFilteringEnabled", bundle: bundle)
    }

    private static func enabled(_ key: String, bundle: Bundle) -> Bool {
        if let value = bundle.object(forInfoDictionaryKey: key) as? NSNumber {
            return value.boolValue
        }
        guard let value = bundle.object(forInfoDictionaryKey: key) as? String else { return false }
        return ["YES", "TRUE", "1"].contains(value.uppercased())
    }
}

/// Unsupported builds continue requesting ordinary notification permission.
enum SynaraNotificationAuthorizationPolicy {
    static func options(criticalAlertsSupported: Bool) -> UNAuthorizationOptions {
        var options: UNAuthorizationOptions = [.alert, .badge, .sound]
        if criticalAlertsSupported {
            options.insert(.criticalAlert)
        }
        return options
    }
}

enum SynaraNotificationPresentationPolicy {
    static let approvalCategory = "synara.agent-approval"

    /// Register this sanitized snapshot as the deadline fallback too. Gateway
    /// hints never grant actions, urgency, or disclosure of message content.
    static func fallback(from original: UNNotificationContent) -> UNMutableNotificationContent {
        let content = original.mutableCopy() as? UNMutableNotificationContent
            ?? UNMutableNotificationContent()
        content.userInfo = original.userInfo
        content.badge = original.badge
        content.title = "Synara"
        content.subtitle = ""
        content.body = "New activity"
        content.categoryIdentifier = ""
        content.interruptionLevel = .active
        content.sound = original.sound == nil ? nil : .default
        return content
    }

    @discardableResult
    static func applyResolvedEvent(
        to content: UNMutableNotificationContent,
        preview: SynaraNotificationPreview?,
        showPreview: Bool,
        approvalAlertsEnabled: Bool,
        isAgentApproval: Bool,
        originServerTimestampMS: UInt64,
        criticalAlertsAuthorized: Bool,
        approvalSummary: SynaraAgentApprovalSummary? = nil,
        now: Date = Date()
    ) -> SynaraNotificationDiagnostics.Stage {
        var stage = SynaraNotificationDiagnostics.Stage.resolvedWithoutPreview
        if showPreview, let preview {
            content.title = preview.title
            content.body = preview.body
            stage = .resolvedPreview
        }
        guard approvalAlertsEnabled, isAgentApproval,
              SynaraAgentApprovalFreshness.isFresh(
                originServerTimestampMS: originServerTimestampMS, now: now
              ) else { return stage }

        if let approvalSummary, approvalSummary.hasDetails {
            content.title = approvalSummary.title
            content.body = approvalSummary.body
        } else if !showPreview || preview == nil {
            content.title = "Agent approval needed"
            content.body = "Review a time-sensitive request in Synara."
        }
        content.categoryIdentifier = approvalCategory
        content.interruptionLevel = criticalAlertsAuthorized ? .critical : .timeSensitive
        content.sound = criticalAlertsAuthorized ? .defaultCritical : .default
        return criticalAlertsAuthorized ? .resolvedCriticalApproval : .resolvedApproval
    }
}

enum SynaraAgentNotificationSuppressionPolicy {
    static func content(
        for stage: SynaraNotificationDiagnostics.Stage,
        filteringEnabled: Bool
    ) -> UNNotificationContent? {
        guard filteringEnabled, stage == .agentPolicyFiltered else { return nil }
        // Apple permits an empty result to drop a remote alert only for an NSE
        // signed with com.apple.developer.usernotifications.filtering.
        return UNNotificationContent()
    }
}

enum SynaraSharedConstants {
    static let appGroupIdentifier = "group.com.whylandcreative.synara"
    static let keychainAccessGroupInfoKey = "SynaraKeychainAccessGroup"
    static let sharedCoreStoreDirectory = "SynaraCore"
    static let sharedCoreStoreReadyMarker = ".synara-nse-store-ready-v1"
    static let lockScreenMessagePreviewsKey = "synara.settings.lockScreenMessagePreviews"
    static let defaultLockScreenMessagePreviews = false
    static let timeSensitiveAgentApprovalsKey = "synara.settings.timeSensitiveAgentApprovals"
    static let defaultTimeSensitiveAgentApprovals = true
    static let approvalNotificationDetailsKey = "synara.settings.showApprovalNotificationDetails"
    static let defaultApprovalNotificationDetails = true
    static let themeBaseColorKey = "themeBaseColor"
    static let hour24ClockKey = "synara.settings.hour24Clock"
    static let hideActivityKey = "synara.settings.hideActivity"
    static let notificationDiagnosticsKey = "synara.notification.previewDiagnostics.v1"

    static var registeredUserDefaults: [String: Any] {
        [
            lockScreenMessagePreviewsKey: defaultLockScreenMessagePreviews,
            timeSensitiveAgentApprovalsKey: defaultTimeSensitiveAgentApprovals,
            approvalNotificationDetailsKey: defaultApprovalNotificationDetails,
            hour24ClockKey: false,
            hideActivityKey: false
        ]
    }

    static func appGroupDefaults() -> UserDefaults? {
        UserDefaults(suiteName: appGroupIdentifier)
    }

    static func boolSetting(_ key: String) -> Bool {
        if let group = appGroupDefaults(), group.object(forKey: key) != nil {
            return group.bool(forKey: key)
        }
        return UserDefaults.standard.bool(forKey: key)
    }

    static func sharedCoreStoreRoot(fileManager: FileManager = .default) -> URL? {
        fileManager.containerURL(forSecurityApplicationGroupIdentifier: appGroupIdentifier)?
            .appendingPathComponent(sharedCoreStoreDirectory, isDirectory: true)
    }

    static func sharedCoreStoreIsReady(
        at storeRoot: URL,
        fileManager: FileManager = .default
    ) -> Bool {
        fileManager.fileExists(
            atPath: storeRoot.appendingPathComponent(sharedCoreStoreReadyMarker).path
        )
    }
}

struct SynaraNotificationDiagnosticEntry: Codable, Equatable, Identifiable {
    let id: UUID
    /// Opaque local correlation only. It is never derived from a Matrix or
    /// APNs identifier and may be absent in records written by older builds.
    let runID: UUID?
    let timestamp: Date
    let stage: String
}

/// A small, device-local notification-service flight recorder.
///
/// Entries intentionally contain only a fixed stage code and timestamp. They
/// never contain Matrix IDs, APNs payloads, sender names, event content,
/// credentials, device tokens, URLs, or raw errors. Both the app and NSE use
/// the App Group defaults so a failed extension invocation can be diagnosed
/// from Settings after the fact.
enum SynaraNotificationDiagnostics {
    enum Stage: String, CaseIterable {
        case received
        case contentCopyFailed = "content-copy-failed"
        case payloadInvalid = "payload-invalid"
        case payloadNoEventReference = "payload-no-event-reference"
        case payloadMissingRoom = "payload-missing-room"
        case payloadMissingEvent = "payload-missing-event"
        case payloadIdentifierTypeInvalid = "payload-identifier-type-invalid"
        case payloadIdentifiersAmbiguous = "payload-identifiers-ambiguous"
        case payloadIdentifiersInvalid = "payload-identifiers-invalid"
        case payloadComplexityExceeded = "payload-complexity-exceeded"
        case preferencesDisabled = "preferences-disabled"
        case appGroupUnavailable = "app-group-unavailable"
        case resolutionQueued = "resolution-queued"
        case resolutionStarted = "resolution-started"
        case coreResolutionStarted = "core-resolution-started"
        case resolutionCancelled = "resolution-cancelled"
        case sharedSessionMissing = "shared-session-missing"
        case sharedStoreNotReady = "shared-store-not-ready"
        case coreResolutionFailed = "core-resolution-failed"
        case coreSessionUnavailable = "core-session-unavailable"
        case coreStoreUnavailable = "core-store-unavailable"
        case coreRestoreFailed = "core-restore-failed"
        case coreFetchFailed = "core-fetch-failed"
        case coreClientInitFailed = "core-client-init-failed"
        case coreNetworkTimeout = "core-network-timeout"
        case coreNetworkUnavailable = "core-network-unavailable"
        case coreSessionRejected = "core-session-rejected"
        case coreAccessDenied = "core-access-denied"
        case coreRateLimited = "core-rate-limited"
        case coreAPIIncompatible = "core-api-incompatible"
        case coreStoreLockFailed = "core-store-lock-failed"
        case coreCryptoUnavailable = "core-crypto-unavailable"
        case coreRoomUnavailable = "core-room-unavailable"
        case coreInvalidEvent = "core-invalid-event"
        case coreContextMissingEvent = "core-context-missing-event"
        case coreSlidingSyncVersionMissing = "core-sliding-sync-version-missing"
        case coreInvalidResponse = "core-invalid-response"
        case coreResolutionTimedOut = "core-resolution-timed-out"
        case coreEventFiltered = "core-event-filtered"
        case coreEventRedacted = "core-event-redacted"
        case coreEventUnavailable = "core-event-unavailable"
        case coreDecryptionUnavailable = "core-decryption-unavailable"
        case resolvedWithoutPreview = "resolved-without-preview"
        case resolvedPreview = "resolved-preview"
        case resolvedApproval = "resolved-approval"
        case resolvedCriticalApproval = "resolved-critical-approval"
        case agentPolicyFiltered = "agent-policy-filtered"
        case agentPolicySuppressed = "agent-policy-suppressed"
        case delivered = "delivered"
        case systemDeadline = "system-deadline"
        case permissionRequested = "permission-requested"
        case permissionAuthorized = "permission-authorized"
        case permissionDenied = "permission-denied"
        case permissionUnavailable = "permission-unavailable"
        case apnsRegistrationRequested = "apns-registration-requested"
        case apnsTokenCaptured = "apns-token-captured"
        case apnsRegistrationFailed = "apns-registration-failed"
        case pusherGatewayUnavailable = "pusher-gateway-unavailable"
        case pusherRegistrationStarted = "pusher-registration-started"
        case pusherRegistrationSucceeded = "pusher-registration-succeeded"
        case pusherRegistrationFailed = "pusher-registration-failed"
        case pusherRegistrationSuperseded = "pusher-registration-superseded"
        case pusherUnregistrationStarted = "pusher-unregistration-started"
        case pusherUnregistrationSucceeded = "pusher-unregistration-succeeded"
        case pusherUnregistrationFailed = "pusher-unregistration-failed"
        case foregroundReceived = "foreground-received"
        case backgroundReceived = "background-received"
        case responseReceived = "response-received"
    }

    /// Never persist arbitrary Core/error text. Unknown future or malformed
    /// codes collapse to the existing fixed generic stage. Every code the
    /// shipping NSE path can emit is pinned, with its stage, in
    /// crates/synara-core/tests/support/notification-policy-vectors.json.
    static func previewFailureStage(coreCode: String) -> Stage {
        switch coreCode {
        case "p4-s11-nse-agent-policy-filtered": return .agentPolicyFiltered
        case "p4-s3b-material-missing", "p4-s3b-identity-invalid": return .coreSessionUnavailable
        case "p4-s3b-restore-failed": return .coreRestoreFailed
        case "p4-s3b-store-root-invalid": return .coreStoreUnavailable
        case "p4-s11-nse-payload-oversize": return .payloadComplexityExceeded
        case "nse-preview-request-cancelled": return .resolutionCancelled
        case "nse-secret-vault-unavailable", "p4-s3-secret-vault-unavailable": return .coreStoreUnavailable
        case "p4-s11-nse-event-fetch-failed": return .coreFetchFailed
        case "p4-s11-nse-client-init-failed": return .coreClientInitFailed
        case "p4-s11-nse-network-timeout": return .coreNetworkTimeout
        case "p4-s11-nse-network-unavailable": return .coreNetworkUnavailable
        case "p4-s11-nse-session-rejected": return .coreSessionRejected
        case "p4-s11-nse-access-denied": return .coreAccessDenied
        case "p4-s11-nse-rate-limited": return .coreRateLimited
        case "p4-s11-nse-api-incompatible": return .coreAPIIncompatible
        case "p4-s11-nse-store-unavailable": return .coreStoreUnavailable
        case "p4-s11-nse-store-lock-failed": return .coreStoreLockFailed
        case "p4-s11-nse-crypto-unavailable": return .coreCryptoUnavailable
        case "p4-s11-nse-room-unavailable": return .coreRoomUnavailable
        case "p4-s11-nse-invalid-event": return .coreInvalidEvent
        case "p4-s11-nse-context-missing-event": return .coreContextMissingEvent
        case "p4-s11-nse-sliding-sync-version-missing": return .coreSlidingSyncVersionMissing
        case "p4-s11-nse-invalid-response": return .coreInvalidResponse
        case "p4-s11-nse-resolution-timeout": return .coreResolutionTimedOut
        case "p4-s11-nse-event-filtered": return .coreEventFiltered
        case "p4-s11-nse-event-redacted": return .coreEventRedacted
        case "p4-s11-nse-event-not-in-store": return .coreEventUnavailable
        case "p4-s11-nse-decryption-unavailable": return .coreDecryptionUnavailable
        default: return .coreResolutionFailed
        }
    }

    static let maximumEntries = 256
    private static let lock = NSLock()

    static func record(
        _ stage: Stage,
        runID: UUID? = nil,
        now: Date = Date(),
        defaults: UserDefaults? = SynaraSharedConstants.appGroupDefaults()
    ) {
        guard let defaults else { return }
        append(
            [.init(id: UUID(), runID: runID, timestamp: now, stage: stage.rawValue)],
            defaults: defaults
        )
    }

    /// Record only deadline completions actually won by the coordinator.
    /// An empty expiration must not create an uncorrelated diagnostic entry.
    static func recordDeadlineDeliveries(
        for runIDs: [UUID],
        now: Date = Date(),
        defaults: UserDefaults? = SynaraSharedConstants.appGroupDefaults()
    ) {
        guard let defaults, runIDs.isEmpty == false else { return }
        let additions = runIDs.flatMap { runID in
            [
                SynaraNotificationDiagnosticEntry(
                    id: UUID(),
                    runID: runID,
                    timestamp: now,
                    stage: Stage.systemDeadline.rawValue
                ),
                SynaraNotificationDiagnosticEntry(
                    id: UUID(),
                    runID: runID,
                    timestamp: now,
                    stage: Stage.delivered.rawValue
                )
            ]
        }
        append(additions, defaults: defaults)
    }

    static func entries(
        defaults: UserDefaults? = SynaraSharedConstants.appGroupDefaults()
    ) -> [SynaraNotificationDiagnosticEntry] {
        guard let defaults else { return [] }
        lock.lock()
        defer { lock.unlock() }
        return entriesWithoutLock(defaults: defaults)
    }

    static func clear(defaults: UserDefaults? = SynaraSharedConstants.appGroupDefaults()) {
        guard let defaults else { return }
        lock.lock()
        defer { lock.unlock() }
        defaults.removeObject(forKey: SynaraSharedConstants.notificationDiagnosticsKey)
    }

    private static func entriesWithoutLock(defaults: UserDefaults) -> [SynaraNotificationDiagnosticEntry] {
        guard let data = defaults.data(forKey: SynaraSharedConstants.notificationDiagnosticsKey),
              let decoded = try? JSONDecoder().decode([SynaraNotificationDiagnosticEntry].self, from: data)
        else {
            return []
        }
        return Array(decoded.suffix(maximumEntries))
    }

    /// One in-process read/modify/write keeps a deadline batch internally
    /// consistent and minimizes work after Apple's completion deadline. The
    /// app and NSE remain separate processes, so this is intentionally not
    /// described as a durable cross-process audit log.
    private static func append(
        _ additions: [SynaraNotificationDiagnosticEntry],
        defaults: UserDefaults
    ) {
        guard additions.isEmpty == false else { return }
        lock.lock()
        defer { lock.unlock() }
        var current = entriesWithoutLock(defaults: defaults)
        current.append(contentsOf: additions)
        if current.count > maximumEntries {
            current = Array(current.suffix(maximumEntries))
        }
        guard let data = try? JSONEncoder().encode(current) else { return }
        defaults.set(data, forKey: SynaraSharedConstants.notificationDiagnosticsKey)
    }
}

struct SynaraNotificationPreviewPayload: Equatable {
    let roomID: String
    let eventID: String
    let kind: String?
    let category: String?

    var isAgentApproval: Bool {
        kind == "agent-approval" || category == "synara.agent-approval"
    }
}

enum SynaraNotificationPreviewPayloadParser {
    struct Reference: Equatable {
        let roomID: String
        let eventID: String
    }

    enum ReferenceFailure: Error {
        case noEventReference, missingRoom, missingEvent, invalidType
        case ambiguous, invalidIdentifier, complexityExceeded

        var stage: SynaraNotificationDiagnostics.Stage {
            switch self {
            case .noEventReference: return .payloadNoEventReference
            case .missingRoom: return .payloadMissingRoom
            case .missingEvent: return .payloadMissingEvent
            case .invalidType: return .payloadIdentifierTypeInvalid
            case .ambiguous: return .payloadIdentifiersAmbiguous
            case .invalidIdentifier: return .payloadIdentifiersInvalid
            case .complexityExceeded: return .payloadComplexityExceeded
            }
        }
    }

    static func payload(from userInfo: [AnyHashable: Any]) -> SynaraNotificationPreviewPayload? {
        try? parse(userInfo).get()
    }

    static func parse(_ userInfo: [AnyHashable: Any]) -> Result<SynaraNotificationPreviewPayload, ReferenceFailure> {
        reference(from: userInfo, trimWhitespace: true).map { reference in
            let flattened = flatten(userInfo)
            return SynaraNotificationPreviewPayload(
                roomID: reference.roomID,
                eventID: reference.eventID,
                kind: firstString(flattened, keys: ["kind", "synara.kind"]),
                category: firstString(flattened, keys: ["aps.category", "category"])
            )
        }
    }

    /// Accumulate every candidate before deduplication. Flattened dictionaries
    /// can overwrite conflicting literal dotted keys and nested fields.
    static func reference(
        from userInfo: [AnyHashable: Any],
        trimWhitespace: Bool
    ) -> Result<Reference, ReferenceFailure> {
        var rooms: [Any] = []
        var events: [Any] = []
        var nodes = 0
        var exceeded = false
        func visit(_ dictionary: [AnyHashable: Any], depth: Int) {
            guard depth <= 12 else { exceeded = true; return }
            for (rawKey, value) in dictionary {
                nodes += 1
                guard nodes <= 512 else { exceeded = true; return }
                guard let key = rawKey as? String else { continue }
                let leaf = key.split(separator: ".").last.map(String.init)
                if leaf == "room_id" || leaf == "roomId" { rooms.append(value) }
                if leaf == "event_id" || leaf == "eventId" { events.append(value) }
                if let nested = value as? [AnyHashable: Any] { visit(nested, depth: depth + 1) }
                if exceeded { return }
            }
        }
        visit(userInfo, depth: 0)
        guard !exceeded else { return .failure(.complexityExceeded) }
        if rooms.isEmpty && events.isEmpty { return .failure(.noEventReference) }
        if rooms.isEmpty { return .failure(.missingRoom) }
        if events.isEmpty { return .failure(.missingEvent) }
        guard (rooms + events).allSatisfy({ $0 is String }) else { return .failure(.invalidType) }
        func normalized(_ values: [Any]) -> Set<String> {
            Set(values.compactMap { value in
                guard let text = value as? String else { return nil }
                return trimWhitespace ? text.trimmingCharacters(in: .whitespacesAndNewlines) : text
            })
        }
        let roomIDs = normalized(rooms), eventIDs = normalized(events)
        guard roomIDs.count == 1, eventIDs.count == 1 else { return .failure(.ambiguous) }
        guard let roomID = roomIDs.first, let eventID = eventIDs.first,
              validID(roomID, prefix: "!"), validID(eventID, prefix: "$") else { return .failure(.invalidIdentifier) }
        // Room version 12 IDs have no server suffix. For legacy IDs, reject
        // empty components; shared Core remains the exact ID authority.
        if let separator = roomID.firstIndex(of: ":"),
           (separator == roomID.index(after: roomID.startIndex) || roomID.index(after: separator) == roomID.endIndex) {
            return .failure(.invalidIdentifier)
        }
        return .success(Reference(roomID: roomID, eventID: eventID))
    }

    private static func validID(_ id: String, prefix: Character) -> Bool {
        id.first == prefix && id.count > 1 && id.utf8.count <= 255 &&
        !id.unicodeScalars.contains {
            CharacterSet.whitespacesAndNewlines.contains($0) ||
            CharacterSet.controlCharacters.contains($0) || $0.properties.generalCategory == .format
        }
    }

    static func flatten(_ payload: [AnyHashable: Any]) -> [String: Any] {
        var values: [String: Any] = [:]

        func visit(_ value: Any, prefix: String?) {
            guard let dictionary = value as? [AnyHashable: Any] else {
                if let prefix {
                    values[prefix] = value
                }
                return
            }

            for (rawKey, rawValue) in dictionary {
                guard let key = rawKey as? String else { continue }
                let flattenedKey = [prefix, key].compactMap { $0 }.joined(separator: ".")
                values[flattenedKey] = rawValue
                values[key] = values[key] ?? rawValue
                visit(rawValue, prefix: flattenedKey)
            }
        }

        visit(payload, prefix: nil)
        return values
    }

    private static func firstString(_ values: [String: Any], keys: [String]) -> String? {
        for key in keys {
            guard let value = values[key] as? String else { continue }
            let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
            if trimmed.isEmpty == false {
                return trimmed
            }
        }
        return nil
    }
}

enum SynaraNotificationPreviewPreference {
    static func isEnabled(defaults: UserDefaults? = SynaraSharedConstants.appGroupDefaults()) -> Bool {
        guard let defaults else {
            return SynaraSharedConstants.defaultLockScreenMessagePreviews
        }

        if defaults.object(forKey: SynaraSharedConstants.lockScreenMessagePreviewsKey) == nil {
            return SynaraSharedConstants.defaultLockScreenMessagePreviews
        }

        return defaults.bool(forKey: SynaraSharedConstants.lockScreenMessagePreviewsKey)
    }
}

enum SynaraTimeSensitiveAgentApprovalPreference {
    static func isEnabled(defaults: UserDefaults? = SynaraSharedConstants.appGroupDefaults()) -> Bool {
        guard let defaults else {
            return SynaraSharedConstants.defaultTimeSensitiveAgentApprovals
        }
        if defaults.object(forKey: SynaraSharedConstants.timeSensitiveAgentApprovalsKey) == nil {
            return SynaraSharedConstants.defaultTimeSensitiveAgentApprovals
        }
        return defaults.bool(forKey: SynaraSharedConstants.timeSensitiveAgentApprovalsKey)
    }
}

/// Independent of message previews: approval prompts may show their reason
/// and command even when ordinary message text stays hidden.
enum SynaraApprovalNotificationDetailsPreference {
    static func isEnabled(defaults: UserDefaults? = SynaraSharedConstants.appGroupDefaults()) -> Bool {
        guard let defaults,
              defaults.object(forKey: SynaraSharedConstants.approvalNotificationDetailsKey) != nil else {
            return SynaraSharedConstants.defaultApprovalNotificationDetails
        }
        return defaults.bool(forKey: SynaraSharedConstants.approvalNotificationDetailsKey)
    }
}

/// Core's bounded `agent_approval_notification_summary` for one prompt.
struct SynaraAgentApprovalSummary: Equatable {
    let sender: String?
    let reason: String?
    let command: String?

    var hasDetails: Bool {
        Self.nonEmpty(reason) != nil || Self.nonEmpty(command) != nil
    }

    var title: String {
        let name = Self.nonEmpty(sender) ?? "Agent"
        let reason = Self.nonEmpty(reason) ?? "Approval required"
        return SynaraMatrixEventPreviewComposer.clamp("\(name): \(reason)", limit: 120)
    }

    var body: String {
        SynaraMatrixEventPreviewComposer.clamp(
            Self.nonEmpty(command) ?? "Review the command in Synara.",
            limit: 240
        )
    }

    private static func nonEmpty(_ value: String?) -> String? {
        guard let trimmed = value?.trimmingCharacters(in: .whitespacesAndNewlines),
              trimmed.isEmpty == false else { return nil }
        return trimmed
    }
}

/// Mirrors `classify_agent_approval` in crates/synara-core/src/app/agent_approvals.rs,
/// which the NSE cannot call over the FFI. Both sides read the vectors in
/// crates/synara-core/tests/support/notification-policy-vectors.json.
enum SynaraAgentApprovalFreshness {
    static let ttlMilliseconds: UInt64 = 5 * 60 * 1_000
    static let futureToleranceMilliseconds: UInt64 = 60 * 1_000

    static func isFresh(originServerTimestampMS: UInt64, now: Date = Date()) -> Bool {
        let rawNow = now.timeIntervalSince1970 * 1_000
        guard rawNow.isFinite, rawNow >= 0, rawNow <= Double(UInt64.max), originServerTimestampMS > 0 else {
            return false
        }
        let nowMS = UInt64(rawNow)
        let futureLimit = nowMS.addingReportingOverflow(futureToleranceMilliseconds)
        guard futureLimit.overflow == false, originServerTimestampMS <= futureLimit.partialValue else {
            return false
        }
        // A prompt stamped slightly ahead of this device's clock (within the
        // tolerance) has zero age, exactly as Core's saturating subtraction.
        let age = nowMS >= originServerTimestampMS ? nowMS - originServerTimestampMS : 0
        return age < ttlMilliseconds
    }
}

struct SynaraMatrixEventPreviewInput: Equatable {
    let eventType: String
    let senderID: String?
    let body: String?
    let messageType: String?

    init(
        eventType: String = "m.room.message",
        senderID: String?,
        body: String?,
        messageType: String? = nil
    ) {
        self.eventType = eventType
        self.senderID = senderID
        self.body = body
        self.messageType = messageType
    }
}

struct SynaraNotificationPreview: Equatable {
    let title: String
    let body: String
}

enum SynaraMatrixEventPreviewComposer {
    static func preview(from input: SynaraMatrixEventPreviewInput) -> SynaraNotificationPreview? {
        guard input.eventType != "m.room.encrypted" else {
            return nil
        }

        let sender = displayName(from: input.senderID)
        let body = messageBody(from: input)

        guard let body, body.isEmpty == false else {
            return nil
        }

        let title = clamp(sender ?? "Synara", limit: 120)
        return SynaraNotificationPreview(
            title: title,
            body: clamp(body, limit: 240)
        )
    }

    static func clamp(_ value: String, limit: Int) -> String {
        let normalized = value
            .components(separatedBy: .whitespacesAndNewlines)
            .filter { $0.isEmpty == false }
            .joined(separator: " ")

        guard normalized.count > limit else {
            return normalized
        }

        let suffix = "..."
        let end = normalized.index(normalized.startIndex, offsetBy: max(0, limit - suffix.count))
        return String(normalized[..<end]) + suffix
    }

    private static func messageBody(from input: SynaraMatrixEventPreviewInput) -> String? {
        switch input.messageType {
        case "m.image":
            return input.body?.isEmpty == false ? input.body : "sent an image"
        case "m.video":
            return input.body?.isEmpty == false ? input.body : "sent a video"
        case "m.file":
            return input.body?.isEmpty == false ? input.body : "sent a file"
        case "m.audio":
            return input.body?.isEmpty == false ? input.body : "sent audio"
        default:
            let trimmed = input.body?.trimmingCharacters(in: .whitespacesAndNewlines)
            return trimmed?.isEmpty == false ? trimmed : nil
        }
    }

    private static func displayName(from senderID: String?) -> String? {
        guard let senderID = senderID?.trimmingCharacters(in: .whitespacesAndNewlines),
              senderID.isEmpty == false else {
            return nil
        }

        if senderID.hasPrefix("@"),
           let separator = senderID.firstIndex(of: ":") {
            let localpart = senderID[senderID.index(after: senderID.startIndex)..<separator]
            if localpart.isEmpty == false {
                return String(localpart)
            }
        }

        return senderID
    }
}
