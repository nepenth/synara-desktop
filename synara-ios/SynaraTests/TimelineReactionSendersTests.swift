import XCTest
@testable import Synara
import SynaraCore

/// Who reacted: Core's senders reach `TimelineItem`, the pills read them to
/// VoiceOver, and the Reactions sheet groups and names them.
final class TimelineReactionSendersTests: XCTestCase {
    private let me = "@me:example.org"

    // MARK: DTO → item

    func testCoreSendersMapToTheItemInCoreOrder() {
        let senders = SharedCoreTimelineRows.reactionSenders(from: [
            TimelineViewReactionDto(key: "👍", count: 3, own: true, senders: [
                TimelineReactionSenderDto(userId: "@bob:example.org", reactionEventId: "$r1"),
                TimelineReactionSenderDto(userId: me, reactionEventId: "$r2"),
                TimelineReactionSenderDto(userId: "@alice:example.org", reactionEventId: nil),
                TimelineReactionSenderDto(userId: "@bob:example.org", reactionEventId: "$r3"),
            ]),
            TimelineViewReactionDto(key: "🎉", count: 1, own: false, senders: []),
            TimelineViewReactionDto(key: "", count: 1, own: false, senders: [
                TimelineReactionSenderDto(userId: "@carol:example.org", reactionEventId: nil),
            ]),
        ])
        XCTAssertEqual(senders, ["👍": ["@bob:example.org", me, "@alice:example.org"]])
    }

    func testMappedRowCarriesReactionSenders() {
        let item = SharedCoreTimelineRows.item(from: row(reactions: [
            TimelineViewReactionDto(key: "👍", count: 2, own: false, senders: [
                TimelineReactionSenderDto(userId: "@alice:example.org", reactionEventId: "$a"),
                TimelineReactionSenderDto(userId: "@bob:example.org", reactionEventId: "$b"),
            ]),
        ]))
        XCTAssertEqual(item?.reactions, ["👍": 2])
        XCTAssertEqual(item?.reactionSenders, ["👍": ["@alice:example.org", "@bob:example.org"]])
    }

    func testCopiesAndMockActionsKeepSenders() async throws {
        let item = TimelineItem(
            id: "$m", eventID: "$m", senderID: "@alice:example.org", timestamp: Date(),
            kind: .text("Hi"), replyToEventID: nil, isEdited: false,
            reactions: ["👍": 1], reactionOwnership: .known([]),
            reactionSenders: ["👍": ["@bob:example.org"]]
        )
        XCTAssertEqual(item.withDeliveryStatus(nil).reactionSenders, item.reactionSenders)
        XCTAssertEqual(item.withSenderAvatarURL(nil).reactionSenders, item.reactionSenders)

        let service = MockEventActionService()
        let edited = try await service.apply(.edit, to: item, currentUserID: me, roomID: "!r:example.org")
        XCTAssertEqual(edited.reactionSenders, item.reactionSenders)
        let reacted = try await service.apply(.react("👍"), to: item, currentUserID: me, roomID: "!r:example.org")
        XCTAssertEqual(reacted.reactionSenders, ["👍": ["@bob:example.org", me]])
        let redacted = try await service.apply(.redact, to: item, currentUserID: me, roomID: "!r:example.org")
        XCTAssertEqual(redacted.reactionSenders, [:])
    }

    // MARK: Pill labels

    func testPillLabelNamesOneTwoThreeAndMoreSenders() {
        XCTAssertEqual(label(count: 1, you: false, ["Alice"]), "👍, 1 reaction: Alice")
        XCTAssertEqual(label(count: 2, you: false, ["Alice", "Bob"]), "👍, 2 reactions: Alice and Bob")
        XCTAssertEqual(label(count: 3, you: false, ["Alice", "Bob", "Carol"]), "👍, 3 reactions: Alice, Bob and Carol")
        XCTAssertEqual(
            label(count: 4, you: false, ["Alice", "Bob", "Carol", "Dan"]),
            "👍, 4 reactions: Alice, Bob and 2 others"
        )
        XCTAssertEqual(
            label(count: 5, you: false, ["Alice", "Bob", "Carol"]),
            "👍, 5 reactions: Alice, Bob and 3 others"
        )
    }

    func testPillLabelSaysYouLast() {
        XCTAssertEqual(label(count: 1, you: true, []), "👍, 1 reaction: you")
        XCTAssertEqual(label(count: 2, you: true, ["Alice"]), "👍, 2 reactions: Alice and you")
        XCTAssertEqual(label(count: 3, you: true, ["Alice", "Bob"]), "👍, 3 reactions: Alice, Bob and you")
        XCTAssertEqual(
            label(count: 4, you: true, ["Alice", "Bob", "Carol"]),
            "👍, 4 reactions: Alice, Bob, you and 1 other"
        )
    }

    func testPillLabelWithoutNamedSendersFallsBackToTheCount() {
        XCTAssertEqual(label(count: 3, you: false, []), "👍, 3 reactions")
        // Core named only one of three.
        XCTAssertEqual(label(count: 3, you: false, ["Alice"]), "👍, 3 reactions: Alice and 2 others")
    }

