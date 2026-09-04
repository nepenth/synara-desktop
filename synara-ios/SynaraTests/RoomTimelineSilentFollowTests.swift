@testable import Synara
import XCTest

final class RoomTimelineSilentFollowTests: XCTestCase {
    func testPinnedBottomOnNonLiveProviderFollowsExactlyOnce() {
        XCTAssertTrue(
            RoomTimelineSilentFollowPolicy.shouldFollow(
                isPinned: true,
                providerIsLive: false,
                isJumpingToLatest: false,
                isSilentlyFollowingLive: false,
                paintedTailEventID: "$tail:example.org"
            )
        )
    }

    func testLiveProviderNeverFollows() {
        XCTAssertFalse(
            RoomTimelineSilentFollowPolicy.shouldFollow(
                isPinned: true,
                providerIsLive: true,
                isJumpingToLatest: false,
                isSilentlyFollowingLive: false,
                paintedTailEventID: "$tail:example.org"
            )
        )
    }

    func testUnpinnedBottomNeverFollows() {
        XCTAssertFalse(
            RoomTimelineSilentFollowPolicy.shouldFollow(
                isPinned: false,
                providerIsLive: false,
                isJumpingToLatest: false,
                isSilentlyFollowingLive: false,
                paintedTailEventID: "$tail:example.org"
            )
        )
    }

    func testExplicitJumpTakesPrecedenceOverSilentFollow() {
        XCTAssertFalse(
            RoomTimelineSilentFollowPolicy.shouldFollow(
                isPinned: true,
                providerIsLive: false,
                isJumpingToLatest: true,
                isSilentlyFollowingLive: false,
                paintedTailEventID: "$tail:example.org"
            )
        )
    }

    func testInflightFollowIsNotDuplicated() {
        XCTAssertFalse(
            RoomTimelineSilentFollowPolicy.shouldFollow(
                isPinned: true,
                providerIsLive: false,
                isJumpingToLatest: false,
                isSilentlyFollowingLive: true,
                paintedTailEventID: "$tail:example.org"
            )
        )
    }

    func testMissingPaintedTailFailsClosed() {
        XCTAssertFalse(
            RoomTimelineSilentFollowPolicy.shouldFollow(
                isPinned: true,
                providerIsLive: false,
                isJumpingToLatest: false,
                isSilentlyFollowingLive: false,
                paintedTailEventID: nil
            )
        )
        XCTAssertFalse(
            RoomTimelineSilentFollowPolicy.shouldFollow(
                isPinned: true,
                providerIsLive: false,
                isJumpingToLatest: false,
                isSilentlyFollowingLive: false,
                paintedTailEventID: "not-an-event"
            )
        )
    }
}
