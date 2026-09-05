import Foundation
import UserNotifications

#if canImport(UIKit)
import UIKit
#endif

protocol SparsePushRouteResolving {
    func resolveRoute(eventID: String) async -> AppRoute?
}

enum SynaraNotificationActionContract {
    static let agentApprovalCategoryIdentifier = "synara.agent-approval"
    static let reviewIdentifier = SynaraAgentApprovalNotificationActionID.review.rawValue
    static let approveOnceIdentifier = SynaraAgentApprovalNotificationActionID.approveOnce.rawValue
    static let approveAlwaysIdentifier = SynaraAgentApprovalNotificationActionID.approveAlways.rawValue
    static let denyIdentifier = SynaraAgentApprovalNotificationActionID.deny.rawValue

    static func registerCategories(
        center: UNUserNotificationCenter = UNUserNotificationCenter.current()
    ) {
        // Approve-always is intentionally omitted from the native category: permanent
        // approval requires an explicit in-app confirmation path. Keep the two
        // time-critical decisions first for compact notification surfaces;
        // tapping the notification body remains the primary Review path.
        let category = UNNotificationCategory(
            identifier: agentApprovalCategoryIdentifier,
            actions: agentApprovalActions(),
            intentIdentifiers: [],
            options: []
        )
        center.setNotificationCategories([category])
    }

    static func agentApprovalActions() -> [UNNotificationAction] {
        [
            UNNotificationAction(
                identifier: approveOnceIdentifier,
                title: "Approve once",
                // Native Matrix approval is an authenticated store mutation.
                // Bring the app to foreground first so UIKit foreground
                // authority—not a notification wake—owns store open/use.
                options: [.authenticationRequired, .foreground]
            ),
            UNNotificationAction(
                identifier: denyIdentifier,
                title: "Deny",
                options: [.authenticationRequired, .destructive, .foreground]
            ),
            UNNotificationAction(
                identifier: reviewIdentifier,
                title: "Review",
                options: [.foreground]
            )
        ]
    }

    /// Plans how a native/push notification action should be handled.
    /// Does not send Matrix traffic; decision callers must use the shared-core
    /// decision route for authoritative event and expiry validation.
    static func planAgentApprovalNotificationAction(
        actionIdentifier: String,
        userInfo: [AnyHashable: Any],
        now _: Date = Date(),
        alreadyActed: Bool = false
    ) -> SynaraAgentApprovalNotificationActionPlan {
        guard let action = SynaraAgentApprovalNotificationActionID(rawValue: actionIdentifier) else {
            return .ignore(reason: "unknown-action-id")
        }

        let candidates = NotificationPushRouteParser.flattenPayload(userInfo)
        guard let roomID = firstString(candidates, keys: ["room_id", "roomId"]),
              let eventID = firstString(candidates, keys: ["event_id", "eventId"]) else {
            return .ignore(reason: "missing-room-or-event-id")
        }

        if action == .review {
            return .openRoom(
                roomID: roomID,
                eventID: eventID,
                reason: "review-requested"
            )
        }

        // Permanent approval must not fire from a background notification action.
        if action == .approveAlways {
            return .openRoom(
                roomID: roomID,
                eventID: eventID,
                reason: "approve-always-requires-in-app-confirmation"
            )
        }

        if alreadyActed {
            return .ignore(reason: "already-acted")
        }

        return .submitDecision(
            SynaraAgentApprovalPromptDecisionRequest(
                roomID: roomID,
                sourceEventID: eventID,
                actionIdentifier: action.rawValue
            )
        )
    }

    /// Convenience parser used by unit tests and callers that only need the
    /// Core-owned decision request. Returns nil for approve-always and malformed payloads.
    static func agentApprovalDecisionRequest(
        actionIdentifier: String,
        userInfo: [AnyHashable: Any],
        now: Date = Date(),
        alreadyActed: Bool = false
    ) -> SynaraAgentApprovalPromptDecisionRequest? {
        if case .submitDecision(let request) = planAgentApprovalNotificationAction(
            actionIdentifier: actionIdentifier,
            userInfo: userInfo,
            now: now,
            alreadyActed: alreadyActed
        ) {
            return request
        }
        return nil
    }

