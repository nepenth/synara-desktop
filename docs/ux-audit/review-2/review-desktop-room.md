# Desktop room view: deep UX/UI review (Slack-informed, second pass)

Date: 2026-11-01. Scope: everything right of the room list on desktop. Method: code reading only, `main` at 3231c7bb. All paths are under `synara/src/` unless noted; iOS paths under `synara-ios/Synara/`.

## Summary

1. The reading surface (sender/body type, day headings, reaction pills, mention pills, hover tint, semantic colour roles, code/table treatment) is in good shape after v2.7.5/2.7.6 and should be left alone; the gaps are now in the *affordances around* messages, not the messages themselves.
2. The biggest miss against Slack is the hover action bar: it exposes only "Add reaction" and "⋯" (`NativeTimelinePresenter.tsx:1242-1329`); Reply, Reply in thread, Edit, Forward and Save all hide behind the overflow menu, so the most frequent actions cost two clicks.
3. Thread affordances are the second-largest gap: the in-timeline thread pill is a generic soft button with verbose copy ("Thread · 3 replies · open latest"), the Threads panel titles threads with raw `$event` IDs, and the thread side panel has no room subtitle or "N replies" divider (iOS already has the subtitle).
4. Several row kinds (redacted, encrypted-unavailable, "other", poll, call, timeline start) still render as plain left-aligned T300 text on a hoverable message surface; the empty room state is one line of text; the composer send button is a quiet glyph rather than the dominant control.
5. Residual inconsistencies: the "Edited" marker is a separate block line (iOS puts lowercase "edited" beside the time), unread-marker copy differs from iOS ("New messages" vs "Unread messages"), the grouped-row hover timestamp can collide with the action rail, and a few secondary texts still use `opacity` instead of the semantic colour roles the prior audit introduced.

## Findings (ordered by impact)

Legend: priority P0/P1/P2; effort S/M/L; "iOS" says whether iOS has a counterpart that should match.

### ROOM-1 · P0 · Hover action bar exposes only two actions

- Where: `app/features/room/NativeTimelinePresenter.tsx:1242-1329` (rail), `:609-845` (menu items).
- Today: the rail that appears on hover/focus-within holds `Add reaction` (SmilePlus) and `More message actions` (VerticalDots). Reply, Reply in thread, Edit, Forward, Pin, Save for later, Copy, View reactions, Redact, Report all live in the popout menu (`NativeTimelineRowActions`). Every common action is two clicks and a menu read.
- Slack: the hover bar shows the three most-used emoji, Add reaction, Reply in thread, Forward, Save, and ⋯; destructive/rare items stay in ⋯.
- Proposal: rail order `[React] [Reply] [Reply in thread] [Edit (own, when capabilities.edit)] [Forward] [Save for later] [⋯]`, all `IconButton size="300" radii="300"` with `title`/`aria-label` and the existing `role="toolbar"`. Keep Copy, View reactions, Pin/Unpin, Redact, Report in ⋯. Add Left/Right arrow key movement inside the toolbar (roving tabindex) so a focused row reaches actions without tabbing through the whole bar. Optional second step: three "frequent reactions" chips from a small LRU in settings.
- Effort: M. Risk: wider rail needs the row's right gutter (see ROOM-28); rail width grows from ~72px to ~230px, so hide labels and keep icons only.
- iOS: no visible bar (long-press/swipe by design, `RoomTimelineView.swift:7297-7300`), but the *ordering* of the context menu (`:7410-7471`) should match the new rail order (Copy, Reply, Reply in Thread, Edit, React, Forward, Save, Report, Redact).

### ROOM-2 · P1 · Grouped-row hover timestamp collides with the action rail

- Where: `nativeTimelineHtml.css.ts:50-74` (`GroupedTimestampReveal`: `position:absolute; top:50%; right:S300`), `:76-87` (`MessageActionRail`: `top:S100; right:calc(S400+S200)`), `NativeTimelinePresenter.tsx:2266-2275`.
- Today: on a one-line grouped message both appear on hover at the right edge: the time at 12px from the right, vertically centred; the rail from 24px leftwards, ~40px tall. A "10:42" label (~34px wide) spans the rail's x-range, so the two overlap on short rows. (Not reproduced in a build; the geometry is from the two style blocks.)
- Slack: the hover time sits in the *left* gutter where the avatar would be, right-aligned, 11–12px.
- Proposal: move the reveal into the 36px avatar column (`NativeTimelinePresenter.tsx:2282`): render `<Time compact>` inside that column when `grouped`, `textAlign:right`, `fontSize: toRem(11)`, `color: var(--synara-content-secondary)`, same `:hover`/`:focus-within` opacity rule. Delete the absolute right-side reveal.
- Effort: S. Risk: none; the column already exists and is empty for grouped rows.
- iOS: tap-to-reveal timestamps exist (`RoomTimelineView.swift:1603-1636`); placement can stay platform-specific.

