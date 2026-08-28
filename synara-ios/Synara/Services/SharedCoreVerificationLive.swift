import Foundation
import SynaraCore

/// P4-S20 map of privacy-safe SharedCore verification rows to product state.
///
/// Uses list/SAS commands only. No tokens, MACs, or recovery secrets.
/// This is not iOS-on-engine and not P4 acceptance.
enum SharedCoreVerificationLive {
    static func isTerminal(phase: String) -> Bool {
        phase == "done" || phase == "mismatched" || phase == "cancelled" || phase == "failed"
    }

    static func selectedFlowId(
        requests: [(flowId: String, phase: String)],
        preferring preferredFlowId: String?
    ) -> String? {
        if let preferredFlowId, requests.contains(where: { $0.flowId == preferredFlowId }) {
            return preferredFlowId
        }
        return requests.first { isTerminal(phase: $0.phase) == false }?.flowId
            ?? requests.first?.flowId
    }

    static func selectRequest(
        from inbox: VerificationInboxDto,
        preferring flowId: String?
    ) -> VerificationRequestDto? {
        guard let selected = selectedFlowId(
            requests: inbox.requests.map { ($0.flowId, $0.phase) },
            preferring: flowId
        ) else {
            return nil
        }
        return inbox.requests.first { $0.flowId == selected }
    }

    static func state(from inbox: VerificationInboxDto, preferring flowId: String? = nil) -> CryptoVerificationState? {
        guard let request = selectRequest(from: inbox, preferring: flowId) else {
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
            return needsSasStart(phase: phase, direction: direction) ? .accepted : .sasStarted
        case "started":
            // The Rust owner accepts every transitioned SAS handle. Started is
            // therefore observation-only; the UI must never own protocol accept.
            return .sasStarted
        case "keys_exchanging":
            return .keysExchanging
        case "sas_ready":
            if emoji.isEmpty == false {
                return .emojis(emoji.map { CryptoVerificationEmoji(symbol: $0.symbol, description: $0.description) })
            }
            if decimals.count == 3 {
                return .decimals(decimals)
            }
            // `sas_ready` without a comparison payload violates the native/FFI
            // contract. Surface the failure instead of leaving the user in an
            // endless waiting state with no possible confirmation action.
            return .failed
        case "confirmed":
            return .confirmed
        case "done":
            return .finished
        case "cancelled":
            return .cancelled
        case "mismatched":
            return .mismatched
        case "failed":
            return .failed
        default:
            return .failed
        }
    }

    /// Same rule as desktop `verificationRequestNeedsSasStart`.
    static func needsSasStart(phase: String, direction: String) -> Bool {
        direction == "outgoing" && phase == "ready"
    }
}
