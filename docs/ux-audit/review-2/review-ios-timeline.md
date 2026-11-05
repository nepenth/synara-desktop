# iOS room timeline and composer: second deep UX/UI review

Scope: `synara-ios/Synara/Features/RoomTimelineView.swift`, `StableTimelineViewport.swift`, `ThreadDisplay.swift`, `RichLinkEmbeds.swift`, `TimelineReactionSenders.swift`, `TimelineAccessibility.swift`, `TimelineRedactConfirmation.swift`, `SelectableMessageTextView.swift`, `RoomMemberActions.swift`, `RoomNotesView.swift`, `Features/Composer/*`, `SharedUI/SynaraMessageBubble.swift`, `SynaraDesign.swift`, `SynaraMotion.swift`, `SynaraHaptics.swift`, `SynaraTimeFormatting.swift`, `AnimatedImage.swift`. Method: code reading only, on `main` (3231c7bb). Paths below are relative to the repo root; `RTV` = `synara-ios/Synara/Features/RoomTimelineView.swift`, `STV` = `synara-ios/Synara/Features/StableTimelineViewport.swift`, `Desktop` = `synara/src/app/features/room/NativeTimelinePresenter.tsx`.

## Summary

1. The 2.7.5 flat-row restyle landed well: grouping, day headings, sender/time row, mention pills, quick-reaction palette and the glass header are consistent and Slack-like; the body type is Dynamic-Type `.body` (17 pt default), not a fixed 15 pt, which is right for iOS.
2. Two correctness-level gaps undercut the Slack feel: the reaction row is a non-wrapping `HStack` (desktop wraps), and swipe-to-reply does not exist even though the code comments claim it; the only horizontal gesture is the leftward timestamp reveal on grouped rows.
3. Several surfaces still show the bubble era or placeholder chrome: the loading skeleton draws right-aligned outgoing bubbles, the thread header has two dead icons, the "edited" marker disappears on grouped messages, and routine successes are reported with modal alerts.
4. The thread/reply system is functionally complete but visually thin: a one-line 12 pt reply quote that cannot be tapped, a "N replies" chip without participants or recency, and a thread screen whose padding and fonts differ from the room.
5. Empty/loading/history states are generic ("No Messages", a spinner for typing, no "beginning of room" marker, no new-message count on the jump button); fixing these plus media sizing and composer type parity would complete the direction.

## Findings (ordered by impact)

### IOST-1 (P0) Reaction pills do not wrap
- Where: `RTV:7533-7548` (`HStack(spacing: SynaraSpacing.small)` holding `ReactionPill`s plus `addReactionPill`); pill metrics `RTV:8781-8783` (32 pt tall, 10 pt side padding, 17 pt emoji).
- Today: five or more distinct reactions on an iPhone exceed the content width (about 320 pt after the 38 pt avatar gutter). SwiftUI compresses the pills and clips the add pill; counts can truncate.
- Proposal: a wrapping flow layout (`Layout` protocol, iOS 16+) with 6 pt row gap and 8 pt item gap; pills keep their size; the add pill is always last and visible. Match the desktop which wraps (`Desktop:1765`, `Box ... wrap="Wrap"`).
- Effort: S. Risk: low (layout only; the cell self-sizes).
- Desktop counterpart: already correct; no change.

### IOST-2 (P0) Swipe-to-reply is advertised but not implemented
- Where: comment `RTV:7297-7299` ("Long-press (and swipe to reply) carry the actions"); the only row gesture is `TimelineTimestampRevealGestureRecognizer` (`STV:232-287`, `STV:973-1054`) which accepts only `translation.x < 0` (`STV:256`) and only on grouped rows (`STV:1020`, `STV:1049`). `grep -ri swipe synara-ios/Synara` finds no reply gesture.
- Today: the only way to reply is the long-press menu (or the VoiceOver action). Slack's primary mobile reply gesture is missing, and the sole row swipe (leftward, grouped rows only) is undiscoverable.
- Proposal: extend the recognizer with a rightward intent: activation 12 pt, same direction-dominance test, on any message row (not system notices). During the drag translate the row up to 56 pt and fade in a 28 pt `arrowshape.turn.up.left` glyph in `SynaraColor.accent` at the leading edge; at 40 pt fire `SynaraHaptics.trigger(.selection)` once and arm; on release while armed call `onReply` and spring the row back (`.spring(response: 0.28, dampingFraction: 1)`; Reduce Motion: no translation, glyph only). Keep the leftward timestamp reveal.
- Effort: M. Risk: medium (shares the table pan; the existing recognizer already has the fail-fast pattern and session validation to copy).
- Desktop counterpart: n/a (hover toolbar).

