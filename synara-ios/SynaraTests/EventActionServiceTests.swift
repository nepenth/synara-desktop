import XCTest
@testable import Synara

final class EventActionServiceTests: XCTestCase {
    func testAvailabilityAllowsAuthorToEditAndRedact() {
        let service = MockEventActionService()
        let item = makeItem(senderID: "@alice:matrix.org")

        let availability = service.availability(for: item, currentUserID: "@alice:matrix.org")

        XCTAssertTrue(availability.canReply)
        XCTAssertTrue(availability.canEdit)
        XCTAssertTrue(availability.canRedact)
        XCTAssertTrue(availability.canReact)
    }

    func testRedactedEventsHaveNoActions() {
        let service = MockEventActionService()
        let item = TimelineItem(
            id: "$redacted:matrix.org",
            eventID: "$redacted:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .redacted,
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )

        let availability = service.availability(for: item, currentUserID: "@alice:matrix.org")

        XCTAssertFalse(availability.canReply)
        XCTAssertFalse(availability.canEdit)
        XCTAssertFalse(availability.canRedact)
        XCTAssertFalse(availability.canReact)
    }

    func testReactAggregatesWithoutDuplicateLocalEcho() async {
        let service = MockEventActionService()
        let item = makeItem(senderID: "@alice:matrix.org")

        let updated = await service.apply(.react("👍"), to: item, currentUserID: "@bob:matrix.org")

        XCTAssertEqual(updated.id, item.id)
        XCTAssertEqual(updated.reactions["👍"], 1)
    }

    func testRedactKeepsStableIdentity() async {
        let service = MockEventActionService()
        let item = makeItem(senderID: "@alice:matrix.org")

        let updated = await service.apply(.redact, to: item, currentUserID: "@alice:matrix.org")

        XCTAssertEqual(updated.id, item.id)
        XCTAssertEqual(updated.kind, .redacted)
    }

    private func makeItem(senderID: String) -> TimelineItem {
        TimelineItem(
            id: "$event:matrix.org",
            eventID: "$event:matrix.org",
            senderID: senderID,
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Hello"),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:]
        )
    }
}
