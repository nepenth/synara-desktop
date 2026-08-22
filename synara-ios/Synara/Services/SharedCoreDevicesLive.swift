import Foundation

/// P4-S34 map of privacy-safe SharedCore device snapshot rows to Settings.
///
/// Uses the existing device snapshot only. No keys, tokens, or IP echo on
/// the product row. This is not iOS-on-engine and not P4 acceptance.
struct SharedCoreSessionDevice: Equatable, Identifiable {
    let id: String
    let displayName: String
    let isCurrent: Bool
    let trust: String
    let lastSeenTs: UInt64?
}

enum SharedCoreDevicesLive {
    static func devices(
        deviceId: String,
        displayName: String?,
        isCurrent: Bool,
        trust: String,
        lastSeenTs: UInt64? = nil
    ) -> SharedCoreSessionDevice {
        let name = displayName?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return SharedCoreSessionDevice(
            id: deviceId,
            displayName: name.isEmpty ? deviceId : name,
            isCurrent: isCurrent,
            trust: trust,
            lastSeenTs: lastSeenTs
        )
    }

    static func trustDisplayName(_ trust: String) -> String {
        switch trust {
        case "verified":
            return "Verified"
        case "unverified":
            return "Unverified"
        default:
            return "Unknown"
        }
    }

    static func lastActivityDisplay(lastSeenTs: UInt64?, now: Date = Date()) -> String? {
        guard let lastSeenTs else { return nil }
        let seconds = TimeInterval(lastSeenTs) / 1000
        guard seconds.isFinite, seconds > 0 else { return nil }
        let lastSeen = Date(timeIntervalSince1970: seconds)
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .full
        return "Last activity \(formatter.localizedString(for: lastSeen, relativeTo: now))"
    }
}
