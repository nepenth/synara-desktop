import UIKit
import XCTest
@testable import Synara

/// Timeline polish (review batch B3b): typing cadence, plain-text links,
/// reaction toggles, the DM glyph, cached formatters, the delivery-status
/// lane and VoiceOver order in the inverted table.
final class TimelinePolishTests: XCTestCase {
    // MARK: Typing cadence (iOS twin of desktop D79)

    func testTypingStartsOnceAndRefreshesEveryHalfSecondWhileActive() {
        let scheduler = ManualTypingScheduler()
        var sent: [Bool] = []
        let notifier = ComposerTypingNotifier(scheduler: scheduler) { sent.append($0) }

        notifier.update(isTyping: true)
        notifier.update(isTyping: true)
        XCTAssertEqual(sent, [true], "later keystrokes do not send on their own")

        scheduler.advance(by: 2)
        XCTAssertEqual(sent, Array(repeating: true, count: 5), "a 500 ms refresh keeps the SDK's 3-4 s window covered")
        XCTAssertTrue(notifier.isTyping)
    }

    func testTypingStopsOnceAfterFiveSecondsIdleAndKeystrokesPushTheStopBack() {
        let scheduler = ManualTypingScheduler()
        var sent: [Bool] = []
        let notifier = ComposerTypingNotifier(scheduler: scheduler) { sent.append($0) }

        notifier.update(isTyping: true)
        scheduler.advance(by: 4)
        notifier.update(isTyping: true)
        scheduler.advance(by: 4)
        XCTAssertFalse(sent.contains(false), "a keystroke resets the 5 s idle timer")

        scheduler.advance(by: 1.1)
        XCTAssertEqual(sent.last, false)
        XCTAssertEqual(sent.filter { $0 == false }.count, 1)
        XCTAssertFalse(notifier.isTyping)

        let countAfterStop = sent.count
        scheduler.advance(by: 10)
        XCTAssertEqual(sent.count, countAfterStop, "no timer outlives the stop")
        XCTAssertEqual(scheduler.activeTimerCount, 0)
    }

    func testTypingFalseGoesOutOnceOnClearSendOrDisappear() {
        let scheduler = ManualTypingScheduler()
        var sent: [Bool] = []
        let notifier = ComposerTypingNotifier(scheduler: scheduler) { sent.append($0) }

        notifier.stop()
        notifier.update(isTyping: false)
        XCTAssertEqual(sent, [], "nothing to stop before typing starts")

        notifier.update(isTyping: true)
        notifier.update(isTyping: false)
        notifier.stop()
        XCTAssertEqual(sent, [true, false])
        XCTAssertEqual(scheduler.activeTimerCount, 0)
    }

    func testReleasingTheNotifierCancelsItsTimers() {
        let scheduler = ManualTypingScheduler()
        var notifier: ComposerTypingNotifier? = ComposerTypingNotifier(scheduler: scheduler) { _ in }
        notifier?.update(isTyping: true)
        XCTAssertEqual(scheduler.activeTimerCount, 2)
        notifier = nil
        XCTAssertEqual(scheduler.activeTimerCount, 0)
    }

    func testSerialSenderKeepsOrderAndDropsQueuedRefreshes() async {
        let recorder = TypingRecorder()
        let send = ComposerTypingNotifier.serialSender { typing in
            await recorder.record(typing)
        }
        await MainActor.run {
            send(true)
            send(true)
            send(true)
            send(false)
        }
        try? await Task.sleep(nanoseconds: 200_000_000)
        let delivered = await recorder.values
        XCTAssertEqual(delivered, [true, false], "a refresh queued behind a waiting true is dropped; false lands last")
    }

    // MARK: Composer text after a send (I19 follow-up)

    func testTextTypedDuringAnUploadSurvivesTheUploadFinishing() {
        // Send "Caption" with an attachment; the keyboard stays up and the
        // user goes on typing while the upload runs.
        let sentAtTap = "Caption"
        var composer = sentAtTap
        composer += " and a follow-up"

        // The upload finishes and the send clears what it sent.
        composer = ComposerSentDraft.remainingText(current: composer, sent: sentAtTap)
        XCTAssertEqual(composer, "and a follow-up", "new text survives; only the sent caption goes")
    }

