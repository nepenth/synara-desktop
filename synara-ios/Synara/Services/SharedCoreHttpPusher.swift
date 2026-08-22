import Foundation
import SynaraCore

/// Live HTTP pusher set/delete. Uses an already-constructed SharedCore.
///
/// Push keys are dedicated arguments only. Failed errors stay static and
/// must not echo push key, gateway URL, or token. Leftover `pusherSet` /
/// `pusherDelete` stay unused by product iOS.
enum SharedCoreHttpPusher {
    static func register(
        core: SharedCore,
        pushKey: String,
        appId: String,
        gatewayUrl: String,
        appDisplayName: String,
        deviceDisplayName: String,
        lang: String
    ) async throws -> PusherWriteDto {
        try await core.registerHttpPusher(
            pushKey: pushKey,
            appId: appId,
            gatewayUrl: gatewayUrl,
            appDisplayName: appDisplayName,
            deviceDisplayName: deviceDisplayName,
            lang: lang
        )
    }

    static func delete(
        core: SharedCore,
        pushKey: String,
        appId: String
    ) async throws -> PusherWriteDto {
        try await core.deleteHttpPusher(pushKey: pushKey, appId: appId)
    }
}
