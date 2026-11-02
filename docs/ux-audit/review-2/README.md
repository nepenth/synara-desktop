# Slack-alignment review 2 (2026-11-01)

Second deep UX/UI review across desktop, iOS and iPad after the 2.7.5/2.7.6 timeline restyle.
Six parallel reviews (Fable 5.1, high effort), code reading only, every finding cited to file:line:

| Report | Area | Findings |
| --- | --- | --- |
| [review-desktop-nav.md](review-desktop-nav.md) | Desktop sidebar, room list, launch route, search entry | 23 |
| [review-desktop-room.md](review-desktop-room.md) | Desktop room header, timeline, composer, threads, members | 30 |
| [review-desktop-settings.md](review-desktop-settings.md) | Desktop settings, dialogs, approvals, verification, updater | 30 |
| [review-ios-timeline.md](review-ios-timeline.md) | iOS timeline, thread screen, composer | 30 |
| [review-ios-shell.md](review-ios-shell.md) | iOS tabs, room list, Later, Settings, login, iPad split layout | 30 |
| [review-design-system.md](review-design-system.md) | Cross-platform tokens: type, colour, radii, avatars, icons, words, motion | 22 + alignment map |

## Shared decisions applied on both platforms

- Message actions, in this order everywhere (desktop hover rail, iOS context menu):
  React · Reply · Reply in thread · Edit (own) · Forward · Save to Later · More (Copy, Select text, Pin, Delete).
- "(edited)" lowercase, inline after the body, meta size.
- Unread divider reads "New messages" over a thin red-tinted rule.
- Badges: red count badge only for mentions and unread DMs; plain unread is a bold name.
- Reaction pill: 28 tall, radius 14, 8 % label fill, 16 emoji, 13/600 count; own = Primary 18 % fill + Primary border.
- Mention gold base `#F5B400` on both platforms; Primary accent `#1245A8` light / `#8DB4F5` dark on both
  (iOS gains an `AccentColor` asset).
- Type scale 11 / 12 / 13 / 15 / 17 / 20 / 24 (caption / meta / label / body / title / heading / display);
  desktop nav rows move to 15 px like the timeline. iOS keeps Dynamic Type `.body` as its base.
- Icons: Later = bookmark, Notes = note/document, Threads = one glyph, Edit = pencil only.
- Copy: sentence case; "Rooms", "Direct messages", "Later", "Threads", "Approvals", "Notes".

## Wave 1 (this branch, done)

Desktop room view: ROOM-1, 2, 4, 5, 6, 7, 9, 10, 11, 12, 23. Desktop nav: NAV-1 to 11, 13.
Desktop settings: SET-1 to 11. iOS timeline: IOST-1 to 8, 10, 11, 13, 15, 16, 17, DS-5.
iOS shell: IOSS-1 to 13, 15, 16, 17, 18, 19, DS-1, DS-2, DS-7. Identity-reset copy is sentence case on both.

Verified: desktop tsc/eslint/prettier clean, 1409 unit tests, six Playwright suites (Chromium);
iOS build-for-testing and 1100 unit tests. Not yet judged in a running app with real rooms.

Gaps found while implementing: iOS has no "Save to Later" API, so the context menu lacks it (DS-9);
Core `TimelineThreadSummary` only carries a count, so thread chips use the loaded window for
participants and recency (ROOM-3/IOST-8); `RoomSummary` carries no topic or creation date for the
room intro block; the redaction confirmation copy ("Remove this message?") is still shared and
should move to "Delete" on both platforms; the iOS timeline header should adopt
`SynaraMetrics.columnHeaderHeight`; the iOS Threads glyph is still four symbols (DS-7).

## Wave 2 (this branch, done): the six decisions

Decided on 2026-11-02, all six as recommended:

1. NAV-12 — desktop shows Favourites, Rooms and a collapsible, activity-sorted Direct messages section in one
   Home list. The Direct rail tab stays as a filter (reopens the last DM, expands and scrolls to the section);
   the Home badge counts room mentions plus unread DM messages; `/direct/...` paths are unchanged.
2. IOSS-14 / DS-8 — "Activity" on both: the desktop rail/pane formerly "Inbox" (route segment `inbox` kept,
   quick switcher accepts both words) and the iOS tab formerly "Notifications" (deep links accept `activity`,
   `notifications`, `notification` and `/inbox/...`). Settings > Notifications keeps its name.
3. DS-8 — "Rooms" everywhere; the iOS "Channels" header and its unused catalog keys are gone.
4. DS-10/11 — desktop `colorMXID` mirrors iOS `SynaraAvatarPalette` (FNV-1a, lower-cased ID, high 32 bits
   mod 8, hues 212/248/285/330/8/28/140/195). Avatar ladder 20/24/28/36(–40)/44/64 with a 22 % corner on
   both platforms. A Core-provided colour stays deferred.
5. IOSS-1/2 — the title menu is the space switcher: per-space unread counts, "All spaces" when a space is
   selected, collapse state persisted. The chip strip stays off.
6. DS-9 — iOS "Save to Later" (bookmark) after Forward in the message menu and VoiceOver actions, writing
   the same `LaterItem` desktop writes through Core's upsert, with a two-second "Saved to Later" notice.
   No trailing swipe: a leftward drag already reveals timestamps.

Verified: desktop tsc/eslint/prettier clean, 1414 unit tests, six Playwright suites (Chromium);
iOS build-for-testing and 1107 unit tests; iOS UI tests not run. Released as 2.8.0.

Known gaps after wave 2: the selected space in the iOS title menu carries the checkmark but no unread
subtitle; locale keys used only by the deleted desktop Direct pane are unused; a Rust push-rules fixture
string still says "New inbox notification"; the 22 % corner applies to the resized avatars only.

## Deferred (P2 or needs Core work)

Thread summaries with participants and last-reply time (Core `TimelineThreadSummary`), Core-side collapsing of
membership runs, 2 h grouping window review, keyboard pane toggle / next-unread, Silver/Butter theme parity,
increased-contrast tokens on desktop, reduce-motion gates on iOS.
