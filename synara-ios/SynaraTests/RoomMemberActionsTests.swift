import XCTest
@testable import Synara

final class RoomMemberActionsTests: XCTestCase {
    func testJoinedModeratorCanRemoveAndBanButNotInvite() {
        let member = RoomMemberSummary(
            userID: "@bob:matrix.org",
            displayName: "Bob",
            membership: "join",
            powerLevel: 0
        )
        let plan = RoomMemberActionPlan.plan(
            member: member,
            ownUserID: "@alice:matrix.org",
            powerLevels: .fullPower
        )

        XCTAssertTrue(plan.canRemove)
        XCTAssertTrue(plan.canBan)
        XCTAssertFalse(plan.canInvite)
        XCTAssertFalse(plan.canUnban)
        XCTAssertTrue(plan.canIgnore)
        XCTAssertTrue(plan.canEditPowerLevel)
        XCTAssertEqual(plan.assignablePowerLevels, [0, 50, 100])
    }

    func testLeftMemberCanBeInvitedAndBanned() {
        let member = RoomMemberSummary(
            userID: "@carol:matrix.org",
            displayName: "Carol",
            membership: "leave",
            powerLevel: 0
        )
        let plan = RoomMemberActionPlan.plan(
            member: member,
            ownUserID: "@alice:matrix.org",
            powerLevels: .fullPower
        )

        XCTAssertTrue(plan.canInvite)
        XCTAssertFalse(plan.canRemove)
        XCTAssertTrue(plan.canBan)
        XCTAssertFalse(plan.canCancelInvite)
    }

    func testSelfAndEqualPowerHideModerationWrites() {
        let selfMember = RoomMemberSummary(
            userID: "@alice:matrix.org",
            displayName: "Alice",
            membership: "join",
            powerLevel: 100
        )
        let selfPlan = RoomMemberActionPlan.plan(
            member: selfMember,
            ownUserID: "@alice:matrix.org",
            powerLevels: .fullPower
        )
        XCTAssertFalse(selfPlan.canRemove)
        XCTAssertFalse(selfPlan.canBan)
        XCTAssertFalse(selfPlan.canIgnore)

        let peerAdmin = RoomMemberSummary(
            userID: "@bob:matrix.org",
            displayName: "Bob",
            membership: "join",
            powerLevel: 100
        )
        let equalPlan = RoomMemberActionPlan.plan(
            member: peerAdmin,
            ownUserID: "@alice:matrix.org",
            powerLevels: .fullPower
        )
        XCTAssertFalse(equalPlan.canRemove)
        XCTAssertFalse(equalPlan.canBan)
        XCTAssertTrue(equalPlan.canIgnore)
    }

    func testBannedMemberExposesUnbanInsteadOfRemove() {
        let member = RoomMemberSummary(
            userID: "@carol:matrix.org",
            displayName: "Carol",
            membership: "ban",
            powerLevel: 0
        )
        let plan = RoomMemberActionPlan.plan(
            member: member,
            ownUserID: "@alice:matrix.org",
            powerLevels: .fullPower
        )

        XCTAssertTrue(plan.canUnban)
        XCTAssertFalse(plan.canBan)
        XCTAssertFalse(plan.canRemove)
        XCTAssertFalse(plan.canInvite)
        XCTAssertFalse(plan.canEditPowerLevel)
    }

    func testEqualPowerBannedMemberCannotBeUnbanned() {
        let member = RoomMemberSummary(
            userID: "@bob:matrix.org",
            displayName: "Bob",
            membership: "ban",
            powerLevel: 100
        )
        let plan = RoomMemberActionPlan.plan(
            member: member,
            ownUserID: "@alice:matrix.org",
            powerLevels: .fullPower
        )

        XCTAssertFalse(plan.canUnban)
        XCTAssertFalse(plan.canRemove)
        XCTAssertFalse(plan.canBan)
    }

    func testKnockingMemberCanBeAcceptedOrDenied() {
        let member = RoomMemberSummary(
            userID: "@dana:matrix.org",
            displayName: "Dana",
            membership: "knock",
            powerLevel: 0
        )
        let plan = RoomMemberActionPlan.plan(
            member: member,
            ownUserID: "@alice:matrix.org",
            powerLevels: .fullPower
        )

        XCTAssertTrue(plan.canAcceptKnock)
        XCTAssertTrue(plan.canDenyKnock)
        XCTAssertFalse(plan.canRemove)
        XCTAssertFalse(plan.canInvite)
        XCTAssertFalse(plan.canCancelInvite)
    }
}