    func testFinishedSendClearsOnlyWhatItSent() {
        XCTAssertEqual(ComposerSentDraft.remainingText(current: "hello", sent: "hello"), "", "untouched: cleared")
        XCTAssertEqual(ComposerSentDraft.remainingText(current: "", sent: "hello"), "")
        XCTAssertEqual(
            ComposerSentDraft.remainingText(current: "next message", sent: "hello"),
            "next message",
            "replaced while sending: kept whole"
        )
        XCTAssertEqual(
            ComposerSentDraft.remainingText(current: "hello\nsecond line", sent: "hello"),
            "second line"
        )
        XCTAssertEqual(
            ComposerSentDraft.remainingText(current: "typed during upload", sent: ""),
            "typed during upload",
            "an attachment without a caption sent no text"
        )
        XCTAssertNotEqual(
            ComposerSentDraft(id: 1, sentText: "ok"),
            ComposerSentDraft(id: 2, sentText: "ok"),
            "sending the same text twice is two completions"
        )
    }

    // MARK: Plain-text links (V9)

    func testBareURLsAndEmailsInPlainBodiesBecomeLinks() {
        let body = "See https://matrix.org/docs/ or example.com, mail ada@example.org."
        let links = SynaraPlainTextLinks.links(in: body)
        XCTAssertEqual(links.map(\.url.absoluteString), [
            "https://matrix.org/docs/",
            "http://example.com",
            "mailto:ada@example.org",
        ])

        let attributed = SynaraPlainTextLinks.attributedText(body)
        XCTAssertEqual(String(attributed.characters), body, "linkifying never changes the text")
        let linkedRuns = attributed.runs.compactMap { run -> String? in
            run.link == nil ? nil : String(attributed[run.range].characters)
        }
        XCTAssertEqual(linkedRuns, ["https://matrix.org/docs/", "example.com", "ada@example.org"])
    }

    func testPlainBodiesWithoutLinksStayUnlinked() {
        let attributed = SynaraPlainTextLinks.attributedText("No links here, just words.")
        XCTAssertTrue(attributed.runs.allSatisfy { $0.link == nil })
        XCTAssertTrue(SynaraPlainTextLinks.links(in: "").isEmpty)
    }

    func testOnlyWebAndMailLinksAreAllowed() {
        XCTAssertEqual(SynaraPlainTextLinks.allowedSchemes, ["http", "https", "mailto"])
        XCTAssertTrue(SynaraPlainTextLinks.links(in: "call +1 555 0100 or open file:///etc/passwd").allSatisfy {
            SynaraPlainTextLinks.allowedSchemes.contains($0.url.scheme ?? "")
        })
    }

    func testLongBareLinksShowAsSiteAndPathButOpenTheWholeAddress() throws {
        let yahoo = "https://www.yahoo.com/entertainment/celebrity/article/patrick-stewart-star-of-x-men-and-star-trek-dies-at-86-195431969.html?guccounter=1&guce_referrer=aHR0cHM6Ly93d3cuZ29vZ2xlLmNvbS8"
        let body = "I got some terrible news guys...\(yahoo) so sad"
        let attributed = SynaraPlainTextLinks.attributedText(body, shortensLinks: true)
        XCTAssertEqual(
            String(attributed.characters),
            "I got some terrible news guys...yahoo.com/…/patrick-stewart-star-of-x-men-and-… so sad"
        )
        let linked = try XCTUnwrap(attributed.runs.first { $0.link != nil })
        XCTAssertEqual(linked.link?.absoluteString, yahoo)
        XCTAssertEqual(
            String(SynaraPlainTextLinks.attributedText(body).characters),
            body,
            "with the setting off the address stays whole"
        )

        func label(_ text: String, _ href: String? = nil) -> String? {
            SynaraShortLinks.label(text: text, url: URL(string: href ?? text)!)
        }
        XCTAssertEqual(label("https://example.com/news/story?utm_source=feed&utm_medium=rss&utm_campaign=x"), "example.com/news/story…")
        XCTAssertEqual(label("https://docs.example.org/guide/install/linux/arch/packages/troubleshooting"), "docs.example.org/…/troubleshooting")
        XCTAssertEqual(
            label("www.example.com/a/b?session=0123456789abcdef0123456789abcdef", "http://www.example.com/a/b?session=0123456789abcdef0123456789abcdef"),
            "example.com/a/b…"
        )
    }

