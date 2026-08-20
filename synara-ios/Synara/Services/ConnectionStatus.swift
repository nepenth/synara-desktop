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

    /// Offline-equivalent statuses wait this long before Connection Lost chrome.
    static let lostHold: TimeInterval = 4
    /// Connected is a recovery flash, not steady-state chrome.
    static let connectedFlash: TimeInterval = 4

    static func holdsBeforeBanner(_ status: MatrixSyncStatus) -> Bool {
        switch status {
        case .reconnecting, .disconnected, .failed:
            return true
        case .connected, .syncing, .starting, .stopped, .restoreFailed:
            return false
        }
    }

    static func presentsBanner(
        _ status: MatrixSyncStatus,
        connectedFlashVisible: Bool = false
    ) -> Bool {
        switch status {
        case .connected:
            return connectedFlashVisible
        case .stopped:
            return false
        case .syncing, .starting, .reconnecting, .disconnected, .restoreFailed, .failed:
            return true
        }
    }
}

/// Published chrome state for the signed-in connection/sync indicator.
final class ConnectionStatusStore: ObservableObject {
    @Published private(set) var status: MatrixSyncStatus = .stopped
    @Published private(set) var isBannerVisible = false

    /// Native SyncService reports Offline/idle/failed during short sliding-sync
    /// gaps. Hold Lost-equivalent chrome until that state lasts.
    private let lostHold: TimeInterval
    private let connectedFlash: TimeInterval
    private var lostHoldWork: DispatchWorkItem?
    private var connectedFlashWork: DispatchWorkItem?
    private var pendingLost: MatrixSyncStatus?
    private var recoveredFromVisibleDisconnect = false

    init(
        reconnectingHold: TimeInterval = ConnectionStatusCopy.lostHold,
        connectedFlash: TimeInterval = ConnectionStatusCopy.connectedFlash
    ) {
        self.lostHold = reconnectingHold
        self.connectedFlash = connectedFlash
    }

    deinit {
        lostHoldWork?.cancel()
        connectedFlashWork?.cancel()
    }

    /// Empty-state / placeholder copy follows held chrome, not the live SDK blip.
    var emptyStateMessage: String {
        ConnectionStatusCopy.banner(status)
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
        if shouldDelay(status) {
            if ConnectionStatusCopy.holdsBeforeBanner(self.status) {
                present(status)
                return
            }
            if pendingLost != nil {
                pendingLost = status
                return
            }
            scheduleLost(status)
            return
        }

        pendingLost = nil
        lostHoldWork?.cancel()
        lostHoldWork = nil
        present(status)
    }

    private func shouldDelay(_ status: MatrixSyncStatus) -> Bool {
        if ConnectionStatusCopy.holdsBeforeBanner(status) {
            return true
        }
        guard status == .starting else {
            return false
        }
        switch self.status {
        case .connected, .syncing:
            return true
        default:
            return pendingLost != nil
        }
    }

    private func present(_ status: MatrixSyncStatus) {
        connectedFlashWork?.cancel()
        connectedFlashWork = nil

        if ConnectionStatusCopy.holdsBeforeBanner(status) || status == .restoreFailed {
            recoveredFromVisibleDisconnect = true
        }

        self.status = status

        if status == .connected {
            if recoveredFromVisibleDisconnect, connectedFlash > 0 {
                isBannerVisible = true
                scheduleConnectedFlashHide()
            } else {
                isBannerVisible = false
                recoveredFromVisibleDisconnect = false
            }
            return
        }

        isBannerVisible = ConnectionStatusCopy.presentsBanner(status)
    }

    private func scheduleLost(_ status: MatrixSyncStatus) {
        pendingLost = status
        lostHoldWork?.cancel()
        hideConnectedFlashForNewHold()
        if lostHold <= 0 {
            let toPresent = pendingLost ?? status
            pendingLost = nil
            lostHoldWork = nil
            present(toPresent)
            return
        }
        let work = DispatchWorkItem { [weak self] in
            guard let self else { return }
            let toPresent = self.pendingLost ?? status
            self.pendingLost = nil
            self.lostHoldWork = nil
            self.present(toPresent)
        }
        lostHoldWork = work
        DispatchQueue.main.asyncAfter(deadline: .now() + lostHold, execute: work)
    }

    private func hideConnectedFlashForNewHold() {
        connectedFlashWork?.cancel()
        connectedFlashWork = nil
        guard status == .connected, isBannerVisible else { return }
        isBannerVisible = false
        recoveredFromVisibleDisconnect = false
    }

    private func scheduleConnectedFlashHide() {
        let work = DispatchWorkItem { [weak self] in
            guard let self else { return }
            self.connectedFlashWork = nil
            guard self.status == .connected, self.pendingLost == nil else { return }
            self.isBannerVisible = false
            self.recoveredFromVisibleDisconnect = false
        }
        connectedFlashWork = work
        DispatchQueue.main.asyncAfter(deadline: .now() + connectedFlash, execute: work)
    }
}
