import Foundation
import SynaraCore

/// P4-S8 typed verification inbox. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps `matrix_verification_list` only. It is not a generic
/// `Core.command` FFI, not SAS start/accept/confirm, and not a product swap.
enum SharedCoreVerificationList {
    static func verificationList(core: SharedCore) async throws -> VerificationInboxDto {
        try await core.verificationList()
    }
}
