import XCTest
@testable import Synara
import SynaraCore

/// Slash commands and typed mentions in the composer. Core parses, filters
/// and runs commands and resolves "@Name" (see the Rust `composer` tests);
/// these cover what Swift decides: when to ask Core, how a pick lands in the
/// text, what the composer does with each answer, and how a typed mention
/// reaches the send.
final class ComposerCommandsTests: XCTestCase {
    private let topic = SynaraCommandSuggestion(
        name: "topic",
        usage: "/topic [topic]",
        summary: "Shows or sets the room topic."
    )

    // MARK: Suggestion query

    func testSuggestionQueryIsTheFirstWordAfterTheSlash() {
        XCTAssertEqual(ComposerCommandInput.suggestionQuery(in: "/to", cursor: 3), "to")
        XCTAssertEqual(ComposerCommandInput.suggestionQuery(in: "/to", cursor: 1), "to")
        XCTAssertEqual(ComposerCommandInput.suggestionQuery(in: "/", cursor: 1), "")
    }

    func testSuggestionQueryEndsOnceASpaceIsTyped() {
        XCTAssertNil(ComposerCommandInput.suggestionQuery(in: "/topic ", cursor: 7))
        XCTAssertNil(ComposerCommandInput.suggestionQuery(in: "/topic Launch", cursor: 3))
        XCTAssertNil(ComposerCommandInput.suggestionQuery(in: "/topic\nx", cursor: 3))
    }

    func testSuggestionQueryIgnoresOtherText() {
        XCTAssertNil(ComposerCommandInput.suggestionQuery(in: "hello", cursor: 5))
        XCTAssertNil(ComposerCommandInput.suggestionQuery(in: "//slash", cursor: 3), "//text is a message")
        XCTAssertNil(ComposerCommandInput.suggestionQuery(in: "/to", cursor: 0))
        XCTAssertNil(ComposerCommandInput.suggestionQuery(in: "/to", cursor: 3, selectionLength: 2))
        XCTAssertNil(ComposerCommandInput.suggestionQuery(in: "", cursor: 0))
    }

    func testOnlySlashTextIsSentToCoreAsACommand() {
        XCTAssertTrue(ComposerCommandInput.isCommand("/topic Launch"))
        XCTAssertTrue(ComposerCommandInput.isCommand("  /me waves"))
        XCTAssertTrue(ComposerCommandInput.isCommand("//not a command"), "Core turns //text into /text")
        XCTAssertFalse(ComposerCommandInput.isCommand("hello /topic"))
        XCTAssertFalse(ComposerCommandInput.isCommand(""))
    }

    // MARK: Suggestion insertion

    func testPickingASuggestionReplacesThePrefixWithTheNameAndOneSpace() {
        let result = ComposerCommandInput.insert(topic, in: "/to")
        XCTAssertEqual(result.text, "/topic ")
        XCTAssertEqual(result.cursor, 7)
    }

    func testPickingASuggestionForABareSlash() {
        let result = ComposerCommandInput.insert(topic, in: "/")
        XCTAssertEqual(result.text, "/topic ")
        XCTAssertEqual(result.cursor, 7)
    }

    func testPickingASuggestionKeepsTextAfterTheFirstWord() {
        let result = ComposerCommandInput.insert(topic, in: "/to Launch day")
        XCTAssertEqual(result.text, "/topic Launch day")
        XCTAssertEqual(result.cursor, 7)
    }

    // MARK: Outcomes

    func testDoneShowsANoticeAndClearsTheComposer() {
        let action = ComposerCommandFlow.action(
            for: .done("Topic changed."),
            text: "/topic Launch",
            gifUnavailableMessage: nil
        )
        XCTAssertEqual(action, .notice("Topic changed."))
        XCTAssertTrue(action.clearsComposerNow)
    }

    func testConfirmationKeepsTheCommandToRunAgain() {
        let action = ComposerCommandFlow.action(
            for: .needsConfirmation(title: "Ban @x:y?", message: "They cannot rejoin.", confirmLabel: "Ban"),
            text: "/ban @x:y",
            gifUnavailableMessage: nil
        )
        guard case let .confirm(confirmation) = action else {
            return XCTFail("expected a confirmation, got \(action)")
        }
        XCTAssertEqual(confirmation.title, "Ban @x:y?")
        XCTAssertEqual(confirmation.message, "They cannot rejoin.")
        XCTAssertEqual(confirmation.confirmLabel, "Ban")
        XCTAssertEqual(confirmation.text, "/ban @x:y")
        XCTAssertFalse(action.clearsComposerNow)
    }

