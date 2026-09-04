@testable import Synara
import XCTest

final class RoomTimelineSilentFollowTests: XCTestCase {
    func testPinnedBottomOnNonLiveProviderFollowsExactlyOnce() {
        XCTAssertTrue(
            RoomTimelineSilentFollowPolicy.shouldFollow(
                isPinned: true,
                providerIsLive: false,
                isJumpingToLatest: false,
                isSilentlyFollowingLive: false
            )
        )
    }

    func testLiveProviderNeverFollows() {
        XCTAssertFalse(
            RoomTimelineSilentFollowPolicy.shouldFollow(
                isPinned: true,
                providerIsLive: true,
                isJumpingToLatest: false,
                isSilentlyFollowingLive: false
            )
        )
    }

    func testUnpinnedBottomNeverFollows() {
        XCTAssertFalse(
            RoomTimelineSilentFollowPolicy.shouldFollow(
                isPinned: false,
                providerIsLive: false,
                isJumpingToLatest: false,
                isSilentlyFollowingLive: false
            )
        )
    }

    func testExplicitJumpTakesPrecedenceOverSilentFollow() {
        XCTAssertFalse(
            RoomTimelineSilentFollowPolicy.shouldFollow(
                isPinned: true,
                providerIsLive: false,
                isJumpingToLatest: true,
                isSilentlyFollowingLive: false
            )
        )
    }

    func testInflightFollowIsNotDuplicated() {
        XCTAssertFalse(
            RoomTimelineSilentFollowPolicy.shouldFollow(
                isPinned: true,
                providerIsLive: false,
                isJumpingToLatest: false,
                isSilentlyFollowingLive: true
            )
        )
    }
}
