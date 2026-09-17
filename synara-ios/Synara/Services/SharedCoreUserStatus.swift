import Foundation
import SynaraCore

/// Typed MSC4426 status / in-call. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps `matrix_user_status_snapshot` / `set` / `clear` only. It never
/// calls `set_call` and is not `m.presence`.
struct SharedCoreUserStatusField: Equatable {
    let emoji: String
    let text: String
}

struct SharedCoreInCall: Equatable {
    let callJoinedTs: UInt64?
}

struct SharedCoreUserStatusSnapshot: Equatable {
    let userID: String
    let userStatus: SharedCoreUserStatusField?
    let inCall: SharedCoreInCall?
}

enum SharedCoreUserStatus {
    static func snapshot(core: SharedCore, userId: String) async throws -> UserStatusSnapshotDto {
        try await core.userStatusSnapshot(userId: userId)
    }

    static func set(core: SharedCore, emoji: String, text: String) async throws -> UserStatusWriteDto {
        try await core.userStatusSet(emoji: emoji, text: text)
    }

    static func clear(core: SharedCore) async throws -> UserStatusWriteDto {
        try await core.userStatusClear()
    }

    static func product(from dto: UserStatusSnapshotDto) -> SharedCoreUserStatusSnapshot {
        SharedCoreUserStatusSnapshot(
            userID: dto.userId,
            userStatus: dto.userStatus.map {
                SharedCoreUserStatusField(emoji: $0.emoji, text: $0.text)
            },
            inCall: dto.inCall.map { SharedCoreInCall(callJoinedTs: $0.callJoinedTs) }
        )
    }
}