    func testShortLinksNeverCutTheSiteOrRelabelOtherLinks() {
        func label(_ text: String, _ href: String? = nil) -> String? {
            SynaraShortLinks.label(text: text, url: URL(string: href ?? text)!)
        }
        XCTAssertEqual(
            label("https://paypal.com.account-verify.secure-login.example-phishing-host.test/x?token=1"),
            "paypal.com.account-verify.secure-login.example-phishing-host.test/…"
        )
        let host = String(repeating: "a", count: 60) + ".test"
        XCTAssertEqual(label("https://\(host)/one/two/three"), "\(host)/…")
        XCTAssertNil(label("https://example.com/a?b=c"))
        let long = "https://example.com/a/very/long/path/that/goes/on/and/on/for/a/while"
        XCTAssertNil(label("Read the full article here please, it is long", long))
        XCTAssertNil(label(long, "https://example.com/other"))
        XCTAssertNil(label("https://user:secret@example.com/a/very/long/path/that/goes/on/and/on"))
        XCTAssertNil(label("mailto:someone.with.a.very.long.address@example-company-domain.test"))
    }

    func testMentionPillsComeFromMatrixToUserLinks() {
        func user(_ link: String) -> String? {
            URL(string: link).flatMap(TimelineMentionPolicy.userID(inPermalink:))
        }
        XCTAssertEqual(user("https://matrix.to/#/@ada:example.org"), "@ada:example.org")
        XCTAssertEqual(user("https://matrix.to/#/%40ada%3Aexample.org"), "@ada:example.org")
        XCTAssertEqual(user("https://matrix.to/#/@ada:example.org?via=example.org"), "@ada:example.org")
        XCTAssertNil(user("https://matrix.to/#/!room:example.org"))
        XCTAssertNil(user("https://matrix.to/#/!room:example.org/$event"))
        XCTAssertNil(user("https://example.org/#/@ada:example.org"))
        XCTAssertEqual(TimelineMentionPolicy.pillLabel("Ada"), "\u{200A}@Ada\u{200A}")
        XCTAssertEqual(TimelineMentionPolicy.pillLabel("@Ada"), "\u{200A}@Ada\u{200A}")
    }

    func testComposerMentionQueryNeedsTheAtToStartAWord() {
        XCTAssertEqual(ComposerMentions.query(in: "hi @al", cursor: 6), .init(location: 3, text: "al"))
        XCTAssertEqual(ComposerMentions.query(in: "@", cursor: 1), .init(location: 0, text: ""))
        XCTAssertNil(ComposerMentions.query(in: "mail@al", cursor: 7))
        XCTAssertNil(ComposerMentions.query(in: "@al ex", cursor: 6))
        XCTAssertNil(ComposerMentions.query(in: "hi", cursor: 2))
    }

    func testComposerMentionSuggestionsRankNamesAndSkipYouAndLeavers() {
        let members = [
            RoomMemberSummary(userID: "@me:example.org", displayName: "Alma", membership: .join, powerLevel: 0),
            RoomMemberSummary(userID: "@alex:example.org", displayName: "Alex Kim", membership: .join, powerLevel: 0),
            RoomMemberSummary(userID: "@sam:example.org", displayName: "Sam Alvarez", membership: .invite, powerLevel: 0),
            RoomMemberSummary(userID: "@al:example.org", displayName: "Gone Al", membership: .leave, powerLevel: 0),
        ]
        let picked = ComposerMentions.suggestions(for: "al", members: members, excluding: "@me:example.org")
        XCTAssertEqual(picked.map(\.userID), ["@alex:example.org", "@sam:example.org"])
    }

    func testComposerMentionInsertAndResolveMakeDesktopPills() {
        let alex = ComposerMention(userID: "@alex:example.org", displayName: "Alex Kim")
        let inserted = ComposerMentions.insert(
            alex,
            replacing: .init(location: 3, text: "al"),
            cursor: 6,
            in: "hi @al"
        )
        XCTAssertEqual(inserted.text, "hi @Alex Kim ")
        XCTAssertEqual(inserted.cursor, 13)

        let me = ComposerMention(userID: "@me:example.org", displayName: "Me")
        let resolved = ComposerMentions.resolve(
            "@Alex Kim and @Me, not @Alex Kimberly",
            mentions: [alex, me],
            currentUserID: "@me:example.org"
        )
        XCTAssertEqual(
            resolved.markdown,
            "[@Alex Kim](https://matrix.to/#/@alex:example.org) and [@Me](https://matrix.to/#/@me:example.org), not @Alex Kimberly"
        )
        XCTAssertEqual(resolved.userIDs, ["@alex:example.org"])

        let html = ComposerMatrixFormatting.formattedBody(for: resolved.markdown) ?? ""
        XCTAssertTrue(html.contains("<a href=\"https://matrix.to/#/@alex:example.org\">@Alex Kim</a>"), html)
    }