### IOST-3 (P1) Loading skeleton previews a bubble layout the timeline no longer has
- Where: `synara-ios/Synara/SharedUI/SynaraDesign.swift:1719-1771` (`SynaraTimelineSkeletonRow`: `isOutgoing` rows right-aligned with a trailing 34 pt avatar; 34 pt avatar, 12 pt bars); used at `RTV:1064` (8 rows) and `RTV:4778` (4 rows, thread).
- Today: for the first second of every room open the user sees a Messages-style bubble conversation, then it snaps to flat left-aligned rows with 38 pt avatars.
- Proposal: one row shape matching `TimelineRow`: 38 pt rounded-square block, 14 pt bold-name bar (92 pt) plus 36 pt time bar, body bars at 12 pt, no outgoing variant, grouped follow-ups (gutter only) every third row, top padding 12/2 pt. Mirror the day-heading rule as the first element.
- Effort: S. Risk: none.
- Desktop counterpart: check desktop skeleton for the same drift.

### IOST-4 (P1) Thread header: two dead icons, 38 pt back button, no room context
- Where: `RTV:5641-5682` (`ThreadHeader`): `Image(systemName: "bell")` and `Image(systemName: "ellipsis")` are plain images with no action (`5658-5661`); back button frame 38x38 (`5651`); subtitle is `rootTitle ?? roomTitle` (`4625`).
- Today: two glyphs look like buttons and do nothing; the back target is under 44 pt; the sheet shows the root message body where Slack shows the channel.
- Proposal: title "Thread", subtitle `#room-name` (channel glyph rules as the room header, `RTV:4405-4417`); remove the bell and ellipsis or wire them (thread notification mode; "Copy link to thread"/"Open in room" menu). Back button 44 pt with `synaraChromeGlass` like the room header (`RTV:4389-4396`). In `.sheet` mode the back chevron should be an `xmark`/"Done".
- Effort: S. Risk: none.
- Desktop counterpart: `ThreadSidePanel.tsx` header; align titles.

### IOST-5 (P1) "edited" marker vanishes on grouped messages
- Where: `RTV:7288-7292` sits inside `if isGroupedWithPrevious == false` (`7278`). Grouped rows render only `messageContent` (`7323`).
- Today: an edited second-in-group message shows no marker at all; the first-in-group marker sits next to the time, 13 pt footnote tertiary.
- Proposal: Slack's placement: `(edited)` appended after the body as a trailing `Text` in `messageMeta` + `tertiaryText`, for every row; drop it from the name/time row. Keep "Edited" in the VoiceOver summary (`OutgoingSendService.swift:140-142`).
- Effort: S. Risk: low (attributed-string suffix for text kinds; a separate line for media/polls).
- Desktop counterpart: already after the body (`Desktop:2357-2361`, "Edited" T200). Match wording: "(edited)" lowercase on both, or "Edited" on both.

### IOST-6 (P1) Context-menu order, palette position and delete wording
- Where: `RTV:7410-7474` (`messageActions`): order is Discard, Copy, Select Text, Reply, Reply in Thread, Open Thread, Edit, quick-reaction palette (`7438-7440`), View Reactions, Forward, Decline Call, Pin to Notes, Report, Redact. Palette: `RTV:7876-7893`. Redact copy: menu "Redact" (`7472`), confirmation "Remove this message?" (`TimelineRedactConfirmation.swift:6-7`), row "Message deleted" (`RTV:7797`), reply snippet "Deleted message" (`RTV:10786`).
- Today: the emoji row appears in the middle of the menu after Edit; Copy outranks Reply; three different words for one action.
- Proposal: Slack order: emoji palette first; Reply; Reply in Thread / Open Thread; Forward; Copy; Select Text; Pin to Notes; View Reactions; Edit; then a `Divider()`; Report; "Delete Message" (destructive). Rename everywhere to Delete / "Message deleted" / "Deleted message" consistently (keep the confirmation copy shared with desktop but say "Delete" if desktop agrees).
- Effort: S. Risk: none.
- Desktop counterpart: `nativeTimelineActions.ts` menu order and `nativeTimelineRedact.ts` copy; align the verb.

