import Foundation
import OSLog

enum PerformanceTrace {
    enum Outcome: Int {
        case failure = 0
        case success = 1
        case cancelled = 2
    }

    private static let log = OSLog(subsystem: "com.whylandcreative.synara", category: "performance")

    @discardableResult
    static func begin(_ name: StaticString) -> OSSignpostID {
        let id = OSSignpostID(log: log)
        os_signpost(.begin, log: log, name: name, signpostID: id)
        return id
    }

    static func end(_ name: StaticString, id: OSSignpostID) {
        os_signpost(.end, log: log, name: name, signpostID: id)
    }

    /// Ends a byte-transfer interval using numeric, content-free metadata only.
    /// Never pass media handles, MXC URLs, room IDs, event IDs, or filenames.
    static func end(
        _ name: StaticString,
        id: OSSignpostID,
        byteCount: Int,
        outcome: Outcome
    ) {
        os_signpost(
            .end,
            log: log,
            name: name,
            signpostID: id,
            "bytes=%{public}d outcome=%{public}d",
            max(0, byteCount),
            outcome.rawValue
        )
    }

    static func event(_ name: StaticString) {
        os_signpost(.event, log: log, name: name)
    }
}
