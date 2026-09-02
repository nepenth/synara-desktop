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
}
