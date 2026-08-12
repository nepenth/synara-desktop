import XCTest
import SynaraCore
@testable import Synara

final class RoomUnreadPresentationTests: XCTestCase {
    func testAdapterInvitedRoomsAlwaysShowUnreadAndHighlight() {
        let unread = RoomUnreadPresentation.make(
            membership: .invited,
            numUnreadMessages: .max,
            numUnreadNotifications: .max,
            numUnreadMentions: .max,
            isMarkedUnread: true
        )

        XCTAssertEqual(unread.unreadCount, 1)
        XCTAssertTrue(unread.hasHighlight)
    }

    func testAdapterUnreadCountUsesTheLargerCanonicalSDKCounter() {
        let notificationCount = RoomUnreadPresentation.make(
            membership: .joined,
            numUnreadMessages: 4,
            numUnreadNotifications: 6
        )
        let messageCount = RoomUnreadPresentation.make(
            membership: .joined,
            numUnreadMessages: 7,
            numUnreadNotifications: 3
        )

        XCTAssertEqual(notificationCount.unreadCount, 6)
        XCTAssertEqual(messageCount.unreadCount, 7)
        XCTAssertFalse(notificationCount.hasHighlight)
        XCTAssertFalse(messageCount.hasHighlight)
    }

    func testAdapterMarkedUnreadShowsAtLeastOneBadgeWhenCountsAreZero() {
        let unread = RoomUnreadPresentation.make(
            membership: .joined,
            isMarkedUnread: true
        )

        XCTAssertEqual(unread.unreadCount, 1)
        XCTAssertFalse(unread.hasHighlight)
    }

    func testAdapterCanonicalMentionCountSetsHighlightState() {
        let mentionUnread = RoomUnreadPresentation.make(
            membership: .joined,
            numUnreadMentions: 1
        )

        XCTAssertEqual(mentionUnread.unreadCount, 0)
        XCTAssertTrue(mentionUnread.hasHighlight)
    }

    func testGeneratedCoreBindingExecutesAndPreservesFullWidthJoinedCount() {
        let presentation = SynaraCore.roomUnreadPresentation(
            membership: .joined,
            numUnreadMessages: .max,
            numUnreadNotifications: 1,
            numUnreadMentions: 0,
            isMarkedUnread: false
        )

        XCTAssertEqual(presentation.unreadCount, .max)
        XCTAssertFalse(presentation.hasHighlight)
    }
}