    private static func firstString(_ values: [String: Any], keys: [String]) -> String? {
        for key in keys {
            if let value = values[key] as? String {
                let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
                if trimmed.isEmpty == false {
                    return trimmed
                }
            }
        }
        return nil
    }
}

enum SynaraAgentApprovalNotificationActionPlan: Equatable {
    case submitDecision(SynaraAgentApprovalPromptDecisionRequest)
    case openRoom(roomID: String, eventID: String, reason: String)
    case ignore(reason: String)
}

final class SynaraAgentApprovalNotificationActionDedupeStore {
    private let defaults: UserDefaults
    private let storageKey: String
    private let maxStoredKeys = 200

    init(
        defaults: UserDefaults = .standard,
        storageKey: String = "synara.agent-approval.native-action-dedupe"
    ) {
        self.defaults = defaults
        self.storageKey = storageKey
    }

    func contains(_ key: String) -> Bool {
        Set(storedKeys()).contains(key)
    }

    func insert(_ key: String) {
        var keys = storedKeys().filter { $0 != key }
        keys.append(key)
        if keys.count > maxStoredKeys {
            keys = Array(keys.suffix(maxStoredKeys))
        }
        defaults.set(keys, forKey: storageKey)
    }

    func remove(_ key: String) {
        defaults.set(storedKeys().filter { $0 != key }, forKey: storageKey)
    }

    static func key(roomID: String, eventID: String, actionIdentifier _: String) -> String {
        "\(roomID)\u{0}\(eventID)"
    }

    private func storedKeys() -> [String] {
        defaults.stringArray(forKey: storageKey) ?? []
    }
}

protocol MatrixPusherServicing {
    var isGatewayConfigured: Bool { get }
    var configuredGatewayURL: URL? { get }
    func bindPusher(to session: AuthenticatedSession) throws -> MatrixPusherAccountServicing
}

/// A capability retained by the push reconciler for exactly one authenticated
/// Matrix client. Rotation and logout must invoke this bound owner rather than
/// resolving through whichever Core session is current at cleanup time.
protocol MatrixPusherAccountServicing: AnyObject {
    func registerPusher(pushKey: String) async throws
    func unregisterPusher(pushKey: String) async throws
    func unregisterAllPushersForDevice(lastPushKey: String?) async throws
}

struct MatrixPusherRegistrationFailure: Error {
    let statusCode: Int
}

@MainActor
final class SynaraPushService: NSObject, @preconcurrency PushServicing {
    private struct PusherBinding: Equatable {
        let session: AuthenticatedSession
        let sessionSignature: String
        let pushKey: String
        let owner: MatrixPusherAccountServicing

        static func == (lhs: PusherBinding, rhs: PusherBinding) -> Bool {
            lhs.session == rhs.session
                && lhs.sessionSignature == rhs.sessionSignature
                && lhs.pushKey == rhs.pushKey
        }
    }

    private(set) var isRegistered = false
    private(set) var fullDeviceToken: String?
    private var registeredBinding: PusherBinding?
    var pushGatewayURL: String? { pusherService.configuredGatewayURL?.absoluteString }
    var tokenSnippet: String? {
        fullDeviceToken?.prefix(10).description
    }
    private(set) var registrationStateDescription = "Waiting for APNs token"
    private(set) var currentSession: AuthenticatedSession?
    private var currentPusherOwner: MatrixPusherAccountServicing?

