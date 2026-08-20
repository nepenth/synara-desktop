import SynaraCore
@testable import Synara
import XCTest

final class RoomListServiceTests: XCTestCase {
    func testRoomsSortByHighlightUnreadThenActivity() {
        let rooms = RoomListFixtures.small().reversed()

        let sorted = RoomListFixtures.sorted(Array(rooms))

        XCTAssertEqual(sorted.first?.id, "!project:matrix.org")
    }

    func testLargeFixtureHasStableIdentifiers() {
        let rooms = RoomListFixtures.large()

        XCTAssertEqual(rooms.count, 1000)
        XCTAssertEqual(Set(rooms.map(\.id)).count, 1000)
    }

    func testMockRoomListReturnsSortedLoadedState() async {
        let rooms = RoomListFixtures.small().reversed()
        let service = MockRoomListService(state: .loaded(Array(rooms)))

        let state = await service.loadRooms()

        XCTAssertEqual(state, .loaded(RoomListFixtures.sorted(RoomListFixtures.small())))
        XCTAssertEqual(service.loadCallCount, 1)
    }

    func testRoomDisplayNameLookupResolvesCachedNames() async {
        let service = MockRoomListService(state: .loaded(RoomListFixtures.small()))
        let state = await service.loadRooms()

        XCTAssertEqual(service.roomDisplayName(roomID: "!project:matrix.org"), "Product")
        XCTAssertEqual(
            RoomDisplayNameLookup.resolve(
                roomID: "!project:matrix.org",
                names: RoomDisplayNameLookup.names(from: state)
            ),
            "Product"
        )
        XCTAssertEqual(
            RoomDisplayNameLookup.resolve(roomID: "!missing:matrix.org", names: [:]),
            "!missing:matrix.org"
        )
    }

    func testClearCacheReturnsEmptyState() async {
        let service = MockRoomListService()

        service.clearCache()
        let state = await service.loadRooms()

        XCTAssertEqual(state, .empty)
        XCTAssertEqual(service.clearCallCount, 1)
    }

    func testMockInviteTransitionAcceptsInviteIntoJoinedRoom() async throws {
        let service = MockInviteTransitionService()

        try await service.acceptInvite(roomID: "!alerts:matrix.org")
        let state = await service.loadRooms()

        guard case let .loaded(rooms) = state else {
            XCTFail("Expected loaded rooms")
            return
        }

        XCTAssertEqual(rooms.first?.id, "!alerts:matrix.org")
        XCTAssertEqual(rooms.first?.membership, .joined)
        XCTAssertEqual(rooms.first?.lastMessagePreview, "Joined room")
    }

    func testMockInviteTransitionRejectsInviteIntoEmptyState() async throws {
        let service = MockInviteTransitionService()

        try await service.rejectInvite(roomID: "!alerts:matrix.org")
        let state = await service.loadRooms()

        XCTAssertEqual(state, .empty)
    }

    func testDefaultRoomUpdatesStreamYieldsInitialSnapshot() async {
        let service = MockInviteTransitionService()
        var updates = service.roomUpdates().makeAsyncIterator()

        let state = await updates.next()

        guard case let .loaded(rooms) = state else {
            XCTFail("Expected the default update stream to emit the initial room snapshot")
            return
        }
        XCTAssertEqual(rooms.map(\.id), ["!alerts:matrix.org"])
    }

    func testSearchFilterExcludesNonMatchingInvites() {
        let invite = RoomSummary(
            id: "!alerts:matrix.org",
            name: "Alerts",
            lastMessagePreview: "You are invited",
            unreadCount: 1,
            hasHighlight: false,
            kind: .room,
            membership: .invited,
            lastActivityAt: RoomListFixtures.now
        )
        let joined = RoomSummary(
            id: "!alice:matrix.org",
            name: "Alice",
            lastMessagePreview: "Hello from Alice",
            unreadCount: 0,
            hasHighlight: false,
            kind: .directMessage,
            membership: .joined,
            lastActivityAt: RoomListFixtures.now
        )
        let rooms = [invite, joined]

        let filtered = RoomListSearchFilter.applySearchQuery("Alice", to: rooms, invitedRooms: [invite])

        XCTAssertEqual(filtered.map(\.id), ["!alice:matrix.org"])
    }

    func testSearchFilterIncludesMatchingInvites() {
        let invite = RoomSummary(
            id: "!alerts:matrix.org",
            name: "Alerts",
            lastMessagePreview: "You are invited",
            unreadCount: 1,
            hasHighlight: false,
            kind: .room,
            membership: .invited,
            lastActivityAt: RoomListFixtures.now
        )
        let joined = RoomSummary(
            id: "!project:matrix.org",
            name: "Project",
            lastMessagePreview: "Latest update",
            unreadCount: 2,
            hasHighlight: false,
            kind: .room,
            membership: .joined,
            lastActivityAt: RoomListFixtures.now
        )
        let rooms = [invite, joined]

        let filtered = RoomListSearchFilter.applySearchQuery("Alerts", to: rooms, invitedRooms: [invite])

        XCTAssertEqual(filtered.map(\.id), ["!alerts:matrix.org"])
    }

