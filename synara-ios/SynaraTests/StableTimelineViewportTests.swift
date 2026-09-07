@testable import Synara
import XCTest
import SwiftUI
import UIKit

final class StableTimelineViewportTests: XCTestCase {
    @MainActor
    func testUnchangedTimelineUpdatesDoNotWriteScrollOffset() async throws {
        let rows = TimelineFixtures.largeTimeline(count: 60).map { item in
            StableTimelineViewportRow(id: .event(item.id), content: .event(.init(
                item: item, isGroupedWithPrevious: false, isTimestampRevealed: false,
                animateSend: false, replyPreview: nil, replyCount: 0,
                availability: .init(canReply: false, canEdit: false, canRedact: false, canReact: false)
            )))
        }
        let viewport = StableTimelineViewport(
            routeID: "fixture", sessionGeneration: 1, rows: rows, command: nil,
            isLive: false, isPaginating: false, backgroundColor: .black,
            rowContent: { row in AnyView(Text(row.eventID ?? "").frame(height: 70)) },
            onBottomPinnedChanged: { _, _, _, _ in },
            onUserInteractionChanged: { _, _, _ in },
            onPaginationThresholdReached: { _, _, _ in false },
            onTimestampRevealRequested: { _, _, _ in },
            onCommandCompleted: { _, _, _, _, _ in }
        )
        let controller = StableTimelineViewController(coordinator: viewport.makeCoordinator())
        let window = UIWindow(frame: CGRect(x: 0, y: 0, width: 390, height: 844))
        window.rootViewController = controller
        window.makeKeyAndVisible()
        defer { window.isHidden = true }
        let configuration = StableTimelineViewController.Configuration(
            routeID: "fixture", sessionGeneration: 1, rows: rows, command: nil,
            isLive: false, isPaginating: false, backgroundColor: .black
        )
        controller.update(configuration: configuration)
        try await Task.sleep(nanoseconds: 100_000_000)
        let table = try XCTUnwrap(controller.view.subviews.first { $0 is UITableView } as? UITableView)
        XCTAssertEqual(table.numberOfRows(inSection: 0), 60)
        table.setContentOffset(CGPoint(x: 0, y: 700), animated: false)
        table.layoutIfNeeded()
        try await Task.sleep(nanoseconds: 100_000_000)
        let initialOffset = table.contentOffset
        var offsetWrites = 0
        let observation = table.observe(\.contentOffset, options: [.new]) { _, _ in offsetWrites += 1 }
        defer { observation.invalidate() }
        for _ in 0..<20 { controller.update(configuration: configuration) }
        try await Task.sleep(nanoseconds: 100_000_000)
        XCTAssertEqual(offsetWrites, 0, "Unchanged rows must not run anchor restoration")
        XCTAssertEqual(table.contentOffset.y, initialOffset.y, accuracy: 0.5)
    }

    func testTimestampRevealGestureWaitsUntilDirectionIsKnown() {
        XCTAssertEqual(
            TimelineTimestampRevealGesturePolicy.intent(translation: CGPoint(x: -8, y: 7)),
            .pending
        )
        XCTAssertEqual(
            TimelineTimestampRevealGesturePolicy.intent(translation: CGPoint(x: -11, y: 2)),
            .pending
        )
    }

    func testTimestampRevealGestureAcceptsOnlyDominantLeftwardIntent() {
        XCTAssertEqual(
            TimelineTimestampRevealGesturePolicy.intent(translation: CGPoint(x: -20, y: 5)),
            .reveal
        )
        XCTAssertEqual(
            TimelineTimestampRevealGesturePolicy.intent(translation: CGPoint(x: -15, y: 14)),
            .reject
        )
        XCTAssertEqual(
            TimelineTimestampRevealGesturePolicy.intent(translation: CGPoint(x: 20, y: 2)),
            .reject
        )
    }

    func testTimestampRevealGestureRejectsVerticalScrollingAtActivationThreshold() {
        XCTAssertEqual(
            TimelineTimestampRevealGesturePolicy.intent(translation: CGPoint(x: -2, y: -12)),
            .reject
        )
        XCTAssertEqual(
            TimelineTimestampRevealGesturePolicy.intent(translation: CGPoint(x: 4, y: 24)),
            .reject
        )
    }

    func testTimestampRevealGestureKeepsHorizontalIntentLatchedAfterRecognition() {
        XCTAssertEqual(
            TimelineTimestampRevealGesturePolicy.intent(
                translation: CGPoint(x: -20, y: 80),
                hasLockedHorizontalIntent: true
            ),
            .reveal
        )
    }

