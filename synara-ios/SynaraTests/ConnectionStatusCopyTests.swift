import XCTest
@testable import Synara

final class ConnectionStatusCopyTests: XCTestCase {
    enum BannerClass { case connected, reconnecting, lost, cold }

    /// Shared connection-status table. The identical rows live in
    /// synara/src/app/features/native-client/__tests__/nativeClientFacade.test.ts
    /// (`SHARED_CONNECTION_STATUS_CASES`), which also checks this copy.
    /// Columns: readiness, command gate, connected earlier in this session, banner.
    let sharedConnectionStatusCases: [(String, String, Bool, BannerClass)] = [
        ("running", "open", false, .connected),
        ("running", "open", true, .connected),
        ("running", "closed", false, .lost),
        ("running", "closed", true, .lost),
        ("running", "unexpected", true, .lost),
        ("offline", "open", true, .reconnecting),
        ("failed", "open", false, .lost),
        ("failed", "open", true, .lost),
        ("terminated", "open", false, .lost),
        ("terminated", "open", true, .lost),
        ("idle", "open", false, .cold),
        ("idle", "open", true, .lost),
        ("unconfigured", "open", false, .cold),
        ("unconfigured", "open", true, .lost),
    ]

    func testConnectionStatusFollowsTheSharedDesktopTable() {
        for (readiness, gate, connectedEarlier, expected) in sharedConnectionStatusCases {
            let status = ConnectionStatusCopy.fromReadiness(
                readiness,
                previous: connectedEarlier ? .connected : .starting,
                commandGate: gate
            )
            let actual: BannerClass
            switch status {
            case .connected, .syncing:
                actual = .connected
            case .reconnecting:
                actual = .reconnecting
            case .disconnected, .failed, .restoreFailed:
                actual = .lost
            case .starting, .stopped:
                actual = .cold
            }
            XCTAssertEqual(actual, expected, "\(readiness)/\(gate)/\(connectedEarlier)")
        }
    }
    func testCopyMatchesDesktopMeaningWithoutSecrets() {
        XCTAssertEqual(ConnectionStatusCopy.banner(.connected), "Connected")
        XCTAssertEqual(ConnectionStatusCopy.banner(.syncing), "Syncing history…")
        XCTAssertEqual(ConnectionStatusCopy.banner(.reconnecting), "Connection Lost! Reconnecting...")
        XCTAssertEqual(ConnectionStatusCopy.banner(.disconnected), "Connection Lost!")
        XCTAssertEqual(
            ConnectionStatusCopy.banner(.restoreFailed),
            "Couldn't restore this session. Sign out and sign in again."
        )
        XCTAssertEqual(ConnectionStatusCopy.banner(.starting), "Connecting…")
        XCTAssertEqual(ConnectionStatusCopy.banner(.stopped), "Not connected")
        XCTAssertEqual(ConnectionStatusCopy.banner(.failed("raw sdk https://user:secret@hs/?password=hunter2")), "Connection Lost!")

        for status in allStatuses {
            let copy = ConnectionStatusCopy.banner(status)
            for forbidden in ["password", "syt_", "token", "https://", "secret", "hunter2"] {
                XCTAssertFalse(copy.contains(forbidden), "\(status) leaked \(forbidden)")
            }
        }
    }

    func testReadinessMappingDoesNotTreatIdleAsCatchup() {
        XCTAssertEqual(ConnectionStatusCopy.fromReadiness("running"), .connected)
        XCTAssertEqual(ConnectionStatusCopy.fromReadiness("idle"), .starting)
        XCTAssertEqual(ConnectionStatusCopy.fromReadiness("idle", previous: .starting), .starting)
        XCTAssertEqual(ConnectionStatusCopy.fromReadiness("idle", previous: .connected), .disconnected)
        XCTAssertEqual(ConnectionStatusCopy.fromReadiness("idle", previous: .syncing), .disconnected)
        XCTAssertEqual(ConnectionStatusCopy.fromReadiness("offline"), .reconnecting)
        XCTAssertEqual(ConnectionStatusCopy.fromReadiness("failed"), .disconnected)
        XCTAssertEqual(ConnectionStatusCopy.fromReadiness("terminated"), .disconnected)
        XCTAssertEqual(ConnectionStatusCopy.fromReadiness("unconfigured"), .starting)
        XCTAssertEqual(
            ConnectionStatusCopy.fromReadiness("unconfigured", previous: .connected),
            .disconnected
        )
        XCTAssertEqual(ConnectionStatusCopy.fromReadiness(nil), .starting)
        XCTAssertEqual(
            ConnectionStatusCopy.fromReadiness("running", commandGate: "closed"),
            .disconnected
        )
        XCTAssertEqual(
            ConnectionStatusCopy.fromReadiness("running", commandGate: "open"),
            .connected
        )
        XCTAssertEqual(
            ConnectionStatusCopy.fromReadiness("running", commandGate: "https://secret.example/token"),
            .disconnected
        )
    }