    func testComposerMentionsComeBackWhenEditingAPilledMessage() {
        let found = ComposerMentions.mentions(
            inHTML: "<a href=\"https://matrix.to/#/@alex:example.org\">@Alex</a> hi"
        )
        XCTAssertEqual(found.map(\.label), ["@Alex", "@alex:example.org"])
        XCTAssertEqual(Set(found.map(\.userID)), ["@alex:example.org"])
    }

    // MARK: Reactions (I21)

    func testReactionPillsToggleOnlyWhenReactingIsAllowedAndOwnershipIsKnown() {
        let canReact = EventActionAvailability(canReply: true, canEdit: false, canRedact: false, canReact: true)
        let cannotReact = EventActionAvailability(canReply: true, canEdit: false, canRedact: false, canReact: false)

        XCTAssertTrue(TimelineReactionTogglePolicy.canToggle(availability: canReact, ownership: .known(["👍"])))
        XCTAssertTrue(TimelineReactionTogglePolicy.canToggle(availability: canReact, ownership: .known([])))
        XCTAssertFalse(TimelineReactionTogglePolicy.canToggle(availability: canReact, ownership: .unknown))
        XCTAssertFalse(TimelineReactionTogglePolicy.canToggle(availability: cannotReact, ownership: .known(["👍"])))
    }

    func testReactionPillLabelsAndHintsNameCountAndOwnership() {
        XCTAssertEqual(TimelineRowAccessibility.reactionLabel(key: "👍", count: 3, isOwn: true), "👍 3, you reacted")
        XCTAssertEqual(TimelineRowAccessibility.reactionLabel(key: "🎉", count: 1, isOwn: false), "🎉 1")
        XCTAssertEqual(TimelineRowAccessibility.reactionToggleHint(isOwn: true), "Removes your reaction")
        XCTAssertEqual(TimelineRowAccessibility.reactionToggleHint(isOwn: false), "Adds your reaction")
    }

    func testOneElementRowsOfferEachReactionAsAToggleAction() {
        let availability = EventActionAvailability(canReply: true, canEdit: false, canRedact: false, canReact: true)
        let actions = TimelineRowAccessibility.actions(
            availability: availability,
            replyCount: 0,
            canCopy: false,
            canPinToNotes: false,
            reactions: ["👍": 3, "🎉": 1],
            ownReactions: .known(["👍"])
        )
        XCTAssertEqual(actions, [
            .reply,
            .react,
            .toggleReaction(key: "🎉", isOwn: false),
            .toggleReaction(key: "👍", isOwn: true),
            .replyInThread,
        ])
        XCTAssertEqual(actions[2].title, "React with 🎉")
        XCTAssertEqual(actions[3].title, "Remove 👍 reaction")
        XCTAssertEqual(Set(actions.map(\.title)).count, actions.count)

        let unknownOwnership = TimelineRowAccessibility.actions(
            availability: availability,
            replyCount: 0,
            canCopy: false,
            canPinToNotes: false,
            reactions: ["👍": 3],
            ownReactions: .unknown
        )
        XCTAssertFalse(unknownOwnership.contains { if case .toggleReaction = $0 { return true } else { return false } })
    }

    // MARK: Quick reactions

    func testQuickReactionsAreTheMostUsedEmojiPaddedWithTheFallback() {
        let recent = [
            SharedCoreRecentEmojiEntry(emoji: "🎉", count: 2),
            SharedCoreRecentEmojiEntry(emoji: "👍", count: 9),
            SharedCoreRecentEmojiEntry(emoji: "🔥", count: 2),
        ]
        XCTAssertEqual(
            TimelineQuickReactions.keys(from: recent),
            ["👍", "🎉", "🔥", "❤️", "😂", "😮"],
            "most-used first, ties in stored (most recent first) order, padded without duplicating 👍"
        )
        XCTAssertEqual(TimelineQuickReactions.keys(from: []), TimelineQuickReactions.fallback)
        XCTAssertEqual(TimelineQuickReactions.fallback, ["👍", "❤️", "😂", "😮", "😢", "🙏"])

        let many = (1 ... 9).map { SharedCoreRecentEmojiEntry(emoji: "e\($0)", count: UInt64($0)) }
        XCTAssertEqual(TimelineQuickReactions.keys(from: many), ["e9", "e8", "e7", "e6", "e5", "e4"])
    }

