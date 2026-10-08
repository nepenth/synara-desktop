import Foundation
import SynaraCore

enum CryptoVerificationPresentationPolicy {
    static func allowsInteractiveDismiss(_ state: CryptoVerificationState?) -> Bool {
        state?.isTerminal == true
    }

}

enum SecuritySettingsVerificationPolicy {
    static func showsVerifyThisDevice(_ status: SessionCryptoStatus) -> Bool {
        status.verification != .verified
    }

    static func enablesVerifyThisDevice(_ status: SessionCryptoStatus) -> Bool {
        status.verification != .verified && status.hasDevicesToVerifyAgainst == true
    }

    static func availabilityMessage(_ status: SessionCryptoStatus) -> String {
        switch status.hasDevicesToVerifyAgainst {
        case true:
            return "Compare emoji or number codes with another verified session. Synara does not mark this device verified until both sides confirm."
        case false:
            return "No eligible verified session is available yet. Open Synara or Element on a device that already verified this account, then refresh."
        case nil:
            return "Synara could not check eligible verified sessions. Check your connection and retry."
        }
    }
}

enum RoomUnreadPresentation {
    static func make(
        membership: RoomSummary.Membership,
        numUnreadMessages: UInt64 = 0,
        numUnreadNotifications: UInt64 = 0,
        numUnreadMentions: UInt64 = 0,
        isMarkedUnread: Bool = false
    ) -> (unreadCount: Int, hasHighlight: Bool) {
        let coreMembership: SynaraCore.RoomUnreadMembership
        switch membership {
        case .joined:
            coreMembership = .joined
        case .invited:
            coreMembership = .invited
        }

        let projection = SynaraCore.roomUnreadPresentation(
            membership: coreMembership,
            numUnreadMessages: numUnreadMessages,
            numUnreadNotifications: numUnreadNotifications,
            numUnreadMentions: numUnreadMentions,
            isMarkedUnread: isMarkedUnread
        )
        return (Int(projection.unreadCount), projection.hasHighlight)
    }
}

