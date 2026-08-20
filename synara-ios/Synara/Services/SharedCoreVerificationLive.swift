import Foundation
import SynaraCore

/// P4-S20 map of privacy-safe SharedCore verification rows to product state.
///
/// Uses list/SAS commands only. No tokens, MACs, or recovery secrets.
/// This is not iOS-on-engine and not P4 acceptance.
enum SharedCoreVerificationLive {
    static func state(from inbox: VerificationInboxDto) -> CryptoVerificationState? {
        guard let request = inbox.requests.first else {
            return nil
        }
        return state(from: request)
    }

    static func state(from request: VerificationRequestDto) -> CryptoVerificationState {
        state(
            phase: request.phase,
            direction: request.direction,
            flowId: request.flowId,
            otherUserId: request.otherUserId,
            otherDeviceId: request.otherDeviceId,
            emoji: request.sas?.emoji ?? [],
            decimals: request.sas?.decimals ?? []
        )
    }

    static func state(
        phase: String,
        direction: String,
        flowId: String,
        otherUserId: String,
        otherDeviceId: String?,
        emoji: [VerificationEmojiDto] = [],
        decimals: [UInt16] = []
    ) -> CryptoVerificationState {
        switch phase {
        case "requested":
            if direction == "incoming" {
                return .requestReceived(
                    CryptoVerificationRequest(
                        userID: otherUserId,
                        displayName: nil,
                        deviceID: otherDeviceId ?? "",
                        deviceDisplayName: nil,
                        flowID: flowId
                    )
                )
            }
            return .requestSent
        case "ready":
            return .accepted
        case "started":
            return .sasStarted
        case "sas_ready":
            if emoji.isEmpty == false {
                return .emojis(emoji.map { CryptoVerificationEmoji(symbol: $0.symbol, description: $0.description) })
            }
            if decimals.count == 3 {
                return .decimals(decimals)
            }
            return .sasStarted
        case "confirmed":
            return .confirmed
        case "done":
            return .finished
        case "cancelled":
            return .cancelled
        case "mismatched":
            return .mismatched
        default:
            return .failed
        }
    }
}
