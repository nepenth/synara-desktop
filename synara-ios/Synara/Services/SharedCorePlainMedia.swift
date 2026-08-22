import Foundation
import SynaraCore

/// Live plain `mxc://` download. Uses an already-constructed SharedCore.
///
/// Bytes are a dedicated return only. Failed errors stay static and must not
/// echo mxc or token. Timeline-media handles stay on `SharedCoreTimelineMedia`.
/// Leftover `mediaDownload` / `mediaThumbnail` stay unused by product iOS.
enum SharedCorePlainMedia {
    static func download(core: SharedCore, contentUri: String) async throws -> Data {
        try await core.downloadPlainMedia(contentUri: contentUri).payload
    }

    static func thumbnail(
        core: SharedCore,
        contentUri: String,
        width: UInt64,
        height: UInt64
    ) async throws -> Data {
        try await core.thumbnailPlainMedia(
            contentUri: contentUri,
            width: width,
            height: height
        ).payload
    }
}