    func testSendsLeaveTheTextUntilTheSendIsAccepted() {
        let message = SynaraCommandMessage(msgType: .emote, text: "waves", body: "waves")
        let send = ComposerCommandFlow.action(for: .sendMessage(message), text: "/me waves", gifUnavailableMessage: nil)
        XCTAssertEqual(send, .sendMessage(message))
        XCTAssertFalse(send.clearsComposerNow)

        let poll = SynaraCommandPoll(question: "Lunch?", answers: ["Yes", "No"], maxSelections: 1)
        let pollAction = ComposerCommandFlow.action(for: .sendPoll(poll), text: "/poll", gifUnavailableMessage: nil)
        XCTAssertEqual(pollAction, .sendPoll(poll))
        XCTAssertFalse(pollAction.clearsComposerNow)
    }

    func testGifPickerOpensOnlyWhenAvailable() {
        XCTAssertEqual(
            ComposerCommandFlow.action(for: .openGifPicker(query: "cats"), text: "/gif cats", gifUnavailableMessage: nil),
            .openGifPicker(query: "cats")
        )
        let unavailable = ComposerCommandFlow.action(
            for: .openGifPicker(query: "cats"),
            text: "/gif cats",
            gifUnavailableMessage: "GIF search is not set up for this app."
        )
        XCTAssertEqual(unavailable, .error("GIF search is not set up for this app."))
        XCTAssertFalse(unavailable.clearsComposerNow)
    }

    func testNavigationProfileAndHelpClearTheComposer() {
        let navigate = ComposerCommandFlow.action(
            for: .navigate(roomID: "!room:x", eventID: "$event"),
            text: "/join #room:x",
            gifUnavailableMessage: nil
        )
        XCTAssertEqual(navigate, .navigate(roomID: "!room:x", eventID: "$event"))
        XCTAssertTrue(navigate.clearsComposerNow)

        let profile = ComposerCommandFlow.action(for: .openProfile(userID: "@x:y"), text: "/whois @x:y", gifUnavailableMessage: nil)
        XCTAssertEqual(profile, .openProfile(userID: "@x:y"))
        XCTAssertTrue(profile.clearsComposerNow)

        let help = ComposerCommandFlow.action(for: .help([topic]), text: "/help", gifUnavailableMessage: nil)
        XCTAssertEqual(help, .help([topic]))
        XCTAssertTrue(help.clearsComposerNow)
    }

    func testCoreOutcomesMapToSwift() {
        let message = ComposerMessage(
            msgtype: .notice,
            text: "hello",
            prefix: nil,
            body: "hello",
            formattedBody: nil,
            allowFormatting: true
        )
        XCTAssertEqual(
            SynaraCommandOutcome(dto: .sendMessage(message: message)),
            .sendMessage(SynaraCommandMessage(msgType: .notice, text: "hello", body: "hello"))
        )
        XCTAssertEqual(
            SynaraCommandOutcome(dto: .navigate(roomId: "!r:x", eventId: nil)),
            .navigate(roomID: "!r:x", eventID: nil)
        )
        XCTAssertEqual(
            SynaraCommandOutcome(dto: .needsConfirmation(title: "T", message: "M", confirmLabel: "Ban")),
            .needsConfirmation(title: "T", message: "M", confirmLabel: "Ban")
        )
        XCTAssertEqual(
            SynaraCommandOutcome(dto: .sendPoll(poll: ComposerPoll(question: "Q", answers: ["A", "B"], maxSelections: 2))),
            .sendPoll(SynaraCommandPoll(question: "Q", answers: ["A", "B"], maxSelections: 2))
        )
    }

    // MARK: Outgoing command messages

    func testCoreBuiltHTMLIsSentAsIs() {
        let message = SynaraCommandMessage(
            text: "secret",
            body: "secret",
            formattedBody: "<span data-mx-spoiler>secret</span>",
            allowFormatting: false
        )
        let outgoing = ComposerOutgoingText.command(message, mentions: [], currentUserID: "@me:x")
        XCTAssertEqual(outgoing.body, "secret")
        XCTAssertEqual(outgoing.formattedBody, "<span data-mx-spoiler>secret</span>")
        XCTAssertNil(outgoing.msgType)
    }

