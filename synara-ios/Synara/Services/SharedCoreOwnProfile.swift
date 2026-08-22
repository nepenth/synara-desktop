import Foundation
import SynaraCore

/// P4-S9-8 / P4-S9-8a typed own display-name / avatar / get-own-profile.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the registered own-profile commands only.
/// Avatar is an `mxc://` (or empty clear) reference. Image bytes stay off.
/// Failed errors stay static. Room name/topic/avatar stay off.
/// It is not a generic `Core.command` FFI and not a product swap.
struct SharedCoreOwnProfileInfo: Equatable {
    let userID: String
    let displayName: String?
    let avatarURL: String?
}

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

    static func getOwnProfile(core: SharedCore) async throws -> OwnProfileDto {
        try await core.getOwnProfile()
    }

    static func uploadAvatar(
        core: SharedCore,
        payload: Data,
        mimeType: String
    ) async throws -> OwnProfileUploadDto {
        try await core.uploadAvatar(payload: payload, mimeType: mimeType)
    }
}
