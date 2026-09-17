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
    let status: String
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

    static func diagnosticCopy(_ snapshot: SharedCoreRtcTransportsSnapshot) -> String {
        if snapshot.status == "unsupported" || snapshot.status == "unavailable" {
            return "This homeserver does not advertise a call transport"
        }
        if let livekit = snapshot.transports.first(where: { $0.kind == "livekit" }) {
            if let url = livekit.serviceURL, url.isEmpty == false {
                return "MatrixRTC transport: LiveKit at \(url)"
            }
            return "MatrixRTC transport: LiveKit"
        }
        if snapshot.transports.contains(where: { $0.kind == "custom" }) {
            return "MatrixRTC transport: custom"
        }
        return "MatrixRTC transport: ready"
    }
}
