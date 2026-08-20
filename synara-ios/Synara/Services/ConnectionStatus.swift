import Combine
import Foundation

/// Privacy-safe connection/sync presentation. Mirrors desktop SyncStatus
/// meaning (connected / syncing / disconnected / restore failed) without
/// tokens, homeserver admin URLs, or raw SDK text.
enum ConnectionStatusCopy {
    static let connected = "Connected"
    static let syncing = "Syncing history…"
    static let starting = "Connecting…"
    static let reconnecting = "Connection Lost! Reconnecting..."
    static let disconnected = "Connection Lost!"
    static let restoreFailed = "Couldn't restore this session. Sign out and sign in again."
    static let stopped = "Not connected"

    static func banner(_ status: MatrixSyncStatus) -> String {
        switch status {
        case .connected:
            return connected
        case .syncing:
            return syncing
        case .starting:
            return starting
        case .reconnecting:
            return reconnecting
        case .disconnected:
            return disconnected
        case .restoreFailed:
            return restoreFailed
        case .stopped:
            return stopped
        case .failed:
            return disconnected
        }
    }

    enum Variant: Equatable {
        case success
        case warning
        case critical
        case neutral
    }

    static func variant(_ status: MatrixSyncStatus) -> Variant {
        switch status {
        case .connected, .syncing:
            return .success
        case .starting, .reconnecting:
            return .warning
        case .disconnected, .restoreFailed, .failed:
            return .critical
        case .stopped:
            return .neutral
        }
    }

    static func systemImage(_ status: MatrixSyncStatus) -> String {
        switch status {
        case .connected:
            return "checkmark.circle.fill"
        case .syncing, .starting:
            return "arrow.triangle.2.circlepath"
        case .reconnecting:
            return "wifi.exclamationmark"
        case .disconnected, .failed, .stopped:
            return "wifi.slash"
        case .restoreFailed:
            return "exclamationmark.triangle.fill"
        }
    }

    static func showsSignOutAction(_ status: MatrixSyncStatus) -> Bool {
        switch status {
        case .restoreFailed, .disconnected, .failed:
            return true
        case .connected, .syncing, .starting, .reconnecting, .stopped:
            return false
        }
    }

    static func showsRetryAction(_ status: MatrixSyncStatus) -> Bool {
        switch status {
        case .disconnected, .failed, .reconnecting:
            return true
        case .restoreFailed, .connected, .syncing, .starting, .stopped:
            return false
        }
    }

    static func fromReadiness(
        _ readiness: String?,
        previous: MatrixSyncStatus = .stopped
    ) -> MatrixSyncStatus {
        switch readiness {
        case "running":
            return .connected
        case "idle":
            switch previous {
            case .connected, .syncing, .reconnecting, .disconnected:
                return .disconnected
            case .starting, .stopped, .restoreFailed, .failed:
                return .starting
            }
        case "offline":
            return .reconnecting
        case "failed", "terminated", "unconfigured":
            return .disconnected
        default:
            return .starting
        }
    }
}

/// Published chrome state for the signed-in connection/sync indicator.
final class ConnectionStatusStore: ObservableObject {
    @Published private(set) var status: MatrixSyncStatus = .stopped

    /// Native SyncService reports Offline during short sliding-sync gaps.
    /// Hold reconnecting until that state lasts so the banner does not bounce.
    private let reconnectingHold: TimeInterval
    private var reconnectingWork: DispatchWorkItem?

    init(reconnectingHold: TimeInterval = 4) {
        self.reconnectingHold = reconnectingHold
    }

    deinit {
        reconnectingWork?.cancel()
    }

    func update(_ status: MatrixSyncStatus) {
        if Thread.isMainThread {
            apply(status)
        } else {
            DispatchQueue.main.async { [weak self] in
                self?.apply(status)
            }
        }
    }

    private func apply(_ status: MatrixSyncStatus) {
        if status == .reconnecting {
            if self.status == .reconnecting { return }
            scheduleReconnecting()
            return
        }
        reconnectingWork?.cancel()
        reconnectingWork = nil
        self.status = status
    }

    private func scheduleReconnecting() {
        reconnectingWork?.cancel()
        if reconnectingHold <= 0 {
            status = .reconnecting
            reconnectingWork = nil
            return
        }
        let work = DispatchWorkItem { [weak self] in
            self?.status = .reconnecting
            self?.reconnectingWork = nil
        }
        reconnectingWork = work
        DispatchQueue.main.asyncAfter(deadline: .now() + reconnectingHold, execute: work)
    }
}