    func testPlainCommandIsNotFormatted() {
        let message = SynaraCommandMessage(text: "**not bold**", body: "**not bold**", allowFormatting: false)
        let outgoing = ComposerOutgoingText.command(message, mentions: [], currentUserID: "@me:x")
        XCTAssertEqual(outgoing.body, "**not bold**")
        XCTAssertNil(outgoing.formattedBody)
    }

    func testEmoteAndNoticeCarryTheirMsgType() {
        let emote = SynaraCommandMessage(msgType: .emote, text: "waves", body: "waves")
        XCTAssertEqual(ComposerOutgoingText.command(emote, mentions: [], currentUserID: "").msgType, "m.emote")
        let notice = SynaraCommandMessage(msgType: .notice, text: "note", body: "note")
        XCTAssertEqual(ComposerOutgoingText.command(notice, mentions: [], currentUserID: "").msgType, "m.notice")
        XCTAssertNil(ComposerOutgoingText.message("hi", mentions: [], currentUserID: "").msgType)
    }

    func testPrefixGoesBeforeTheFormattedTextUnformatted() throws {
        let shrug = #"¯\_(ツ)_/¯"#
        let message = SynaraCommandMessage(text: "**fine**", prefix: shrug, body: shrug + " **fine**")
        let outgoing = ComposerOutgoingText.command(message, mentions: [], currentUserID: "")
        XCTAssertTrue(outgoing.body.hasPrefix(shrug + " "), outgoing.body)
        let html = try XCTUnwrap(outgoing.formattedBody)
        XCTAssertTrue(html.hasPrefix(shrug + " "), html)
        XCTAssertTrue(html.contains("<strong>fine</strong>"), html)
    }

    func testPrefixWithoutTextSendsCoresBody() {
        let flip = "(╯°□°)╯︵ ┻━┻"
        let message = SynaraCommandMessage(text: "", prefix: flip, body: flip)
        let outgoing = ComposerOutgoingText.command(message, mentions: [], currentUserID: "")
        XCTAssertEqual(outgoing.body, flip)
        XCTAssertNil(outgoing.formattedBody)
    }

