import XCTest
@testable import Synara

final class AgentApprovalHistoryProjectionTests: XCTestCase {
    func testUnionRecentKeepsPendingOutAndDoesNotLetAccountDataNameAnUnprovenDecision() {
        let recent = AgentApprovalHistoryProjection.unionRecent(
            inboxItems: [
                inbox(eventId: "$pending", status: .pending),
                inbox(eventId: "$expired", status: .expired, originServerTs: 500),
                inbox(
                    eventId: "$same",
                    status: .decided,
                    body: "inbox body",
                    originServerTs: 1_500
                ),
            ],
            historyItems: [
                history(eventId: "$same", decidedAt: 9_000, summary: "account summary"),
                history(eventId: "$synced", decidedAt: 8_000, summary: "ls"),
            ]
        )

        XCTAssertEqual(recent.map(\.eventId), ["$same", "$synced", "$expired"])
        XCTAssertEqual(recent[0].summary, "account summary")
        XCTAssertEqual(recent[0].status, .decided)
        XCTAssertNil(recent[0].decision)
        XCTAssertEqual(recent[2].status, .expired)
        XCTAssertNil(recent.first { $0.eventId == "$pending" })
    }

    func testInboxDecisionWinsAConflictWithHistory() {
        let recent = AgentApprovalHistoryProjection.unionRecent(
            inboxItems: [
                inbox(
                    eventId: "$same",
                    status: .decided,
                    decision: .deny,
                    decidedAt: 4_000
                ),
            ],
            historyItems: [
                history(
                    eventId: "$same",
                    decision: .approveAlways,
                    decidedAt: 9_000,
                    summary: "ls"
                ),
            ]
        )

        XCTAssertEqual(recent.count, 1)
        XCTAssertEqual(recent[0].decision, .deny)
        XCTAssertEqual(recent[0].status, .decided)
        XCTAssertEqual(recent[0].summary, "ls")
    }

    func testExpiredInboxWithoutADecisionStillShowsTheHistoryDecision() {
        let recent = AgentApprovalHistoryProjection.unionRecent(
            inboxItems: [
                inbox(eventId: "$same", status: .expired, originServerTs: 1_000),
            ],
            historyItems: [
                history(eventId: "$same", decision: .approveAlways, decidedAt: 9_000, summary: "ls"),
            ]
        )

        XCTAssertEqual(recent[0].decision, .approveAlways)
        XCTAssertEqual(recent[0].status, .decided)
        XCTAssertEqual(
            AgentApprovalHistoryProjection.decisionLabel(
                decision: recent[0].decision,
                status: recent[0].status
            ),
            "Approved always"
        )
    }

    func testHistoryItemsNeverRenderAsExpiredAndKeepDecisionLabels() {
        let item = AgentApprovalHistoryProjection.inboxItem(
            from: history(
                eventId: "$old",
                decidedAt: 1_700_000_000_000,
                expiresAt: 2
            )
        )

        XCTAssertEqual(item.status, .decided)
        XCTAssertEqual(
            AgentApprovalHistoryProjection.decisionLabel(decision: item.decision, status: item.status),
            "Approved once"
        )
        XCTAssertEqual(
            AgentApprovalHistoryProjection.decisionLabel(decision: .approveAlways, status: .decided),
            "Approved always"
        )
        XCTAssertEqual(
            AgentApprovalHistoryProjection.decisionLabel(decision: .deny, status: .decided),
            "Denied"
        )
        XCTAssertEqual(
            AgentApprovalHistoryProjection.decisionLabel(decision: nil, status: .expired),
            "Expired"
        )
        XCTAssertEqual(
            AgentApprovalHistoryProjection.decisionLabel(decision: .deny, status: .expired),
            "Expired"
        )
    }

    func testUnionRecentSortsByDecidedAtThenOriginServerTs() {
        let recent = AgentApprovalHistoryProjection.unionRecent(
            inboxItems: [
                inbox(eventId: "$older", status: .decided, originServerTs: 4_000, decidedAt: 5_000),
            ],
            historyItems: [
                history(eventId: "$newer", decidedAt: 9_000),
                history(eventId: "$mid", decidedAt: 7_000),
            ]
        )

        XCTAssertEqual(recent.map(\.eventId), ["$newer", "$mid", "$older"])
    }

    func testEmptyOrWhitespaceSummariesFallBackToReadableLabel() {
        let blank = AgentApprovalHistoryProjection.inboxItem(from: history(eventId: "$blank", summary: ""))
        let spaces = AgentApprovalHistoryProjection.inboxItem(from: history(eventId: "$spaces", summary: "   "))

        XCTAssertEqual(blank.summary, "Command not recorded")
        XCTAssertEqual(blank.body, "Command not recorded")
        XCTAssertEqual(spaces.summary, "Command not recorded")
    }

    func testRecentSortIsStableForEqualTimestampsAndMissingDecidedAt() {
        let recent = AgentApprovalHistoryProjection.unionRecent(
            inboxItems: [
                inbox(eventId: "$b", status: .expired, originServerTs: 1_000),
                inbox(eventId: "$a", status: .expired, originServerTs: 1_000),
            ],
            historyItems: [
                history(
                    eventId: "$late",
                    roomId: "!other:example.org",
                    decidedAt: 5_000,
                    originServerTs: 1
                ),
            ]
        )

        XCTAssertEqual(recent.map(\.eventId), ["$late", "$a", "$b"])
        XCTAssertTrue(
            AgentApprovalHistoryProjection.compareRecent(recent[1], recent[2])
        )
    }

    func testDuplicateHistoryIdentitiesKeepASingleRow() {
        let long = String(repeating: "あ", count: 240)
        let recent = AgentApprovalHistoryProjection.unionRecent(
            inboxItems: [
                inbox(eventId: "$dup", status: .expired, originServerTs: 1),
            ],
            historyItems: [
                history(eventId: "$dup", decidedAt: 3, summary: "first"),
                history(eventId: "$dup", decidedAt: 4, summary: long),
            ]
        )

        XCTAssertEqual(recent.count, 1)
        XCTAssertEqual(recent[0].summary, long)
        XCTAssertEqual(recent[0].status, .decided)
    }

    private func inbox(
        eventId: String,
        status: AgentApprovalInboxStatus,
        body: String = "Approval Required: Dangerous Command",
        originServerTs: Double = 1_000,
        decision: AgentApprovalHistoryDecision? = nil,
        decidedAt: Double? = nil
    ) -> AgentApprovalInboxRecord {
        AgentApprovalInboxRecord(
            roomId: "!room:example.org",
            eventId: eventId,
            sender: "@hermes:example.org",
            body: body,
            canSendReaction: true,
            bodyTruncated: false,
            originServerTs: originServerTs,
            expiresAt: 301_000,
            status: status,
            decision: decision,
            decidedAt: decidedAt,
            summary: nil
        )
    }

    private func history(
        eventId: String,
        roomId: String = "!room:example.org",
        decision: AgentApprovalHistoryDecision = .approveOnce,
        decidedAt: Double = 2_000,
        originServerTs: Double = 1_000,
        expiresAt: Double = 301_000,
        summary: String = "rm file"
    ) -> AgentApprovalHistoryRecord {
        AgentApprovalHistoryRecord(
            roomId: roomId,
            eventId: eventId,
            sender: "@hermes:example.org",
            decision: decision,
            decidedAt: decidedAt,
            originServerTs: originServerTs,
            expiresAt: expiresAt,
            summary: summary
        )
    }
}