    private let pusherService: MatrixPusherServicing
    private let sparseRouteResolver: SparsePushRouteResolving?
    private let logger: LoggingServicing
    private(set) var isRegistrationAvailable: Bool = true
    private var currentSessionSignature: String?
    private var reconciliationRevision: UInt64 = 0
    private var reconciliationTask: Task<Void, Never>?
    private var registrationDiagnosticRunID: UUID?
    private var isRegistrationTeardownInProgress = false
    /// UIKit may rotate the APNs token while logout is suspended on remote or
    /// Keychain work. Keep only the latest value in memory: successful logout
    /// discards it, while a failed local handoff restores the correct token.
    private var pendingDeviceTokenDuringTeardown: String?

    #if targetEnvironment(simulator)
    private let isSimulator = true
    #else
    private let isSimulator = false
    #endif

    init(
        logger: LoggingServicing = AppLogger(),
        pusherService: MatrixPusherServicing? = nil,
        sparseRouteResolver: SparsePushRouteResolving? = nil,
        isRegistrationAvailable: Bool? = nil
    ) {
        self.logger = logger
        self.pusherService = pusherService ?? DisabledMatrixPusherService()
        self.sparseRouteResolver = sparseRouteResolver

        let defaultAvailability = {
            #if targetEnvironment(simulator)
            return false
            #else
            return true
            #endif
        }()

        self.isRegistrationAvailable = isRegistrationAvailable ?? defaultAvailability
        if self.isRegistrationAvailable == false,
           isSimulator {
            registrationStateDescription = "Simulator: APNs unavailable"
        }
    }

    func beginRegistration() {
        guard isRegistrationAvailable else {
            logger.info("Push registration unavailable on this device", category: .push)
            return
        }
        guard isRegistrationTeardownInProgress == false else { return }

        let runID = UUID()
        registrationDiagnosticRunID = runID
        SynaraNotificationDiagnostics.record(.apnsRegistrationRequested, runID: runID)

        #if canImport(UIKit)
        Task { @MainActor in
            UIApplication.shared.registerForRemoteNotifications()
        }
        #endif
    }

    func handleDeviceToken(_ tokenData: Data) {
        guard isRegistrationAvailable else {
            return
        }
        let token = tokenData.map { String(format: "%02.2hhx", $0) }.joined()
        guard isRegistrationTeardownInProgress == false else {
            pendingDeviceTokenDuringTeardown = token
            return
        }
        acceptDeviceToken(token)
    }

    private func acceptDeviceToken(_ token: String) {
        let tokenChanged = fullDeviceToken.map { $0 != token } ?? false
        if tokenChanged {
            registrationStateDescription = "Token changed, re-registering"
            registrationDiagnosticRunID = UUID()
        }

        fullDeviceToken = token
        registrationStateDescription = pusherService.isGatewayConfigured
            ? "Token captured for APNs"
            : "Push gateway not configured"
        logger.info("APNs token captured", category: .push)
        let runID = registrationDiagnosticRunID ?? UUID()
        registrationDiagnosticRunID = runID
        SynaraNotificationDiagnostics.record(.apnsTokenCaptured, runID: runID)
        schedulePusherReconciliation()
    }

    func handleRegistrationFailure() {
        guard isRegistrationTeardownInProgress == false else { return }
        let runID = registrationDiagnosticRunID ?? UUID()
        registrationDiagnosticRunID = runID
        SynaraNotificationDiagnostics.record(.apnsRegistrationFailed, runID: runID)
        registrationStateDescription = isRegistered
            ? "APNs registration failed; existing pusher retained"
            : "APNs registration failed"
    }