### IOST-7 (P1) Reply quote is too quiet and cannot be tapped
- Where: `RTV:8043-8057` (`replyQuoteLabel`): `messageMeta` (12 pt caption), `lineLimit(1)`, 3 pt bar at `accent.opacity(0.55)`, `secondaryText` for both name and snippet; no tap action; fallback "Replying to a message" (`8059-8063`).
- Today: the quoted context is the least legible element in the row (prior audit P1.3 called this out) and gives no way to reach the original message.
- Proposal: `messageTime` (13 pt footnote), sender `semibold` in `primaryText`, snippet `secondaryText`, 2 lines, bar 3 pt full `accent`; whole quote is a `Button` that routes to `.room(id:eventID:)` (the path `RoomDetailsView.onOpenMessage` already uses, `RTV:764-770`) and briefly highlights the target row (0.6 s accent 12% fill fade); add a VoiceOver action "Go to original message". Show a 20 pt sender avatar in the quote at regular width only.
- Effort: M. Risk: low (routing exists; highlight needs a `highlightedEventID` state on the row value).
- Desktop counterpart: check whether the desktop quote is clickable; match.

### IOST-8 (P1) Thread chip lacks participants and recency
- Where: `RTV:7551-7571`: `bubble.left.and.bubble.right.fill` + "N replies" in `chipLabel` (12 pt semibold) accent, `minHeight: 44`.
- Today: a 12 pt caption chip; no sense of who is in the thread or how fresh it is. Slack: stacked participant avatars, "N replies", "Last reply 2h ago", chevron.
- Proposal: `HStack`: up to three overlapping 20 pt `TimelineAvatar`s (-6 pt overlap), "N replies" in `messageTime.weight(.semibold)` + `accentText`, "Last reply <relative>" in `messageMeta` `secondaryText`, trailing `chevron.right` 12 pt. Inline mode keeps the chevron up/down. Needs `TimelineReplyCounter` (`TimelineService.swift:1667`) to also surface participants and last-reply timestamp (local count exists; Core thread summary may be required for off-window replies).
- Effort: M (L if Core must project thread summary). Risk: medium (data availability).
- Desktop counterpart: "Thread · N replies" (`Desktop:1732`); upgrade both together.

### IOST-9 (P1) Image attachments: fixed 180 pt crop inside a file card
- Where: `RTV:8097` (`imageThumbnailHeight = 180`), `8113-8125` (`scaledToFill` + `clipped`, then `mediaMetadataRow` with filename and size, wrapped in `synaraCard`), `8255-8256` (thumbnail request at screen width x 180).
- Today: every image is a letterboxed 180 pt strip with its filename and byte size underneath, in a card. Slack shows the image at its own aspect ratio with no filename.
- Proposal: aspect-fit to the content width, height clamped 120-280 pt (320 at regular width), corner radius 12, no card and no filename/size for `isImageMedia` (available in the viewer title); keep the file card for documents. Request the thumbnail at the clamped size. Group consecutive image events from the same sender within 2 min into a 2-up grid (optional, M).
- Effort: S (M with grid). Risk: low; cell heights change but the table self-sizes.
- Desktop counterpart: check `nativeTimelineFilePreview.ts`/media rendering for parity.

### IOST-10 (P1) Typing indicator: spinner plus Matrix localparts
- Where: `RTV:517-536` (`RoomTypingPresentation.displayName` strips `@` and server; text "X is typing..."), `RTV:4266-4284` (`ProgressView().controlSize(.mini)` + caption, full-width `surface` band).
- Today: "alice is typing..." with an activity spinner, using the Matrix ID localpart even when a display name is known; a band that appears/disappears and reflows the composer. Desktop uses member display names with localpart fallback and animated dots (`RoomViewTyping.tsx:50-56`, `85`).
- Proposal: resolve names from the loaded members (`environment.roomManagement.roomDetails` members, as the composer does at `RTV:9503`) falling back to localpart; three-dot bounce (Reduce Motion: static dots); bold names; typographic ellipsis; reserve a fixed 20 pt slot above the composer so the layout does not jump (desktop has `RoomViewTypingSlot`).
- Effort: S/M. Risk: low.
- Desktop counterpart: already correct.

