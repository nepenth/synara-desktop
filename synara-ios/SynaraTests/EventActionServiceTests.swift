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

    func testFailedLocalTextMessagesCanBeEdited() {
        let service = MockEventActionService()
        let item = TimelineItem.pendingMessage(
            body: "Retry me",
            senderID: "@alice:matrix.org",
            replyToEventID: nil,
            deliveryStatus: .failed
        )

        let availability = service.availability(for: item, currentUserID: "@alice:matrix.org")

        XCTAssertFalse(availability.canReply)
        XCTAssertTrue(availability.canEdit)
        XCTAssertFalse(availability.canRedact)
        XCTAssertFalse(availability.canReact)
    }

    func testSendingLocalMessagesHaveNoActions() {
        let service = MockEventActionService()
        let item = TimelineItem.pendingMessage(
            body: "Still sending",
            senderID: "@alice:matrix.org",
            replyToEventID: nil,
            deliveryStatus: .sending
        )

        let availability = service.availability(for: item, currentUserID: "@alice:matrix.org")

        XCTAssertFalse(availability.canReply)
        XCTAssertFalse(availability.canEdit)
        XCTAssertFalse(availability.canRedact)
        XCTAssertFalse(availability.canReact)
    }

    func testFailedLocalMessagesFromOthersCannotBeEdited() {
        let service = MockEventActionService()
        let item = TimelineItem.pendingMessage(
            body: "Retry me",
            senderID: "@bob:matrix.org",
            replyToEventID: nil,
            deliveryStatus: .failed
        )

        let availability = service.availability(for: item, currentUserID: "@alice:matrix.org")

        XCTAssertFalse(availability.canEdit)
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

    func testEncryptedMediaHasNoActions() throws {
        let service = MockEventActionService()
        let item = TimelineItem(
            id: "$encrypted-media:matrix.org",
            eventID: "$encrypted-media:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .mediaPlaceholder(
                MediaResource(
                    id: "$encrypted-media:matrix.org",
                    filename: "secret.png",
                    authenticatedURL: try XCTUnwrap(URL(string: "mxc://matrix.org/secret")),
                    requiresAuthentication: true,
                    isEncrypted: true
                )
            ),
            replyToEventID: nil,
            isEdited: false,
            reactions: [:],
            isEncrypted: true
        )

        let availability = service.availability(for: item, currentUserID: "@alice:matrix.org")

        XCTAssertFalse(availability.canReply)
        XCTAssertFalse(availability.canEdit)
        XCTAssertFalse(availability.canRedact)
        XCTAssertFalse(availability.canReact)
    }

    func testReactAggregatesWithoutDuplicateLocalEcho() async throws {
        let service = MockEventActionService()
        let item = makeItem(senderID: "@alice:matrix.org")

        let updated = try await service.apply(.react("👍"), to: item, currentUserID: "@bob:matrix.org", roomID: "!room:matrix.org")

        XCTAssertEqual(updated.id, item.id)
        XCTAssertEqual(updated.reactions["👍"], 1)
    }

    func testRedactKeepsStableIdentity() async throws {
        let service = MockEventActionService()
        let item = makeItem(senderID: "@alice:matrix.org")

        let updated = try await service.apply(.redact, to: item, currentUserID: "@alice:matrix.org", roomID: "!room:matrix.org")

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