### ROOM-3 · P1 · Thread entry pill is generic and verbose

- Where: `NativeTimelinePresenter.tsx:1709-1740` (`NativeTimelineThreadSurface`), `nativeTimelineView.ts:294-298`, `crates/synara-core/src/app/timeline/view.rs:183-188`.
- Today: a `Button size="300" fill="Soft"` reading `Thread · 3 replies · open latest` (or `· open root`, or `Thread reply · open thread`). The summary DTO carries only `rootEventId`, `replyCount`, `latestEventId`.
- Slack: participant avatar stack, "3 replies" as an accent link, "Last reply 2h ago" secondary, "View thread →" on hover.
- Proposal (desktop now): replace the soft button with a text-link row: `Icons.Thread` 16px + "3 replies" in `color.Primary.Main` 13px/600, then on hover a secondary "View thread" and chevron; no fill, 28px tall, hover underline. Drop "open latest/open root". (Core later, M): extend `TimelineThreadSummary` with `latest_reply_ts` and up to 3 `participant_user_ids` so desktop can show "Last reply 2h ago" and avatars.
- Effort: S (desktop) + M (Core). Risk: low.
- iOS: has a thread pill too; both should use the same "N replies · Last reply …" copy.

### ROOM-4 · P1 · Reply quote and composer reply preview use two visual languages

- Where: timeline quote `NativeTimelinePresenter.tsx:1684-1707` + `nativeTimelineHtml.css.ts:525-553` (`ReplySurface`: filled `--synara-table-even`, 3px rule, body `pre-wrap` unclamped, "Replying to {name}" in the 15px/700 `SenderName` class); composer preview `RoomInput.tsx:1369-1412` using legacy `ReplyLayout` (`components/message/Reply.tsx:23-41`, `Reply.css.ts:33-41` with `opacity: P300` content).
- Today: the quote block is as heavy as a message (bold 15px name, full body, filled card). The composer preview is a different component with opacity-dimmed text and a 200px name column.
- Slack: quoted text is a thin left bar, 13px, 2 lines max, no fill; the composer "Replying to" strip is one line with an X.
- Proposal: one `ReplyQuote` recipe used by both: `borderLeft: 2px solid var(--synara-content-separator)`, no background, `paddingLeft: S300`, name line `13px/600 var(--synara-content-secondary)` with `Icons.ReplyArrow` 12px, body `13px/1.4` clamped with `display:-webkit-box; -webkit-line-clamp:2; overflow:hidden`. Replace `opacity: P300` with the secondary colour role. Hover: name turns `--synara-content-heading`.
- Effort: S–M. Risk: row-height estimates shift slightly (`estimateNativeTimelineRowSize`); the measured-size cache handles it.
- iOS: reply preview exists (`RoomTimelineView.swift:8051-8059`); match the 2-line clamp.

### ROOM-5 · P1 · "Edited" is a separate block line and diverges from iOS

- Where: `NativeTimelinePresenter.tsx:2357-2361` (`<Text size="T200" className={Metadata}>Edited</Text>` after the body, inside `MessageBody`).
- Today: a capitalised "Edited" on its own line under the message, adding a row of height to every edited message.
- Slack: "(edited)" inline after the last word, 12px, tertiary. iOS: lowercase "edited" beside the time in the header, tertiary (`RoomTimelineView.swift:7289-7292`).
- Proposal: render ` (edited)` inline as a trailing `<span>` inside the last paragraph (for `NativePlainMessageBody`) or appended after `NativeFormattedBody` with `display:inline`, `fontSize: toRem(12)`, `color: var(--synara-content-tertiary)`, `marginLeft: 4px`. Both clients adopt lowercase "edited".
- Effort: S. Risk: formatted bodies ending in a block (code, table) need the span to fall to the next line; acceptable.
- iOS: yes; agree on one placement (recommend inline after text on both, or header-adjacent on both).

### ROOM-6 · P1 · Two read/unread markers with different copy, and copy differs from iOS