    func testScopeFilterUnreadKeepsInvitesAndUnreadRooms() {
        let invite = RoomSummary(
            id: "!alerts:matrix.org",
            name: "Alerts",
            lastMessagePreview: "You are invited",
            unreadCount: 1,
            hasHighlight: false,
            kind: .room,
            membership: .invited,
            lastActivityAt: RoomListFixtures.now
        )
        let unread = RoomSummary(
            id: "!project:matrix.org",
            name: "Product",
            lastMessagePreview: "Mina: Here's the latest spec",
            unreadCount: 3,
            hasHighlight: true,
            kind: .room,
            membership: .joined,
            lastActivityAt: RoomListFixtures.now
        )
        let read = RoomSummary(
            id: "!general:matrix.org",
            name: "General",
            lastMessagePreview: "Kai: Project update looks good",
            unreadCount: 0,
            hasHighlight: false,
            kind: .room,
            membership: .joined,
            lastActivityAt: RoomListFixtures.now
        )

        let filtered = RoomListScopeFilter.filteredRooms(
            from: [invite, unread, read],
            filter: .unread,
            selectedSpaceID: nil,
            searchQuery: ""
        )

        XCTAssertEqual(Set(filtered.map(\.id)), Set(["!alerts:matrix.org", "!project:matrix.org"]))
    }

    func testScopeFilterMentionsExcludesNonHighlightedRooms() {
        let rooms = RoomListFixtures.small()

        let filtered = RoomListScopeFilter.filteredRooms(
            from: rooms,
            filter: .mentions,
            selectedSpaceID: nil,
            searchQuery: ""
        )

        XCTAssertTrue(filtered.allSatisfy(\.hasHighlight))
        XCTAssertEqual(filtered.map(\.id), ["!project:matrix.org"])
    }

    func testScopeFilterRespectsSelectedSpace() {
        let rooms = RoomListFixtures.small()

        let filtered = RoomListScopeFilter.filteredRooms(
            from: rooms,
            filter: .all,
            selectedSpaceID: "!workspace:matrix.org",
            searchQuery: ""
        )

        XCTAssertEqual(
            filtered.map(\.id),
            ["!general:matrix.org", "!project:matrix.org", "!design:matrix.org"]
        )
    }

    func testScopeFilterAgentsIncludesOnlyAgentRooms() {
        let rooms = RoomListFixtures.small()

        let filtered = RoomListScopeFilter.filteredRooms(
            from: rooms,
            filter: .agents,
            selectedSpaceID: nil,
            searchQuery: ""
        )

        XCTAssertTrue(filtered.allSatisfy(\.isAgentRoom))
        XCTAssertEqual(
            filtered.map(\.id),
            ["!agent-workflows:matrix.org", "!security-agent:matrix.org"]
        )
    }

    func testAgentRoomStaysClassifiedWithLatestAgentCardWithoutRecentActivityFlag() {
        let room = RoomSummary(
            id: "!agent:matrix.org",
            name: "Agent Workflows",
            lastMessagePreview: "Latest message",
            unreadCount: 0,
            hasHighlight: false,
            kind: .room,
            membership: .joined,
            lastActivityAt: RoomListFixtures.now,
            hasAgentActivity: false,
            latestAgentCard: RoomListFixtures.pendingAgentApprovalCard()
        )

        XCTAssertTrue(room.isAgentRoom)
    }

    func testAgentRoomRequiresRecentAgentActivity() {
        let agentRoom = RoomSummary(
            id: "!agent:matrix.org",
            name: "Agent Workflows",
            lastMessagePreview: "Approval required",
            unreadCount: 1,
            hasHighlight: false,
            kind: .room,
            membership: .joined,
            lastActivityAt: RoomListFixtures.now,
            hasAgentActivity: true
        )
        let generalRoom = RoomSummary(
            id: "!general:matrix.org",
            name: "General",
            lastMessagePreview: "Hello",
            unreadCount: 0,
            hasHighlight: false,
            kind: .room,
            membership: .joined,
            lastActivityAt: RoomListFixtures.now,
            hasAgentActivity: false
        )

        XCTAssertTrue(agentRoom.isAgentRoom)
        XCTAssertFalse(generalRoom.isAgentRoom)
    }

    func testNotificationsInboxCaughtUpWhenNoSectionsOrPendingAgents() {
        let sections = NotificationsInboxSections(
            mentions: [],
            invites: [],
            unreadRooms: []
        )

        XCTAssertTrue(NotificationsInboxSections.isCaughtUp(sections: sections, agentPendingCount: 0))
        XCTAssertFalse(NotificationsInboxSections.isCaughtUp(sections: sections, agentPendingCount: 2))
    }

    func testSpaceUnreadCountsSumUnreadRoomsPerSpace() {
        let rooms = RoomListFixtures.small()

        let counts = RoomListSpaceGrouping.unreadCountsBySpaceID(from: rooms)

        XCTAssertEqual(counts["!workspace:matrix.org"], 10)
        XCTAssertEqual(counts["!ops:matrix.org"], 4)
    }

    func testSpaceChannelGroupsUsePrimaryParentSpace() {
        let rooms = RoomListFixtures.small().filter { $0.kind == .room }

        let groups = RoomListSpaceGrouping.spaceChannelGroups(from: rooms)

        XCTAssertEqual(groups.map(\.space.name), ["Ops", "Workspace"])
        XCTAssertEqual(
            groups.first(where: { $0.space.id == "!workspace:matrix.org" })?.rooms.map(\.id),
            ["!general:matrix.org", "!project:matrix.org", "!design:matrix.org"]
        )
        XCTAssertEqual(
            groups.first(where: { $0.space.id == "!ops:matrix.org" })?.rooms.map(\.id),
            ["!security:matrix.org", "!agent-workflows:matrix.org", "!security-agent:matrix.org"]
        )
    }

    func testUngroupedChannelRoomsExcludeParentedChannels() {
        let rooms = RoomListFixtures.small().filter { $0.kind == .room }

        let ungrouped = RoomListSpaceGrouping.ungroupedChannelRooms(from: rooms)

        XCTAssertTrue(ungrouped.isEmpty)
    }

