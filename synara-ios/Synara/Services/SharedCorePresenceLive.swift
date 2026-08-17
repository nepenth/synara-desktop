import Foundation

/// P4-S37 map of privacy-safe SharedCore presence snapshots to product rows.
///
/// Uses the existing presence snapshot only. No tokens. This is not
/// iOS-on-engine and not P4 acceptance.
struct SharedCorePresence: Equatable, Identifiable {
    let userID: String
    let state: String
    let currentlyActive: Bool
    let statusMessage: String?

    var id: String { userID }

    var displayName: String {
        let trimmed = state.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty || trimmed == "unknown" {
            return currentlyActive ? "Active" : "Unknown"
        }
        return trimmed.replacingOccurrences(of: "_", with: " ").capitalized
    }
}

enum SharedCorePresenceLive {
    static func presence(
        userId: String,
        state: String?,
        currentlyActive: Bool,
        statusMsg: String?
    ) -> SharedCorePresence {
        let normalized = state?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return SharedCorePresence(
            userID: userId,
            state: normalized.isEmpty ? "unknown" : normalized,
            currentlyActive: currentlyActive,
            statusMessage: statusMsg?.trimmingCharacters(in: .whitespacesAndNewlines)
        )
    }
}