    @discardableResult
    func clearRegistrationState() async -> Bool {
        let diagnosticRunID = registrationDiagnosticRunID ?? UUID()
        isRegistrationTeardownInProgress = true
        reconciliationRevision &+= 1
        reconciliationTask?.cancel()
        await reconciliationTask?.value
        reconciliationTask = nil

        var logoutOwner = registeredBinding?.owner ?? currentPusherOwner
        if logoutOwner == nil, let session = currentSession {
            do {
                logoutOwner = try pusherService.bindPusher(to: session)
                currentPusherOwner = logoutOwner
            } catch {
                isRegistered = false
                SynaraNotificationDiagnostics.record(
                    .pusherUnregistrationFailed,
                    runID: diagnosticRunID
                )
                registrationStateDescription = "Push cleanup unavailable; finishing local sign out"
                return false
            }
        }
        if let logoutOwner {
            SynaraNotificationDiagnostics.record(
                .pusherUnregistrationStarted,
                runID: diagnosticRunID
            )
            do {
                // Logout always enumerates exact app+device pushers in Core.
                // This removes stale registrations left by an earlier crash;
                // exact-key deletion remains rotation/supersession-only.
                try await logoutOwner.unregisterAllPushersForDevice(
                    lastPushKey: [registeredBinding?.pushKey, fullDeviceToken]
                        .compactMap { $0 }
                        .first { $0.isEmpty == false }
                )
                SynaraNotificationDiagnostics.record(
                    .pusherUnregistrationSucceeded,
                    runID: diagnosticRunID
                )
            } catch {
                logger.error("Push unregister failed", category: .push)
                SynaraNotificationDiagnostics.record(
                    .pusherUnregistrationFailed,
                    runID: diagnosticRunID
                )
                isRegistered = false
                // Keep registration quiescent while local credentials are
                // removed. Only a failed local deletion may resume this owner.
                registrationStateDescription = "Push cleanup failed; finishing local sign out"
                return false
            }
        }

        registeredBinding = nil
        isRegistered = false
        registrationStateDescription = "Pusher cleanup complete, finishing sign out"
        return true
    }

    func completeRegistrationTeardown() {
        guard isRegistrationTeardownInProgress else { return }
        registeredBinding = nil
        isRegistered = false
        currentSessionSignature = nil
        currentSession = nil
        currentPusherOwner = nil
        fullDeviceToken = nil
        pendingDeviceTokenDuringTeardown = nil
        registrationDiagnosticRunID = nil
        registrationStateDescription = isSimulator ? "Simulator: APNs unavailable" : "Waiting for APNs token"
        // Stay gated while signed out. SessionCoordinator explicitly resumes
        // registration before a later authenticated session is configured.
    }

    func cancelRegistrationTeardown() {
        guard isRegistrationTeardownInProgress else { return }
        abortRegistrationTeardown(
            statusDescription: "Sign out cancelled, restoring push registration"
        )
    }

    private func abortRegistrationTeardown(statusDescription: String) {
        isRegistrationTeardownInProgress = false
        registrationStateDescription = statusDescription
        if let pendingDeviceTokenDuringTeardown {
            self.pendingDeviceTokenDuringTeardown = nil
            acceptDeviceToken(pendingDeviceTokenDuringTeardown)
        } else if fullDeviceToken == nil {
            beginRegistration()
        } else {
            schedulePusherReconciliation()
        }
    }

    func resumeRegistrationLifecycle() {
        isRegistrationTeardownInProgress = false
    }

    func configure(with session: AuthenticatedSession) {
        guard isRegistrationTeardownInProgress == false else { return }
        let nextSignature = sessionSignature(for: session)
        let previousSignature = currentSessionSignature
        if let previousSignature = currentSessionSignature, previousSignature != nextSignature {
            registrationStateDescription = "Session changed, updating push registration"
            registrationDiagnosticRunID = UUID()
        }

        do {
            currentPusherOwner = try pusherService.bindPusher(to: session)
            currentSession = session
            currentSessionSignature = nextSignature
        } catch {
            if previousSignature != nextSignature {
                currentPusherOwner = nil
            }
            currentSession = session
            currentSessionSignature = nextSignature
            if currentPusherOwner == nil {
                isRegistered = false
                registrationStateDescription = "Pusher owner unavailable"
            }
            logger.error("Push owner binding failed", category: .push)
        }
        schedulePusherReconciliation()
    }