    func testRestoreFailedOffersSignOutWithoutRetry() {
        XCTAssertTrue(ConnectionStatusCopy.showsSignOutAction(.restoreFailed))
        XCTAssertFalse(ConnectionStatusCopy.showsRetryAction(.restoreFailed))
        XCTAssertTrue(ConnectionStatusCopy.showsSignOutAction(.disconnected))
        XCTAssertTrue(ConnectionStatusCopy.showsRetryAction(.disconnected))
        XCTAssertTrue(ConnectionStatusCopy.showsRetryAction(.reconnecting))
        XCTAssertFalse(ConnectionStatusCopy.showsSignOutAction(.connected))
        XCTAssertFalse(ConnectionStatusCopy.showsSignOutAction(.syncing))
        XCTAssertFalse(ConnectionStatusCopy.showsRetryAction(.connected))
        XCTAssertFalse(ConnectionStatusCopy.showsRetryAction(.starting))
    }

    func testVariantsStayGlanceableNotToastOnly() {
        XCTAssertEqual(ConnectionStatusCopy.variant(.connected), .success)
        XCTAssertEqual(ConnectionStatusCopy.variant(.syncing), .success)
        XCTAssertEqual(ConnectionStatusCopy.variant(.starting), .warning)
        XCTAssertEqual(ConnectionStatusCopy.variant(.reconnecting), .warning)
        XCTAssertEqual(ConnectionStatusCopy.variant(.restoreFailed), .critical)
        XCTAssertEqual(ConnectionStatusCopy.variant(.disconnected), .critical)
    }

