import Foundation
import SynaraCore

protocol SharedCoreHttpPusherOwning: AnyObject {
    func registerHttpPusher(
        pushKey: String,
        appId: String,
        gatewayUrl: String,
        appDisplayName: String,
        lang: String
    ) async throws -> PusherWriteDto

    func deleteHttpPusher(pushKey: String, appId: String) async throws -> PusherWriteDto
    func deleteHttpPushersForDevice(
        appId: String
    ) async throws -> PusherWriteDto
}

extension HttpPusherOwner: SharedCoreHttpPusherOwning {}

/// Live HTTP pusher set/delete. `bind` captures the account-bound Core owner;
/// later writes never resolve through a mutable SharedCore session slot.
///
/// Push keys are dedicated arguments only. Failed errors stay static and
/// must not echo push key, gateway URL, or token. Leftover `pusherSet` /
/// `pusherDelete` stay unused by product iOS.
enum SharedCoreHttpPusher {
    static func bind(
        core: SharedCore,
        userID: String,
        deviceID: String,
        homeserverURL: String
    ) throws -> SharedCoreHttpPusherOwning {
        try core.bindHttpPusherOwner(
            userId: userID,
            deviceId: deviceID,
            homeserverUrl: homeserverURL
        )
    }

    static func register(
        owner: SharedCoreHttpPusherOwning,
        pushKey: String,
        appId: String,
        gatewayUrl: String,
        appDisplayName: String,
        lang: String
    ) async throws -> PusherWriteDto {
        try await owner.registerHttpPusher(
            pushKey: pushKey,
            appId: appId,
            gatewayUrl: gatewayUrl,
            appDisplayName: appDisplayName,
            lang: lang
        )
    }

    static func delete(
        owner: SharedCoreHttpPusherOwning,
        pushKey: String,
        appId: String
    ) async throws -> PusherWriteDto {
        try await owner.deleteHttpPusher(pushKey: pushKey, appId: appId)
    }

    static func deleteForDevice(
        owner: SharedCoreHttpPusherOwning,
        appId: String
    ) async throws -> PusherWriteDto {
        try await owner.deleteHttpPushersForDevice(appId: appId)
    }
}
