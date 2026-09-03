import Foundation
import SynaraCore

/// A narrow seam around the generated FFI owner. Tests provide this operation
/// by composition instead of subclassing `SharedCore`, whose required
/// initializer is generator-owned and can legitimately change on regeneration.
protocol SharedCoreTimelineMediaBytesProviding: AnyObject {
    func timelineMediaBytes(handleId: String) async throws -> LeftoverBytesDto
}

extension SharedCore: SharedCoreTimelineMediaBytesProviding {}

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

    static func mediaBytes(
        core: any SharedCoreTimelineMediaBytesProviding,
        handleId: String
    ) async throws -> Data {
        let trace = PerformanceTrace.begin("TimelineMediaFetch")

        do {
            try Task.checkCancellation()
            let data = try await core.timelineMediaBytes(handleId: handleId).payload
            try Task.checkCancellation()
            PerformanceTrace.end(
                "TimelineMediaFetch",
                id: trace,
                byteCount: data.count,
                outcome: .success
            )
            return data
        } catch is CancellationError {
            PerformanceTrace.end(
                "TimelineMediaFetch",
                id: trace,
                byteCount: 0,
                outcome: .cancelled
            )
            throw CancellationError()
        } catch {
            if Task.isCancelled {
                PerformanceTrace.end(
                    "TimelineMediaFetch",
                    id: trace,
                    byteCount: 0,
                    outcome: .cancelled
                )
                throw CancellationError()
            }
            PerformanceTrace.end(
                "TimelineMediaFetch",
                id: trace,
                byteCount: 0,
                outcome: .failure
            )
            throw error
        }
    }
}
