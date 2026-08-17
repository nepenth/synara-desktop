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
}

enum SharedCoreDevicesLive {
    static func devices(
        deviceId: String,
        displayName: String?,
        isCurrent: Bool,
        trust: String
    ) -> SharedCoreSessionDevice {
        let name = displayName?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return SharedCoreSessionDevice(
            id: deviceId,
            displayName: name.isEmpty ? deviceId : name,
            isCurrent: isCurrent,
            trust: trust
        )
    }
}