    @MainActor
    func testRecordingAUseUpdatesTheQuickReactions() async {
        let store = TimelineQuickReactionStore()
        XCTAssertEqual(store.keys, TimelineQuickReactions.fallback, "the fallback shows before the list loads")

        await store.refresh { [SharedCoreRecentEmojiEntry(emoji: "🚀", count: 3)] }
        XCTAssertEqual(store.keys.first, "🚀")

        var recorded: [String] = []
        await store.recordUse("🎯") { emoji in
            recorded.append(emoji)
            return [
                SharedCoreRecentEmojiEntry(emoji: "🎯", count: 4),
                SharedCoreRecentEmojiEntry(emoji: "🚀", count: 3),
            ]
        }
        XCTAssertEqual(recorded, ["🎯"])
        XCTAssertEqual(Array(store.keys.prefix(2)), ["🎯", "🚀"])

        await store.recordUse("👋") { _ in nil }
        XCTAssertEqual(Array(store.keys.prefix(2)), ["🎯", "🚀"], "a failed write keeps the last list")
    }

    func testQuickReactionsBecomeToggleActionsForVoiceOver() {
        let availability = EventActionAvailability(canReply: false, canEdit: false, canRedact: false, canReact: true)
        let actions = TimelineRowAccessibility.actions(
            availability: availability,
            replyCount: 0,
            canCopy: false,
            canPinToNotes: false,
            reactions: ["👍": 2],
            ownReactions: .known(["👍"]),
            quickReactions: ["👍", "❤️"]
        )
        XCTAssertEqual(actions, [
            .toggleReaction(key: "👍", isOwn: true),
            .toggleReaction(key: "❤️", isOwn: false),
        ])
        XCTAssertEqual(actions.map(\.title), ["Remove 👍 reaction", "React with ❤️"])
    }

    // MARK: DM glyph (V3)

    func testDirectRoomsUseThePersonsInitialAndChannelsKeepTheHash() {
        XCTAssertEqual(SynaraRoomGlyph.glyph(for: room(kind: .room, name: "Design Review")), .channel)
        XCTAssertEqual(SynaraRoomGlyph.glyph(for: room(kind: .space, name: "Team")), .channel)
        XCTAssertEqual(SynaraRoomGlyph.glyph(for: nil), .channel)
        XCTAssertEqual(
            SynaraRoomGlyph.glyph(for: room(kind: .directMessage, name: "bob builder", peer: "@bob:example.org")),
            .person(initial: "B")
        )
        XCTAssertEqual(
            SynaraRoomGlyph.glyph(
                for: room(kind: .directMessage, name: "@carol:example.org", peer: "@carol:example.org")
            ),
            .person(initial: "C"),
            "an MXID name skips the sigil"
        )
        XCTAssertEqual(SynaraRoomGlyph.initial(for: "  ", fallbackID: "@dan:example.org"), "D")
        XCTAssertEqual(SynaraRoomGlyph.initial(for: "", fallbackID: nil), "?")
    }

    // MARK: Room-list call status batching (I15)

    func testRoomListAsksAboutEachDirectPeerOnce() {
        let rooms = [
            room(kind: .directMessage, name: "Bob", peer: "@bob:example.org"),
            room(kind: .directMessage, name: "Bob again", peer: "@bob:example.org"),
            room(kind: .directMessage, name: "Ada", peer: "@ada:example.org"),
            room(kind: .directMessage, name: "Live", peer: "@live:example.org", hasActiveCall: true),
            room(kind: .room, name: "Channel", peer: "@nobody:example.org"),
        ]
        XCTAssertEqual(RoomListPeerCallStatus.peerIDs(in: rooms), ["@ada:example.org", "@bob:example.org"])
        XCTAssertTrue(RoomListPeerCallStatus.isInCall(rooms[0], peersInCall: ["@bob:example.org"]))
        XCTAssertFalse(RoomListPeerCallStatus.isInCall(rooms[3], peersInCall: ["@live:example.org"]))
        XCTAssertFalse(RoomListPeerCallStatus.isInCall(rooms[4], peersInCall: ["@nobody:example.org"]))
    }

