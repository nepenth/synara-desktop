import XCTest
@testable import Synara

final class StableTimelineViewportTests: XCTestCase {
    func testFeatureFlagDefaultsEnabledAndHonorsExplicitOverrides() {
        XCTAssertTrue(StableScrollAnchoringFeatureFlag.resolve(environmentValue: nil, persistedValue: nil))
        XCTAssertFalse(StableScrollAnchoringFeatureFlag.resolve(environmentValue: "false", persistedValue: true))
        XCTAssertTrue(StableScrollAnchoringFeatureFlag.resolve(environmentValue: "1", persistedValue: false))
        XCTAssertFalse(StableScrollAnchoringFeatureFlag.resolve(environmentValue: "unknown", persistedValue: false))
    }

    func testSnapshotsAreDeferredForDragAndInertia() {
        XCTAssertTrue(StableTimelineViewportPolicy.shouldDeferSnapshot(isDragging: true, isDecelerating: false))
        XCTAssertTrue(StableTimelineViewportPolicy.shouldDeferSnapshot(isDragging: false, isDecelerating: true))
        XCTAssertFalse(StableTimelineViewportPolicy.shouldDeferSnapshot(isDragging: false, isDecelerating: false))
        XCTAssertFalse(
            StableTimelineViewportPolicy.shouldRestoreAnchor(
                isDragging: true,
                isDecelerating: false,
                hasAnchor: true
            )
        )
    }

    func testExactAnchorDeltaRestoresPrependAndHeightChangeOffset() {
        XCTAssertEqual(
            StableTimelineViewportPolicy.restoredContentOffset(
                currentContentOffset: 900,
                previousAnchorMinY: 40,
                updatedAnchorMinY: 290
            ),
            650
        )
        XCTAssertEqual(
            StableTimelineViewportPolicy.restoredContentOffset(
                currentContentOffset: 325,
                previousAnchorMinY: -120,
                updatedAnchorMinY: -80
            ),
            285
        )
    }

    func testBottomFollowRequiresLiveAndConfirmedPinned() {
        XCTAssertTrue(StableTimelineViewportPolicy.shouldFollowNewest(isLive: true, wasConfirmedPinned: true))
        XCTAssertFalse(StableTimelineViewportPolicy.shouldFollowNewest(isLive: true, wasConfirmedPinned: false))
        XCTAssertFalse(StableTimelineViewportPolicy.shouldFollowNewest(isLive: false, wasConfirmedPinned: true))
    }

    func testPaginationThresholdHonorsInteractionAndOneInFlightRequest() {
        XCTAssertTrue(
            StableTimelineViewportPolicy.shouldRequestPagination(
                contentOffset: 8_000,
                contentHeight: 10_000,
                viewportHeight: 1_000,
                hasUserInteracted: true,
                isPaginating: false,
                requestInFlight: false
            )
        )
        XCTAssertFalse(
            StableTimelineViewportPolicy.shouldRequestPagination(
                contentOffset: 8_000,
                contentHeight: 10_000,
                viewportHeight: 1_000,
                hasUserInteracted: true,
                isPaginating: false,
                requestInFlight: true
            )
        )
        XCTAssertFalse(
            StableTimelineViewportPolicy.shouldRequestPagination(
                contentOffset: 8_000,
                contentHeight: 10_000,
                viewportHeight: 1_000,
                hasUserInteracted: false,
                isPaginating: false,
                requestInFlight: false
            )
        )
    }

    func testNewTailEventRearmsExactReadMarkerOnlyWhenPinnedAndIdle() {
        XCTAssertTrue(
            RoomTimelineReadAcknowledgementPolicy.shouldSchedule(
                isLive: true,
                isConfirmedPinned: true,
                isJumpingToLatest: false,
                isUserInteracting: false,
                eventID: "$new-tail",
                lastMarkedEventID: "$old-tail"
            )
        )
        XCTAssertFalse(
            RoomTimelineReadAcknowledgementPolicy.shouldSchedule(
                isLive: true,
                isConfirmedPinned: true,
                isJumpingToLatest: false,
                isUserInteracting: false,
                eventID: "$old-tail",
                lastMarkedEventID: "$old-tail"
            )
        )
        XCTAssertFalse(
            RoomTimelineReadAcknowledgementPolicy.shouldSchedule(
                isLive: true,
                isConfirmedPinned: true,
                isJumpingToLatest: false,
                isUserInteracting: true,
                eventID: "$new-tail",
                lastMarkedEventID: "$old-tail"
            )
        )
    }

    func testNonLiveTimelineAlwaysOffersJumpLatestEvenAtListBottom() {
        XCTAssertTrue(
            RoomTimelineJumpLatestPolicy.shouldShow(
                isLive: false,
                isConfirmedPinned: true,
                hasItems: true,
                requested: false
            )
        )
        XCTAssertFalse(
            RoomTimelineJumpLatestPolicy.shouldShow(
                isLive: true,
                isConfirmedPinned: true,
                hasItems: true,
                requested: true
            )
        )
    }

    func testFiveThousandEventInputAndVisibleCellsRemainBounded() {
        let bounded = TimelineWindowPolicy.replacingServerWindow(TimelineFixtures.largeTimeline(count: 5_000))
        XCTAssertEqual(bounded.count, TimelineWindowPolicy.stableEventLimit)
        XCTAssertEqual(bounded.first?.eventID, "$synthetic-4700:matrix.org")
        XCTAssertEqual(
            StableTimelineViewportPolicy.boundedVisibleCellUpperBound(
                viewportHeight: 1_000,
                minimumEstimatedRowHeight: 44
            ),
            31
        )
    }

    func testBoundedLRUEvictsLeastRecentlyUsedAcrossManyRooms() {
        var cache = BoundedLRUCache<String, Int>(capacity: 8)
        for index in 0..<100 {
            cache.insert(index, forKey: "!room-\(index):matrix.org|live")
        }

        XCTAssertEqual(cache.count, 8)
        XCTAssertNil(cache.value(forKey: "!room-0:matrix.org|live"))
        XCTAssertEqual(cache.value(forKey: "!room-99:matrix.org|live"), 99)

        cache.insert(100, forKey: "!room-100:matrix.org|live")
        XCTAssertEqual(cache.count, 8)
        XCTAssertEqual(cache.keysInEvictionOrder.last, "!room-100:matrix.org|live")

        cache.removeAll()
        XCTAssertEqual(cache.count, 0)
    }
}