    func testRoomSummarySpaceConveniencesExposePrimaryParent() {
        let room = RoomListFixtures.small().first { $0.id == "!general:matrix.org" }

        XCTAssertEqual(room?.parentSpaceID, "!workspace:matrix.org")
        XCTAssertEqual(room?.spaceName, "Workspace")
    }

    func testSelectedSpaceNameResolvesFromSpaceSummaries() {
        let spaces = Array(Set(RoomListFixtures.small().flatMap(\.parentSpaces)))

        XCTAssertEqual(
            RoomListSpaceGrouping.selectedSpaceName(in: spaces, selectedSpaceID: "!ops:matrix.org"),
            "Ops"
        )
        XCTAssertNil(RoomListSpaceGrouping.selectedSpaceName(in: spaces, selectedSpaceID: nil))
    }

    func testNotificationBadgeSummaryMatchesSharedContractFixture() {
        let summary = NotificationBadgeSummary.summarizeNotifications(
            NotificationSummaryInput(
                unreadCounts: [
                    BadgeUnreadSource(total: 4, highlight: 2),
                    BadgeUnreadSource(total: 3),
                ],
                laterActiveCount: 5,
                inviteCount: 2,
                agentApprovalCount: 1
            )
        )

        XCTAssertEqual(summary?.appBadgeCount, 10)
        XCTAssertEqual(summary?.inboxBadgeCount, 8)
        XCTAssertEqual(summary?.laterActiveCount, 5)
        XCTAssertEqual(summary?.inviteCount, 2)
        XCTAssertEqual(summary?.agentApprovalCount, 1)
        XCTAssertEqual(summary?.highlightCount, 2)
        XCTAssertEqual(summary?.unreadCount, 3)
    }

    func testNotificationBadgeSummaryClampsInvalidCounts() {
        let summary = NotificationBadgeSummary.summarizeNotifications(
            NotificationSummaryInput(
                unreadCounts: [
                    BadgeUnreadSource(total: -1, highlight: -2),
                    BadgeUnreadSource(total: 3),
                ],
                laterActiveCount: 2,
                inviteCount: -1,
                agentApprovalCount: -4
            )
        )

        XCTAssertEqual(summary?.appBadgeCount, 5)
        XCTAssertEqual(summary?.inboxBadgeCount, 2)
        XCTAssertEqual(summary?.highlightCount, 0)
        XCTAssertEqual(summary?.unreadCount, 3)
    }

    func testNotificationsInboxSectionsPartitionRooms() {
        let rooms = [
            RoomSummary(
                id: "!project:matrix.org",
                name: "Product",
                lastMessagePreview: "Mention",
                unreadCount: 3,
                hasHighlight: true,
                kind: .room,
                membership: .joined,
                lastActivityAt: RoomListFixtures.now
            ),
            RoomSummary(
                id: "!alerts:matrix.org",
                name: "Alerts",
                lastMessagePreview: "Invite",
                unreadCount: 1,
                hasHighlight: true,
                kind: .room,
                membership: .invited,
                lastActivityAt: RoomListFixtures.now
            ),
            RoomSummary(
                id: "!general:matrix.org",
                name: "General",
                lastMessagePreview: "Unread only",
                unreadCount: 2,
                hasHighlight: false,
                kind: .room,
                membership: .joined,
                lastActivityAt: RoomListFixtures.now
            ),
            RoomSummary(
                id: "!design:matrix.org",
                name: "Design",
                lastMessagePreview: "Read",
                unreadCount: 0,
                hasHighlight: false,
                kind: .room,
                membership: .joined,
                lastActivityAt: RoomListFixtures.now
            ),
        ]

        let sections = NotificationsInboxSections.make(from: rooms)

        XCTAssertEqual(sections.mentions.map(\.id), ["!project:matrix.org"])
        XCTAssertEqual(sections.invites.map(\.id), ["!alerts:matrix.org"])
        XCTAssertEqual(sections.unreadRooms.map(\.id), ["!general:matrix.org"])
        XCTAssertEqual(NotificationsInboxSections.notificationRooms(from: rooms).count, 3)
    }

    func testTabBadgeCountsUseRoomListState() {
        let badges = TabBadgeCounts.make(from: RoomListFixtures.small())

        XCTAssertEqual(badges.notifications, 8)
        XCTAssertEqual(badges.rooms, 13)
    }

    func testAgentPendingInboxBuildsRowsFromLatestAgentCard() {
        let rooms = RoomListFixtures.small()
        let pending = AgentPendingInbox.pendingApprovals(from: rooms)

        XCTAssertEqual(pending.count, 2)
        XCTAssertEqual(pending.first?.roomID, "!security-agent:matrix.org")
        XCTAssertEqual(pending.first?.title, "Access review required")
        XCTAssertEqual(pending.first?.eventID, "$agent-access-approval:matrix.org")
        XCTAssertEqual(pending.last?.roomID, "!agent-workflows:matrix.org")
        XCTAssertEqual(pending.last?.title, "Deploy approval required")
        XCTAssertEqual(pending.last?.eventID, "$agent-deploy-approval:matrix.org")
    }

    func testRoomSummaryRequiresAgentApprovalFromLatestCard() {
        let rooms = RoomListFixtures.small()
        let agentRoom = rooms.first { $0.id == "!agent-workflows:matrix.org" }
        let generalRoom = rooms.first { $0.id == "!general:matrix.org" }

        XCTAssertEqual(agentRoom?.requiresAgentApproval, true)
        XCTAssertEqual(generalRoom?.requiresAgentApproval, false)
    }

