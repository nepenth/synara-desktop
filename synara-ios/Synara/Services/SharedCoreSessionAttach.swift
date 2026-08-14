import Foundation
import SynaraCore

/// P4-S3d owner-attach entry. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This is not `Core.command`, not leftover registration, and not a
/// product room-list swap. XCTest construction of `SharedCore` is not
/// iOS-on-engine.
enum SharedCoreSessionAttach {
    static func attachSessionOwners(core: SharedCore) async throws -> SessionAttachDto {
        try await core.attachSessionOwners()
    }
}
