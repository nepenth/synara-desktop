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
        let publicError = String(describing: store.status)
        for forbidden in ["password", "syt_", "token"] {
            XCTAssertFalse(publicError.contains(forbidden))
        }
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
}