    // MARK: Formatter cache (I15)

    func testClockFormatterIsReusedUntilTheTwentyFourHourSettingChanges() throws {
        let utc = try XCTUnwrap(TimeZone(identifier: "UTC"))
        let locale = Locale(identifier: "en_US")
        let twelveHour = SynaraTimeFormatting.Configuration(uses24HourClock: false, locale: locale, timeZone: utc)
        let twentyFourHour = SynaraTimeFormatting.Configuration(uses24HourClock: true, locale: locale, timeZone: utc)
        let date = Date(timeIntervalSince1970: 1_767_272_700) // 2026-01-01 13:05 UTC

        let first = SynaraTimeFormatting.formatters(for: twelveHour)
        XCTAssertTrue(
            first === SynaraTimeFormatting.formatters(for: twelveHour),
            "unchanged settings reuse the formatters"
        )
        XCTAssertTrue(first.clockTime.format(date).contains("PM"))

        let switched = SynaraTimeFormatting.formatters(for: twentyFourHour)
        XCTAssertFalse(first === switched, "the 24-hour setting rebuilds them")
        XCTAssertEqual(switched.clockTime.format(date), "13:05")
        XCTAssertEqual(switched.weekday.format(date), "Thu")
        XCTAssertEqual(switched.monthDay.format(date), "Jan 1")
        XCTAssertEqual(switched.monthDayYear.format(date), "Jan 1, 2026")

        let back = SynaraTimeFormatting.formatters(for: twelveHour)
        XCTAssertFalse(back === switched)
        XCTAssertFalse(back.clockTime.format(date).contains("13"))
    }

    func testClockTimeFollowsTheStoredSetting() throws {
        // Settings writes the app group when it exists, else standard defaults.
        let defaults = SynaraSharedConstants.appGroupDefaults() ?? .standard
        let key = SynaraSharedConstants.hour24ClockKey
        let previous = defaults.object(forKey: key)
        defer {
            defaults.set(previous, forKey: key)
        }

        defaults.set(true, forKey: key)
        XCTAssertTrue(SynaraTimeFormatting.Configuration.current.uses24HourClock)
        defaults.set(false, forKey: key)
        XCTAssertFalse(SynaraTimeFormatting.Configuration.current.uses24HourClock)
    }

    // MARK: Delivery-status lane (I25)

    func testDeliveryStatusNeverSharesSpaceWithText() {
        func lane(
            _ status: TimelineDeliveryStatus?,
            _ alignment: SynaraMessageBubbleAlignment = .own,
            shield: Bool = false
        ) -> SynaraMessageBubbleStatusLane {
            SynaraMessageBubbleMetrics.statusLane(
                deliveryStatus: status,
                alignment: alignment,
                hasEncryptionShield: shield
            )
        }

        for status in [TimelineDeliveryStatus.sending, .queued, .sent] {
            XCTAssertEqual(lane(status), .compactTrailing)
            XCTAssertTrue(lane(status).reservesTrailingLane)
        }
        XCTAssertEqual(lane(.failed), .failedRow)
        XCTAssertTrue(lane(.failed).showsFailedRow)
        XCTAssertFalse(lane(.failed).reservesTrailingLane, "Retry and Discard sit under the text, not beside it")
        XCTAssertEqual(lane(.failed, shield: true), .failedRowWithShield)
        XCTAssertTrue(lane(.failed, shield: true).reservesTrailingLane)
        XCTAssertEqual(lane(nil), .none)
        XCTAssertEqual(lane(.sending, .other), .none, "only own messages show delivery status")
        XCTAssertEqual(lane(nil, shield: true), .shieldOnly)

        // Indicator: 4 pt in from the edge, 16 pt wide, 4 pt gap; bubble text is 8 pt in.
        XCTAssertEqual(SynaraMessageBubbleMetrics.trailingLaneInset(contentPadding: SynaraSpacing.small), 16)
        XCTAssertEqual(SynaraMessageBubbleMetrics.trailingLaneInset(contentPadding: 0), 24)
    }

    // MARK: VoiceOver order (I16)