    func testAgentCardRequiresUserApprovalWhenApproveActionsPresent() throws {
        let card = try SynaraAgentCard(
            title: "Review deploy",
            status: "pending",
            summary: "Needs your review",
            actions: [
                SynaraAgentCardAction(id: "approve", title: "Approve", kind: "approve", prompt: "ok"),
                SynaraAgentCardAction(id: "reject", title: "Reject", kind: "reject", prompt: "no"),
            ]
        )

        XCTAssertTrue(card.requiresUserApproval)
    }

    func testMockRoomListStreamYieldsMultipleStates() async {
        let initialRooms = RoomListFixtures.small()
        let updatedRooms = [
            RoomSummary(
                id: "!new:matrix.org",
                name: "New room",
                lastMessagePreview: "Fresh activity",
                unreadCount: 2,
                hasHighlight: true,
                kind: .room,
                membership: .joined,
                lastActivityAt: RoomListFixtures.now
            ),
        ] + initialRooms
        let service = MockRoomListService(state: .loaded(initialRooms))
        service.updateStates = [
            .loaded(initialRooms),
            .loaded(updatedRooms),
        ]

        var states: [RoomListState] = []
        for await state in service.roomUpdates() {
            states.append(state)
        }

        XCTAssertEqual(states.count, 2)
        guard case let .loaded(firstBatch) = states[0],
              case let .loaded(secondBatch) = states[1]
        else {
            XCTFail("Expected loaded room list states")
            return
        }
        XCTAssertEqual(firstBatch.count, initialRooms.count)
        XCTAssertEqual(secondBatch.first?.id, "!new:matrix.org")
    }

    func testLatestSnapshotAccumulatorCoalescesRapidDiffsDuringSlowMapping() {
        let accumulator = RoomListLatestSnapshotAccumulator<String>()
        accumulator.yield(
            RoomListCoalescingSnapshot(
                rooms: ["!one", "!two"],
                changedRoomIDs: ["!one"],
                requiresFullRemap: false
            )
        )
        let mappingInProgress = accumulator.takePendingSnapshot()

        accumulator.yield(
            RoomListCoalescingSnapshot(
                rooms: ["!one", "!two", "!three"],
                changedRoomIDs: ["!two"],
                requiresFullRemap: false
            )
        )
        accumulator.yield(
            RoomListCoalescingSnapshot(
                rooms: ["!two", "!three"],
                changedRoomIDs: ["!three"],
                requiresFullRemap: false
            )
        )
        let coalesced = accumulator.takePendingSnapshot()
        accumulator.finish()

        XCTAssertEqual(mappingInProgress?.rooms, ["!one", "!two"])
        XCTAssertEqual(coalesced?.rooms, ["!two", "!three"])
        XCTAssertEqual(coalesced?.changedRoomIDs, Set(["!two", "!three"]))
        XCTAssertEqual(coalesced?.requiresFullRemap, false)
        XCTAssertNil(accumulator.takePendingSnapshot())
    }

    func testLatestSnapshotAccumulatorRetainsRemovalInLatestRoomArray() {
        let accumulator = RoomListLatestSnapshotAccumulator<String>()
        accumulator.yield(
            RoomListCoalescingSnapshot(
                rooms: ["!one", "!two", "!three"],
                changedRoomIDs: [],
                requiresFullRemap: false
            )
        )
        _ = accumulator.takePendingSnapshot()

        accumulator.yield(
            RoomListCoalescingSnapshot(
                rooms: ["!one", "!three"],
                changedRoomIDs: [],
                requiresFullRemap: false
            )
        )
        let removal = accumulator.takePendingSnapshot()
        accumulator.finish()

        XCTAssertEqual(removal?.rooms, ["!one", "!three"])
        XCTAssertEqual(removal?.changedRoomIDs, [])
        XCTAssertEqual(removal?.requiresFullRemap, false)
    }

    func testLatestSnapshotAccumulatorCarriesResetAcrossLaterDiffs() {
        let accumulator = RoomListLatestSnapshotAccumulator<String>()
        accumulator.yield(
            RoomListCoalescingSnapshot(
                rooms: ["!reset"],
                changedRoomIDs: ["!reset"],
                requiresFullRemap: true
            )
        )
        accumulator.yield(
            RoomListCoalescingSnapshot(
                rooms: ["!reset", "!after-reset"],
                changedRoomIDs: ["!after-reset"],
                requiresFullRemap: false
            )
        )
        let coalesced = accumulator.takePendingSnapshot()
        accumulator.finish()

        XCTAssertEqual(coalesced?.rooms, ["!reset", "!after-reset"])
        XCTAssertEqual(coalesced?.changedRoomIDs, Set(["!reset", "!after-reset"]))
        XCTAssertEqual(coalesced?.requiresFullRemap, true)
    }

    func testLatestSnapshotAccumulatorPreservesEmptyResetAsAuthoritative() {
        let accumulator = RoomListLatestSnapshotAccumulator<String>()
        accumulator.yield(
            RoomListCoalescingSnapshot(
                rooms: ["!stale"],
                changedRoomIDs: ["!stale"],
                requiresFullRemap: false
            )
        )
        accumulator.yield(
            RoomListCoalescingSnapshot(
                rooms: [],
                changedRoomIDs: [],
                requiresFullRemap: true
            )
        )
        let reset = accumulator.takePendingSnapshot()
        accumulator.finish()

        XCTAssertEqual(reset?.rooms, [])
        XCTAssertEqual(reset?.changedRoomIDs, ["!stale"])
        XCTAssertEqual(reset?.requiresFullRemap, true)
    }

