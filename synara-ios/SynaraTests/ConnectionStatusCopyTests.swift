import XCTest
@testable import Synara

final class ConnectionStatusCopyTests: XCTestCase {
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
        XCTAssertEqual(ConnectionStatusCopy.fromReadiness("unconfigured"), .disconnected)
        XCTAssertEqual(ConnectionStatusCopy.fromReadiness(nil), .starting)
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
        XCTAssertTrue(ConnectionStatusCopy.holdsBeforeBanner(.disconnected))
        XCTAssertTrue(ConnectionStatusCopy.holdsBeforeBanner(.failed("raw sdk blip")))
        XCTAssertFalse(ConnectionStatusCopy.holdsBeforeBanner(.restoreFailed))
        XCTAssertFalse(ConnectionStatusCopy.holdsBeforeBanner(.connected))
        XCTAssertFalse(ConnectionStatusCopy.holdsBeforeBanner(.starting))
        XCTAssertEqual(ConnectionStatusCopy.lostHold, 4)
        XCTAssertEqual(ConnectionStatusCopy.connectedFlash, 4)
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

    func testDisconnectedHoldDoesNotShowImmediateLost() {
        let store = ConnectionStatusStore(reconnectingHold: 4)
        store.update(.connected)
        store.update(.disconnected)
        XCTAssertEqual(store.status, .connected)
        XCTAssertFalse(store.isBannerVisible)
        store.update(.reconnecting)
        XCTAssertEqual(store.status, .connected)
        XCTAssertFalse(store.isBannerVisible)
        store.update(.connected)
        XCTAssertEqual(store.status, .connected)
        XCTAssertFalse(store.isBannerVisible)
    }

    func testFailedHoldDoesNotShowImmediateLost() {
        let store = ConnectionStatusStore(reconnectingHold: 4)
        store.update(.connected)
        store.update(.failed("raw sdk https://user:secret@hs/?password=hunter2"))
        XCTAssertEqual(store.status, .connected)
        XCTAssertFalse(store.isBannerVisible)
        store.update(.connected)
        XCTAssertEqual(store.status, .connected)
        XCTAssertFalse(store.isBannerVisible)
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
        store.update(.disconnected)
        XCTAssertEqual(store.status, .connected)
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

    func testLostHoldExpiresToDisconnected() {
        let store = ConnectionStatusStore(reconnectingHold: 0.05)
        store.update(.connected)
        store.update(.disconnected)
        XCTAssertEqual(store.status, .connected)
        XCTAssertFalse(store.isBannerVisible)

        let shown = expectation(description: "lost after hold")
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.2) {
            XCTAssertEqual(store.status, .disconnected)
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

    func testEmptyStateCopyUsesHeldStatusNotLiveLost() {
        let store = ConnectionStatusStore(reconnectingHold: 4)
        store.update(.connected)
        XCTAssertEqual(store.emptyStateMessage, ConnectionStatusCopy.connected)

        store.update(.reconnecting)
        XCTAssertEqual(store.emptyStateMessage, ConnectionStatusCopy.connected)
        XCTAssertNotEqual(store.emptyStateMessage, ConnectionStatusCopy.reconnecting)
        XCTAssertNotEqual(store.emptyStateMessage, MatrixSyncStatus.reconnecting.description)

        store.update(.disconnected)
        XCTAssertEqual(store.emptyStateMessage, ConnectionStatusCopy.connected)
        XCTAssertNotEqual(store.emptyStateMessage, ConnectionStatusCopy.disconnected)

        store.update(.failed("raw sdk https://user:secret@hs/?password=hunter2"))
        XCTAssertEqual(store.emptyStateMessage, ConnectionStatusCopy.connected)
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