### IOST-11 (P1) No "beginning of the room" and a generic empty state
- Where: empty state `RTV:1071-1076` ("No Messages" / "Messages will appear here.", `text.bubble` 42 pt via `SynaraEmptyState`); reaching the start only sets `hasReachedOldestMessages` (`RTV:3118-3123`) and shows nothing; the "Load older messages" bordered button floats top-leading when there is one row (`RTV:3437-3452`).
- Today: scrolling to the top just stops; a new room shows a system-icon placeholder. Slack: "This is the very beginning of #room" with the room avatar and topic.
- Proposal: a `roomStart` row (new `StableTimelineViewportItemID`) rendered when `hasReachedOldestMessages`: 56 pt room avatar, "This is the very beginning of #room" (`dayHeader` font), topic in `supporting`, created date in `messageMeta`. The empty state reuses the same block with a "Say hello" hint and focuses the composer. Replace the floating "Load older messages" button with the normal pagination spinner.
- Effort: M. Risk: low.
- Desktop counterpart: "No messages in this view yet." (`Desktop:4122`); match the copy and the start block.

### IOST-12 (P1) Header subtitle "N members" is really "N unique senders"
- Where: `RTV:1867-1882` (`timelineSubtitle` counts `Set(items.map(\.senderID))`), fallback "Matrix room"; shown in `TimelineHeader` at `RTV:4426-4440`.
- Today: a 3-member room with one active speaker reads "1 members"; the count changes as history pages in.
- Proposal: use the real member count (`RoomDetails.memberCount` exists, `RTV:6637`; cache it in `RoomSummary`/room list) and pluralize; omit the subtitle for DMs (show presence instead); tap the subtitle to open members. Drop "Matrix room".
- Effort: S. Risk: low.
- Desktop counterpart: header has a members button (`RoomViewHeader.tsx:806-819`); fine.

### IOST-13 (P1) Message search results are inert
- Where: `RTV:4535-4556` (`List(filteredItems)` rows have no action); field placeholder "Search loaded messages" (`4499`).
- Today: a result cannot be opened; search covers only the loaded window with no explanation of the limit.
- Proposal: tapping a row dismisses the sheet and routes to `.room(id:eventID:)` with the highlight from IOST-7; add a footer "Searches messages loaded in this room" and, when Core supports it, a "Search server" action.
- Effort: S. Risk: none.
- Desktop counterpart: check `RoomSidePanel.tsx` search.

### IOST-14 (P1) Routine success feedback uses modal alerts
- Where: `.alert("Message Action")` `RTV:846-853`, "Agent Action" `854-863`, "Personal Notes" `874-881` (thread: `4693-4697`, `4763-4770`); fed by `performAction` success messages ("Vote submitted.", "Call declined.", "Message forwarded." `RTV:3931-3933`, `3966`) and `pinToNotes` (`1852`).
- Today: pinning or forwarding a message pops an alert titled "Message Action" with an OK button; the user must dismiss it.
- Proposal: a shared transient toast (bottom, above the composer, `ultraThinMaterial` capsule, 2.5 s, `UIAccessibility.post(.announcement)`) for successes; keep alerts for errors that need acknowledgement. Reuse the Jump-button visual language (`RTV:4286-4316`).
- Effort: M. Risk: low.
- Desktop counterpart: check for an existing toast; match copy.

### IOST-15 (P1) Composer text is 16 pt while messages are 17 pt
- Where: `synara-ios/Synara/Features/Composer/ComposerTextView.swift:106` (`.preferredFont(forTextStyle: .callout)`), `:224`, `:541`, `:660`, `:783`, `:811`, `:845`, `:868`; `RTV:9416`, `9880`, `9888`; placeholder `SynaraTypography.composerPlaceholder = .callout` (`SynaraDesign.swift:1294`); timeline body `.body.leading(.tight)` (`SynaraDesign.swift:1282`).
- Today: text looks smaller while typing than after sending.
- Proposal: one `UIFont.preferredFont(forTextStyle: .body)` constant in `ComposerTextMetrics` used by the text view, placeholder, markdown presentation and the SwiftUI height estimate; `singleLineHeight` recomputes automatically.
- Effort: S. Risk: low (height cap 240 pt `ComposerTextView.swift:9` still fine).
- Desktop counterpart: n/a.