    func testTimestampRevealGestureRequiresExclusiveTrackedTouchOwnership() {
        XCTAssertTrue(
            TimelineTimestampRevealGesturePolicy.ownsSingleTouch(
                activeTouchCount: 1,
                isTrackedTouch: true
            )
        )
        XCTAssertFalse(
            TimelineTimestampRevealGesturePolicy.ownsSingleTouch(
                activeTouchCount: 2,
                isTrackedTouch: true
            )
        )
        XCTAssertFalse(
            TimelineTimestampRevealGesturePolicy.ownsSingleTouch(
                activeTouchCount: 1,
                isTrackedTouch: false
            )
        )
    }

    func testTimestampRevealGestureUsesValidCancellationTransition() {
        XCTAssertEqual(
            TimelineTimestampRevealGesturePolicy.cancellationState(from: .possible),
            .failed
        )
        XCTAssertEqual(
            TimelineTimestampRevealGesturePolicy.cancellationState(from: .began),
            .cancelled
        )
        XCTAssertEqual(
            TimelineTimestampRevealGesturePolicy.cancellationState(from: .changed),
            .cancelled
        )
        XCTAssertNil(TimelineTimestampRevealGesturePolicy.cancellationState(from: .ended))
    }

    func testTimestampRevealSessionRequiresTheCurrentlyAppliedSnapshot() {
        XCTAssertTrue(
            TimelineTimestampRevealGesturePolicy.sessionMatchesAppliedSnapshot(
                sessionRevision: 7,
                appliedRevision: 7
            )
        )
        XCTAssertFalse(
            TimelineTimestampRevealGesturePolicy.sessionMatchesAppliedSnapshot(
                sessionRevision: 7,
                appliedRevision: 8
            )
        )
        XCTAssertFalse(
            TimelineTimestampRevealGesturePolicy.sessionMatchesAppliedSnapshot(
                sessionRevision: 7,
                appliedRevision: nil
            )
        )
    }

    func testTimestampRevealAutoReturnsAfterTwoAndAHalfSeconds() {
        XCTAssertEqual(
            RoomTimelineTimestampRevealPolicy.displayDurationNanoseconds,
            2_500_000_000
        )
    }

    func testTimestampRevealOffsetAppliesOnlyToGroupedRevealedRows() {
        XCTAssertEqual(
            RoomTimelineTimestampRevealPolicy.horizontalOffset(
                isGroupedWithPrevious: true,
                isRevealed: true,
                width: 64
            ),
            -64
        )
        XCTAssertEqual(
            RoomTimelineTimestampRevealPolicy.horizontalOffset(
                isGroupedWithPrevious: true,
                isRevealed: false,
                width: 64
            ),
            0
        )
        XCTAssertEqual(
            RoomTimelineTimestampRevealPolicy.horizontalOffset(
                isGroupedWithPrevious: false,
                isRevealed: true,
                width: 64
            ),
            0
        )
    }

    func testTimestampRevealDismissTaskMustOwnLatestRestartedGeneration() {
        XCTAssertTrue(
            RoomTimelineTimestampRevealPolicy.taskMayDismiss(
                taskGeneration: 8,
                currentGeneration: 8,
                taskEventID: "$second",
                revealedEventID: "$second",
                isCancelled: false
            )
        )
        XCTAssertFalse(
            RoomTimelineTimestampRevealPolicy.taskMayDismiss(
                taskGeneration: 7,
                currentGeneration: 8,
                taskEventID: "$second",
                revealedEventID: "$second",
                isCancelled: false
            )
        )
        XCTAssertFalse(
            RoomTimelineTimestampRevealPolicy.taskMayDismiss(
                taskGeneration: 8,
                currentGeneration: 8,
                taskEventID: "$first",
                revealedEventID: "$second",
                isCancelled: false
            )
        )
        XCTAssertFalse(
            RoomTimelineTimestampRevealPolicy.taskMayDismiss(
                taskGeneration: 8,
                currentGeneration: 8,
                taskEventID: "$second",
                revealedEventID: "$second",
                isCancelled: true
            )
        )
    }

