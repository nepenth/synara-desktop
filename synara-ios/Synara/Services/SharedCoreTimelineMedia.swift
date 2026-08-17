import Foundation
import SynaraCore

/// P4-S33 native media-handle channel. Bytes are a dedicated UniFFI
/// argument, not a `Core.command` envelope. Handles never include `mxc://`.
/// This is not leftover `media_download` and not P4 acceptance.
enum SharedCoreTimelineMedia {
    static let urlScheme = "synara-timeline-media"

    static func handleId(from url: URL?) -> String? {
        guard let url, url.scheme == urlScheme else {
            return nil
        }
        let handle = url.host ?? url.path.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        return handle.isEmpty ? nil : handle
    }

    static func mediaBytes(core: SharedCore, handleId: String) async throws -> Data {
        try await core.timelineMediaBytes(handleId: handleId).payload
    }
}