- Where: `NativeTimelinePresenter.tsx:2536-2551` (`read_marker` "Read up to here", `unread_marker` "New messages"), `nativeTimelineHtml.css.ts:555-570, 642-645`.
- Today: "New messages" label is `Primary.Main` 600 but the flanking rules stay in `--synara-content-separator`; a second, grey "Read up to here" row can also appear. iOS says "Unread messages" (`RoomTimelineView.swift:8333`).
- Slack: one thin accent line with a right-aligned "New" caption.
- Proposal: one marker kind rendered as a 1px `color.Primary.Main` rule with a right-aligned 11px/600 uppercase "NEW" caption (or "New messages"); colour the rules Primary too (`UnreadSystemRow` currently colours only text). Hide `read_marker` when `unread_marker` is present, or render it identically. Pick one string for both clients.
- Effort: S. Risk: none.
- iOS: yes, copy alignment.

### ROOM-7 · P1 · Non-message rows are not consistently "system" rows

- Where: `NativeTimelinePresenter.tsx:2552-2557` (timeline_start), `:2558-2585` (redacted "Message removed"), `:2586-2613` (encrypted_unavailable), `:2614-2642` (other), `:2488-2526` (call), `:2402-2430` (membership/state use `SystemRow`).
- Today: membership/state are centred 13px tertiary (good). Redacted, encrypted-unavailable, "other", call and timeline_start are plain `Text size="T300"` left-aligned on a full `MessageRow surface` (hover tint, rail) with no icon or tone. "Message removed" reads like a message. Consecutive membership events are not collapsed (`crates/synara-core/src/app/timeline/view.rs:922` emits one summary per event).
- Slack: "This message was deleted." italic tertiary in the message column; joins collapse ("A, B and 3 others joined").
- Proposal: (a) redacted/encrypted_unavailable: keep the message column indent (36px gutter) and render `15px italic var(--synara-content-tertiary)` with a small `Icons.Delete`/`Icons.Lock` 14px; (b) call/other: `SystemRow` centred like membership; (c) timeline_start: see ROOM-8; (d) Core: coalesce runs of membership summaries within one day into one row with a `+N` expander (`view.rs` summary builder). Hover tint stays only on rows with actions.
- Effort: S (a–c), M (d). Risk: grouping already breaks on these kinds (`nativeTimelineGrouping.ts:15-16`), so no header changes.
- iOS: membership rows exist (`isSystemNotice`); collapsing should land in Core so both benefit.

### ROOM-8 · P1 · Empty room and "beginning of timeline" states are bare

- Where: `NativeTimelinePresenter.tsx:4116-4124` ("No messages in this view yet."), `:2552-2557` ("Beginning of timeline"), loading `:3981-3994`.
- Today: one T300 line centred; the start-of-history row is a left-aligned T300 line on a hoverable surface.
- Slack: a channel intro at the top of history: avatar, "This is the very beginning of the #channel", purpose/topic, created date, and Add people / Set description actions.
- Proposal: a `TimelineIntro` block rendered for `timeline_start` (and alone when `rows.length === 0`): room glyph/avatar 48px, heading `toRem(18)/700 var(--synara-content-heading)` "This is the start of #room" (DM: "This is the start of your conversation with Name"), topic in 15px secondary, then `Button size="300" fill="Soft"` Invite (when `permissions.action('invite')`) and Set topic (when state permitted), reusing `RoomMenu` handlers (`RoomViewHeader.tsx:131-147`). Loading state can stay as spinner + text.
- Effort: M. Risk: the intro row height must feed `estimateNativeTimelineRowSize` (`nativeTimelineViewportPolicy.ts`) to avoid anchor drift; give it a fixed estimate.
- iOS: no intro found; should get the same block later.

### ROOM-9 · P1 · Send button is a quiet glyph, not the dominant control

- Where: `RoomInput.tsx:1785-1795` (`IconButton variant="Primary" fill="None" size="300"`), `Editor.css.ts:85-95` (184px float reserved for three 32px buttons), `:129-144`.
- Today: send, emoji and formatting-toggle are all `fill="None"` quiet actions of the same size; send is distinguishable only by hue. The reserved first-line float (184px) is ~80px wider than the three buttons need, shortening the first line of every draft. There is no disabled/empty state.
- Slack: a filled accent send square (28px, radius 4) that is grey while the draft is empty.
- Proposal: send → `fill="Solid" variant="Primary" size="300" radii="300"`, `aria-disabled` + `Secondary` fill when `isEmptyEditor(editor) && selectedFiles.length === 0` (still clickable to surface the "nothing to send" no-op? better: disabled). Reduce the float to `toRem(108)` (3×32 + 2×2 + 3×2 padding). Keep `+`, formatting, emoji quiet.
- Effort: S. Risk: the e2e composer specs assert on the send button's aria-label only; styling change is safe.
- iOS: 44pt send exists; parity in "filled when sendable, muted when empty".