    func testRoomActivityTimestampIsMonotonicAcrossPartialAndStaleSnapshots() {
        let previous = RoomListFixtures.now.addingTimeInterval(-120)
        let staleLatest = previous.addingTimeInterval(-600)

        XCTAssertEqual(RoomActivityTimestamp.resolve(latest: nil, previous: previous), previous)
        XCTAssertEqual(RoomActivityTimestamp.resolve(latest: staleLatest, previous: previous), previous)
        XCTAssertEqual(
            RoomActivityTimestamp.resolve(latest: RoomListFixtures.now, previous: previous),
            RoomListFixtures.now
        )
        XCTAssertEqual(RoomActivityTimestamp.resolve(latest: nil, previous: nil), .distantPast)
    }

    func testRecentActivityQualificationExcludesEphemeralAndOrdinaryStateEvents() {
        XCTAssertTrue(RoomActivityQualification.qualifies(.messageLike))
        XCTAssertTrue(RoomActivityQualification.qualifies(.localEcho))
        XCTAssertTrue(RoomActivityQualification.qualifies(.invite))
        XCTAssertFalse(RoomActivityQualification.qualifies(.receipt))
        XCTAssertFalse(RoomActivityQualification.qualifies(.typing))
        XCTAssertFalse(RoomActivityQualification.qualifies(.state))
    }

    func testRecentColdStartRecoveryGatePreservesNilDistantPastAndKnownDateParity() {
        let knownDate = RoomListFixtures.now

        XCTAssertTrue(
            RoomActivityRecoveryPolicy.shouldRecover(
                latestRequiresRecovery: true,
                previousActivityAt: nil
            )
        )
        XCTAssertTrue(
            RoomActivityRecoveryPolicy.shouldRecover(
                latestRequiresRecovery: true,
                previousActivityAt: .distantPast
            )
        )
        XCTAssertFalse(
            RoomActivityRecoveryPolicy.shouldRecover(
                latestRequiresRecovery: true,
                previousActivityAt: knownDate
            )
        )
        XCTAssertFalse(
            RoomActivityRecoveryPolicy.shouldRecover(
                latestRequiresRecovery: false,
                previousActivityAt: nil
            )
        )
        XCTAssertFalse(
            RoomActivityRecoveryPolicy.shouldRecover(
                latestRequiresRecovery: false,
                previousActivityAt: .distantPast
            )
        )
        XCTAssertFalse(
            RoomActivityRecoveryPolicy.shouldRecover(
                latestRequiresRecovery: false,
                previousActivityAt: knownDate
            )
        )
    }

    func testGeneratedCoreRecoveryGateBindingExecutesExhaustiveTruthTable() {
        XCTAssertFalse(
            SynaraCore.roomActivityRecoveryRequired(
                latestRequiresRecovery: false,
                previousState: .missing
            )
        )
        XCTAssertFalse(
            SynaraCore.roomActivityRecoveryRequired(
                latestRequiresRecovery: false,
                previousState: .known
            )
        )
        XCTAssertTrue(
            SynaraCore.roomActivityRecoveryRequired(
                latestRequiresRecovery: true,
                previousState: .missing
            )
        )
        XCTAssertFalse(
            SynaraCore.roomActivityRecoveryRequired(
                latestRequiresRecovery: true,
                previousState: .known
            )
        )
    }

    func testRecentColdStartRecoveryChoosesNewestQualifyingMessageBehindState() {
        let messageTimestamp = RoomListFixtures.now.addingTimeInterval(-60)
        let candidates = [
            RoomActivityRecoveryPolicy.Candidate(
                timestamp: messageTimestamp.addingTimeInterval(-60),
                kind: .messageLike
            ),
            RoomActivityRecoveryPolicy.Candidate(timestamp: messageTimestamp, kind: .localEcho),
            RoomActivityRecoveryPolicy.Candidate(
                timestamp: RoomListFixtures.now,
                kind: .state
            ),
        ]

        XCTAssertEqual(
            RoomActivityRecoveryPolicy.newestQualifyingTimestamp(from: candidates),
            messageTimestamp
        )
    }

    func testRecentColdStartRecoveryCannotInspectBeyondOneBoundedPage() {
        let hiddenOutsideBound = RoomActivityRecoveryPolicy.Candidate(
            timestamp: RoomListFixtures.now,
            kind: .messageLike
        )
        let boundedStatePage = (0 ..< RoomActivityRecoveryPolicy.maximumTimelineEvents).map { offset in
            RoomActivityRecoveryPolicy.Candidate(
                timestamp: RoomListFixtures.now.addingTimeInterval(TimeInterval(offset + 1)),
                kind: .state
            )
        }

        XCTAssertNil(
            RoomActivityRecoveryPolicy.newestQualifyingTimestamp(
                from: [hiddenOutsideBound] + boundedStatePage
            )
        )
    }

    func testFavoritesPartitionSeparatesTaggedJoinedRoomsFromRemaining() {
        let favorite = makeActivityRoom(id: "!fav:matrix.org", name: "Fav", activity: RoomListFixtures.now)
            .withFavorite(true)
        let remaining = makeActivityRoom(id: "!room:matrix.org", name: "Room", activity: RoomListFixtures.now)
        let taggedInvite = RoomSummary(
            id: "!invite:matrix.org",
            name: "Invite",
            lastMessagePreview: "Invited",
            unreadCount: 0,
            hasHighlight: false,
            kind: .room,
            membership: .invited,
            lastActivityAt: RoomListFixtures.now,
            isFavorite: true
        )
        let partition = RoomListFavorites.partition(from: [remaining, favorite, taggedInvite])

        XCTAssertEqual(partition.favorites.map(\.id), ["!fav:matrix.org"])
        XCTAssertEqual(partition.remaining.map(\.id), ["!room:matrix.org", "!invite:matrix.org"])
    }