    func testStorePublishesWithoutEchoingSecrets() {
        let store = ConnectionStatusStore()
        store.update(.restoreFailed)
        XCTAssertEqual(store.status, .restoreFailed)
        XCTAssertTrue(store.isBannerVisible)
        let publicError = String(describing: store.status)
        for forbidden in ["password", "syt_", "token"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
    }

    func testHoldsLostEquivalentBeforeBanner() {
        XCTAssertTrue(ConnectionStatusCopy.holdsBeforeBanner(.reconnecting))
        XCTAssertFalse(ConnectionStatusCopy.holdsBeforeBanner(.disconnected))
        XCTAssertFalse(ConnectionStatusCopy.holdsBeforeBanner(.failed("raw sdk blip")))
        XCTAssertFalse(ConnectionStatusCopy.holdsBeforeBanner(.restoreFailed))
        XCTAssertFalse(ConnectionStatusCopy.holdsBeforeBanner(.connected))
        XCTAssertFalse(ConnectionStatusCopy.holdsBeforeBanner(.starting))
        XCTAssertEqual(ConnectionStatusCopy.lostHold, 4)
        XCTAssertEqual(ConnectionStatusCopy.connectedFlash, 12)
    }

    func testConnectedBannerIsNotSticky() {
        XCTAssertFalse(ConnectionStatusCopy.presentsBanner(.connected))
        XCTAssertTrue(ConnectionStatusCopy.presentsBanner(.connected, connectedFlashVisible: true))
        XCTAssertFalse(ConnectionStatusCopy.presentsBanner(.syncing))
        XCTAssertFalse(ConnectionStatusCopy.presentsBanner(.stopped))
        XCTAssertTrue(ConnectionStatusCopy.presentsBanner(.disconnected))
        XCTAssertTrue(ConnectionStatusCopy.presentsBanner(.restoreFailed))

        let store = ConnectionStatusStore()
        store.update(.connected)
        XCTAssertEqual(store.status, .connected)
        XCTAssertFalse(store.isBannerVisible)
    }

    func testReconnectingHoldDoesNotBounceConnectedOnABlip() {
        let store = ConnectionStatusStore(reconnectingHold: 4)
        store.update(.connected)
        store.update(.reconnecting)
        XCTAssertEqual(store.status, .connected)
        XCTAssertFalse(store.isBannerVisible)
        store.update(.connected)
        XCTAssertEqual(store.status, .connected)
        XCTAssertFalse(store.isBannerVisible)
    }

    func testDisconnectedPresentsImmediately() {
        let store = ConnectionStatusStore(reconnectingHold: 4)
        store.update(.connected)
        store.update(.disconnected)
        XCTAssertEqual(store.status, .disconnected)
        XCTAssertTrue(store.isBannerVisible)
        XCTAssertEqual(store.emptyStateMessage, ConnectionStatusCopy.disconnected)
    }

    func testFailedPresentsImmediatelyWithoutSdkText() {
        let store = ConnectionStatusStore(reconnectingHold: 4)
        store.update(.connected)
        store.update(.failed("raw sdk https://user:secret@hs/?password=hunter2"))
        XCTAssertEqual(store.status, .failed("raw sdk https://user:secret@hs/?password=hunter2"))
        XCTAssertTrue(store.isBannerVisible)
        XCTAssertEqual(store.emptyStateMessage, ConnectionStatusCopy.disconnected)
        XCTAssertFalse(store.emptyStateMessage.contains("https://"))
        XCTAssertFalse(store.emptyStateMessage.contains("hunter2"))
    }

    func testStartingHoldFromConnectedDoesNotShowImmediateConnecting() {
        let store = ConnectionStatusStore(reconnectingHold: 4)
        store.update(.connected)
        store.update(.starting)
        XCTAssertEqual(store.status, .connected)
        XCTAssertFalse(store.isBannerVisible)
        store.update(.connected)
        XCTAssertEqual(store.status, .connected)
        XCTAssertFalse(store.isBannerVisible)
    }

    func testRestoreFailedShowsImmediatelyDuringHold() {
        let store = ConnectionStatusStore(reconnectingHold: 4)
        store.update(.connected)
        store.update(.restoreFailed)
        XCTAssertEqual(store.status, .restoreFailed)
        XCTAssertTrue(store.isBannerVisible)
    }

    func testReconnectingHoldZeroAppliesImmediately() {
        let store = ConnectionStatusStore(reconnectingHold: 0)
        store.update(.connected)
        store.update(.reconnecting)
        XCTAssertEqual(store.status, .reconnecting)
        XCTAssertTrue(store.isBannerVisible)
    }

    func testLostHoldExpiresToReconnecting() {
        let store = ConnectionStatusStore(reconnectingHold: 0.05)
        store.update(.connected)
        store.update(.reconnecting)
        XCTAssertEqual(store.status, .connected)
        XCTAssertFalse(store.isBannerVisible)

        let shown = expectation(description: "reconnecting after hold")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
            XCTAssertEqual(store.status, .reconnecting)
            XCTAssertTrue(store.isBannerVisible)
            shown.fulfill()
        }
        wait(for: [shown], timeout: 1)
    }

    func testConnectedFlashOnlyAfterVisibleLost() {
        let store = ConnectionStatusStore(reconnectingHold: 0, connectedFlash: 4)
        store.update(.connected)
        XCTAssertFalse(store.isBannerVisible)
        store.update(.disconnected)
        XCTAssertEqual(store.status, .disconnected)
        XCTAssertTrue(store.isBannerVisible)
        store.update(.connected)
        XCTAssertEqual(store.status, .connected)
        XCTAssertTrue(store.isBannerVisible)
    }

    func testConnectedFlashZeroIsNotStickyAfterLost() {
        let store = ConnectionStatusStore(reconnectingHold: 0, connectedFlash: 0)
        store.update(.connected)
        store.update(.disconnected)
        store.update(.connected)
        XCTAssertEqual(store.status, .connected)
        XCTAssertFalse(store.isBannerVisible)
    }