### IOST-16 (P1) Send button pops in and out without a transition
- Where: `RTV:9695-9713` (`if canSubmit { Button ... }`), inside `HStack(alignment: .bottom)` `9646-9649`; `.animation` modifiers exist only for the formatting bar, metrics and mentions (`9498-9500`).
- Today: the composer width jumps 48 pt on the first and last character; no state for "disabled".
- Proposal: keep a 44 pt slot always; `.transition(.scale(0.8).combined(with: .opacity))` with `.animation(.easeOut(duration: 0.15), value: canSubmit)`; when sending show a `ProgressView` in the same slot.
- Effort: S. Risk: none.
- Desktop counterpart: n/a.

### IOST-17 (P1) Haptics fire on every successful send
- Where: `RTV:2826` and `RTV:5390` (`SynaraHaptics.trigger(.lightImpact)` on `.sent`); warnings on failure (`2834`, `5388`); approvals `.success` (`4072`).
- Today: a tap every time a message is confirmed, a second or more after the tap. Slack is silent on send and uses haptics for gesture thresholds.
- Proposal: remove the sent impact; add `.selection` at the swipe-to-reply threshold (IOST-2) and one `.lightImpact` when a reaction toggle succeeds (`RTV:3889-3898`). Keep `.warning` on failure.
- Effort: S. Risk: none.

### IOST-18 (P2) Room and thread rows disagree on padding, fonts and spacing
- Where: room rows padding `SynaraSpacing.large` (`RTV:1897-1899`, `1536`), thread rows `SynaraSpacing.medium` (`RTV:4842`); inline thread reply rows use `SynaraTypography.emphasis` and `.body` (`RTV:10749-10760`) instead of `messageSender`/`messageBody`; thread day headers padded `xSmall` (`4824`) vs room `small` (`1481`).
- Proposal: a `TimelineLayout` enum with `horizontalPadding`, `rowTopPadding`, `groupedTopPadding`, `dayHeaderPadding` used by both; inline replies use the row fonts at a 22 pt avatar.
- Effort: S. Risk: none.
- Desktop counterpart: n/a.

### IOST-19 (P2) Jump-to-latest has no new-message count
- Where: `RTV:4286-4316` (44 pt circle, `arrow.down`); shown via `RoomTimelineJumpLatestPolicy.shouldShow` (`RTV:428-433`).
- Today: new live messages arriving while reading history are invisible until the user scrolls.
- Proposal: count live appends while `timelinePosition == .readingHistory` (reset on pin); when > 0 render a capsule "3 new messages" + arrow (same material, `fineMetaBold`), collapsing back to the circle at 0; VoiceOver value announces the count.
- Effort: M. Risk: low.
- Desktop counterpart: `Desktop:4247` jumpToLatest button; check for a count.

### IOST-20 (P2) Media viewer has no share, save, double-tap or swipe-down
- Where: `RTV:10449-10547` (`MediaViewer`: Done only), `10549-10587` (`ZoomableMediaImage`: `MagnificationGesture` only).
- Proposal: toolbar `ShareLink` and "Save to Photos" (`PHPhotoLibrary`), double-tap toggles 2x at the tap point, vertical drag past 120 pt dismisses with fade; sender and time as the subtitle; black background with `.toolbarBackground(.hidden)`.
- Effort: M. Risk: low.
- Desktop counterpart: `NativeTimelineMediaViewer.tsx`; align actions.

### IOST-21 (P2) Own-message "sent" checkmark and reserved trailing lane
- Where: `synara-ios/Synara/SharedUI/SynaraMessageBubble.swift:132-137` (`.sent` draws a checkmark), `317-333` (`.compactTrailing` lane for sending/queued/sent), `39` and `337-339` (trailing inset reserved).
- Today: a bubble-era delivery check appears at the bottom-right of flat rows and indents text while the status is `.sent`.
- Proposal: show only `sending` (mini spinner, inline after the time) and `failed` (chips); no indicator for `sent`; no trailing inset when nothing is shown.
- Effort: S. Risk: low.
- Desktop counterpart: check delivery indicators.

### IOST-22 (P2) Send animation slides own messages in from the right
- Where: `RTV:7365` (`synaraSendSlideIn(isEnabled: animateSend, fromTrailing: isOutgoing)`), `SynaraMotion.swift:6-33` (16 pt horizontal offset).
- Today: a right-to-left slide left over from right-aligned bubbles; rows are now all left-aligned.
- Proposal: fade + 8 pt upward offset for every new row (own or not), 0.2 s easeOut; Reduce Motion: fade only.
- Effort: S. Risk: none.

