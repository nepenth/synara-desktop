import XCTest
@testable import Synara

final class RoomUnreadPresentationTests: XCTestCase {
    func testInvitedRoomsAlwaysShowUnreadAndHighlight() {
        let unread = RoomUnreadPresentation.make(membership: .invited)

        XCTAssertEqual(unread.unreadCount, 1)
        XCTAssertTrue(unread.hasHighlight)
    }

    func testUnreadCountPrefersMessageCountOverNotificationOnlyFields() {
        let unread = RoomUnreadPresentation.make(
            membership: .joined,
            numUnreadMessages: 4,
            numUnreadNotifications: 0,
            notificationCount: 0
        )

        XCTAssertEqual(unread.unreadCount, 4)
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

    func testMentionsAndHighlightsSetHighlightState() {
        let mentionUnread = RoomUnreadPresentation.make(
            membership: .joined,
            numUnreadMentions: 1
        )
        let highlightUnread = RoomUnreadPresentation.make(
            membership: .joined,
            highlightCount: 2
        )

        XCTAssertTrue(mentionUnread.hasHighlight)
        XCTAssertTrue(highlightUnread.hasHighlight)
    }
}