import Foundation
import SynaraCore

/// P4-S9-11 typed room directory search. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps the three registered directory-search commands only.
/// Search results stay metadata (room ids, names, aliases, mxc). Avatar bytes stay off.
/// Failed errors stay static. Room leave/join stay off.
/// Directory visibility stays on SharedCoreDirectoryVisibility.
/// It is not a generic `Core.command` FFI and not a product swap.
enum SharedCoreDirectorySearch {
    static func roomDirectoryProtocols(
        core: SharedCore
    ) async throws -> RoomDirectoryProtocolsDto {
        try await core.roomDirectoryProtocols()
    }

    static func roomDirectorySearch(
        core: SharedCore,
        sessionGeneration: UInt64,
        requestId: UInt64,
        serverName: String?,
        term: String?,
        roomType: String?,
        thirdPartyInstanceId: String?,
        limit: UInt64,
        since: String?
    ) async throws -> RoomDirectorySearchDto {
        try await core.roomDirectorySearch(
            sessionGeneration: sessionGeneration,
            requestId: requestId,
            serverName: serverName,
            term: term,
            roomType: roomType,
            thirdPartyInstanceId: thirdPartyInstanceId,
            limit: limit,
            since: since
        )
    }

    static func roomDirectoryCancel(
        core: SharedCore,
        sessionGeneration: UInt64,
        requestId: UInt64
    ) async throws -> RoomDirectorySearchDto {
        try await core.roomDirectoryCancel(
            sessionGeneration: sessionGeneration,
            requestId: requestId
        )
    }
}