### ROOM-10 · P1 · Jump-to-latest shows no "new messages" count

- Where: `NativeTimelinePresenter.tsx:4203-4255` (`Jump to Last Read` pill + chevron `IconButton`), `nativeTimelineViewportPolicy.ts` (`shouldShowJumpToLatest`, `nativeVisibleReadFrontier`).
- Today: a 32px outlined chevron at bottom-right while scrolled up; new arrivals are only announced to screen readers (`:3787-3795`).
- Slack: "N new messages ↓" bar / pill while scrolled up; plain "↓" otherwise.
- Proposal: when `arrivalPositionKind` is live and rows exist after `readyState.snapshot.readState.ownReadEventId`, render the same pill as Jump to Last Read with `variant="Primary" fill="Solid"` and label `${n} new message(s)`; fall back to the chevron when n = 0. Count is computable client-side from `rows` and `rowEventId`.
- Effort: M. Risk: count must reset on `jumpToLatest`/follow-live; reuse `announcedTailRef` logic.
- iOS: has Jump to Latest / Jump to Last Read (`RoomTimelineView.swift:1283, 4328`); align labels.

### ROOM-11 · P1 · Header title size jumps per room and the header carries eight icon actions

- Where: `RoomViewHeader.tsx:511` (`size={topic ? 'H5' : 'H3'}`), `:554-871` (Call, Widgets, Search, Threads, Pinned, Notes, Members, More), `RoomViewHeader.css.ts:18-22`.
- Today: the room name is H3 in rooms without a topic and H5 with one, so switching rooms changes title size. Eight 24px icon buttons sit in a row; Slack keeps two or three.
- Slack: 18px/700 title with a details chevron, 13px topic truncated to one line, member avatar stack + count at right, ⋯.
- Proposal: fixed `H5` title regardless of topic (topic line is optional below). Keep visible: Search, Threads, Members (with count badge from `useRoomMembers`), ⋯. Move Pinned, Notes, Widgets and Call into ⋯ (or into a tab strip at the top of the side panel). Give the title a click target that opens room settings (the Slack "details" gesture), matching `HeaderTopic` hover underline.
- Effort: M. Risk: Pinned/Notes badges lose at-a-glance visibility; keep a dot on ⋯ when either has a count.
- iOS: header is platform-specific; no parity needed.

### ROOM-12 · P1 · Members drawer: mixed control sizes, no "Invite" entry point

- Where: `MembersDrawer.tsx:72-117` (header H5), `:288-355` (filter/sort `Chip size="400"` with `Text size="T200"`), `:357-389` (`Input size="400" radii="400"`), `:147-179` (rows: `MenuItem` + `Avatar size="200"` + `Text size="T400"`), `MembersDrawer.css.ts:4-6` (266px).
- Today: density is close to Slack (40px rows, 24px avatars) but chips, input and rows use three different size tokens and radii; there is no Invite action in the pane; power-level sections are `L400` labels.
- Slack: "Add people" row at the top, 36px rows, presence dots, section headers 13px/600.
- Proposal: add an "Invite people" `MenuItem` (Icons.UserPlus) at the top when `permissions.action('invite')` (reuse `InviteUserPrompt`); unify chips/input to `size="300" radii="300"`; section labels `toRem(13)/600 var(--synara-content-secondary)`; row height 36px (`estimateSize: () => 36`), name `T300`. Presence is not available from Core, so do not fake it.
- Effort: S. Risk: none.
- iOS: no drawer; skip.

### ROOM-13 · P1 · Threads panel titles threads with raw event IDs

- Where: `RoomThreadListPanel.tsx:100-102` (`{thread.latestEventId ?? thread.rootEventId}`), `nativeThreadList.ts`.
- Today: the primary text of each row is a `$…` event ID; secondary line "3 replies · participated · 2 unread". Rows are unstyled `<button>`s with `padding: S200` and no hover state.
- Slack: Threads view shows channel, participants, root snippet and latest reply.
- Proposal: Core: add `root_sender_name`, `root_body_preview` (first 120 chars of plain body), `latest_reply_ts` to the thread list summary. Desktop: row = sender 13px/600 + relative time right-aligned, root snippet 15px clamped to 2 lines, meta line "3 replies · 2 unread" with unread in Primary; `MenuItem variant="Surface" radii="300"` with hover tint. Until Core lands, never print an ID: show "Thread · 3 replies".
- Effort: M (Core + UI). Risk: low.
- iOS: thread list exists; same Core fields.