    func testOwnAvatarHydrationInstallsOnlyForTheActiveUserAndTimelineTask() {
        XCTAssertTrue(
            RoomTimelineOwnAvatarPolicy.mayInstall(
                profileUserID: "@alice:matrix.org",
                expectedUserID: "@alice:matrix.org",
                expectedTimelineTaskID: "!room:matrix.org",
                currentTimelineTaskID: "!room:matrix.org",
                isCancelled: false
            )
        )
        XCTAssertFalse(
            RoomTimelineOwnAvatarPolicy.mayInstall(
                profileUserID: "@alice:matrix.org",
                expectedUserID: "@alice:matrix.org",
                expectedTimelineTaskID: "!old:matrix.org",
                currentTimelineTaskID: "!new:matrix.org",
                isCancelled: false
            )
        )
        XCTAssertFalse(
            RoomTimelineOwnAvatarPolicy.mayInstall(
                profileUserID: "@other:matrix.org",
                expectedUserID: "@alice:matrix.org",
                expectedTimelineTaskID: "!room:matrix.org",
                currentTimelineTaskID: "!room:matrix.org",
                isCancelled: false
            )
        )
        XCTAssertFalse(
            RoomTimelineOwnAvatarPolicy.mayInstall(
                profileUserID: "@alice:matrix.org",
                expectedUserID: "@alice:matrix.org",
                expectedTimelineTaskID: "!room:matrix.org",
                currentTimelineTaskID: "!room:matrix.org",
                isCancelled: true
            )
        )
    }

    func testReceiptFrontierOnlyChangeReconfiguresTheSameVisibleRow() throws {
        let original = try XCTUnwrap(TimelineFixtures.largeTimeline(count: 1).first)
        var edited = original
        edited.readReceiptEventID = "$edit:example.org"
        let changed = StableTimelineViewportPolicy.changedIdentifiers(
            currentIDs: Set([original.id]), currentValues: [original.id: original],
            incomingValues: [(id: edited.id, value: edited)]
        )
        XCTAssertEqual(changed, [original.id])
        XCTAssertEqual(edited.eventID, original.eventID)
        XCTAssertEqual(edited.withSenderAvatarURL(nil).readReceiptEventID, "$edit:example.org")
        XCTAssertEqual(edited.withDeliveryStatus(nil).readReceiptEventID, "$edit:example.org")
        XCTAssertTrue(RoomTimelineReadAcknowledgementPolicy.shouldSchedule(
            isApplicationActive: true, allowsReadReceipts: true, isLive: true,
            isConfirmedPinned: true, isJumpingToLatest: false, isUserInteracting: false,
            eventID: try XCTUnwrap(edited.readReceiptEventID), lastMarkedEventID: original.eventID
        ))
    }

    func testContentOnlyRowChangesAreReconfiguredForStableIdentifiers() {
        let changed = StableTimelineViewportPolicy.changedIdentifiers(
            currentIDs: Set(["message", "unchanged", "removed"]),
            currentValues: [
                "message": false,
                "unchanged": true,
                "removed": false,
            ],
            incomingValues: [
                (id: "message", value: true),
                (id: "unchanged", value: true),
                (id: "inserted", value: true),
            ]
        )

        XCTAssertEqual(changed, ["message"])
    }

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

    func testSnapshotApplyGateCoalescesReentrantRequestsToNewestState() throws {
        var gate = StableTimelineSnapshotApplyGate<String>()

        let first = try XCTUnwrap(gate.schedule(value: "revision-1", resetPosition: false))
        XCTAssertEqual(first.value, "revision-1")
        XCTAssertTrue(gate.isApplying)
        XCTAssertNil(gate.schedule(value: "revision-2", resetPosition: true))
        XCTAssertNil(gate.schedule(value: "revision-3", resetPosition: false))

        let coalesced = try XCTUnwrap(gate.complete())
        XCTAssertEqual(coalesced.value, "revision-3")
        XCTAssertTrue(coalesced.resetPosition)
        XCTAssertFalse(gate.isApplying)

        let newest = try XCTUnwrap(
            gate.schedule(value: coalesced.value, resetPosition: coalesced.resetPosition)
        )
        XCTAssertEqual(newest.value, "revision-3")
        XCTAssertTrue(newest.resetPosition)
        XCTAssertNil(gate.complete())
        XCTAssertFalse(gate.isApplying)
    }

