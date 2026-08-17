import Foundation
import SynaraCore

/// P4-S14 poll of queued timeline view-delta summaries.
/// Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This drains privacy-safe summaries only (no row bodies). iOS re-fetches
/// snapshot via the existing timeline commands. This is not `Platform.emit`,
/// not `Core.command`, and not a product NSE swap. NSE still cannot poll.
/// XCTest construction of `SharedCore` is not iOS-on-engine.
enum SharedCoreTimelineViewUpdates {
    static func poll(core: SharedCore) async throws -> [TimelineViewUpdateDto] {
        try await core.pollTimelineViewUpdates()
    }
}