    func testPillNamesExcludeTheUserAndUseTheDirectory() {
        let item = TimelineItem(
            id: "$m", eventID: "$m", senderID: "@alice:example.org", timestamp: Date(),
            kind: .text("Hi"), replyToEventID: nil, isEdited: false,
            reactions: ["👍": 3], reactionOwnership: .known([]),
            reactionSenders: ["👍": ["@alice:example.org", me, "@bob:example.org"]]
        )
        let names = TimelineReactorDirectory(items: [
            TimelineItem(
                id: "$a", eventID: "$a", senderID: "@alice:example.org", senderProfileDisplayName: "Alice A",
                timestamp: Date(), kind: .text("x"), replyToEventID: nil, isEdited: false, reactions: [:]
            ),
        ]).names(for: item)
        XCTAssertEqual(
            TimelineReactionSenders.otherNames(key: "👍", in: item, currentUserID: me, names: names),
            ["Alice A", "bob"]
        )
        // In the sender list even before ownership is known.
        XCTAssertTrue(TimelineReactionSenders.includesCurrentUser(key: "👍", in: item, currentUserID: me))
    }

    func testOneElementRowsOfferViewReactionsAfterTheToggles() {
        let availability = EventActionAvailability(canReply: false, canEdit: false, canRedact: false, canReact: true)
        let actions = TimelineRowAccessibility.actions(
            availability: availability,
            replyCount: 0,
            canCopy: false,
            canPinToNotes: false,
            reactions: ["👍": 1],
            ownReactions: .known([]),
            canViewReactions: true
        )
        XCTAssertEqual(actions, [.react, .toggleReaction(key: "👍", isOwn: false), .viewReactions])
        XCTAssertEqual(TimelineRowAccessibility.Action.viewReactions.title, "View Reactions")
        XCTAssertFalse(
            TimelineRowAccessibility.actions(
                availability: availability, replyCount: 0, canCopy: false, canPinToNotes: false,
                canViewReactions: true
            ).contains(.viewReactions),
            "no reactions, nothing to view"
        )
    }

    // MARK: Sheet model

    func testSheetGroupsPutYouFirstAndNameEveryone() {
        let avatar = URL(string: "mxc://example.org/alice")
        var directory = TimelineReactorDirectory(items: [
            TimelineItem(
                id: "$a", eventID: "$a", senderID: "@alice:example.org", senderProfileDisplayName: "Alice",
                senderAvatarURL: avatar, timestamp: Date(), kind: .text("x"), replyToEventID: nil,
                isEdited: false, reactions: [:]
            ),
        ])
        directory.merge(members: [
            RoomMemberSummary(userID: "@bob:example.org", displayName: "Bob B", membership: .join, powerLevel: 0),
            RoomMemberSummary(userID: "@alice:example.org", displayName: "Alice Member", membership: .join, powerLevel: 0),
        ])
        let item = TimelineItem(
            id: "$m", eventID: "$m", senderID: "@alice:example.org", timestamp: Date(),
            kind: .text("Hi"), replyToEventID: nil, isEdited: false,
            reactions: ["👍": 3, "🎉": 2],
            reactionOwnership: .known(["👍"]),
            reactionSenders: [
                "👍": ["@alice:example.org", "@bob:example.org", me],
                "🎉": ["@carol:example.org"],
            ]
        )
        let groups = TimelineReactionSenders.groups(for: item, currentUserID: me, directory: directory)

        XCTAssertEqual(groups.map(\.key), ["🎉", "👍"])
        let thumbs = groups[1]
        XCTAssertEqual(thumbs.count, 3)
        XCTAssertEqual(thumbs.reactors.map(\.userID), [me, "@alice:example.org", "@bob:example.org"])
        // The timeline profile wins over the member list; members fill gaps.
        XCTAssertEqual(thumbs.reactors.map(\.title), ["You", "Alice", "Bob B"])
        XCTAssertEqual(thumbs.reactors[1].avatarURL, avatar)
        XCTAssertEqual(thumbs.unnamedCount, 0)

        let party = groups[0]
        XCTAssertEqual(party.reactors.map(\.title), ["carol"], "falls back to the localpart")
        XCTAssertEqual(party.unnamedCount, 1)

        XCTAssertEqual(TimelineReactionsSheetTarget(item: item, initialKey: "👍").selectedKey(in: groups), "👍")
        XCTAssertEqual(TimelineReactionsSheetTarget(item: item, initialKey: nil).selectedKey(in: groups), "🎉")
        XCTAssertEqual(TimelineReactionsSheetTarget(item: item, initialKey: "😂").selectedKey(in: groups), "🎉")
    }

    // MARK: Helpers

    private func label(count: Int, you: Bool, _ names: [String]) -> String {
        TimelineRowAccessibility.reactionPillLabel(key: "👍", count: count, includesYou: you, otherNames: names)
    }

    private func row(reactions: [TimelineViewReactionDto]) -> TimelineViewRowDto {
        TimelineViewRowDto(
            kind: .message,
            itemId: "item-1",
            eventId: "$message:example.org",
            sender: "@alice:example.org",
            senderName: "Alice",
            senderAvatarUrl: nil,
            body: "Hello",
            originServerTs: 1_700_000_000_000,
            edited: false,
            replyToEventId: nil,
            replyPreview: nil,
            threadRootEventId: nil,
            threadSummary: nil,
            poll: nil,
            capabilities: nil,
            decryptionState: nil,
            messageType: "m.text",
            forwardTransport: "text",
            formattedBody: nil,
            agentCardJson: nil,
            isAgentApproval: false,
            agentApprovalDecision: nil,
            agentApprovalDecidedAt: nil,
            mediaFilename: nil,
            mediaCaption: nil,
            reactions: reactions,
            mediaHandleId: nil,
            mediaMimeType: nil,
            mediaWidth: nil,
            mediaHeight: nil,
            mediaDurationMs: nil,
            encryptionShield: nil,
            transactionId: nil,
            localEchoState: nil,
            sendFailure: nil,
            sendFailureRetryable: nil
        )
    }
}