### IOST-23 (P2) Composer syntax colours are not theme tokens
- Where: `ComposerTextView.swift:505` (`quoteBars.fillColor = UIColor.systemTeal`), `:633`, `:635`, `:690` (`UIColor.tertiarySystemFill` for inline code/spoiler/code block); timeline uses `SynaraColor.richTextInlineCodeBackground`, `richTextCodeBlockBackground` (`RTV:6364-6367`, `6457`) and quote bar `secondaryText.opacity(0.75)` (`RTV:6292`).
- Proposal: resolve the same tokens via `SynaraThemeRamp.uiColor` (as `applyTextAppearance` already does for text colour, `ComposerTextView.swift:228-232`); quote bar = accent at 3 pt like the reply quote.
- Effort: S. Risk: none.

### IOST-24 (P2) Reply/edit banner styling
- Where: `RTV:10420-10447` (`ComposerRelationBanner`: card on `secondarySurface`, text button "Cancel", 2-line snippet).
- Proposal: Slack's compact bar: 3 pt accent bar, `pencil`/`arrowshape.turn.up.left` 13 pt, "Replying to **Name**" + one-line snippet, trailing `xmark.circle.fill` in a 44 pt target; no card fill (sits on the composer glass); appears with `.move(edge: .bottom) + .opacity`.
- Effort: S. Risk: none.

### IOST-25 (P2) Mention suggestions without avatars; no emoji autocomplete
- Where: `RTV:10050` (`TimelineAvatar(senderID:..., avatarURL: nil, size: 30)`), suggestions limited to 5 (`ComposerMentions.swift:62`); no `:emoji` completion anywhere in `Composer/`.
- Proposal: pass `member.avatarURL`; add `:name` completion over the quick-reaction store plus a small built-in list (same glass panel, max 6); keep the 48 pt rows.
- Effort: M. Risk: low.
- Desktop counterpart: `CommandAutocomplete.tsx`; check emoji completion parity.

### IOST-26 (P2) Encrypted placeholder and media copy is technical
- Where: `RTV:7673-7676` ("Encrypted content unavailable. Actions and media downloads are blocked until keys are available."), `RTV:8172` ("Encrypted media requires recovered keys"), `RTV:10518`.
- Proposal: "Waiting for this message's keys" with the lock glyph, `secondaryText`, and a single "Learn more" that opens the crypto banner/Settings; keep the detail in the banner (`CryptoRecoveryBanner`, `RTV:7133-7184`).
- Effort: S. Risk: none.
- Desktop counterpart: `nativeTimelineErrorCopy.ts`; share the strings.

### IOST-27 (P2) Consecutive system notices are not collapsed
- Where: `RTV:8304-8322` (`TimelineSystemNoticeRow`), one row per event; grouping policy excludes notices (`TimelineService.swift:234-236`).
- Proposal: collapse runs of membership notices into "Ada, Bob and 3 others joined" (expand on tap); centred caption is fine.
- Effort: M (row-builder policy + Core sentence pieces). Risk: medium.
- Desktop counterpart: check `NativeTimelineNotice.tsx`.

### IOST-28 (P2) iPad regular width: no reading column
- Where: `RTV:7325-7327` (`frame(maxWidth: .infinity)`), `RTV:1897-1899` (padding 16 at every size class), `RTV:8030-8032` (avatar 40 at regular).
- Today: 17 pt prose runs the full detail-pane width (often 700-900 pt). Prior audit P1.3 persists.
- Proposal: at `horizontalSizeClass == .regular` cap the row at 720 pt leading-aligned with 24 pt side padding (the link card already caps at 460, `RichLinkEmbeds.swift:436`); keep images/tables inside the column.
- Effort: S. Risk: none.

### IOST-29 (P2) Avatar sizes are fixed, not Dynamic-Type aware
- Where: `RTV:8030-8032` (38/40), `6412` (22 header), `8918` (36), `10050` (30), `10746` (22); skeleton 34 (`SynaraDesign.swift:1727`).
- Proposal: `@ScaledMetric(relativeTo: .body)` for the row avatar (38 base, clamp 32-52) and a named `SynaraAvatarSize` enum (row, inline, header, list) to end the one-off numbers (prior audit P2.1).
- Effort: S. Risk: low.