    func testSnapshotApplyGateCanDiscardSupersededPendingGeneration() throws {
        var gate = StableTimelineSnapshotApplyGate<Int>()

        _ = try XCTUnwrap(gate.schedule(value: 1, resetPosition: false))
        XCTAssertNil(gate.schedule(value: 2, resetPosition: false))
        gate.discardPending()

        XCTAssertNil(gate.complete())
        XCTAssertFalse(gate.isApplying)
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
                isApplicationActive: true,
                allowsReadReceipts: true,
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
                isApplicationActive: true,
                allowsReadReceipts: true,
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
                isApplicationActive: true,
                allowsReadReceipts: true,
                isLive: true,
                isConfirmedPinned: true,
                isJumpingToLatest: false,
                isUserInteracting: true,
                eventID: "$new-tail",
                lastMarkedEventID: "$old-tail"
            )
        )
        XCTAssertFalse(
            RoomTimelineReadAcknowledgementPolicy.shouldSchedule(
                isApplicationActive: false,
                allowsReadReceipts: true,
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
                isApplicationActive: true,
                allowsReadReceipts: false,
                isLive: true,
                isConfirmedPinned: true,
                isJumpingToLatest: false,
                isUserInteracting: false,
                eventID: "$new-tail",
                lastMarkedEventID: "$old-tail"
            )
        )
    }

    func testReadMarkerDebounceReplacesVisibleAndReceiptIdentityTogether() throws {
        let firstQueuedAt = Date(timeIntervalSince1970: 100)
        let first = RoomTimelineReadObservation(
            visibleEventID: "$message-a", receiptEventID: "$edit-a"
        )
        let newer = RoomTimelineReadObservation(
            visibleEventID: "$message-b", receiptEventID: "$edit-b"
        )
        var queue = RoomTimelineReadMarkerQueue()
        queue.enqueue(first, now: firstQueuedAt)
        // B replaces A while the original delayed task is still installed.
        queue.enqueue(newer, now: firstQueuedAt.addingTimeInterval(0.5))
        XCTAssertEqual(queue.firstQueuedAt, firstQueuedAt)
        let write = try XCTUnwrap(queue.dequeue())
        XCTAssertEqual(write.receiptEventID, "$edit-b")
        XCTAssertEqual(write.visibleEventID, "$message-b")
        XCTAssertNil(queue.pending)
        XCTAssertNil(queue.firstQueuedAt)
        // A later observation cannot change the pair retained by B's write.
        queue.enqueue(first, now: firstQueuedAt.addingTimeInterval(1))
        XCTAssertEqual(write, newer)
        XCTAssertEqual(queue.pending, first)
        queue.clear()
        XCTAssertNil(queue.pending)
        XCTAssertNil(queue.firstQueuedAt)
    }

    func testReadMarkerQueueHasMaximumLatencyAndCancelsSupersededTask() {
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
        XCTAssertTrue(RoomTimelineReadMarkerTaskPolicy.ownsInstalledTask(
            installedGeneration: 8,
            currentGeneration: 8
        ))
        XCTAssertFalse(RoomTimelineReadMarkerTaskPolicy.ownsInstalledTask(
            installedGeneration: 8,
            currentGeneration: 9
        ))
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

    func testNonAnimatedLatestRetriesUntilConfirmedPinnedAndFailureShowsRecovery() {
        XCTAssertFalse(StableTimelineViewportPolicy.commandSucceeded(
            targetsLatest: true,
            isTargetVisible: true,
            isConfirmedPinned: false
        ))
        XCTAssertTrue(StableTimelineViewportPolicy.commandSucceeded(
            targetsLatest: true,
            isTargetVisible: true,
            isConfirmedPinned: true
        ))
        XCTAssertTrue(StableTimelineViewportPolicy.shouldScheduleCommandRetry(
            firedRetryCount: 2,
            hasScheduledRetry: false
        ))
        XCTAssertFalse(StableTimelineViewportPolicy.shouldScheduleCommandRetry(
            firedRetryCount: 3,
            hasScheduledRetry: false
        ))
        XCTAssertTrue(RoomTimelineLatestCommandCompletionPolicy.shouldShowRecovery(success: false))
        XCTAssertFalse(RoomTimelineLatestCommandCompletionPolicy.shouldShowRecovery(success: true))
    }

    func testOpportunisticSnapshotEntriesDoNotConsumeOrPostponeTimedRetryBudget() {
        var firedRetryCount = 0
        var hasScheduledRetry = false

        for _ in 0 ..< 20 {
            if StableTimelineViewportPolicy.shouldScheduleCommandRetry(
                firedRetryCount: firedRetryCount,
                hasScheduledRetry: hasScheduledRetry
            ) {
                hasScheduledRetry = true
            }
        }
        XCTAssertEqual(firedRetryCount, 0)
        XCTAssertTrue(hasScheduledRetry)

        for expectedFiredCount in 1 ... StableTimelineViewportPolicy.maximumMissingTargetAttempts {
            hasScheduledRetry = false
            firedRetryCount = StableTimelineViewportPolicy.nextFiredCommandRetryCount(firedRetryCount)
            XCTAssertEqual(firedRetryCount, expectedFiredCount)
            if StableTimelineViewportPolicy.shouldScheduleCommandRetry(
                firedRetryCount: firedRetryCount,
                hasScheduledRetry: hasScheduledRetry
            ) {
                hasScheduledRetry = true
            }
        }
        XCTAssertEqual(firedRetryCount, StableTimelineViewportPolicy.maximumMissingTargetAttempts)
        XCTAssertFalse(hasScheduledRetry)

        var successPathFiredCount = 0
        var successPathHasScheduledRetry = StableTimelineViewportPolicy.shouldScheduleCommandRetry(
            firedRetryCount: successPathFiredCount,
            hasScheduledRetry: false
        )
        XCTAssertTrue(successPathHasScheduledRetry)
        successPathHasScheduledRetry = false
        successPathFiredCount = StableTimelineViewportPolicy.nextFiredCommandRetryCount(successPathFiredCount)
        XCTAssertTrue(StableTimelineViewportPolicy.commandSucceeded(
            targetsLatest: true,
            isTargetVisible: true,
            isConfirmedPinned: true
        ))
        XCTAssertEqual(successPathFiredCount, 1)
        XCTAssertFalse(successPathHasScheduledRetry)
        let supersededCommandID: UInt64? = 10
        let replacementCommandID: UInt64 = 11
        XCTAssertTrue(StableTimelineViewportPolicy.shouldReplaceScheduledCommandRetry(
            scheduledCommandID: supersededCommandID,
            currentCommandID: replacementCommandID
        ))
        let replacementAlreadyHasScheduledRetry = supersededCommandID == replacementCommandID
        XCTAssertFalse(replacementAlreadyHasScheduledRetry)
        XCTAssertTrue(StableTimelineViewportPolicy.shouldScheduleCommandRetry(
            firedRetryCount: 0,
            hasScheduledRetry: replacementAlreadyHasScheduledRetry
        ))
        XCTAssertFalse(StableTimelineViewportPolicy.shouldReplaceScheduledCommandRetry(
            scheduledCommandID: 11,
            currentCommandID: 11
        ))
    }

    func testUserDragAbandonsOnlyCurrentNonAnimatedCommandRetry() {
        XCTAssertTrue(StableTimelineViewportPolicy.shouldAbandonPendingNonAnimatedCommandRetry(
            hasScheduledRetry: true,
            scheduledCommandID: 41,
            currentCommandID: 41,
            currentCommandIsAnimated: false
        ))
        XCTAssertFalse(StableTimelineViewportPolicy.shouldAbandonPendingNonAnimatedCommandRetry(
            hasScheduledRetry: false,
            scheduledCommandID: nil,
            currentCommandID: 41,
            currentCommandIsAnimated: false
        ))
        XCTAssertFalse(StableTimelineViewportPolicy.shouldAbandonPendingNonAnimatedCommandRetry(
            hasScheduledRetry: true,
            scheduledCommandID: 40,
            currentCommandID: 41,
            currentCommandIsAnimated: false
        ))
        XCTAssertFalse(StableTimelineViewportPolicy.shouldAbandonPendingNonAnimatedCommandRetry(
            hasScheduledRetry: true,
            scheduledCommandID: 41,
            currentCommandID: 41,
            currentCommandIsAnimated: true
        ))
    }

    func testAbandonedRetryIdentityCannotExecuteOrRescroll() {
        let abandonedRetryID = UUID()

        XCTAssertTrue(StableTimelineViewportPolicy.commandRetryMayExecute(
            retryID: abandonedRetryID,
            installedRetryID: abandonedRetryID
        ))
        XCTAssertFalse(StableTimelineViewportPolicy.commandRetryMayExecute(
            retryID: abandonedRetryID,
            installedRetryID: nil
        ))
        XCTAssertFalse(StableTimelineViewportPolicy.commandRetryMayExecute(
            retryID: abandonedRetryID,
            installedRetryID: UUID()
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
