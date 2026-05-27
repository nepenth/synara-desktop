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
            isEdited: true,
            mediaURL: nil
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
            isEdited: false,
            mediaURL: nil
        )

        XCTAssertEqual(TimelineMapper.map(event).kind, .unknown(type: "synara.agent.card"))
    }

    func testMediaEventsUseSafeResourceDescription() throws {
        let mediaURL = try XCTUnwrap(URL(string: "mxc://matrix.org/private-media-id"))
        let event = RawTimelineEvent(
            eventID: "$media:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            type: "m.room.media",
            body: "photo.jpg",
            replyToEventID: nil,
            isEdited: false,
            mediaURL: mediaURL
        )

        let item = TimelineMapper.map(event)

        guard case .mediaPlaceholder(let resource) = item.kind else {
            XCTFail("Expected media placeholder")
            return
        }
        XCTAssertEqual(resource.safeDescription, "photo.jpg")
        XCTAssertFalse(resource.safeDescription.contains("matrix.org"))
        XCTAssertTrue(resource.requiresAuthentication)
    }

    func testMockTimelineCanLoadInitialAndOlderEvents() async {
        let service = MockTimelineService()

        let initial = await service.loadInitialTimeline(roomID: "!room:matrix.org")
        let older = await service.loadOlderTimeline(roomID: "!room:matrix.org", before: initial[0].eventID)

        XCTAssertEqual(initial.count, 4)
        XCTAssertEqual(older.count, 3)
    }

    func testLargeTimelineFixtureHasStableIdentity() {
        let items = TimelineFixtures.largeTimeline()

        XCTAssertEqual(items.count, 10_000)
        XCTAssertEqual(Set(items.map(\.id)).count, 10_000)
    }
}
