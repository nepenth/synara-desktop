import Foundation
import SynaraCore

/// P4-S9-8 typed own display-name / avatar. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the two registered own-profile commands only.
/// Avatar is an `mxc://` (or empty clear) reference. Image bytes stay off.
/// Failed errors stay static. Room name/topic/avatar stay off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreOwnProfile {
    static func setOwnDisplayName(
        core: SharedCore,
        displayName: String
    ) async throws -> OwnProfileWriteDto {
        try await core.setOwnDisplayName(displayName: displayName)
    }

    static func setOwnAvatar(core: SharedCore, mxc: String) async throws -> OwnProfileWriteDto {
        try await core.setOwnAvatar(mxc: mxc)
    }
}