    func testPrefixIsEscapedInHTML() {
        XCTAssertEqual(ComposerOutgoingText.escapeHTML(#"<b>&"x""#), "&lt;b&gt;&amp;&quot;x&quot;")
    }

    func testCommandMessageMentionsAreLinked() throws {
        let mention = ComposerMention(userID: "@ada:x", displayName: "Ada")
        let message = SynaraCommandMessage(msgType: .emote, text: "waves at @Ada", body: "waves at @Ada")
        let outgoing = ComposerOutgoingText.command(message, mentions: [mention], currentUserID: "@me:x")
        XCTAssertEqual(outgoing.mentionUserIDs, ["@ada:x"])
        XCTAssertTrue(try XCTUnwrap(outgoing.formattedBody).contains("https://matrix.to/#/@ada:x"))
    }

    // MARK: Typed mentions

    func testTypedMentionsJoinPickedMentionsUnderTheTypedText() {
        let picked = [ComposerMention(userID: "@bob:x", displayName: "Bob")]
        let typed = [
            SynaraTypedMention(start: 3, end: 16, userID: "@ada:x", text: "@ada lovelace"),
            SynaraTypedMention(start: 21, end: 25, userID: "@bob:x", text: "@Bob"),
        ]
        let merged = ComposerTypedMentions.merge(typed, into: picked)
        XCTAssertEqual(merged, [
            ComposerMention(userID: "@bob:x", displayName: "Bob"),
            ComposerMention(userID: "@ada:x", displayName: "@ada lovelace"),
        ])
        XCTAssertEqual(merged[1].label, "@ada lovelace", "the label is exactly the typed text")

        let resolved = ComposerMentions.resolve("hi @ada lovelace and @Bob", mentions: merged, currentUserID: "@me:x")
        XCTAssertEqual(
            resolved.markdown,
            "hi [@ada lovelace](https://matrix.to/#/@ada:x) and [@Bob](https://matrix.to/#/@bob:x)"
        )
        XCTAssertEqual(resolved.userIDs, ["@ada:x", "@bob:x"])
    }

    func testTypedMentionsFromAStubResolverReachTheSend() {
        let stub = StubMentionResolver(result: [
            SynaraTypedMention(start: 0, end: 4, userID: "@ada:x", text: "@Ada"),
        ])
        let members = [RoomMemberSummary(userID: "@ada:x", displayName: "Ada", membership: .join, powerLevel: 0)]
        let typed = stub.resolveMentions(text: "@Ada look", members: members, ownUserID: "@me:x")
        let merged = ComposerTypedMentions.merge(typed, into: [])
        let outgoing = ComposerOutgoingText.message("@Ada look", mentions: merged, currentUserID: "@me:x")
        XCTAssertEqual(outgoing.mentionUserIDs, ["@ada:x"])
        XCTAssertEqual(stub.lastText, "@Ada look")
    }

    func testCandidatesAreJoinedAndInvitedMembers() {
        let members = [
            RoomMemberSummary(userID: "@a:x", displayName: "Ada", membership: .join, powerLevel: 0),
            RoomMemberSummary(userID: "@b:x", displayName: "  ", membership: .invite, powerLevel: 0),
            RoomMemberSummary(userID: "@c:x", displayName: "Cy", membership: .leave, powerLevel: 0),
        ]
        XCTAssertEqual(ComposerTypedMentions.candidates(from: members), [
            ComposerTypedMentions.Candidate(userID: "@a:x", displayName: "Ada"),
            ComposerTypedMentions.Candidate(userID: "@b:x", displayName: nil),
        ])
    }

    func testTypedTextThatIsNotAMentionIsDropped() {
        let typed = [SynaraTypedMention(start: 0, end: 1, userID: "@a:x", text: "@")]
        XCTAssertEqual(ComposerTypedMentions.merge(typed, into: []), [])
    }

    // MARK: Send plumbing

    func testQueuedCommandMessageKeepsItsMsgType() {
        let queued = OutgoingQueuedMessage(
            id: "$pending-1",
            roomID: "!r:x",
            body: "waves",
            formattedBody: nil,
            replyToEventID: nil,
            threadRootEventID: nil,
            senderID: "@me:x",
            timestamp: Date(),
            deliveryStatus: .queued,
            msgType: "m.emote"
        )
        XCTAssertEqual(queued.sendRequest().msgType, "m.emote")
    }

    // MARK: UI-test fixture

    func testFixtureAsksBeforeABanAndAnswersLikeCore() async throws {
        let service = MockComposerCommandService.uiTestFixture()
        let suggestions = await service.suggestions(roomID: "!r:x", prefix: "to")
        XCTAssertEqual(suggestions.map(\.name), ["topic"])

        let first = try await service.run(roomID: "!r:x", text: "/ban @x:y", confirmed: false)
        guard case .needsConfirmation = first else {
            return XCTFail("expected a confirmation, got \(first)")
        }
        let second = try await service.run(roomID: "!r:x", text: "/ban @x:y", confirmed: true)
        XCTAssertEqual(second, .done("Banned @x:y."))

        do {
            _ = try await service.run(roomID: "!r:x", text: "/nope", confirmed: false)
            XCTFail("an unknown command throws")
        } catch let error as SynaraCommandError {
            XCTAssertEqual(error.reason, .unknownCommand)
            XCTAssertEqual(error.localizedDescription, "Synara does not know the command /nope.")
        }
    }

    func testDefaultServiceSendsTextAsTyped() async throws {
        let service = MockComposerCommandService()
        let outcome = try await service.run(roomID: "!r:x", text: "/anything here", confirmed: false)
        XCTAssertEqual(outcome, .sendMessage(SynaraCommandMessage(text: "/anything here", body: "/anything here")))
    }
}

private final class StubMentionResolver: ComposerCommandServicing {
    let result: [SynaraTypedMention]
    private(set) var lastText: String?

    init(result: [SynaraTypedMention]) {
        self.result = result
    }

    func suggestions(roomID: String, prefix: String) async -> [SynaraCommandSuggestion] {
        []
    }

    func run(roomID: String, text: String, confirmed: Bool) async throws -> SynaraCommandOutcome {
        .sendMessage(SynaraCommandMessage(text: text, body: text))
    }

    func resolveMentions(text: String, members: [RoomMemberSummary], ownUserID: String) -> [SynaraTypedMention] {
        lastText = text
        return result
    }

    func sendPoll(
        _ poll: SynaraCommandPoll,
        roomID: String,
        threadRootEventID: String?,
        replyToEventID: String?
    ) async throws {}
}
