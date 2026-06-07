import Foundation
import UserNotifications

#if canImport(UIKit)
import UIKit
#endif

protocol MatrixPusherServicing {
    var isGatewayConfigured: Bool { get }
    var configuredGatewayURL: URL? { get }
    func registerPusher(session: AuthenticatedSession, pushKey: String) async throws
    func unregisterPusher(session: AuthenticatedSession, pushKey: String) async throws
}

struct MatrixPusherRegistrationFailure: Error {
    let statusCode: Int
}

final class MatrixPusherService: MatrixPusherServicing {
    private let clientStore: MatrixRustSDKClientStore
    private let gatewayURL: URL?
    private let appID: String
    private let logger: LoggingServicing

    var isGatewayConfigured: Bool {
        gatewayURL != nil
    }

    var configuredGatewayURL: URL? {
        gatewayURL
    }

    init(
        clientStore: MatrixRustSDKClientStore,
        appID: String = "com.whylandcreative.synara",
        gatewayURL: URL? = nil,
        logger: LoggingServicing = AppLogger()
    ) {
        self.clientStore = clientStore
        self.appID = appID
        self.gatewayURL = gatewayURL
        self.logger = logger
    }

    func registerPusher(session: AuthenticatedSession, pushKey: String) async throws {
        guard let gatewayURL else {
            logger.info("Push gateway URL is not configured; skipping pusher registration", category: .push)
            return
        }

        guard gatewayURL.scheme?.lowercased() == "https",
              gatewayURL.host?.isEmpty == false else {
            logger.info("Push gateway URL is not configured; skipping pusher registration", category: .push)
            return
        }

        try await clientStore.setPusher(
            pushKey: pushKey,
            appID: appID,
            gatewayURL: gatewayURL,
            appDisplayName: "Synara",
            deviceDisplayName: session.deviceID,
            lang: "en-US",
            session: session
        )
        logger.info("Push pusher registered", category: .push)
    }

    func unregisterPusher(session: AuthenticatedSession, pushKey: String) async throws {
        guard let gatewayURL else {
            logger.info("Push gateway URL is not configured; skipping pusher unregister", category: .push)
            return
        }

        guard gatewayURL.scheme?.lowercased() == "https",
              gatewayURL.host?.isEmpty == false else {
            logger.info("Push gateway URL is not configured; skipping pusher unregister", category: .push)
            return
        }

        try await clientStore.deletePusher(
            pushKey: pushKey,
            appID: appID,
            session: session
        )
        logger.info("Push pusher unregistered", category: .push)
    }
}

final class SynaraPushService: NSObject, PushServicing {
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
        isRegistrationAvailable: Bool? = nil
    ) {
        self.logger = logger
        self.pusherService = pusherService ?? DisabledMatrixPusherService()

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

    func clearRegistrationState() {
        let session = currentSession
        let pushKey = sessionBoundPushKey ?? fullDeviceToken
        Task {
            if let session, let pushKey {
                do {
                    try await pusherService.unregisterPusher(session: session, pushKey: pushKey)
                } catch {
                    logger.error("Push unregister failed", category: .push)
                }
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
            if let parsed = IntValueParser.parse(value) {
                return parsed
            }

            if let parsed = extractSummaryBadgeCount(from: value) {
                return parsed
            }
        }

        return nil
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

private struct DisabledMatrixPusherService: MatrixPusherServicing {
    var isGatewayConfigured: Bool { false }
    var configuredGatewayURL: URL? { nil }

    func registerPusher(session: AuthenticatedSession, pushKey: String) async throws {}
    func unregisterPusher(session: AuthenticatedSession, pushKey: String) async throws {}
}

enum NotificationPushRouteParser {
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
