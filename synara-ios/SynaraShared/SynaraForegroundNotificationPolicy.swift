import Foundation

enum SynaraForegroundNotificationPolicy {
    /// Only an unambiguous, exact Matrix event reference may suppress an alert.
    /// Never infer a sender or category from gateway hints or route strings.
    static func target(from userInfo: [AnyHashable: Any]) -> (roomID: String, eventID: String)? {
        guard let reference = try? SynaraNotificationPreviewPayloadParser.reference(
            from: userInfo, trimWhitespace: false
        ).get() else { return nil }
        return (reference.roomID, reference.eventID)
    }

    static func shouldSuppress(
        eventAllowed: Bool,
        foregroundActive: Bool,
        sameSession: Bool,
        initialAccountEpoch: Int,
        currentAccountEpoch: Int
    ) -> Bool {
        !eventAllowed && foregroundActive && sameSession && initialAccountEpoch == currentAccountEpoch
    }

}
