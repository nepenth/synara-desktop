import Foundation
import SynaraCore

/// P4-S9-6 typed m.direct account-data. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the three registered m.direct commands only.
/// Snapshot DTOs may return user/room ids. Failed errors stay static.
/// Room notes and profile writes stay off.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreMDirect {
    static func mdirectSnapshot(core: SharedCore) async throws -> MDirectSnapshotDto {
        try await core.mdirectSnapshot()
    }

    static func mdirectAdd(
        core: SharedCore,
        roomId: String,
        userId: String
    ) async throws -> MDirectMutationDto {
        try await core.mdirectAdd(roomId: roomId, userId: userId)
    }

    static func mdirectRemove(core: SharedCore, roomId: String) async throws -> MDirectMutationDto {
        try await core.mdirectRemove(roomId: roomId)
    }
}
