import Foundation
import SynaraCore

/// Typed MatrixRTC transport snapshot. Uses an already-constructed SharedCore.
///
/// The caller owns the core so UniFFI does not free the retained Client.
/// This wraps `matrix_rtc_transports_snapshot` and `matrix_rtc_transports_refresh`
/// only. It is not a widget driver and not a join path.
struct SharedCoreRtcTransport: Equatable {
    let kind: String
    let serviceURL: String?
}

struct SharedCoreRtcTransportsSnapshot: Equatable {
    let status: RtcTransportsStatusDto
    let transports: [SharedCoreRtcTransport]
}

enum SharedCoreRtcTransports {
    static func snapshot(core: SharedCore) async throws -> RtcTransportsSnapshotDto {
        try await core.rtcTransportsSnapshot()
    }

    static func refresh(core: SharedCore) async throws -> RtcTransportsSnapshotDto {
        try await core.rtcTransportsRefresh()
    }

    static func product(from dto: RtcTransportsSnapshotDto) -> SharedCoreRtcTransportsSnapshot {
        SharedCoreRtcTransportsSnapshot(
            status: dto.status,
            transports: dto.transports.map {
                SharedCoreRtcTransport(kind: $0.kind, serviceURL: $0.serviceUrl)
            }
        )
    }

    /// Settings copy for the Calls section; transport names and URLs are
    /// left to logs.
    static func diagnosticCopy(_ snapshot: SharedCoreRtcTransportsSnapshot) -> String {
        if snapshot.status == .unsupported || snapshot.status == .unavailable {
            return String(localized: "Calls aren’t set up on this server.")
        }
        return String(localized: "Calls are available on this server.")
    }
}
