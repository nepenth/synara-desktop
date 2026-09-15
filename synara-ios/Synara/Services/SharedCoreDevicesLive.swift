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
    let isCrossSignedByOwner: Bool
    let firstSeenTs: UInt64?
    let ed25519Fingerprint: String?
}

enum SharedCoreDevicesLive {
    static func devices(
        deviceId: String,
        displayName: String?,
        isCurrent: Bool,
        trust: String,
        lastSeenTs: UInt64? = nil,
        isCrossSignedByOwner: Bool = false,
        firstSeenTs: UInt64? = nil,
        ed25519Fingerprint: String? = nil
    ) -> SharedCoreSessionDevice {
        let name = displayName?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let fingerprint = ed25519Fingerprint?.trimmingCharacters(in: .whitespacesAndNewlines)
        return SharedCoreSessionDevice(
            id: deviceId,
            displayName: name.isEmpty ? deviceId : name,
            isCurrent: isCurrent,
            trust: trust,
            lastSeenTs: lastSeenTs,
            isCrossSignedByOwner: isCrossSignedByOwner,
            firstSeenTs: firstSeenTs,
            ed25519Fingerprint: (fingerprint?.isEmpty == false) ? fingerprint : nil
        )
    }

    static func trustDisplayName(_ trust: String) -> String {
        switch trust {
        case "verified", "verified_locally_only":
            return "Verified"
        case "unverified":
            return "Unverified"
        case "dehydrated":
            return "Backup device"
        case "no_encryption", "unsupported":
            return "Not encrypted"
        default:
            return "Unknown"
        }
    }

    static func lastActivityDisplay(lastSeenTs: UInt64?, now: Date = Date()) -> String? {
        relativeTimestampDisplay(prefix: "Last activity", timestampMs: lastSeenTs, now: now)
    }

    static func firstSeenDisplay(firstSeenTs: UInt64?, now: Date = Date()) -> String? {
        relativeTimestampDisplay(prefix: "First seen", timestampMs: firstSeenTs, now: now)
    }

    private static func relativeTimestampDisplay(
        prefix: String,
        timestampMs: UInt64?,
        now: Date
    ) -> String? {
        guard let timestampMs else { return nil }
        let seconds = TimeInterval(timestampMs) / 1000
        guard seconds.isFinite, seconds > 0 else { return nil }
        let date = Date(timeIntervalSince1970: seconds)
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .full
        return "\(prefix) \(formatter.localizedString(for: date, relativeTo: now))"
    }
}
