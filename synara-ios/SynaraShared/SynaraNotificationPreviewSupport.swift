import Foundation

enum SynaraSharedConstants {
    static let appGroupIdentifier = "group.com.whylandcreative.synara"
    static let keychainAccessGroupInfoKey = "SynaraKeychainAccessGroup"
    static let lockScreenMessagePreviewsKey = "synara.settings.lockScreenMessagePreviews"
    static let defaultLockScreenMessagePreviews = false

    static var registeredUserDefaults: [String: Any] {
        [
            lockScreenMessagePreviewsKey: defaultLockScreenMessagePreviews
        ]
    }

    static func appGroupDefaults() -> UserDefaults? {
        UserDefaults(suiteName: appGroupIdentifier)
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