    func route(from notificationPayload: [AnyHashable: Any]) -> AppRoute? {
        NotificationPushRouteParser.route(from: notificationPayload)
    }

    func resolveRoute(from notificationPayload: [AnyHashable: Any]) async -> AppRoute? {
        if let route = route(from: notificationPayload) {
            return route
        }

        guard let eventID = NotificationPushRouteParser.sparseEventID(from: notificationPayload),
              let sparseRouteResolver else {
            return nil
        }

        return await sparseRouteResolver.resolveRoute(eventID: eventID)
    }

    func parseBadgeCount(from notificationPayload: [AnyHashable: Any]) -> Int? {
        let flattened = NotificationPushRouteParser.flattenPayload(notificationPayload)
        let candidates: [Any?] = [
            notificationPayload["badge"],
            notificationPayload["badge_count"],
            notificationPayload["synara.badge"],
            flattened["badge"],
            flattened["badge_count"],
            flattened["appBadgeCount"],
            (notificationPayload["aps"] as? [AnyHashable: Any])?["badge"],
            flattened["aps.badge"],
            notificationPayload["notification_summary"],
            flattened["notification_summary"],
            flattened["synara"],
            flattened["synara.notification_summary"],
            flattened["count"]
        ]

        for value in candidates {
            if let badge = badgeCount(from: value) {
                return badge
            }
        }

        return nil
    }

    private func badgeCount(from value: Any?) -> Int? {
        if let parsed = IntValueParser.parse(value) {
            return parsed
        }
        return extractSummaryBadgeCount(from: value)
    }

    private func extractSummaryBadgeCount(from value: Any?) -> Int? {
        guard let summary = value as? [AnyHashable: Any] else {
            return nil
        }

        return IntValueParser.parse(summary["appBadgeCount"]) ??
            IntValueParser.parse(summary["badge"]) ??
            IntValueParser.parse(summary["count"])
    }

    func applyIncomingBadge(from notificationPayload: [AnyHashable: Any]) {
        guard let badge = parseBadgeCount(from: notificationPayload) else {
            return
        }
        let logger = logger

        Task {
            await MainActor.run {
                UNUserNotificationCenter.current().setBadgeCount(badge) { error in
                    if let error {
                        logger.error("Push badge update failed: \(error.localizedDescription)", category: .push)
                    }
                }
            }
        }
    }

    private func schedulePusherReconciliation() {
        guard isRegistrationTeardownInProgress == false else { return }
        reconciliationRevision &+= 1
        guard reconciliationTask == nil else { return }

        reconciliationTask = Task { [weak self] in
            await self?.drainPusherReconciliation()
        }
    }

    private func drainPusherReconciliation() async {
        while Task.isCancelled == false {
            let revision = reconciliationRevision
            await reconcilePusher()
            guard Task.isCancelled == false, revision != reconciliationRevision else {
                break
            }
        }
        reconciliationTask = nil
    }

