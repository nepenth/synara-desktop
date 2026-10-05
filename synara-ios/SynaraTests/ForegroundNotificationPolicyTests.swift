import XCTest
@testable import Synara

final class ForegroundNotificationPolicyTests: XCTestCase {
    func testOnlyExactUnambiguousMatrixEventReferencesAreEligible() {
        let target = SynaraForegroundNotificationPolicy.target(from: ["synara": ["room_id": "!room:example.org", "event_id": "$event"]])
        XCTAssertEqual(target?.roomID, "!room:example.org")
        XCTAssertEqual(target?.eventID, "$event")
        XCTAssertEqual(
            SynaraForegroundNotificationPolicy.target(from: ["room_id": "!Nhcu5BS-UMnFX7hBVfVSoXiD7OgH6iRT-xyIuqDnpYQ", "event_id": "$event"])?.roomID,
            "!Nhcu5BS-UMnFX7hBVfVSoXiD7OgH6iRT-xyIuqDnpYQ"
        )
        for payload: [AnyHashable: Any] in [
            ["room_id": " !room:example.org", "event_id": "$event"],
            ["room_id": "!room:example.org", "event_id": "$event\n"],
            ["room_id": "!room:", "event_id": "$event"],
            ["room_id": "!room:example.org", "event_id": "event"],
            ["event_id": "$event"],
            ["room_id": "!room:example.org", "event_id": "$one", "eventId": "$two"],
            ["room_id": "!room:example.org", "event_id": 42],
            ["room_id": "!room:example.org", "synara.event_id": "$one", "synara": ["event_id": "$two"]],
            ["room_id": "!room:example.org", "event_id": "$\u{202e}event"]
        ] {
            XCTAssertNil(SynaraForegroundNotificationPolicy.target(from: payload))
        }
    }

    func testSuppressionFailsOpenWhenAccountOrForegroundAuthorityChanges() {
        func suppressed(allowed: Bool = false, active: Bool = true, sameSession: Bool = true, epoch: Int = 1) -> Bool {
            SynaraForegroundNotificationPolicy.shouldSuppress(
                eventAllowed: allowed, foregroundActive: active, sameSession: sameSession,
                initialAccountEpoch: 1, currentAccountEpoch: epoch
            )
        }
        XCTAssertTrue(suppressed())
        XCTAssertFalse(suppressed(allowed: true))
        XCTAssertFalse(suppressed(active: false))
        XCTAssertFalse(suppressed(sameSession: false))
        XCTAssertFalse(suppressed(epoch: 2))
    }
}
