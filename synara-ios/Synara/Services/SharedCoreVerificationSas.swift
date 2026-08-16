import Foundation
import SynaraCore

/// P4-S9 typed verification SAS. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps start/accept/begin_sas/confirm/mismatch/cancel/dismiss only.
/// It is not a generic `Core.command` FFI, not the S8 list, and not a product swap.
enum SharedCoreVerificationSas {
    static func verificationStart(core: SharedCore, deviceId: String?) async throws -> VerificationRequestDto {
        try await core.verificationStart(deviceId: deviceId)
    }

    static func verificationAccept(core: SharedCore, flowId: String) async throws -> VerificationRequestDto {
        try await core.verificationAccept(flowId: flowId)
    }

    static func verificationBeginSas(core: SharedCore, flowId: String) async throws -> VerificationRequestDto {
        try await core.verificationBeginSas(flowId: flowId)
    }

    static func verificationConfirm(core: SharedCore, flowId: String) async throws -> VerificationRequestDto {
        try await core.verificationConfirm(flowId: flowId)
    }

    static func verificationMismatch(core: SharedCore, flowId: String) async throws -> VerificationRequestDto {
        try await core.verificationMismatch(flowId: flowId)
    }

    static func verificationCancel(core: SharedCore, flowId: String) async throws -> VerificationRequestDto {
        try await core.verificationCancel(flowId: flowId)
    }

    static func verificationDismiss(core: SharedCore, flowId: String) async throws {
        try await core.verificationDismiss(flowId: flowId)
    }
}
