@testable import Synara
import XCTest

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
                contentOffset: 8000,
                contentHeight: 10000,
                viewportHeight: 1000,
                hasUserInteracted: true,
                isPaginating: false,
                requestInFlight: false
            )
        )
        XCTAssertFalse(
            StableTimelineViewportPolicy.shouldRequestPagination(
                contentOffset: 8000,
                contentHeight: 10000,
                viewportHeight: 1000,
                hasUserInteracted: true,
                isPaginating: false,
                requestInFlight: true
            )
        )
        XCTAssertFalse(
            StableTimelineViewportPolicy.shouldRequestPagination(
                contentOffset: 8000,
                contentHeight: 10000,
                viewportHeight: 1000,
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

    func testReadMarkerQueueHasMaximumLatencyAndFlushesOnRoomSwitch() {
        let firstQueuedAt = Date(timeIntervalSince1970: 100)

        XCTAssertEqual(
            RoomTimelineReadMarkerQueuePolicy.delayNanoseconds(
                firstQueuedAt: firstQueuedAt,
                now: firstQueuedAt.addingTimeInterval(1.75),
                debounceNanoseconds: 1_000_000_000,
                maximumLatencyNanoseconds: 2_000_000_000
            ),
            250_000_000
        )
        XCTAssertEqual(
            RoomTimelineReadMarkerQueuePolicy.flushCandidate(
                pendingEventID: "$new-server-event",
                lastCandidateEventID: "$older-server-event",
                lastMarkedEventID: nil
            ),
            "$new-server-event"
        )
        XCTAssertNil(
            RoomTimelineReadMarkerQueuePolicy.flushCandidate(
                pendingEventID: nil,
                lastCandidateEventID: "$already-marked",
                lastMarkedEventID: "$already-marked"
            )
        )
    }

    func testMissingFocusRetriesAreBounded() {
        XCTAssertTrue(StableTimelineViewportPolicy.shouldRetryMissingTarget(attempt: 1))
        XCTAssertTrue(StableTimelineViewportPolicy.shouldRetryMissingTarget(attempt: 2))
        XCTAssertFalse(StableTimelineViewportPolicy.shouldRetryMissingTarget(attempt: 3))
        XCTAssertFalse(StableTimelineViewportPolicy.shouldRetryMissingTarget(attempt: 4))
    }

    func testAnimatedCommandSettlesForNoOpInterruptionTimeoutAndDelegate() {
        XCTAssertTrue(StableTimelineViewportPolicy.animatedCommandSucceeded(
            settlement: .noOp,
            targetsLatest: false,
            isTargetVisible: true,
            isConfirmedPinned: false
        ))
        XCTAssertTrue(StableTimelineViewportPolicy.animatedCommandSucceeded(
            settlement: .animationEnded,
            targetsLatest: true,
            isTargetVisible: true,
            isConfirmedPinned: true
        ))
        XCTAssertFalse(StableTimelineViewportPolicy.animatedCommandSucceeded(
            settlement: .userInterrupted,
            targetsLatest: true,
            isTargetVisible: true,
            isConfirmedPinned: false
        ))
        XCTAssertFalse(StableTimelineViewportPolicy.animatedCommandSucceeded(
            settlement: .timeout,
            targetsLatest: true,
            isTargetVisible: true,
            isConfirmedPinned: false
        ))
    }

    func testFocusedPlacementCompletionAllowsPaginationPolicy() {
        XCTAssertTrue(
            RoomTimelinePaginationPolicy.shouldLoadOlderHistory(
                rowIndex: 0,
                topThreshold: 3,
                hasUserInteractedWithTimeline: true,
                hasPositionedInitialTimeline: true,
                isJumpingToLatest: false,
                isPaginating: false,
                hasReachedOldestMessages: false
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
        let bounded = TimelineWindowPolicy.replacingServerWindow(TimelineFixtures.largeTimeline(count: 5000))
        XCTAssertEqual(bounded.count, TimelineWindowPolicy.stableEventLimit)
        XCTAssertEqual(bounded.first?.eventID, "$synthetic-4700:matrix.org")
        XCTAssertEqual(
            StableTimelineViewportPolicy.boundedVisibleCellUpperBound(
                viewportHeight: 1000,
                minimumEstimatedRowHeight: 44
            ),
            31
        )
    }

    func testBoundedLRUEvictsLeastRecentlyUsedAcrossManyRooms() {
        var cache = BoundedLRUCache<String, Int>(capacity: 8)
        for index in 0 ..< 100 {
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