    func testRoomListSortOrdersByNameAndRecentActivity() {
        let rooms = [
            makeActivityRoom(id: "!b:matrix.org", name: "Zeta", activity: RoomListFixtures.now.addingTimeInterval(-60)),
            makeActivityRoom(id: "!a:matrix.org", name: "Alpha", activity: RoomListFixtures.now.addingTimeInterval(-600)),
            makeActivityRoom(id: "!none:matrix.org", name: "None", activity: .distantPast),
        ]

        XCTAssertEqual(
            RoomListFavorites.sorted(rooms, order: .name).map(\.id),
            ["!a:matrix.org", "!none:matrix.org", "!b:matrix.org"]
        )
        XCTAssertEqual(
            RoomListFavorites.sorted(rooms, order: .recent).map(\.id),
            ["!b:matrix.org", "!a:matrix.org", "!none:matrix.org"]
        )
        XCTAssertEqual(RoomListSortOrder.defaultOrder, .recent)
        XCTAssertEqual(RoomListSortOrder.storageKey, "synara.roomListSort")
    }

    func testRoomListViewReplacedRecentTwentyFourHourSectionWithFavoritesAndSort() throws {
        let root = Self.repositoryRoot()
        let view = try String(
            contentsOfFile: "\(root)/synara-ios/Synara/Features/RoomListView.swift",
            encoding: .utf8
        )
        let service = try String(
            contentsOfFile: "\(root)/synara-ios/Synara/Services/RoomListService.swift",
            encoding: .utf8
        )
        let tests = try String(contentsOfFile: #filePath, encoding: .utf8)
        XCTAssertFalse(view.contains("Recent activity (24h)"))
        XCTAssertFalse(service.contains("enum RoomListRecentActivity"))
        XCTAssertFalse(service.contains("TimeInterval = 86400"))
        XCTAssertFalse(tests.contains("RoomListRecentActivity.partition"))
        XCTAssertFalse(tests.contains("TimeInterval = 86400"))
        XCTAssertTrue(view.contains("Favorites"))
        XCTAssertTrue(view.contains("RoomListSortMenu"))
        XCTAssertTrue(view.contains("Add to Favorites"))
        XCTAssertTrue(service.contains("synara.roomListSort"))
    }

    func testDynamicRoomListRequestsEveryPageBeyondOneHundredRooms() {
        XCTAssertEqual(
            RoomListDynamicPagingPolicy.nextRequestedPageCount(
                snapshotCount: 100,
                requestedPageCount: 1,
                pageSize: 100
            ),
            2
        )
        XCTAssertEqual(
            RoomListDynamicPagingPolicy.nextRequestedPageCount(
                snapshotCount: 200,
                requestedPageCount: 2,
                pageSize: 100
            ),
            3
        )
        XCTAssertNil(
            RoomListDynamicPagingPolicy.nextRequestedPageCount(
                snapshotCount: 250,
                requestedPageCount: 3,
                pageSize: 100
            )
        )
    }

    func testDynamicHeadRetainsOffPageRoomsAndOnlyAppliesExplicitRemoval() {
        let previousIDs = Set((0 ..< 150).map { "!room-\($0):matrix.org" })
        let dynamicHeadIDs = Set((0 ..< 100).map { "!room-\($0):matrix.org" })
        let removedID = "!room-149:matrix.org"

        let retained = RoomListCacheRetentionPolicy.retainedPreviousIDs(
            previousIDs: previousIDs,
            explicitlyRemovedIDs: [removedID]
        )

        XCTAssertEqual(retained.count, 149)
        XCTAssertTrue(dynamicHeadIDs.isSubset(of: retained))
        XCTAssertTrue(retained.contains("!room-120:matrix.org"))
        XCTAssertFalse(retained.contains(removedID))
    }

    func testOffPageLeaveIsPrunedFromRetainedRoomsUsingAuthoritativeMembership() {
        let previousIDs = Set((0 ..< 150).map { "!room-\($0):matrix.org" })
        let offPageLeftID = "!room-149:matrix.org"
        let authoritativeKnownIDs = previousIDs
        let authoritativeActiveIDs = previousIDs.subtracting([offPageLeftID])

        let provenRemovedIDs = RoomListAuthoritativePruningPolicy.provenRemovedIDs(
            knownRoomIDs: authoritativeKnownIDs,
            joinedOrInvitedRoomIDs: authoritativeActiveIDs
        )
        let retained = RoomListCacheRetentionPolicy.retainedPreviousIDs(
            previousIDs: previousIDs,
            explicitlyRemovedIDs: provenRemovedIDs
        )

        XCTAssertEqual(provenRemovedIDs, [offPageLeftID])
        XCTAssertEqual(retained.count, 149)
        XCTAssertTrue(retained.contains("!room-120:matrix.org"))
        XCTAssertFalse(retained.contains(offPageLeftID))
        XCTAssertTrue(RoomListAuthoritativePruningPolicy.shouldReconcile(
            cachedRoomIDs: previousIDs,
            dynamicSnapshotRoomIDs: Set((0 ..< 100).map { "!room-\($0):matrix.org" }),
            requiresFullRemap: false
        ))
        XCTAssertFalse(RoomListAuthoritativePruningPolicy.shouldReconcile(
            cachedRoomIDs: previousIDs,
            dynamicSnapshotRoomIDs: Set((0 ..< 100).map { "!room-\($0):matrix.org" }),
            requiresFullRemap: false,
            currentCatchUpPageCount: 1,
            lastAttemptedCatchUpPageCount: 1,
            lastAttemptedAt: Date(timeIntervalSince1970: 100),
            now: Date(timeIntervalSince1970: 120),
            minimumInterval: 30
        ))
        XCTAssertTrue(RoomListAuthoritativePruningPolicy.shouldReconcile(
            cachedRoomIDs: previousIDs,
            dynamicSnapshotRoomIDs: Set((0 ..< 100).map { "!room-\($0):matrix.org" }),
            requiresFullRemap: false,
            currentCatchUpPageCount: 1,
            lastAttemptedCatchUpPageCount: 1,
            lastAttemptedAt: Date(timeIntervalSince1970: 100),
            now: Date(timeIntervalSince1970: 130),
            minimumInterval: 30
        ))
        XCTAssertTrue(RoomListAuthoritativePruningPolicy.shouldReconcile(
            cachedRoomIDs: previousIDs,
            dynamicSnapshotRoomIDs: Set((0 ..< 100).map { "!room-\($0):matrix.org" }),
            requiresFullRemap: false,
            currentCatchUpPageCount: 2,
            lastAttemptedCatchUpPageCount: 1,
            lastAttemptedAt: Date(timeIntervalSince1970: 100),
            now: Date(timeIntervalSince1970: 101),
            minimumInterval: 30
        ))
        XCTAssertTrue(RoomListAuthoritativePruningPolicy.shouldReconcile(
            cachedRoomIDs: previousIDs,
            dynamicSnapshotRoomIDs: Set((0 ..< 100).map { "!room-\($0):matrix.org" }),
            requiresFullRemap: true,
            currentCatchUpPageCount: 1,
            lastAttemptedCatchUpPageCount: 1,
            lastAttemptedAt: Date(timeIntervalSince1970: 100),
            now: Date(timeIntervalSince1970: 101),
            minimumInterval: 30
        ))
    }

    func testClearReconnectPreservesKnownActiveRoomsAndPrunesOnlyProvenLeftRooms() {
        let cachedIDs: Set<String> = ["!joined-off-page", "!invited-off-page", "!left-off-page"]
        let provenRemovedIDs = RoomListAuthoritativePruningPolicy.provenRemovedIDs(
            knownRoomIDs: cachedIDs,
            joinedOrInvitedRoomIDs: ["!joined-off-page", "!invited-off-page"]
        )

        let retainedAfterClear = RoomListCacheRetentionPolicy.retainedPreviousIDs(
            previousIDs: cachedIDs,
            explicitlyRemovedIDs: provenRemovedIDs
        )

        XCTAssertEqual(retainedAfterClear, ["!joined-off-page", "!invited-off-page"])
        XCTAssertFalse(retainedAfterClear.contains("!left-off-page"))
        XCTAssertTrue(RoomListAuthoritativePruningPolicy.shouldReconcile(
            cachedRoomIDs: cachedIDs,
            dynamicSnapshotRoomIDs: [],
            requiresFullRemap: true
        ))
        XCTAssertFalse(RoomListAuthoritativePruningPolicy.shouldReconcile(
            cachedRoomIDs: cachedIDs,
            dynamicSnapshotRoomIDs: cachedIDs,
            requiresFullRemap: false
        ))
        let ambiguousMissingID = "!missing-from-authoritative-snapshot"
        let knownAuthoritativeIDs: Set<String> = ["!joined-off-page", "!left-off-page"]
        let activeAuthoritativeIDs: Set<String> = ["!joined-off-page"]
        let provenRemoved = RoomListAuthoritativePruningPolicy.provenRemovedIDs(
            knownRoomIDs: knownAuthoritativeIDs,
            joinedOrInvitedRoomIDs: activeAuthoritativeIDs
        )
        XCTAssertFalse(provenRemoved.contains(ambiguousMissingID))
        XCTAssertTrue(RoomListCacheRetentionPolicy.retainedPreviousIDs(
            previousIDs: cachedIDs.union([ambiguousMissingID]),
            explicitlyRemovedIDs: provenRemoved
        ).contains(ambiguousMissingID))
    }

    func testAuthoritativeNonemptyRoomArrayNeverFallsBackToGhostCache() {
        XCTAssertTrue(RoomListAuthoritativeFallbackPolicy.shouldUseCachedFallback(
            authoritativeRoomCount: 0,
            cachedRoomCount: 1
        ))
        XCTAssertFalse(RoomListAuthoritativeFallbackPolicy.shouldUseCachedFallback(
            authoritativeRoomCount: 1,
            cachedRoomCount: 1
        ))
        XCTAssertFalse(RoomListAuthoritativeFallbackPolicy.shouldUseCachedFallback(
            authoritativeRoomCount: 0,
            cachedRoomCount: 0
        ))
        XCTAssertTrue(RoomListReconciliationHeartbeatPolicy.shouldContinue(
            isCancelled: false,
            isCurrentSession: true,
            currentGeneration: 7,
            expectedGeneration: 7
        ))
        XCTAssertFalse(RoomListReconciliationHeartbeatPolicy.shouldContinue(
            isCancelled: true,
            isCurrentSession: true,
            currentGeneration: 7,
            expectedGeneration: 7
        ))
        XCTAssertFalse(RoomListReconciliationHeartbeatPolicy.shouldContinue(
            isCancelled: false,
            isCurrentSession: false,
            currentGeneration: 7,
            expectedGeneration: 7
        ))
        XCTAssertFalse(RoomListReconciliationHeartbeatPolicy.shouldContinue(
            isCancelled: false,
            isCurrentSession: true,
            currentGeneration: 8,
            expectedGeneration: 7
        ))
        XCTAssertTrue(RoomListReconciliationHeartbeatPolicy.shouldEmit(
            cachedRoomIDs: ["!visible", "!off-page"],
            dynamicSnapshotRoomIDs: ["!visible"]
        ))
        XCTAssertFalse(RoomListReconciliationHeartbeatPolicy.shouldEmit(
            cachedRoomIDs: ["!visible"],
            dynamicSnapshotRoomIDs: ["!visible"]
        ))
        XCTAssertFalse(RoomListReconciliationHeartbeatPolicy.shouldEmit(
            cachedRoomIDs: [],
            dynamicSnapshotRoomIDs: []
        ))
    }

    func testLatestSnapshotAccumulatorCarriesExplicitRemovalAcrossCoalescing() {
        let accumulator = RoomListLatestSnapshotAccumulator<String>()
        accumulator.yield(RoomListCoalescingSnapshot(
            rooms: ["!one"],
            changedRoomIDs: [],
            requiresFullRemap: false,
            explicitlyRemovedRoomIDs: ["!removed"]
        ))
        accumulator.yield(RoomListCoalescingSnapshot(
            rooms: ["!one", "!two"],
            changedRoomIDs: ["!two"],
            requiresFullRemap: false
        ))

        let snapshot = accumulator.takePendingSnapshot()
        accumulator.finish()

        XCTAssertEqual(snapshot?.rooms, ["!one", "!two"])
        XCTAssertEqual(snapshot?.explicitlyRemovedRoomIDs, ["!removed"])
        XCTAssertFalse(snapshot?.isReconciliationHeartbeat ?? true)

        let heartbeatOnlyAccumulator = RoomListLatestSnapshotAccumulator<String>()
        heartbeatOnlyAccumulator.yield(RoomListCoalescingSnapshot(
            rooms: ["!one"],
            changedRoomIDs: [],
            requiresFullRemap: false,
            isReconciliationHeartbeat: true
        ))
        heartbeatOnlyAccumulator.yield(RoomListCoalescingSnapshot(
            rooms: ["!one"],
            changedRoomIDs: [],
            requiresFullRemap: false,
            isReconciliationHeartbeat: true
        ))
        XCTAssertTrue(heartbeatOnlyAccumulator.takePendingSnapshot()?.isReconciliationHeartbeat ?? false)
        heartbeatOnlyAccumulator.finish()

        let heartbeatThenReal = RoomListLatestSnapshotAccumulator<String>()
        heartbeatThenReal.yield(RoomListCoalescingSnapshot(
            rooms: ["!one"],
            changedRoomIDs: [],
            requiresFullRemap: false,
            isReconciliationHeartbeat: true
        ))
        heartbeatThenReal.yield(RoomListCoalescingSnapshot(
            rooms: ["!one", "!two"],
            changedRoomIDs: ["!two"],
            requiresFullRemap: true,
            explicitlyRemovedRoomIDs: ["!removed"]
        ))
        let heartbeatThenRealSnapshot = heartbeatThenReal.takePendingSnapshot()
        XCTAssertFalse(heartbeatThenRealSnapshot?.isReconciliationHeartbeat ?? true)
        XCTAssertEqual(heartbeatThenRealSnapshot?.changedRoomIDs, ["!two"])
        XCTAssertEqual(heartbeatThenRealSnapshot?.explicitlyRemovedRoomIDs, ["!removed"])
        XCTAssertTrue(heartbeatThenRealSnapshot?.requiresFullRemap ?? false)
        heartbeatThenReal.finish()

        let realThenHeartbeat = RoomListLatestSnapshotAccumulator<String>()
        realThenHeartbeat.yield(RoomListCoalescingSnapshot(
            rooms: ["!one"],
            changedRoomIDs: ["!one"],
            requiresFullRemap: true,
            explicitlyRemovedRoomIDs: ["!removed"]
        ))
        realThenHeartbeat.yield(RoomListCoalescingSnapshot(
            rooms: ["!one", "!two"],
            changedRoomIDs: [],
            requiresFullRemap: false,
            isReconciliationHeartbeat: true
        ))
        let realThenHeartbeatSnapshot = realThenHeartbeat.takePendingSnapshot()
        XCTAssertFalse(realThenHeartbeatSnapshot?.isReconciliationHeartbeat ?? true)
        XCTAssertEqual(realThenHeartbeatSnapshot?.rooms, ["!one", "!two"])
        XCTAssertEqual(realThenHeartbeatSnapshot?.changedRoomIDs, ["!one"])
        XCTAssertEqual(realThenHeartbeatSnapshot?.explicitlyRemovedRoomIDs, ["!removed"])
        XCTAssertTrue(realThenHeartbeatSnapshot?.requiresFullRemap ?? false)
        realThenHeartbeat.finish()
    }

    private func makeActivityRoom(id: String, name: String, activity: Date) -> RoomSummary {
        RoomSummary(
            id: id,
            name: name,
            lastMessagePreview: "Activity",
            unreadCount: 0,
            hasHighlight: false,
            kind: .room,
            membership: .joined,
            lastActivityAt: activity
        )
    }

    private static func repositoryRoot() -> String {
        var url = URL(fileURLWithPath: #filePath)
        while url.pathComponents.count > 1 {
            url.deleteLastPathComponent()
            if FileManager.default.fileExists(atPath: url.appendingPathComponent("synara-ios").path) {
                return url.path
            }
        }
        return url.path
    }
}

private extension RoomSummary {
    func withFavorite(_ isFavorite: Bool) -> RoomSummary {
        RoomSummary(
            id: id,
            name: name,
            lastMessagePreview: lastMessagePreview,
            unreadCount: unreadCount,
            hasHighlight: hasHighlight,
            kind: kind,
            membership: membership,
            lastActivityAt: lastActivityAt,
            parentSpaces: parentSpaces,
            avatarURL: avatarURL,
            hasAgentActivity: hasAgentActivity,
            latestAgentCard: latestAgentCard,
            latestAgentCardEventID: latestAgentCardEventID,
            pendingAgentApprovals: pendingAgentApprovals,
            isFavorite: isFavorite
        )
    }
}
