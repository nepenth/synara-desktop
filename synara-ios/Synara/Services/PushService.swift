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
    /// Max age for acting on a native/push approval notification without in-app confirmation.
    static let nativeActionTTL: TimeInterval = 5 * 60

    static func registerCategories(
        center: UNUserNotificationCenter = UNUserNotificationCenter.current()
    ) {
        // Approve-always is intentionally omitted from the native category: permanent
        // approval requires an explicit in-app confirmation path. Keep the two
        // time-critical decisions first for compact notification surfaces;
        // tapping the notification body remains the primary Review path.
        let actions = [
            UNNotificationAction(
                identifier: approveOnceIdentifier,
                title: "Approve once",
                options: [.authenticationRequired]
            ),
            UNNotificationAction(
                identifier: denyIdentifier,
                title: "Deny",
                options: [.authenticationRequired, .destructive]
            ),
            UNNotificationAction(
                identifier: reviewIdentifier,
                title: "Review",
                options: [.foreground]
            )
        ]

        let category = UNNotificationCategory(
            identifier: agentApprovalCategoryIdentifier,
            actions: actions,
            intentIdentifiers: [],
            options: []
        )
        center.setNotificationCategories([category])
    }

    /// Plans how a native/push notification action should be handled.
    /// Does not send Matrix traffic; reaction callers must use the shared-core
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

        guard let reactionKey = action.reactionKey else {
            return .ignore(reason: "unsupported-action")
        }

        return .submitReaction(
            SynaraAgentApprovalReactionRequest(
                roomID: roomID,
                sourceEventID: eventID,
                reactionKey: reactionKey
            )
        )
    }

    /// Backward-compatible parser used by unit tests and callers that only need reaction payloads.
    /// Returns nil for approve-always and malformed/expired payloads.
    static func agentApprovalReactionRequest(
        actionIdentifier: String,
        userInfo: [AnyHashable: Any],
        now: Date = Date(),
        alreadyActed: Bool = false
    ) -> SynaraAgentApprovalReactionRequest? {
        if case .submitReaction(let request) = planAgentApprovalNotificationAction(
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
    case submitReaction(SynaraAgentApprovalReactionRequest)
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
    func registerPusher(session: AuthenticatedSession, pushKey: String) async throws
    func unregisterPusher(session: AuthenticatedSession, pushKey: String) async throws
}

struct MatrixPusherRegistrationFailure: Error {
    let statusCode: Int
}

@MainActor
final class SynaraPushService: NSObject, @preconcurrency PushServicing {
    private(set) var isRegistered = false
    private(set) var fullDeviceToken: String?
    private var sessionBoundPushKey: String?
    var pushGatewayURL: String? { pusherService.configuredGatewayURL?.absoluteString }
    var tokenSnippet: String? {
        fullDeviceToken?.prefix(10).description
    }
    private(set) var registrationStateDescription = "Waiting for APNs token"
    private(set) var currentSession: AuthenticatedSession?

    private let pusherService: MatrixPusherServicing
    private let sparseRouteResolver: SparsePushRouteResolving?
    private let logger: LoggingServicing
    private(set) var isRegistrationAvailable: Bool = true
    private var currentSessionSignature: String?

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
        if let existingToken = fullDeviceToken, existingToken != token {
            registrationStateDescription = "Token changed, re-registering"
        }

        fullDeviceToken = token
        registrationStateDescription = pusherService.isGatewayConfigured
            ? "Token captured for APNs"
            : "Push gateway not configured"
        logger.info("APNs token captured", category: .push)

        Task {
            await registerWithMatrixIfPossible()
        }
    }

    func clearRegistrationState() async {
        let session = currentSession
        let pushKey = sessionBoundPushKey ?? fullDeviceToken
        if let session, let pushKey {
            do {
                try await pusherService.unregisterPusher(session: session, pushKey: pushKey)
            } catch {
                logger.error("Push unregister failed", category: .push)
            }
        }

        isRegistered = false
        sessionBoundPushKey = nil
        currentSessionSignature = nil
        currentSession = nil
        fullDeviceToken = nil
        registrationStateDescription = isSimulator ? "Simulator: APNs unavailable" : "Waiting for APNs token"
    }

    func configure(with session: AuthenticatedSession) {
        let nextSignature = sessionSignature(for: session)
        if let previousSignature = currentSessionSignature, previousSignature != nextSignature {
            isRegistered = false
            sessionBoundPushKey = nil
            registrationStateDescription = "Session changed, updating push registration"
        }

        currentSession = session
        currentSessionSignature = nextSignature
        Task {
            await registerWithMatrixIfPossible()
        }
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

    private func registerWithMatrixIfPossible() async {
        guard isRegistrationAvailable,
              let token = fullDeviceToken,
              let session = currentSession else {
            return
        }

        guard pusherService.isGatewayConfigured else {
            registrationStateDescription = "Push gateway not configured"
            return
        }

        if isRegistered {
            if let sessionBoundPushKey, sessionBoundPushKey == token {
                return
            }

            if let currentSession {
                do {
                    try await pusherService.unregisterPusher(session: currentSession, pushKey: sessionBoundPushKey ?? token)
                    isRegistered = false
                    registrationStateDescription = "Replacing previous push registration"
                } catch {
                    logger.error("Push unregister failed during rotation: \(error)", category: .push)
                    isRegistered = false
                }
            }
        }

        do {
            try await pusherService.registerPusher(session: session, pushKey: token)
            sessionBoundPushKey = token
            isRegistered = true
            registrationStateDescription = "Pusher registration complete"
        } catch {
            logger.error("Push registration failed: \(error)", category: .push)
            isRegistered = false
            registrationStateDescription = "Pusher registration failed"
        }
    }

    private func sessionSignature(for session: AuthenticatedSession) -> String {
        "\(session.userID)|\(session.deviceID)|\(session.homeserverURL.absoluteString)"
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

    func registerPusher(session: AuthenticatedSession, pushKey: String) async throws {}
    func unregisterPusher(session: AuthenticatedSession, pushKey: String) async throws {}
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
