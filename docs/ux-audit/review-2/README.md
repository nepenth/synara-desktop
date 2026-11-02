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

## Wave 1 (this branch)

Desktop room view: ROOM-1, 2, 4, 5, 6, 7, 9, 10, 11, 12. Desktop nav: NAV-1 to 11, 13, DS-7.
Desktop settings: SET-1 to 11. iOS timeline: IOST-1 to 8, 10, 11, 15, 16, 17, DS-5.
iOS shell: IOSS-1 to 13, 15, 16, 17, 19, DS-1, DS-2, DS-7.

## Needs the user's decision (not done)

1. NAV-12 / IOSS-… — show Rooms and Direct messages in one list (Slack) instead of a separate rail tab / tab.
2. IOSS-14 / DS-8 — rename the iOS "Notifications" tab and desktop "Inbox" to one name ("Activity").
3. DS-8 — iOS says "Channels"; desktop says "Rooms". Pick one word.
4. DS-10/11 — avatar fallback colours: share one hash and palette (Core-provided) so a user looks the same on every device.
5. IOSS-1 — keep space chips in the Rooms header, or fold space switching into the title menu.
6. Later on iOS (DS-9): iOS cannot save a message to Later today; add it to the context menu.

## Deferred (P2 or needs Core work)

Thread summaries with participants and last-reply time (Core `TimelineThreadSummary`), Core-side collapsing of
membership runs, 2 h grouping window review, keyboard pane toggle / next-unread, Silver/Butter theme parity,
increased-contrast tokens on desktop, reduce-motion gates on iOS.
