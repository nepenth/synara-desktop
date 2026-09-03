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
        XCTAssertFalse(availability.canReport)
        XCTAssertFalse(availability.canForward)
        XCTAssertFalse(availability.canVote)
        XCTAssertFalse(availability.canDeclineCall)
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
        XCTAssertFalse(availability.canReport)
        XCTAssertFalse(availability.canForward)
        XCTAssertFalse(availability.canVote)
        XCTAssertFalse(availability.canDeclineCall)
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
        XCTAssertFalse(availability.canReport)
        XCTAssertFalse(availability.canForward)
        XCTAssertFalse(availability.canVote)
        XCTAssertFalse(availability.canDeclineCall)
    }

    func testCoreCapabilitiesOverridePresenterInference() {
        let service = MockEventActionService()
        let item = TimelineItem(
            id: "$event:matrix.org",
            eventID: "$event:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Hello"),
            replyToEventID: nil,
            actionCapabilities: TimelineRowActionCapabilities(
                canReact: false,
                canReply: false,
                canEdit: false,
                canRedact: false,
                canReport: false,
                canPin: false,
                canForward: false,
                canVote: false,
                canDeclineCall: false
            ),
            isEdited: false,
            reactions: [:]
        )

        let availability = service.availability(for: item, currentUserID: "@alice:matrix.org")

        XCTAssertFalse(availability.canReply)
        XCTAssertFalse(availability.canEdit)
        XCTAssertFalse(availability.canRedact)
        XCTAssertFalse(availability.canReact)
        XCTAssertFalse(availability.canReport)
        XCTAssertFalse(availability.canForward)
        XCTAssertFalse(availability.canVote)
        XCTAssertFalse(availability.canDeclineCall)
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

    func testCoreCapabilitiesDriveExtendedActionsWithoutPresenterInference() {
        let service = MockEventActionService()
        let item = TimelineItem(
            id: "$event:matrix.org",
            eventID: "$event:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Hello"),
            replyToEventID: nil,
            actionCapabilities: TimelineRowActionCapabilities(
                canReact: false,
                canReply: false,
                canEdit: false,
                canRedact: false,
                canReport: true,
                canPin: false,
                canForward: true,
                canVote: true,
                canDeclineCall: true
            ),
            forwardTransport: .text,
            isEdited: false,
            reactions: [:]
        )

        let availability = service.availability(for: item, currentUserID: "@bob:matrix.org")

        XCTAssertTrue(availability.canReport)
        XCTAssertTrue(availability.canForward)
        XCTAssertTrue(availability.canVote)
        XCTAssertTrue(availability.canDeclineCall)
    }

    func testForwardCapabilityFailsClosedWithoutCoreTransport() {
        let service = MockEventActionService()
        let item = TimelineItem(
            id: "$event:matrix.org",
            eventID: "$event:matrix.org",
            senderID: "@alice:matrix.org",
            timestamp: TimelineFixtures.baseDate,
            kind: .text("Hello"),
            replyToEventID: nil,
            actionCapabilities: TimelineRowActionCapabilities(
                canReact: false,
                canReply: false,
                canEdit: false,
                canRedact: false,
                canReport: false,
                canPin: false,
                canForward: true,
                canVote: false,
                canDeclineCall: false
            ),
            forwardTransport: .unavailable,
            isEdited: false,
            reactions: [:]
        )

        XCTAssertFalse(
            service.availability(for: item, currentUserID: "@bob:matrix.org").canForward
        )
    }

    func testInFlightKeysSerializeChangedPayloadsPerActionClass() {
        XCTAssertEqual(
            EventActionType.forward(
                targetRoomID: "!one:example.org",
                asQuote: false,
                confirmedEncryptionDowngrade: false
            ).inFlightKey,
            EventActionType.forward(
                targetRoomID: "!two:example.org",
                asQuote: true,
                confirmedEncryptionDowngrade: true
            ).inFlightKey
        )
        XCTAssertEqual(
            EventActionType.pollVote(answerIDs: ["a"]).inFlightKey,
            EventActionType.pollVote(answerIDs: ["b", "c"]).inFlightKey
        )
    }

    func testPollSelectionHonorsCoreMaximumAndSupportsClearingVote() {
        let available: Set<String> = ["a", "b", "c"]
        XCTAssertEqual(
            TimelinePollSelectionPolicy.toggledSelection(
                current: ["a", "b"],
                answerID: "c",
                availableAnswerIDs: available,
                maximumSelections: 2
            ),
            ["a", "b"]
        )
        XCTAssertEqual(
            TimelinePollSelectionPolicy.toggledSelection(
                current: ["a"],
                answerID: "a",
                availableAnswerIDs: available,
                maximumSelections: 1
            ),
            []
        )
        XCTAssertEqual(
            TimelinePollSelectionPolicy.submission(
                selection: [],
                original: ["a"],
                availableAnswerIDs: available,
                maximumSelections: 1,
                canVote: true,
                isClosed: false
            ),
            []
        )
    }

    func testPollSelectionRejectsUnchangedClosedOrCapabilityDeniedSubmission() {
        let available: Set<String> = ["a", "b"]
        XCTAssertNil(
            TimelinePollSelectionPolicy.submission(
                selection: ["a"],
                original: ["a"],
                availableAnswerIDs: available,
                maximumSelections: 1,
                canVote: true,
                isClosed: false
            )
        )
        XCTAssertNil(
            TimelinePollSelectionPolicy.submission(
                selection: ["b"],
                original: ["a"],
                availableAnswerIDs: available,
                maximumSelections: 1,
                canVote: false,
                isClosed: false
            )
        )
        XCTAssertNil(
            TimelinePollSelectionPolicy.submission(
                selection: ["b"],
                original: ["a"],
                availableAnswerIDs: available,
                maximumSelections: 1,
                canVote: true,
                isClosed: true
            )
        )
    }

    func testTimelineActionReadbackMustMatchExactOwnerContract() {
        XCTAssertTrue(
            TimelineActionReadbackPolicy.accepts(
                schemaVersion: 1,
                action: "report",
                roomID: "!room:matrix.org",
                eventID: "$event:matrix.org",
                status: "reported",
                expectedAction: "report",
                expectedRoomID: "!room:matrix.org",
                expectedStatus: "reported",
                expectedEventID: "$event:matrix.org"
            )
        )
        XCTAssertFalse(
            TimelineActionReadbackPolicy.accepts(
                schemaVersion: 1,
                action: "report",
                roomID: "!other:matrix.org",
                eventID: "$event:matrix.org",
                status: "reported",
                expectedAction: "report",
                expectedRoomID: "!room:matrix.org",
                expectedStatus: "reported",
                expectedEventID: "$event:matrix.org"
            )
        )
        XCTAssertFalse(
            TimelineActionReadbackPolicy.accepts(
                schemaVersion: 2,
                action: "report",
                roomID: "!room:matrix.org",
                eventID: "$event:matrix.org",
                status: "reported",
                expectedAction: "report",
                expectedRoomID: "!room:matrix.org",
                expectedStatus: "reported",
                expectedEventID: "$event:matrix.org"
            )
        )
    }

    func testIOSActionPresentationIsCapabilityGatedAccessibleAndCoreRouted() throws {
        let repositoryRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let source = try String(
            contentsOf: repositoryRoot
                .appendingPathComponent("synara-ios/Synara/Features/RoomTimelineView.swift"),
            encoding: .utf8
        )

        XCTAssertTrue(source.contains("if availability.canForward"))
        XCTAssertTrue(source.contains("if availability.canReport"))
        XCTAssertTrue(source.contains("if availability.canDeclineCall"))
        XCTAssertTrue(source.contains("canVote: availability.canVote"))
        XCTAssertTrue(source.contains("TimelineItemForward-\\(item.eventID)"))
        XCTAssertTrue(source.contains("TimelineItemReport-\\(item.eventID)"))
        XCTAssertTrue(source.contains("TimelineItemDeclineCall-\\(item.eventID)"))
        XCTAssertTrue(source.contains("TimelinePollAnswer-\\(answer.id)"))
        XCTAssertTrue(source.contains(".accessibilityAddTraits(selectedAnswerIDs.contains(answer.id) ? .isSelected : [])"))
        XCTAssertTrue(source.contains("guard inFlightTimelineActionKeys.insert(inFlightKey).inserted"))
        XCTAssertFalse(source.contains("MatrixRustSDK"))
    }

    func testSessionCoordinatorExcludesDuplicateActionAcrossViewRecreation() async {
        let coordinator = TimelineActionInFlightCoordinator()
        let firstAction = EventActionType.forward(
            targetRoomID: "!one:matrix.org",
            asQuote: false,
            confirmedEncryptionDowngrade: false
        )
        let changedPayload = EventActionType.forward(
            targetRoomID: "!two:matrix.org",
            asQuote: true,
            confirmedEncryptionDowngrade: true
        )
        let firstKey = "!source:matrix.org\u{0}$event:matrix.org\u{0}\(firstAction.inFlightKey)"
        let recreatedViewKey = "!source:matrix.org\u{0}$event:matrix.org\u{0}\(changedPayload.inFlightKey)"

        let firstClaim = coordinator.begin(firstKey)
        let duplicateClaim = coordinator.begin(recreatedViewKey)
        XCTAssertTrue(firstClaim)
        XCTAssertFalse(duplicateClaim)
        coordinator.end(firstKey)
        let reclaimed = coordinator.begin(recreatedViewKey)
        XCTAssertTrue(reclaimed)
    }

    func testSessionCoordinatorRetainsPollUntilDispatchThenExactCoreProjection() {
        let coordinator = TimelineActionInFlightCoordinator()
        let key = "!room:matrix.org\u{0}$poll:matrix.org\u{0}poll-vote"
        XCTAssertTrue(coordinator.beginPoll(key, answerIDs: ["b", "a"]))

        coordinator.observePollProjection(key, ownAnswerIDs: ["old"])
        XCTAssertTrue(coordinator.contains(key))
        coordinator.settlePollDispatch(key)
        XCTAssertTrue(coordinator.contains(key))
        coordinator.observePollProjection(key, ownAnswerIDs: ["a", "b"])
        XCTAssertFalse(coordinator.contains(key))
        XCTAssertTrue(coordinator.begin(key))
    }

    func testSessionCoordinatorRetainsPollWhenExactProjectionArrivesBeforeDispatchSettles() {
        let coordinator = TimelineActionInFlightCoordinator()
        let key = "!room:matrix.org\u{0}$poll:matrix.org\u{0}poll-vote"
        XCTAssertTrue(coordinator.beginPoll(key, answerIDs: ["a"]))

        coordinator.observePollProjection(key, ownAnswerIDs: ["a"])
        XCTAssertTrue(coordinator.contains(key))
        coordinator.settlePollDispatch(key)
        XCTAssertFalse(coordinator.contains(key))
    }

    func testSessionCoordinatorPollFailureClearsBothPhases() {
        let coordinator = TimelineActionInFlightCoordinator()
        let key = "!room:matrix.org\u{0}$poll:matrix.org\u{0}poll-vote"
        XCTAssertTrue(coordinator.beginPoll(key, answerIDs: ["a"]))
        coordinator.observePollProjection(key, ownAnswerIDs: ["a"])
        coordinator.end(key)

        XCTAssertFalse(coordinator.contains(key))
        XCTAssertTrue(coordinator.beginPoll(key, answerIDs: ["b"]))
    }

    func testSessionCoordinatorClearsPersistentLocksOnSessionTransition() {
        let coordinator = TimelineActionInFlightCoordinator()
        let key = "!room:matrix.org\u{0}$poll:matrix.org\u{0}poll-vote"
        coordinator.bindSession(epoch: 1)
        XCTAssertTrue(coordinator.beginPoll(key, answerIDs: ["a"]))
        XCTAssertTrue(coordinator.contains(key))

        coordinator.bindSession(epoch: 2)
        XCTAssertFalse(coordinator.contains(key))
        XCTAssertTrue(coordinator.beginPoll(key, answerIDs: ["b"]))
    }

    func testReactionReadbackRequiresExactMutationAndProjectedOwnership() {
        XCTAssertTrue(
            TimelineReactionReadbackPolicy.acceptsToggle(
                roomID: "!room:matrix.org",
                targetEventID: "$event:matrix.org",
                key: "👍",
                mutation: "added",
                readbackKey: "👍",
                readbackOwnsReaction: true,
                expectedRoomID: "!room:matrix.org",
                expectedTargetEventID: "$event:matrix.org",
                expectedKey: "👍",
                expectedOwn: true
            )
        )
        XCTAssertTrue(
            TimelineReactionReadbackPolicy.acceptsToggle(
                roomID: "!room:matrix.org",
                targetEventID: "$event:matrix.org",
                key: "👍",
                mutation: "removed",
                readbackKey: nil,
                readbackOwnsReaction: nil,
                expectedRoomID: "!room:matrix.org",
                expectedTargetEventID: "$event:matrix.org",
                expectedKey: "👍",
                expectedOwn: false
            )
        )
        XCTAssertFalse(
            TimelineReactionReadbackPolicy.acceptsToggle(
                roomID: "!room:matrix.org",
                targetEventID: "$event:matrix.org",
                key: "👍",
                mutation: "added",
                readbackKey: "👎",
                readbackOwnsReaction: true,
                expectedRoomID: "!room:matrix.org",
                expectedTargetEventID: "$event:matrix.org",
                expectedKey: "👍",
                expectedOwn: true
            )
        )
        XCTAssertFalse(
            TimelineReactionReadbackPolicy.acceptsToggle(
                roomID: "!room:matrix.org",
                targetEventID: "$event:matrix.org",
                key: "👍",
                mutation: "removed",
                readbackKey: "👍",
                readbackOwnsReaction: true,
                expectedRoomID: "!room:matrix.org",
                expectedTargetEventID: "$event:matrix.org",
                expectedKey: "👍",
                expectedOwn: false
            )
        )
        XCTAssertFalse(
            TimelineReactionReadbackPolicy.acceptsToggle(
                roomID: "!room:matrix.org",
                targetEventID: "$event:matrix.org",
                key: "👍",
                mutation: "redacted",
                readbackKey: nil,
                readbackOwnsReaction: nil,
                expectedRoomID: "!room:matrix.org",
                expectedTargetEventID: "$event:matrix.org",
                expectedKey: "👍",
                expectedOwn: false
            )
        )
    }

    func testReactionReadbackAcceptsCommittedAddWithoutImmediateProjection() {
        XCTAssertTrue(
            TimelineReactionReadbackPolicy.acceptsToggle(
                roomID: "!room:matrix.org",
                targetEventID: "$event:matrix.org",
                key: "👍",
                mutation: "added",
                readbackKey: nil,
                readbackOwnsReaction: nil,
                expectedRoomID: "!room:matrix.org",
                expectedTargetEventID: "$event:matrix.org",
                expectedKey: "👍",
                expectedOwn: true
            )
        )
    }

    func testSessionCoordinatorRetainsReactionUntilDispatchAndProjectionComplete() {
        let coordinator = TimelineActionInFlightCoordinator()
        let prefix = "1\u{0}!room:matrix.org\u{0}$event:matrix.org\u{0}react:"
        let key = "\(prefix)👍"
        coordinator.bindSession(epoch: 1)
        XCTAssertTrue(coordinator.beginReaction(key, reactionKey: "👍", expectedOwn: true))

        coordinator.settleReactionDispatch(key)
        XCTAssertTrue(coordinator.contains(key))
        coordinator.observeReactionProjection(prefix, ownership: .known([]))
        XCTAssertTrue(coordinator.contains(key))
        coordinator.observeReactionProjection(prefix, ownership: .known(["👍"]))
        XCTAssertFalse(coordinator.contains(key))

        XCTAssertTrue(coordinator.beginReaction(key, reactionKey: "👍", expectedOwn: false))
        coordinator.observeReactionProjection(prefix, ownership: .known([]))
        XCTAssertTrue(coordinator.contains(key))
        coordinator.settleReactionDispatch(key)
        XCTAssertFalse(coordinator.contains(key))
    }

    func testLateOldSessionCompletionCannotReleaseNewSessionClaim() {
        let coordinator = TimelineActionInFlightCoordinator()
        let oldKey = "1\u{0}!room:matrix.org\u{0}$event:matrix.org\u{0}react:👍"
        let newKey = "2\u{0}!room:matrix.org\u{0}$event:matrix.org\u{0}react:👍"
        coordinator.bindSession(epoch: 1)
        XCTAssertTrue(coordinator.beginReaction(oldKey, reactionKey: "👍", expectedOwn: true))
        coordinator.bindSession(epoch: 2)
        XCTAssertTrue(coordinator.beginReaction(newKey, reactionKey: "👍", expectedOwn: true))

        coordinator.settleReactionDispatch(oldKey)
        coordinator.end(oldKey)
        XCTAssertTrue(coordinator.contains(newKey))
    }

    func testForwardSecurityRequiresAuthoritativeSourceStateAndConfirmsDowngrade() {
        XCTAssertEqual(
            TimelineForwardSecurityPolicy.decision(
                sourceEncryption: .unknown,
                targetEncryption: .notEncrypted
            ),
            .unavailable
        )
        XCTAssertEqual(
            TimelineForwardSecurityPolicy.decision(
                sourceEncryption: .encrypted,
                targetEncryption: .notEncrypted
            ),
            .confirmDowngrade
        )
        XCTAssertEqual(
            TimelineForwardSecurityPolicy.decision(
                sourceEncryption: .encrypted,
                targetEncryption: .encrypted
            ),
            .proceed
        )
        XCTAssertEqual(
            TimelineForwardSecurityPolicy.decision(
                sourceEncryption: .notEncrypted,
                targetEncryption: .notEncrypted
            ),
            .proceed
        )
        XCTAssertEqual(
            TimelineForwardSecurityPolicy.decision(
                sourceEncryption: .notEncrypted,
                targetEncryption: .unknown
            ),
            .unavailable
        )
        XCTAssertEqual(
            TimelineForwardSecurityPolicy.decision(
                sourceEncryption: .encrypted,
                targetEncryption: .unavailable
            ),
            .unavailable
        )
        XCTAssertEqual(
            TimelineForwardSecurityPolicy.decision(
                sourceEncryption: .unavailable,
                targetEncryption: .encrypted
            ),
            .unavailable
        )
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
