import XCTest
@testable import Synara
import SynaraCore

/// Question cards: Core's projection reaches the Swift model intact, the card
/// settles the way Hermes reads answers, and answers go through Core's typed
/// route (Core parses the content key and the `❓` fallback; see the Rust
/// `agent_questions` and `agent_question_cards` tests).
final class AgentQuestionCardTests: XCTestCase {
    private func dto(
        source: AgentQuestionSourceDto = .structured,
        state: AgentQuestionStateDto = .pending,
        answer: String? = nil,
        multiSelect: Bool = false,
        allowSkip: Bool = true,
        canAnswer: Bool = true,
        expiresAtMs: UInt64? = nil,
        ownChoiceIndex: UInt32? = nil,
        ownSkipped: Bool = false
    ) -> TimelineViewAgentQuestionDto {
        TimelineViewAgentQuestionDto(
            source: source,
            clarifyId: source == .structured ? "clarify-1" : nil,
            question: "Which colour should the cover use?",
            choices: [
                TimelineViewAgentQuestionChoiceDto(index: 1, label: "Red", recommended: true),
                TimelineViewAgentQuestionChoiceDto(index: 2, label: "Blue", recommended: false),
                TimelineViewAgentQuestionChoiceDto(index: 3, label: "Green", recommended: false),
            ],
            multiSelect: multiSelect,
            allowOther: true,
            allowSkip: allowSkip,
            requesterUserId: "@me:example.org",
            expiresAtMs: expiresAtMs,
            state: state,
            answer: answer,
            canAnswer: canAnswer,
            ownChoiceIndex: ownChoiceIndex,
            ownSkipped: ownSkipped
        )
    }

    func testDtoMapsChoicesStateRequesterAndOwnReaction() {
        let question = SynaraAgentQuestion(dto: dto(expiresAtMs: 1_700_000_000_000, ownChoiceIndex: 2))
        XCTAssertEqual(question.source, .structured)
        XCTAssertEqual(question.clarifyID, "clarify-1")
        XCTAssertEqual(question.choices.map(\.label), ["Red", "Blue", "Green"])
        XCTAssertEqual(question.choices.first?.recommended, true)
        XCTAssertEqual(question.requesterUserID, "@me:example.org")
        XCTAssertEqual(question.expiresAt, Date(timeIntervalSince1970: 1_700_000_000))
        XCTAssertEqual(question.state, .pending)
        XCTAssertEqual(question.ownAnswer, .choice(2))
        XCTAssertEqual(SynaraAgentQuestion(dto: dto(ownSkipped: true)).ownAnswer, .skipped)
        XCTAssertEqual(SynaraAgentQuestion(dto: dto(source: .fallback)).source, .fallback)
        XCTAssertFalse(SynaraAgentQuestion(dto: dto(canAnswer: false)).canAnswer)
    }

    func testHermesSettledStateWinsAndReadsAsOneLine() {
        let now = Date()
        let answered = SynaraAgentQuestion(dto: dto(state: .answered, answer: "Blue"))
        let settled = SynaraAgentQuestionPresentation.settle(answered, now: now, localAnswer: .skip)
        XCTAssertEqual(settled, .answered("Blue"))
        XCTAssertEqual(settled.title, "Answered: Blue")
        XCTAssertEqual(
            SynaraAgentQuestionPresentation.settle(SynaraAgentQuestion(dto: dto(state: .skipped)), now: now).title,
            "Skipped"
        )
        XCTAssertEqual(
            SynaraAgentQuestionPresentation.settle(SynaraAgentQuestion(dto: dto(state: .expired)), now: now).title,
            "No answer (expired)"
        )
        XCTAssertEqual(
            SynaraAgentQuestionPresentation.settle(SynaraAgentQuestion(dto: dto(state: .cancelled)), now: now).title,
            "Cancelled"
        )
        XCTAssertNil(SynaraAgentQuestionPresentation.settle(SynaraAgentQuestion(dto: dto()), now: now).title)
    }

    func testOwnAnswersSettleAPendingCard() {
        let now = Date()
        let pending = SynaraAgentQuestion(dto: dto())
        XCTAssertEqual(
            SynaraAgentQuestionPresentation.settle(SynaraAgentQuestion(dto: dto(ownChoiceIndex: 3)), now: now),
            .answered("Green")
        )
        XCTAssertEqual(
            SynaraAgentQuestionPresentation.settle(pending, now: now, localAnswer: .choices([3, 1])),
            .answered("Red, Green")
        )
        XCTAssertEqual(
            SynaraAgentQuestionPresentation.settle(pending, now: now, localAnswer: .other("  Teal ")),
            .answered("Teal")
        )
        XCTAssertEqual(
            SynaraAgentQuestionPresentation.settle(pending, now: now, ownReplyBody: "2"),
            .answered("Blue")
        )
        XCTAssertEqual(
            SynaraAgentQuestionPresentation.settle(pending, now: now, ownReplyBody: "skip"),
            .skipped
        )
        XCTAssertEqual(SynaraAgentQuestionPresentation.settle(pending, now: now), .pending)
    }