    private func reconcilePusher() async {
        guard isRegistrationAvailable,
              let token = fullDeviceToken,
              let session = currentSession,
              let sessionSignature = currentSessionSignature else {
            return
        }

        let diagnosticRunID = registrationDiagnosticRunID ?? UUID()
        if registrationDiagnosticRunID == nil {
            registrationDiagnosticRunID = diagnosticRunID
        }
        guard pusherService.isGatewayConfigured else {
            registrationStateDescription = "Push gateway not configured"
            SynaraNotificationDiagnostics.record(
                .pusherGatewayUnavailable,
                runID: diagnosticRunID
            )
            return
        }

        if let binding = registeredBinding,
           bindingMatchesDesired(
               binding,
               session: session,
               sessionSignature: sessionSignature,
               pushKey: token
           ) {
            isRegistered = true
            return
        }

        if let previousBinding = registeredBinding {
            registrationStateDescription = "Replacing previous push registration"
            SynaraNotificationDiagnostics.record(
                .pusherUnregistrationStarted,
                runID: diagnosticRunID
            )
            do {
                try await previousBinding.owner.unregisterPusher(pushKey: previousBinding.pushKey)
                if registeredBinding == previousBinding {
                    registeredBinding = nil
                    isRegistered = false
                }
                SynaraNotificationDiagnostics.record(
                    .pusherUnregistrationSucceeded,
                    runID: diagnosticRunID
                )
            } catch {
                logger.error("Push unregister failed during rotation", category: .push)
                isRegistered = false
                registrationStateDescription = "Previous pusher cleanup failed"
                SynaraNotificationDiagnostics.record(
                    .pusherUnregistrationFailed,
                    runID: diagnosticRunID
                )
                // Keep the exact old binding so a later trigger or logout can
                // retry deletion. Never overwrite it with a new registration
                // and lose the only credentials that can remove it.
                return
            }
        }

        guard Task.isCancelled == false,
              currentSession == session,
              currentSessionSignature == sessionSignature,
              fullDeviceToken == token else {
            return
        }
        guard let owner = currentPusherOwner else {
            isRegistered = false
            registrationStateDescription = "Pusher owner unavailable"
            return
        }
        let desiredBinding = PusherBinding(
            session: session,
            sessionSignature: sessionSignature,
            pushKey: token,
            owner: owner
        )
        guard
              isDesired(desiredBinding) else {
            return
        }

        SynaraNotificationDiagnostics.record(
            .pusherRegistrationStarted,
            runID: diagnosticRunID
        )
        do {
            try await desiredBinding.owner.registerPusher(pushKey: token)
            if Task.isCancelled == false,
               isDesired(desiredBinding) {
                registeredBinding = desiredBinding
                isRegistered = true
                registrationStateDescription = "Pusher registration complete"
                SynaraNotificationDiagnostics.record(
                    .pusherRegistrationSucceeded,
                    runID: diagnosticRunID
                )
            } else {
                // The target changed while registration was in flight. Remove
                // the stale binding with the exact session that created it;
                // the drain loop will then reconcile the latest target.
                SynaraNotificationDiagnostics.record(
                    .pusherRegistrationSuperseded,
                    runID: diagnosticRunID
                )
                SynaraNotificationDiagnostics.record(
                    .pusherUnregistrationStarted,
                    runID: diagnosticRunID
                )
                do {
                    try await desiredBinding.owner.unregisterPusher(pushKey: desiredBinding.pushKey)
                    SynaraNotificationDiagnostics.record(
                        .pusherUnregistrationSucceeded,
                        runID: diagnosticRunID
                    )
                } catch {
                    logger.error("Stale push registration cleanup failed", category: .push)
                    registeredBinding = desiredBinding
                    isRegistered = false
                    registrationStateDescription = "Stale pusher cleanup failed"
                    SynaraNotificationDiagnostics.record(
                        .pusherUnregistrationFailed,
                        runID: diagnosticRunID
                    )
                }
            }
        } catch {
            logger.error("Push registration failed", category: .push)
            isRegistered = false
            registrationStateDescription = "Pusher registration failed"
            SynaraNotificationDiagnostics.record(
                .pusherRegistrationFailed,
                runID: diagnosticRunID
            )
        }
    }

    private func sessionSignature(for session: AuthenticatedSession) -> String {
        "\(session.userID)|\(session.deviceID)|\(session.homeserverURL.absoluteString)"
    }

    private func isDesired(_ binding: PusherBinding) -> Bool {
        currentSession == binding.session
            && currentSessionSignature == binding.sessionSignature
            && fullDeviceToken == binding.pushKey
    }

    private func bindingMatchesDesired(
        _ binding: PusherBinding,
        session: AuthenticatedSession,
        sessionSignature: String,
        pushKey: String
    ) -> Bool {
        binding.session == session
            && binding.sessionSignature == sessionSignature
            && binding.pushKey == pushKey
    }