### ROOM-14 · P1 · Thread side panel lacks a subtitle and a replies divider

- Where: `ThreadSidePanel.tsx:113-128` (header: "Thread" H5 + X), `ThreadSidePanel.css.ts:12-16` (48px).
- Today: no room name, no root/replies separation; the stream shows root then replies with normal grouping rules.
- Slack: "Thread" + "#channel" subtitle; root message followed by a "3 replies" rule.
- Proposal: header subtitle `Text size="T200" priority="300"` with `#room` (`useRoomName`); in thread position, render a `SystemRow`-style rule with "N replies" (`thread.replyCount` from the root row) between the root and the first reply; make the X `quietInteractiveSurface` like other panel headers (`RoomSidePanel.tsx:33-42`).
- Effort: S–M. Risk: the divider must be a virtual row or part of the root row's bottom to keep anchor maths stable; attach it to the root row.
- iOS: `ThreadHeader` already shows a subtitle (`RoomTimelineView.swift:5667-5676`); desktop should match.

### ROOM-15 · P2 · Action menu uses raw form controls and Matrix jargon

- Where: `NativeTimelinePresenter.tsx:963-1059` (`<textarea>`/`<input>` with inline `style`), `:755-773` ("Redact"), `:825-845` ("Save for later").
- Today: Report and Forward expand inside the 240px menu as unstyled native controls; the forward target list is a column of `Button fill="Soft"` items capped at 40.
- Slack: "Delete message…" opens a dialog; Forward opens a search dialog with a message preview.
- Proposal: labels "Delete message" (keep Matrix "redact" only in the confirmation body) and "Save for later" is fine; move Report and Forward into `Dialog`s using folds `Input`/`TextArea`, with the forward list as `MenuItem`s showing room glyph + name + encryption badge.
- Effort: M. Risk: redact-confirm e2e (`native-timeline-redact-confirm.spec.ts`) asserts on `REDACT_CONFIRM_COPY`; keep those strings or update the spec.
- iOS: also says "Redact" (`RoomTimelineView.swift:7471`); rename on both.

### ROOM-16 · P2 · File attachments render as an underlined link, not a file card

- Where: `NativeTimelinePresenter.tsx:1355-1413` (`NativeTimelineFileChip`), `:1488-1506` (mime type line), `nativeTimelineHtml.css.ts:457-477` (`FileDownload`).
- Today: filename as an underlined Primary text button, a second "Download" link for markdown, and the raw MIME type (e.g. `application/pdf`) as metadata.
- Slack: a bordered card with a file-type icon tile, name 15px/600, "PDF · 1.2 MB", and hover actions.
- Proposal: `FileCard` style: `border: 1px solid var(--synara-content-separator)`, `radius R400`, 40px icon tile (`Icons.File`), name `15px/600`, meta `13px secondary` using a friendly type label (map MIME → "PDF", "ZIP", "Markdown") and size if Core exposes it; Download/Preview as `IconButton`s on hover/focus. Max width 360px.
- Effort: S–M. Risk: none.
- iOS: file rows exist; align card copy.

### ROOM-17 · P2 · Link unfurl card layout and opacity text