    func testInvertedTimelineListsCellsOldestFirstForVoiceOver() throws {
        let window = UIWindow(frame: CGRect(x: 0, y: 0, width: 320, height: 480))
        let table = StableTimelineTableView(frame: window.bounds, style: .plain)
        let dataSource = NumberedRowsDataSource(count: 6)
        table.dataSource = dataSource
        table.rowHeight = 44
        table.transform = CGAffineTransform(scaleX: 1, y: -1)
        window.addSubview(table)
        window.isHidden = false
        table.reloadData()
        table.layoutIfNeeded()

        let order = try XCTUnwrap(table.accessibilityElements as? [UITableViewCell])
        let rows = order.compactMap { table.indexPath(for: $0)?.row }
        XCTAssertEqual(rows, [5, 4, 3, 2, 1, 0], "row 0 is the newest, drawn at the bottom: read it last")
        let tops = order.map { $0.convert($0.bounds, to: nil).minY }
        XCTAssertEqual(tops, tops.sorted(), "VoiceOver walks the screen top to bottom")

        XCTAssertEqual(table.accessibilityElementCount(), 6)
        XCTAssertTrue(table.accessibilityElement(at: 0) as? UITableViewCell === order.first)
        XCTAssertEqual(table.index(ofAccessibilityElement: order[2]), 2)
        XCTAssertEqual(table.index(ofAccessibilityElement: "not a cell"), NSNotFound)
        _ = dataSource
    }

    func testAccessibilityOrderSortsByScreenPosition() {
        XCTAssertEqual(
            StableTimelineAccessibilityOrder.topToBottom([("newest", 400.0), ("oldest", 10.0), ("middle", 200.0)]),
            ["oldest", "middle", "newest"]
        )
    }

    // MARK: Helpers

    private func room(
        kind: RoomSummary.RoomKind,
        name: String,
        peer: String? = nil,
        hasActiveCall: Bool = false
    ) -> RoomSummary {
        RoomSummary(
            id: "!\(name.filter(\.isLetter)):example.org",
            name: name,
            lastMessagePreview: "",
            unreadCount: 0,
            hasHighlight: false,
            kind: kind,
            membership: .joined,
            lastActivityAt: Date(timeIntervalSince1970: 0),
            hasActiveCall: hasActiveCall,
            directUserId: peer
        )
    }
}

private final class ManualTypingScheduler: ComposerTypingScheduling {
    private final class ManualTimer: ComposerTypingTimer {
        var fireAt: TimeInterval
        let interval: TimeInterval
        let repeats: Bool
        let action: () -> Void
        var isCancelled = false

        init(fireAt: TimeInterval, interval: TimeInterval, repeats: Bool, action: @escaping () -> Void) {
            self.fireAt = fireAt
            self.interval = interval
            self.repeats = repeats
            self.action = action
        }

        func cancel() {
            isCancelled = true
        }
    }

    private var now: TimeInterval = 0
    private var timers: [ManualTimer] = []

    var activeTimerCount: Int {
        timers.filter { $0.isCancelled == false }.count
    }

    func schedule(after interval: TimeInterval, repeats: Bool, _ action: @escaping () -> Void) -> ComposerTypingTimer {
        let timer = ManualTimer(fireAt: now + interval, interval: interval, repeats: repeats, action: action)
        timers.append(timer)
        return timer
    }

    func advance(by duration: TimeInterval) {
        let end = now + duration
        while let next = nextDueTimer(before: end) {
            now = next.fireAt
            if next.repeats {
                next.fireAt += next.interval
            } else {
                next.isCancelled = true
            }
            next.action()
        }
        now = end
        timers.removeAll { $0.isCancelled }
    }

    private func nextDueTimer(before end: TimeInterval) -> ManualTimer? {
        timers.filter { $0.isCancelled == false && $0.fireAt <= end + 1e-9 }
            .min { $0.fireAt < $1.fireAt }
    }
}

private actor TypingRecorder {
    private(set) var values: [Bool] = []

    func record(_ typing: Bool) {
        values.append(typing)
    }
}

private final class NumberedRowsDataSource: NSObject, UITableViewDataSource {
    let count: Int

    init(count: Int) {
        self.count = count
    }

    func tableView(_: UITableView, numberOfRowsInSection _: Int) -> Int {
        count
    }

    func tableView(_: UITableView, cellForRowAt indexPath: IndexPath) -> UITableViewCell {
        let cell = UITableViewCell(style: .default, reuseIdentifier: nil)
        cell.textLabel?.text = "Row \(indexPath.row)"
        return cell
    }
}