    private func currentDesiredBinding() -> PusherBinding? {
        guard let session = currentSession,
              let sessionSignature = currentSessionSignature,
              let pushKey = fullDeviceToken,
              let owner = currentPusherOwner else {
            return nil
        }
        return PusherBinding(
            session: session,
            sessionSignature: sessionSignature,
            pushKey: pushKey,
            owner: owner
        )
    }

}

enum IntValueParser {
    static func parse(_ value: Any?) -> Int? {
        switch value {
        case let intValue as Int:
            return intValue
        case let stringValue as String:
            return Int(stringValue)
        case let number as NSNumber:
            return number.intValue
        case let doubleValue as Double:
            return Int(doubleValue)
        default:
            return nil
        }
    }
}

struct NotificationPayloadAlertShape: Equatable {
    let alertKind: String
    let hasTitle: Bool
    let titleLength: Int
    let hasBody: Bool
    let bodyLength: Int
    let category: String?
    let hasRoomID: Bool
    let hasEventID: Bool
    let synaraKind: String?
    let contentAvailable: Bool
    let mutableContent: Bool

    var logSummary: String {
        [
            "alert=\(alertKind)",
            "title=\(hasTitle ? "present:\(titleLength)" : "missing")",
            "body=\(hasBody ? "present:\(bodyLength)" : "missing")",
            "category=\(category ?? "none")",
            "room_id=\(hasRoomID ? "present" : "missing")",
            "event_id=\(hasEventID ? "present" : "missing")",
            "synara_kind=\(synaraKind ?? "none")",
            "content_available=\(contentAvailable)",
            "mutable_content=\(mutableContent)"
        ].joined(separator: " ")
    }
}

private struct DisabledMatrixPusherService: MatrixPusherServicing {
    var isGatewayConfigured: Bool { false }
    var configuredGatewayURL: URL? { nil }

    func bindPusher(to session: AuthenticatedSession) throws -> MatrixPusherAccountServicing {
        _ = session
        return DisabledMatrixPusherAccountService()
    }
}

private final class DisabledMatrixPusherAccountService: MatrixPusherAccountServicing {
    func registerPusher(pushKey: String) async throws {
        _ = pushKey
    }

    func unregisterPusher(pushKey: String) async throws {
        _ = pushKey
    }

    func unregisterAllPushersForDevice(lastPushKey: String?) async throws {
        _ = lastPushKey
    }
}

enum NotificationPushRouteParser {
    static func alertShape(from payload: [AnyHashable: Any]) -> NotificationPayloadAlertShape {
        let aps = payload["aps"] as? [AnyHashable: Any]
        let alertValue = aps?["alert"]
        let alertKind: String
        let title: String?
        let body: String?

        if let alert = alertValue as? String {
            alertKind = "string"
            title = nil
            body = alert
        } else if let alert = alertValue as? [AnyHashable: Any] {
            alertKind = "dictionary"
            title = alert["title"] as? String
            body = alert["body"] as? String
        } else if alertValue == nil {
            alertKind = "missing"
            title = nil
            body = nil
        } else {
            alertKind = "other"
            title = nil
            body = nil
        }

        let normalizedTitle = title?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let normalizedBody = body?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let candidates = flattenPayload(payload)
        let synara = payload["synara"] as? [AnyHashable: Any]

        return NotificationPayloadAlertShape(
            alertKind: alertKind,
            hasTitle: normalizedTitle.isEmpty == false,
            titleLength: normalizedTitle.count,
            hasBody: normalizedBody.isEmpty == false,
            bodyLength: normalizedBody.count,
            category: aps?["category"] as? String,
            hasRoomID: (candidates["room_id"] as? String)?.isEmpty == false || (candidates["roomId"] as? String)?.isEmpty == false,
            hasEventID: (candidates["event_id"] as? String)?.isEmpty == false || (candidates["eventId"] as? String)?.isEmpty == false,
            synaraKind: synara?["kind"] as? String,
            contentAvailable: IntValueParser.parse(aps?["content-available"]) == 1,
            mutableContent: IntValueParser.parse(aps?["mutable-content"]) == 1
        )
    }

