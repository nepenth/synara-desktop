import XCTest
@testable import Synara

final class TimelineServiceTests: XCTestCase {
    func testMapperKeepsStableIdentityAndMetadata() {
        let event = RawTimelineEvent(
            eventID: "$event:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            type: "m.room.message",
            body: "Hello",
            replyToEventID: "$parent:matrix.org",
            isEdited: true
        )

        let item = TimelineMapper.map(event)

        XCTAssertEqual(item.id, "$event:matrix.org")
        XCTAssertEqual(item.eventID, "$event:matrix.org")
        XCTAssertEqual(item.senderID, "@alice:matrix.org")
        XCTAssertEqual(item.kind, .text("Hello"))
        XCTAssertEqual(item.replyToEventID, "$parent:matrix.org")
        XCTAssertTrue(item.isEdited)
    }

    func testUnknownEventsRenderAsSafePlaceholders() {
        let event = RawTimelineEvent(
            eventID: "$unknown:matrix.org",
            senderID: "@agent:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            type: "synara.agent.card",
            body: nil,
            replyToEventID: nil,
            isEdited: false
        )

        XCTAssertEqual(TimelineMapper.map(event).kind, .unknown(type: "synara.agent.card"))
    }

    func testMockTimelineCanLoadInitialAndOlderEvents() async {
        let service = MockTimelineService()

        let initial = await service.loadInitialTimeline(roomID: "!room:matrix.org")
        let older = await service.loadOlderTimeline(roomID: "!room:matrix.org", before: initial[0].eventID)

        XCTAssertEqual(initial.count, 3)
        XCTAssertEqual(older.count, 2)
    }
}