- Where: `nativeLinkUnfurlCard.tsx:68-100` (96px thumb left, description `opacity: 0.8`), `nativeTimelineHtml.css.ts:995-1029`.
- Today: horizontal card; description dimmed via opacity (the prior audit's "no opacity for secondary text" rule).
- Slack: 4px left bar, site name 13px, title in link colour 15px/700, description 15px, image below for landscape thumbnails.
- Proposal: `borderLeft: 4px solid var(--synara-content-separator)`, no outer border, `padding: S200 S300`; source `12px secondary`, title `15px/700 var(--tc-link)`, description `15px var(--synara-content-secondary)` clamped to 3 lines; thumbnail below the text when `width > height`, else right at 80px. Replace `opacity: 0.8`.
- Effort: S. Risk: row-height estimate changes.
- iOS: unfurl cards exist; match the left-bar layout.

### ROOM-18 · P2 · Two different "Load older messages" controls

- Where: `NativeTimelinePresenter.tsx:4099-4105` (full-width default `Button` above the scroll when sparse) and `NativeTimelineHistoryStatus.tsx:135-148` (outlined pill overlay).
- Today: both can exist for the same intent with different styling and placement.
- Proposal: keep only the overlay pill; remove `hasSparseLoadButton` block or make it render the same pill centred.
- Effort: S. Risk: `resolveTimelineHistoryOverlay` takes `hasSparseLoadButton` to suppress the pill when the button exists; invert that.
- iOS: n/a.

### ROOM-19 · P2 · Typing indicator placement, X button and the empty 28px strip

- Where: `RoomView.tsx:122` (typing above the composer), `RoomViewTyping.tsx:144-152` ("Drop Typing Status" X button), `RoomViewFollowing.tsx:14-16` + `.css.ts:12-23` (empty 28px strip under the composer).
- Today: typing appears between timeline and composer with bold names and an X that deletes other users' typing state locally; a permanent 28px empty strip sits below the composer waiting for read receipts Core does not supply.
- Slack: "Name is typing…" 13px secondary directly under the composer.
- Proposal: render typing inside the under-composer strip (`RoomViewFollowing` slot) as `T200 var(--synara-content-secondary)` with names in 600; drop the X (or gate behind the developer-tools setting); when nothing is typing, collapse the strip to `S200` so the composer sits closer to the window edge.
- Effort: S. Risk: composer height change affects "last message above composer" anchoring; the strip is outside the scroll region so no anchor impact.
- iOS: typing exists; copy parity only.

### ROOM-20 · P2 · Three inline banner styles for errors/notices

- Where: `RoomComposer.css.ts:64-71` (`ComposerError`), `nativeTimelineHtml.css.ts:104-127` (`TimelineNotice` recipe), `RoomIdentityWarningBanner.tsx:77-137` (inline `style` objects), `NativeTimelineHistoryStatus.tsx:111-134` (`HistoryStatusCardError`).
- Today: similar banners with different radii (R300/R400), margins and icon sizes.
- Proposal: one `InlineNotice` recipe (`tone: error | warning | info`, `radii R300`, `padding S200 S300`, `Icon size="100"`, optional action + dismiss) consumed by all four; the identity banner adopts it.
- Effort: S–M. Risk: none.
- iOS: n/a.

### ROOM-21 · P2 · Poll rows lack the message chrome

- Where: `NativeTimelinePresenter.tsx:2431-2487`.
- Today: no avatar or sender name (only `Time`), question in `L400`, "Poll open/closed" T300, answers as `Button fill="Soft"` with `(count)`.
- Proposal: render inside the standard row (avatar + sender + time) with the poll as the body; answers as option rows with a percentage bar (`Primary.Main` at 18%) and count right-aligned; "Poll closed" as a tertiary caption. Allow grouping like messages (`nativeTimelineRowShowsSender` would need `poll`).
- Effort: M. Risk: grouping change must be mirrored in `estimateNativeTimelineRowSize`.
- iOS: polls render; align option-bar style.

### ROOM-22 · P2 · Grouping window is two hours on both platforms

- Where: `nativeTimelineGrouping.ts:1` (`2 * 60 * 60 * 1000`), iOS `Services/TimelineService.swift:227`.
- Today: a message 119 minutes after the previous one from the same sender shows no header, so its time is visible only on hover/tap.
- Slack: compact rows only within a few minutes; a new header after ~5 minutes of silence.
- Proposal: 10 minutes on both platforms (shared constant; `NATIVE_TIMELINE_GROUP_WINDOW_MS = 10 * 60 * 1000`, `maximumInterval = 10 * 60`). Measured-size cache keys include `grouped`, so no stale heights.
- Effort: S. Risk: more headers → slightly taller timelines; product call.
- iOS: yes, same constant.

### ROOM-23 · P2 · Permanent 96px right gutter on every row

- Where: `nativeTimelineHtml.css.ts:253` (`paddingRight: toRem(96)`), `:76-87` (rail inside the row at `top: S100`).
- Today: every row reserves 96px on the right for a rail that appears only on hover, narrowing the measure in the thread pane and at 960px windows.
- Slack: the toolbar floats over the row's top-right edge (straddling the row boundary, `top ≈ -16px`), and rows keep a small right padding.
- Proposal: `paddingRight: toRem(40)`; rail `top: calc(-1 * S300)` so it overlaps the row above's bottom padding rather than the message's first line; rail already has an opaque floating surface. Pair with ROOM-1 (wider rail) and ROOM-2 (timestamp moves left).
- Effort: S. Risk: the rail may cover the end of a long first line of the hovered message (Slack accepts this); verify with a long sender name.
- iOS: n/a.

### ROOM-24 · P2 · Day header vs floating date chip use different styles

- Where: inline header `nativeTimelineHtml.css.ts:572-590` (bold 15px over rule) and floating chip `NativeTimelineHistoryStatus.tsx:76-88` + `nativeTimelineHtml.css.ts:1147-1158` (SurfaceVariant card, `T200`).
- Today: the same concept ("Today", "3 March") appears in two styles.
- Proposal: keep the inline header (it shipped and was liked). Restyle the chip as a floating pill using `floatingSurface`, `toRem(13)/700 var(--synara-content-heading)`, `radii Pill`, `padding S100 S300`, so it reads as the header lifted off the page.
- Effort: S. Risk: none.
- iOS: `TimelineDayHeader` exists; chip is desktop-only.

### ROOM-25 · P2 · "Pinned" is a plain metadata word

- Where: `NativeTimelinePresenter.tsx:2298-2302, 2679-2683`.
- Today: "Pinned" in `T200 Metadata` after the time.
- Slack: pin icon + "Pinned by X" caption above the message, with a faint accent-tinted row background.
- Proposal: `Icons.Pin` 12px + "Pinned" in `12px var(--synara-content-secondary)` as a caption line above the sender row; row background `color-mix(in srgb, color.Primary.Main 6%, transparent)`.
- Effort: S. Risk: tinted rows must stay under the hover tint; use the same `MessageRow` selector order.
- iOS: pins exist; caption parity.

### ROOM-26 · P2 · Residual opacity-based secondary text and hard-coded colour

- Where: `nativeTimelineHtml.css.ts:329` (`EncryptionShield opacity: 0.75`), `:1068` (`XPostMedia background: '#000'`), `:1081` (`XPostSource opacity: 0.75`), `nativeLinkUnfurlCard.tsx:95` (`opacity: 0.8`), `components/message/Reply.css.ts:9, 34` (`config.opacity.P300`), `RoomViewHeader.css.ts:27` (`opacity: P500` on topic hover).
- Today: the prior audit's "opaque semantic roles, not opacity" is applied to body/metadata but not to these.
- Proposal: use `var(--synara-content-secondary)` / `tertiary`; `#000` → `color.Background.Container`.
- Effort: S. Risk: none.
- iOS: n/a.

### ROOM-27 · P2 · Keyboard: no "Up to edit last message"

- Where: `RoomInput.tsx:1115-1154` (`handleKeyDown`), `RoomView.tsx:91-106` (global type-to-focus).
- Today: good coverage (Enter/Mod+Enter, Escape cancels reply, autocomplete arrows, End/Home in timeline, focusable rows with rail on focus). Missing the Slack/Discord staple: Up arrow in an empty composer edits your last message.
- Proposal: in `handleKeyDown`, when `isKeyHotkey('up', evt)` and `isEmptyEditor(editor)` and no autocomplete, find the latest own editable message in the presenter (`editingEventId` setter is presenter state; expose `requestEditLastOwn` via the existing `nativeTimelineNavigation` observer pattern) and open the in-place editor.
- Effort: S–M. Risk: needs a small cross-component channel; reuse `observeRoomLatestAfterSend` style.
- iOS: n/a.

### ROOM-28 · P2 · Call header button leads to a disabled-only menu

- Where: `RoomViewHeader.tsx:320-386, 554-594`.
- Today: the Phone button opens a menu with a disabled "Start call" and a reason line.
- Proposal: hide the button when `startCallDisabledReason({status})` is non-null (or move into ⋯ with the reason as a tooltip).
- Effort: S. Risk: none.

### ROOM-29 · P2 · Toolbar copy and sizing inconsistencies

- Where: `components/editor/Toolbar.tsx:84, 120` (`size="400"`), `:320-353` (tooltips "Strike Through"/"Block Code" vs labels "Strikethrough"/"Code block"), `:397` (Markdown toggle `size="300"`).
- Proposal: all toolbar buttons `size="300" radii="300"` like the composer actions; tooltip text = aria-label.
- Effort: S.

### ROOM-30 · P2 · Composer "+" menu and attachment affordance

- Where: `RoomInput.tsx:1428-1515` (`+` → Attach file / GIF / Poll), `components/upload-board/UploadBoard.css.ts:4-34` (upload board floats absolutely above the composer, max 400×450).
- Today: attach is one menu level down; the upload board is a floating card that overlaps the timeline bottom.
- Slack: attachments appear as inline chips/tiles *inside* the composer above the text; "+" holds the rest.
- Proposal: show selected files as 64px tiles (image thumb or file icon) in the composer's `top` slot (`RoomComposer.tsx:61-68`) with an X per tile, and keep the board only for metadata (spoiler/caption). Keep `+` for GIF/Poll/Attach.
- Effort: M. Risk: `UploadCardRenderer` state (progress, errors) must move to the tile.
- iOS: attachment tiles exist in the composer; parity.

## Already good, leave alone

- Sender/body type scale (15px/1.47, bold 15px names in heading colour), day headings over a rule, 28px reaction pills with own-accent state and the trailing add-reaction pill, mention pills (gold for me, primary for others), composer placeholder copy and AA placeholder colour (`Editor.tsx:123-138`).
- Instant, flat hover tint with no border/lift (`nativeTimelineHtml.css.ts:246-290`) and `prefers-contrast: more` edges.
- Focus handling: focusable rows, inset Primary focus rings inside the clipped rail (`:129-142`), focus return after menu/redact/editor/media viewer (`NativeTimelinePresenter.tsx:1214-1229`, `NativeTimelineMediaViewer.tsx:38-46`), redact confirm defaulting to Cancel (`NativeTimelineRedactConfirm.tsx`).
- In-place editor with Save/Cancel chips, formatting toggle and the "Enter to save · Escape to cancel" hint (`NativeTimelineMessageEditor.tsx:224-317`).
- Code blocks: language label, Copy with "Copied" feedback, line numbers hidden from AT, adaptive syntax palettes incl. increased contrast (`nativeTimelineHtml.css.ts:647-663`, `nativeTimelineFormattedBody.tsx:80-159`).
- Tables as a focusable scroll region with header/zebra roles (`nativeTimelineHtml.css.ts:867-942`).
- Links underlined, host-mismatch warning, unopenable scheme shown as text (`nativeTimelineFormattedBody.tsx:208-273`).
- Composer send queue that restores text on failure and reports queued sends (`RoomInput.tsx:777-818, 1301-1317`); drag-and-drop overlay; link preview with dismiss.
- Reduced-motion branches on reveal, typing dots, tactile surfaces; forced-colors handled centrally in `index.css:249`.
- Thread side pane: resizable with keyboard (`ThreadSidePanel.tsx:66-77`), Escape to close, own composer and drop region.

## Prior-audit items that persist

From `docs/ux-audit/desktop-cross-client-audit.md`:

- #1 syntax contrast: resolved (`NATIVE_SYNTAX_PALETTES`, `.prism-light/.prism-dark` vars).
- #2 semantic text roles: resolved for body/metadata via `--synara-content-*` (`ThemeManager.tsx:62-73`); residual opacity use persists in the spots listed in ROOM-26. Unrestricted `colorMXID` sender colour is now opt-in only (`legacyUsernameColor`, `NativeTimelinePresenter.tsx:2014-2017`): acceptable.
- #3 system font stack and heading scale: resolved (`index.css:57-58`, `nativeTimelineHtml.css.ts:756-770`).
- #4 tables: resolved.
- #5 rich-content rhythm: links ✓, reply "Replying to {name}" with sender-based accessible name ✓, membership/state rows quiet ✓; unread row is "accent-labelled" but its rules are grey (ROOM-6); redacted/other rows still carry message weight (ROOM-7).
- #8 composer: sticker/GIF/poll moved under `+` ✓; formatting in a stable bottom row ✓; send is not visually dominant ✗ (ROOM-9); error states use the critical role ✓ but three banner styles coexist (ROOM-20).

From `docs/ux-audit/grok-4.6-high-review.md`:

- P0-2 heading scale on desktop: resolved.
- P1-4 composer clearance: resolved (composer is outside the scroll region; jump controls sit inside it).
- P1-6 reduced motion / focus: largely resolved; `forced-colors` handled once in `index.css:249` rather than per surface (acceptable).
- P2 "Metadata rhythm: sender/time/reaction grouping": partially open; see ROOM-2, ROOM-5, ROOM-22.

From `docs/ux-audit/grok-4.6-high-final-diff-review.md` (desktop items): the computed-style contrast test gap remains a test gap, not a UI defect; no UI action here.