    static func route(from payload: [AnyHashable: Any]) -> AppRoute? {
        let candidates = flattenPayload(payload)

        if let roomID = candidates["room_id"] as? String {
            let eventID = candidates["event_id"] as? String
            return AppRoute.room(id: roomID, eventID: eventID)
        }

        if let route = candidates["route"] as? String,
           let parsed = parseRoute(route) {
            return parsed
        }

        if let routePath = candidates["synara.route"] as? String,
           let parsed = parseRoute(routePath) {
            return parsed
        }

        if let route = candidates["roomId"] as? String,
           let eventID = candidates["eventId"] as? String {
            return AppRoute.room(id: route, eventID: eventID)
        }

        if let route = candidates["roomId"] as? String {
            return AppRoute.room(id: route)
        }

        return nil
    }

    static func sparseEventID(from payload: [AnyHashable: Any]) -> String? {
        let candidates = flattenPayload(payload)
        if let roomID = candidates["room_id"] as? String, roomID.isEmpty == false {
            return nil
        }
        if let roomID = candidates["roomId"] as? String, roomID.isEmpty == false {
            return nil
        }

        let eventCandidates: [Any?] = [
            payload["event_id"],
            payload["eventId"],
            candidates["event_id"],
            candidates["eventId"]
        ]

        for value in eventCandidates {
            if let eventID = value as? String, eventID.isEmpty == false {
                return eventID
            }
        }

        return nil
    }

    static func flattenPayload(_ payload: [AnyHashable: Any]) -> [String: Any] {
        var result: [String: Any] = [:]

        for (key, value) in payload {
            let keyString = key as? String

            if let keyString {
                result[keyString] = value
            }

            if let keyString,
               keyStringIsContainer(keyString),
               let nested = value as? [AnyHashable: Any] {
                result.merge(flattenPayload(nested)) { _, new in new }
            }
        }

        return result
    }

    private static func parseRoute(_ route: String) -> AppRoute? {
        if let routeURL = URL(string: route),
           let direct = directRoute(from: routeURL) {
            return direct
        }

        if let parsed = parseRoutePath(route) {
            return parsed
        }

        return nil
    }

    private static func directRoute(from routeURL: URL) -> AppRoute? {
        if let parsed = AppDeepLink(url: routeURL) {
            return parsed.toAppRoute()
        }

        guard let host = routeURL.host?.lowercased() else {
            return nil
        }

        if host == "route" {
            let encodedPath = String(routeURL.path.dropFirst())
           
            return parseRoutePath(encodedPath)
        }

        if host == "synara.app" {
            let pathSegments = routeURL.path.split(separator: "/").map(String.init)
            if pathSegments.first == "r", pathSegments.count > 1 {
                return parseRoutePath(pathSegments.dropFirst().joined(separator: "/"))
            }
        }

        return nil
    }

    private static func parseRoutePath(_ routePath: String) -> AppRoute? {
        let normalized = routePath.hasPrefix("/") ? routePath : "/\(routePath)"
        let decoded = normalized.removingPercentEncoding ?? normalized
        let routeURL = URL(string: "synara://host\(decoded)")
        if let parsed = routeURL.flatMap({ AppDeepLink(url: $0)?.toAppRoute() }) {
            return parsed
        }

        return nil
    }

    private static func keyStringIsContainer(_ keyString: String) -> Bool {
        let containerKeys: Set<String> = ["content", "synara", "notification", "room"]
        return containerKeys.contains(keyString)
    }
}

private extension AppDeepLink {
    func toAppRoute() -> AppRoute {
        switch self {
        case .room(let id, let eventID):
            return .room(id: id, eventID: eventID)
        case .settings:
            return .settings
        case .notifications:
            return .notifications
        case .later:
            return .later
        }
    }
}