    func testRepeatedConnectedUpdatesDoNotExtendRecoveryFlash() {
        let store = ConnectionStatusStore(reconnectingHold: 0, connectedFlash: 0.08)
        store.update(.disconnected)
        store.update(.connected)
        XCTAssertTrue(store.isBannerVisible)

        let hidden = expectation(description: "original recovery deadline wins")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) {
            store.update(.connected)
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.11) {
            XCTAssertEqual(store.status, .connected)
            XCTAssertFalse(store.isBannerVisible)
            hidden.fulfill()
        }
        wait(for: [hidden], timeout: 1)
    }

    func testEmptyStateCopyUsesHeldStatusNotLiveLost() {
        let store = ConnectionStatusStore(reconnectingHold: 4)
        store.update(.connected)
        XCTAssertEqual(store.emptyStateMessage, ConnectionStatusCopy.connected)

        store.update(.reconnecting)
        XCTAssertEqual(store.emptyStateMessage, ConnectionStatusCopy.connected)
        XCTAssertNotEqual(store.emptyStateMessage, ConnectionStatusCopy.reconnecting)
        XCTAssertNotEqual(store.emptyStateMessage, MatrixSyncStatus.reconnecting.description)

        store.update(.disconnected)
        XCTAssertEqual(store.emptyStateMessage, ConnectionStatusCopy.disconnected)
        XCTAssertFalse(store.emptyStateMessage.contains("https://"))

        store.update(.failed("raw sdk https://user:secret@hs/?password=hunter2"))
        XCTAssertEqual(store.emptyStateMessage, ConnectionStatusCopy.disconnected)
        XCTAssertFalse(store.emptyStateMessage.contains("https://"))

        store.update(.restoreFailed)
        XCTAssertEqual(store.emptyStateMessage, ConnectionStatusCopy.restoreFailed)
    }

    func testConnectedFlashHidesWhenLostHoldStarts() {
        let store = ConnectionStatusStore(reconnectingHold: 4, connectedFlash: 4)
        store.update(.restoreFailed)
        XCTAssertTrue(store.isBannerVisible)
        store.update(.connected)
        XCTAssertEqual(store.status, .connected)
        XCTAssertTrue(store.isBannerVisible)

        store.update(.reconnecting)
        XCTAssertEqual(store.status, .connected)
        XCTAssertFalse(store.isBannerVisible)
        XCTAssertEqual(store.emptyStateMessage, ConnectionStatusCopy.connected)
    }

    func testEmptyStatesReadHeldConnectionStatusNotLiveMatrixDescription() throws {
        let root = repositoryRoot()
        let roomList = try String(
            contentsOfFile: "\(root)/synara-ios/Synara/Features/RoomListView.swift",
            encoding: .utf8
        )
        let placeholder = try String(
            contentsOfFile: "\(root)/synara-ios/Synara/Features/PlaceholderScreen.swift",
            encoding: .utf8
        )
        XCTAssertFalse(roomList.contains("matrix.syncStatusDescription"))
        XCTAssertFalse(placeholder.contains("matrix.syncStatusDescription"))
        XCTAssertTrue(roomList.contains("HeldConnectionEmptyState"))
        XCTAssertTrue(placeholder.contains("HeldConnectionEmptyState"))
        XCTAssertTrue(roomList.contains("environment.connectionStatus"))
        XCTAssertTrue(placeholder.contains("environment.connectionStatus"))
    }

    func testMatrixSyncStatusDescriptionUsesPrivacySafeCopy() {
        XCTAssertEqual(MatrixSyncStatus.syncing.description, "Syncing history…")
        XCTAssertEqual(MatrixSyncStatus.starting.description, "Connecting…")
        XCTAssertEqual(MatrixSyncStatus.restoreFailed.description, ConnectionStatusCopy.restoreFailed)
        XCTAssertEqual(MatrixSyncStatus.failed("syt_secret_token").description, "Connection Lost!")
    }

    private var allStatuses: [MatrixSyncStatus] {
        [
            .stopped,
            .starting,
            .syncing,
            .connected,
            .reconnecting,
            .disconnected,
            .restoreFailed,
            .failed("syt_secret_token")
        ]
    }

    private func repositoryRoot() -> String {
        var url = URL(fileURLWithPath: #filePath)
        while url.pathComponents.count > 1 {
            url.deleteLastPathComponent()
            if FileManager.default.fileExists(atPath: url.appendingPathComponent("synara-ios").path) {
                return url.path
            }
        }
        return url.path
    }
}
