@testable import Synara
import XCTest

final class RoomReadMarkerServiceTests: XCTestCase {
    func testServerEventPolicyAcceptsOnlyPersistedMatrixEventIDs() {
        XCTAssertTrue(MatrixServerEventIDPolicy.canAcknowledge("$event:matrix.example"))
        XCTAssertFalse(MatrixServerEventIDPolicy.canAcknowledge("$pending-local"))
        XCTAssertFalse(MatrixServerEventIDPolicy.canAcknowledge("$local-generated"))
        XCTAssertFalse(MatrixServerEventIDPolicy.canAcknowledge("transaction-123"))
        XCTAssertFalse(MatrixServerEventIDPolicy.canAcknowledge(""))
    }

    func testMockMarkRoomAsReadUsesLatestEventMarker() async {
        let service = MockRoomReadMarkerService()

        let acknowledgedEventID = await service.markRoomAsRead(roomID: "!room:matrix.example")

        XCTAssertEqual(acknowledgedEventID, "$latest:!room:matrix.example")
        XCTAssertEqual(service.eventID, "$latest:!room:matrix.example")
    }

    func testExplicitMarkReadIsNilWithoutAFullyReadReceipt() {
        XCTAssertNil(
            ExplicitRoomReadReceipt.acknowledgedEventID(receiptSent: false, acknowledgedEventID: "$event:example.org")
        )
        XCTAssertNil(
            ExplicitRoomReadReceipt.acknowledgedEventID(receiptSent: nil, acknowledgedEventID: "$event:example.org")
        )
        XCTAssertEqual(
            ExplicitRoomReadReceipt.acknowledgedEventID(receiptSent: true, acknowledgedEventID: "$event:example.org"),
            "$event:example.org"
        )
    }

    @MainActor
    func testRejectedAuthenticationRetiresWithoutWipeOrRemoteLogout() throws {
        XCTAssertTrue(
            RejectedAuthenticationRetirement.shouldRetire(
                failureDiagnosticID: "p4.1-session-authentication-rejected"
            )
        )
        XCTAssertFalse(RejectedAuthenticationRetirement.shouldRetire(failureDiagnosticID: "p4.1-sync-service-error"))
        XCTAssertFalse(RejectedAuthenticationRetirement.shouldRetire(failureDiagnosticID: nil))

        let service = try String(
            contentsOfFile: Self.repositoryRoot() + "/synara-ios/Synara/Services/SharedCoreProductServices.swift",
            encoding: .utf8
        )
        let retirement = service.components(separatedBy: "private func retireRejectedAuthentication").dropFirst().first
            ?? ""
        let retirementBody = retirement.components(separatedBy: "private func publish").first ?? retirement
        XCTAssertFalse(retirementBody.contains("wipePersistedStores"))
        XCTAssertFalse(retirementBody.contains("revokeServerSession"))
        XCTAssertFalse(retirementBody.contains("logoutAndWipe"))
        XCTAssertTrue(retirementBody.contains("forgetPersistedSession"))
        XCTAssertTrue(retirementBody.contains("noteSessionExpired"))
        XCTAssertTrue(retirementBody.contains("signOut()"))

        let store = AppSessionStore()
        store.noteSessionExpired()
        try store.signOut()
        XCTAssertEqual(store.currentState, .signedOut)
        XCTAssertTrue(store.sessionExpiredNotice)
    }

    private static func repositoryRoot() -> String {
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
