import XCTest
@testable import Synara

final class RoomUnreadPresentationTests: XCTestCase {
    func testInvitedRoomsAlwaysShowUnreadAndHighlight() {
        let unread = RoomUnreadPresentation.make(membership: .invited)

        XCTAssertEqual(unread.unreadCount, 1)
        XCTAssertTrue(unread.hasHighlight)
    }

    func testUnreadCountUsesCanonicalSDKCounters() {
        let unread = RoomUnreadPresentation.make(
            membership: .joined,
            numUnreadMessages: 4,
            numUnreadNotifications: 6
        )

        XCTAssertEqual(unread.unreadCount, 6)
        XCTAssertFalse(unread.hasHighlight)
    }

    func testMarkedUnreadShowsAtLeastOneBadgeWhenCountsAreZero() {
        let unread = RoomUnreadPresentation.make(
            membership: .joined,
            isMarkedUnread: true
        )

        XCTAssertEqual(unread.unreadCount, 1)
        XCTAssertFalse(unread.hasHighlight)
    }

    func testCanonicalMentionCountSetsHighlightState() {
        let mentionUnread = RoomUnreadPresentation.make(
            membership: .joined,
            numUnreadMentions: 1
        )

        XCTAssertTrue(mentionUnread.hasHighlight)
    }
}