### IOST-30 (P2) Unread divider styling
- Where: `RTV:8324-8351` ("Unread messages", `chipLabel` accent, 0.5 pt rules at 55% separator, 46 pt leading inset).
- Proposal: keep the rule but label it "New" in a small capsule at the trailing end (Slack's pattern) in `accentText`; the policy is already right (first other-sender message after the marker, `RTV:259-278`).
- Effort: S. Risk: none.
- Desktop counterpart: align label.

## Already good, leave alone

- Flat rows, 38/40 pt rounded-square avatars with a deterministic palette (`RTV:6490-6580`), bold sender + footnote time (`RTV:7279-7293`), 12/2 pt group spacing (`RTV:7329`), and the 2-hour same-day grouping rule (`TimelineService.swift:226-245`).
- Day headings: bold label over a hairline rule, relative names, year only when needed (`RTV:8276-8299`, `TimelineService.swift:248-320`).
- Mention pills gold-for-me/blue-for-others with hair-space padding (`RTV:6381-6386`, `7203-7207`).
- Quick-reaction palette as a `ControlGroup(.palette)` with `menuActionDismissBehavior(.enabled)` and the pre-iOS-17 fallback (`RTV:7876-7893`); pill tap toggles, long-press shows who reacted, and the Reactions sheet with tabs (`RTV:8727-8945`).
- Redaction confirmation copy shared with desktop and modelled so dismissal never redacts (`TimelineRedactConfirmation.swift`).
- Jump to latest / Jump to last read as one material cluster, 44 pt, with AX-size icon fallback (`RTV:4286-4364`, `1424-1448`).
- Header: glass capsules, 44 pt targets, title fitted to text at regular width, `#`/person glyph (`RTV:4366-4480`).
- Composer: Liquid Glass field, 44 pt hit regions around 34 pt glyphs, AX-size stacked controls (`RTV:9634-9644`), keyboard stays up after send, sent-text reconciliation (`RTV:10016-10027`), `/gif`, paste of images and rich HTML, markdown presentation in place (`ComposerTextView.swift`).
- Typing notifier timing contract (`ComposerTypingNotifier.swift`).
- VoiceOver: one element per row with rotor actions ordered Reply/React first (`OutgoingSendService.swift:62-122`), chronological reading order in the inverted table (`STV:632-740`), actions button shown only for VoiceOver/Switch Control (`RTV:7828-7830`).
- Rich text: tables with zebra rows and visible scroll indicators (`RTV:6124-6269`), code blocks with language and Copy, spoilers, details, link preview card capped at 460 pt (`RichLinkEmbeds.swift:398-450`), inline GIFs sized by aspect (`RichLinkEmbeds.swift:302-340`).
- Attachment sheet and GIF picker (trending, attribution, 3 columns) (`RTV:10130-10270`, `GifPickerSheet.swift`).

## Prior-audit items that persist (docs/ux-audit/ios-holistic-audit.md)

- P0.1 composer at accessibility sizes: largely addressed (stacked controls `RTV:9634-9644`; placeholder bottom constraint now `lessThanOrEqualTo` and container clips, `ComposerTextView.swift:889`, `910-913`). Still open: the height cap is a constant 240 pt (`ComposerTextView.swift:9`), not viewport-derived.
- P1.3 reply legibility and iPad reading width: persist (IOST-7, IOST-28). Within-group spacing was tuned (2 pt grouped / 12 pt new sender).
- P1.4 rich messages: tables still use fixed minimum cell widths 128/176 (`RTV:6189-6191`); code blocks still hide horizontal scroll indicators (`RTV:6461`); tables now show indicators (`RTV:6147`). Headings inside messages are capped at body size (`RTV:5913-5922`), which is a deliberate improvement.
- P1.6 avatar identity: resolved for the timeline (`SynaraAvatarPalette`, `RTV:6578`); shapes still differ between header person glyph (circle, `SynaraRoomAvatar.swift:106-130`) and row avatars (rounded square, `RTV:6497-6499`).
- P2.1 one-off geometry: avatar sizes 22/30/34/36/38/40 and padding 12 vs 16 remain ad hoc (IOST-18, IOST-29).
- P2.5 accessibility record: Attach/Format/Send now have 44 pt hit frames around 34/28 pt glyphs (`RTV:9659-9663`, `9681-9685`, `9700-9704`); the checklist can be updated.
- P2.4 motion gates: Reduce Motion is respected for send slide, reaction pop and row animation (`SynaraMotion.swift`, `STV:1084`, `1164-1167`).
