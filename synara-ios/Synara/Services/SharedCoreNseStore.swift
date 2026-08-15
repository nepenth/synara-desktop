import Foundation
import SynaraCore

/// P4-S11 NSE read-only store.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This opens the persisted store and looks up a local event preview only.
/// It never starts SyncService and never attaches session owners.
/// Failed errors stay static and must not echo room id, event id, or tokens.
/// Session/status reads stay on SharedCoreSessionStatus.
/// It is not a generic `Core.command` FFI and not a product NSE swap.
enum SharedCoreNseStore {
    static func openReadOnly(
        core: SharedCore,
        userId: String,
        homeserverUrl: String,
        storeRoot: String
    ) async throws -> NseStoreDto {
        try await core.nseOpenReadOnlyStore(
            userId: userId,
            homeserverUrl: homeserverUrl,
            storeRoot: storeRoot
        )
    }

    static func storeStatus(core: SharedCore) async throws -> NseStoreDto {
        try await core.nseStoreStatus()
    }

    static func eventPreview(
        core: SharedCore,
        roomId: String,
        eventId: String
    ) async throws -> NseEventPreviewDto {
        try await core.nseEventPreview(roomId: roomId, eventId: eventId)
    }
}
