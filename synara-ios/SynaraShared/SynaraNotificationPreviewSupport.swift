import Foundation

enum SynaraSharedConstants {
    static let appGroupIdentifier = "group.com.whylandcreative.synara"
    static let keychainAccessGroupInfoKey = "SynaraKeychainAccessGroup"
    static let sharedCoreStoreDirectory = "SynaraCore"
    static let sharedCoreStoreReadyMarker = ".synara-nse-store-ready-v1"
    static let lockScreenMessagePreviewsKey = "synara.settings.lockScreenMessagePreviews"
    static let defaultLockScreenMessagePreviews = false
    static let timeSensitiveAgentApprovalsKey = "synara.settings.timeSensitiveAgentApprovals"
    static let defaultTimeSensitiveAgentApprovals = false
    static let themeBaseColorKey = "themeBaseColor"
    static let hour24ClockKey = "synara.settings.hour24Clock"
    static let hideActivityKey = "synara.settings.hideActivity"
    static let notificationDiagnosticsKey = "synara.notification.previewDiagnostics.v1"

    static var registeredUserDefaults: [String: Any] {
        [
            lockScreenMessagePreviewsKey: defaultLockScreenMessagePreviews,
            timeSensitiveAgentApprovalsKey: defaultTimeSensitiveAgentApprovals,
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
        case preferencesDisabled = "preferences-disabled"
        case appGroupUnavailable = "app-group-unavailable"
        case resolutionQueued = "resolution-queued"
        case resolutionCancelled = "resolution-cancelled"
        case sharedSessionMissing = "shared-session-missing"
        case sharedStoreNotReady = "shared-store-not-ready"
        case coreResolutionFailed = "core-resolution-failed"
        case coreSessionUnavailable = "core-session-unavailable"
        case coreStoreUnavailable = "core-store-unavailable"
        case coreRestoreFailed = "core-restore-failed"
        case coreFetchFailed = "core-fetch-failed"
        case coreResolutionTimedOut = "core-resolution-timed-out"
        case coreEventFiltered = "core-event-filtered"
        case coreEventRedacted = "core-event-redacted"
        case coreEventUnavailable = "core-event-unavailable"
        case coreDecryptionUnavailable = "core-decryption-unavailable"
        case resolvedWithoutPreview = "resolved-without-preview"
        case resolvedPreview = "resolved-preview"
        case resolvedApproval = "resolved-approval"
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
    /// codes collapse to the existing fixed generic stage.
    static func previewFailureStage(coreCode: String) -> Stage {
        switch coreCode {
        case "p4-s3b-material-missing": return .coreSessionUnavailable
        case "p4-s3b-restore-failed": return .coreRestoreFailed
        case "p4-s3-secret-vault-unavailable": return .coreStoreUnavailable
        case "p4-s11-nse-event-fetch-failed", "p4-s11-nse-client-init-failed": return .coreFetchFailed
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
    static func payload(from userInfo: [AnyHashable: Any]) -> SynaraNotificationPreviewPayload? {
        let flattened = flatten(userInfo)
        guard let roomID = firstString(flattened, keys: ["room_id", "roomId"]),
              let eventID = firstString(flattened, keys: ["event_id", "eventId"]) else {
            return nil
        }

        return SynaraNotificationPreviewPayload(
            roomID: roomID,
            eventID: eventID,
            kind: firstString(flattened, keys: ["kind", "synara.kind"]),
            category: firstString(flattened, keys: ["aps.category", "category"])
        )
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
        return nowMS >= originServerTimestampMS
            && nowMS - originServerTimestampMS < ttlMilliseconds
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