    func testExpiryIsFoldedInForPendingCards() {
        let expiring = SynaraAgentQuestion(dto: dto(expiresAtMs: 1_000))
        XCTAssertEqual(
            SynaraAgentQuestionPresentation.settle(expiring, now: Date(timeIntervalSince1970: 0.999)),
            .pending
        )
        XCTAssertEqual(
            SynaraAgentQuestionPresentation.settle(expiring, now: Date(timeIntervalSince1970: 1)),
            .expired
        )
    }

    func testRepliesReadLikeHermesReadsThem() {
        let question = SynaraAgentQuestion(dto: dto())
        XCTAssertEqual(SynaraAgentQuestionPresentation.describeReply("1, 3", for: question), .answered("Red, Green"))
        XCTAssertEqual(SynaraAgentQuestionPresentation.describeReply("9", for: question), .answered("9"))
        XCTAssertEqual(
            SynaraAgentQuestionPresentation.describeReply("Something else", for: question),
            .answered("Something else")
        )
        XCTAssertEqual(
            SynaraAgentQuestionPresentation.describeReply("skip", for: SynaraAgentQuestion(dto: dto(allowSkip: false))),
            .answered("skip")
        )
        XCTAssertEqual(SynaraAgentQuestionPresentation.describeReply("  ", for: question), .pending)
    }

    func testMultiSelectToggleStaysSortedAndUnique() {
        XCTAssertEqual(SynaraAgentQuestionPresentation.toggle(3, in: []), [3])
        XCTAssertEqual(SynaraAgentQuestionPresentation.toggle(1, in: [3]), [1, 3])
        XCTAssertEqual(SynaraAgentQuestionPresentation.toggle(3, in: [1, 3]), [1])
    }

    func testAnswersMapToTheTypedCoreRoute() {
        XCTAssertEqual(SynaraAgentQuestionAnswer.choice(2).dto, .choice(index: 2))
        XCTAssertEqual(SynaraAgentQuestionAnswer.choices([1, 3]).dto, .choices(indices: [1, 3]))
        XCTAssertEqual(SynaraAgentQuestionAnswer.other("Teal").dto, .other(text: "Teal"))
        XCTAssertEqual(SynaraAgentQuestionAnswer.skip.dto, .skip)
    }

    func testOwnRepliesAreKeyedByCardOnlyWhenACardIsLoaded() {
        var card = item(id: "$card", sender: "@hermes:example.org", body: "❓ Pick")
        card.agentQuestion = SynaraAgentQuestion(dto: dto(source: .fallback))
        let own = item(id: "$own", sender: "@me:example.org", body: "2", replyTo: "$card")
        let other = item(id: "$other", sender: "@bob:example.org", body: "1", replyTo: "$card")
        XCTAssertEqual(
            SynaraAgentQuestionPresentation.ownReplies(in: [card, other, own], currentUserID: "@me:example.org"),
            ["$card": "2"]
        )
        XCTAssertEqual(
            SynaraAgentQuestionPresentation.ownReplies(in: [own], currentUserID: "@me:example.org"),
            [:]
        )
    }

    func testMockServiceRecordsAnswersAndReportsFailures() async throws {
        let service = MockAgentQuestionService()
        let outcome = try await service.answer(.choice(1), roomID: "!r:x", eventID: "$card")
        XCTAssertEqual(outcome, .sent)
        XCTAssertEqual(service.submitted.first?.answer, .choice(1))
        let failing = MockAgentQuestionService(error: .failed)
        do {
            _ = try await failing.answer(.skip, roomID: "!r:x", eventID: "$card")
            XCTFail("expected failure")
        } catch {
            XCTAssertEqual(error as? SynaraAgentQuestionError, .failed)
            XCTAssertEqual(
                (error as? SynaraAgentQuestionError)?.errorDescription,
                "Your answer could not be sent. Try again."
            )
        }
    }

    private func item(id: String, sender: String, body: String, replyTo: String? = nil) -> TimelineItem {
        TimelineItem(
            id: id,
            eventID: id,
            senderID: sender,
            senderProfileDisplayName: nil,
            senderAvatarURL: nil,
            timestamp: Date(timeIntervalSince1970: 1_700_000_000),
            kind: .text(body),
            replyToEventID: replyTo,
            isEdited: false,
            reactions: [:]
        )
    }
}
