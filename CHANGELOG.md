# Changelog

## Unreleased

- Room list (desktop and iOS): signing in or starting with an empty cache no longer fetches history for every room. The first sync used to send about two `/messages` requests per room (49 on a 41-room test account, and a refused one for a pending invite); it now sends none. Each room arrives with its last 5 events for its preview and unread count, and only the 20 most recent rooms plus the open one are fully synced. Rooms with nothing to preview in that window sort by the server's activity order instead of alphabetically.
- Room list: your own membership previews as "You joined" instead of your name, or as "Encrypted message" when newer messages can't be decrypted yet. Badges in encrypted rooms are counted on the device only, so encrypted reactions and edits no longer show as unread on servers that count them; unencrypted rooms still use the server's counts when it sends any.
- Room list: rooms that name a space as their parent are grouped under it when the space's admins put them there, as in Element, even if the space does not list them. Removing a room from a space also removes the room's link back to the space when you are allowed to.
- iOS room list: spaces no longer appear as channels, low-priority rooms get their own "Low priority" section at the bottom (on desktop they move to the bottom of the list), and a list that has not synced yet shows the loading placeholder instead of "No Rooms".
- Presence: other people's online status now shows on desktop and iOS. Sliding sync never delivers it, so Synara reads it from the server for the people you are looking at, at most once a minute each.
- Background work: if the room-list owner or a sync watchdog stops on an error, it restarts instead of leaving the room list without updates for the rest of the session. Bursts of updates no longer push other pending updates out of the iOS update queue, and invites cost fewer database reads per refresh.
- iOS: a message you send is never lost if Synara is closed or killed before it goes out. Every send goes straight into the persistent send queue, also while Synara is still connecting or offline, and is sent when the connection is back; a message sent before Synara had a session is kept on the device and handed over on the next launch. Text you typed but did not send stays in the composer across relaunches, and is removed when you sign out.
- iOS: sending the same text twice ("ok", "ok") shows and sends both, each exactly once, and a retry or reconnect never sends a message a second time. A message that is still sending shows a spinner on its bubble; one the server refused shows Retry and Discard on the bubble (Discard only when retrying cannot help, such as a message too large for the server), and Discard is also in the message's actions while it is sending. Retry resends the same message instead of adding a copy.
- Desktop: sending a message returns as soon as it is queued, so the composer is ready again at once. When the server refuses an attachment or edit, the error says why (too large, not allowed, encryption) instead of a generic refusal.
- Sending (desktop and iOS): after a failed send because of a network or server error, Synara waits 5 to 60 seconds before trying again (or until the connection is back) instead of retrying at once, and it no longer wakes the send queue while the app is suspending.
- Timeline: a room that is slow to open, jump to a message or load older messages no longer holds up images, reactions, snapshots or opening other rooms, on desktop and iOS. Opening a room no longer waits up to 10 seconds for background history loading, loading older messages gives up after 20 seconds instead of retrying for minutes, and after a laptop sleep or a long background a busy room offers its earlier messages again instead of claiming it has reached the beginning.
- iOS: Synara's core runs on several threads instead of one, so a large sync, a decryption burst or a slow Keychain call no longer stalls message updates, sends and the notification extension's shared key-store lock.
- iOS: opening a room from a notification loads its earlier messages and keeps receiving new ones, the same as opening it from the room list. You no longer need to leave the room or relaunch Synara to see its history.
- Desktop: click an image to view it full size, or use the corner button on a video; the viewer can save the original. An image or video that can't be shown falls back to its file. Inline thread replies show formatting, and a thread that fails to load or page offers Retry. Search results, pinned messages, the notifications inbox and room topics now render text the same way as the timeline, including mentions and links that open in Synara.
- Desktop: less work in the background. Room-list updates no longer get lost while a refresh is running, a change to one room no longer re-renders every room, polling slows down while the window is hidden, and sending a message no longer reloads the timeline. Starting a direct message reuses an existing one, the sign-out warning about encrypted rooms works again, and about 10,000 lines of unused code are gone. Error messages across the app explain what went wrong instead of showing raw error text.
- Desktop timeline: "Hide Profile Change" hides only membership changes, so room name, topic, permission and encryption changes always show. Editing a message happens in place with the normal composer and keeps its formatting and mentions. Custom emoji, matrix.to links (which open in Synara) and messages that mention you render properly, and links show where they go. Load errors and gaps say what happened and offer Try again or Go to latest instead of bare text. Media auto-load, user name colours and Show Hidden Events settings now take effect, and hiding read receipts no longer stops Synara from marking rooms read. Screen readers get landmarks, a room heading, announcements of new messages and proper menus; animations respect Reduce Motion.
- Desktop notifications: clicking a notification for a direct message or a room in a space opens the conversation instead of a "View" card. Messages are no longer dropped while the Notifications list was the last page you opened. Notification sounds play only when a banner is shown, delivered notifications and the badge clear when you read a room elsewhere or sign out, invite notifications open your invites, and Hermes action buttons work every time.
- Desktop composer: the message box clears as soon as you send, so you can keep typing while a slow send finishes, and messages always go out in the order you sent them. If a send fails, its text comes back into the box (with anything queued after it) and the error says what went wrong instead of "Could not send message". Sending in the room no longer closes an open thread pane. Enter picks the highlighted mention, command or emoji suggestion; Ctrl/Cmd+Enter always sends. Slash commands report failures and usage mistakes. The homeserver's upload size limit is applied, staged files no longer show a false "uploaded" check, and typing with an input method (Japanese, Chinese, Korean) no longer triggers Escape, Backspace or Tab actions mid-composition.
- Desktop accessibility: dialogs are announced as dialogs, take focus when they open, close on Escape and return focus to what opened them, and the app behind them can't be reached by keyboard or screen reader. Menus return focus when they close. Keyboard focus is clearly visible in the room list, text fields and the Following bar. About 100 icon-only buttons and settings switches now have names. The room list supports arrow keys, Home and End, and the quick switcher (Cmd+K) works as a screen-reader combobox. Timestamps, read room names, muted text, the composer placeholder, links and light-theme user name colours now meet WCAG AA contrast, so several of them are a little darker. Links in messages are underlined. Page zoom goes up to 200%.
- iOS: notification previews and approval Approve/Deny keep working when Synara has been in the background for a while. Signing in on iPhone no longer requests a refresh token, so the access token the notification extension uses does not expire after a few minutes (Synapse's default is 5). Sessions signed in with an earlier version keep refreshing in the app; if previews still show generic text after an update, sign out and sign in again. The notification diagnostic report counts previews that failed because the server rejected the session. Desktop is unchanged.
- Media: photos, videos and files you send to an encrypted room load on the device that sent them, on desktop and iOS. Ones that stayed broken after an earlier send load again too.
- Media: large images, videos and file downloads no longer fail after 30 seconds on a slow connection. A download now fails only when the server stops sending for 30 seconds.
- Opening a room or the security settings no longer starts a loop of encryption-status requests, on desktop and iOS. Accounts without cross-signing saw about 60 requests a second and 20-27% CPU while a room was open; status is now read again only when it changes.
- iOS: sending a photo or file while the connection is slow or the server is failing no longer crashes the room. The attachment shows as sending, with your own preview, until the server has it, instead of a broken image.
- Threads: a notification or link to a reply in a thread opens that thread at the reply, with the room behind it, and new replies keep arriving. Previously the room showed only that thread's replies, out of order, as if they were the room's messages.
- Desktop: opening a notification or link for a recent message keeps the room live, so newer messages appear without Jump to latest.
- A notification or link to a message that was deleted, or that you can no longer see, now says so and opens the room at its latest messages instead of showing "Messages are temporarily unavailable".
- iOS: rooms stay live while you read history. Scrolling up to load earlier messages no longer stops new messages, edits, reactions and read receipts; back at the bottom they are there without Jump to latest, and the messages you were reading stay in place. Opening a room whose unread messages fit on screen marks them read without a scroll. Rooms whose recent events are mostly hidden ones load earlier messages on their own, and history after a reconnect can be loaded again. Jump to Last Read hides while the last-read message is on screen, scrolls to it when it is already loaded, and otherwise opens it without closing the live room, or says it couldn't be loaded. A message opened from further back loads newer messages as you scroll down and becomes the live room again at the bottom.
- iOS: room and direct-message invites now appear in Rooms, first in their section, and in Notifications, where you can accept or decline them. One invite whose details can't be read no longer hides the others, on desktop and iOS; desktop shows it as "Sender unknown".
- Desktop: actions that fail now say so where you acted, keep what you typed, and never report success they didn't achieve. This covers moving rooms between spaces, adding existing rooms, blocking and unblocking users, bulk invite actions (which now continue past a failure and list what didn't work), notification settings, space reordering, marking rooms read or unread, room notes, Later items, and cancelling verification. The identity-change banner says when it couldn't check a room instead of showing nothing.
- Desktop: logging out clears unsent drafts and per-account sidebar layout. The login and registration fields work with password managers, and a session that signs in but can't open offers Retry instead of an endless spinner.
- Sign-in: iOS finds your homeserver through `.well-known/matrix/client`, so typing a server name such as example.org signs in at the server it delegates to (for example matrix.example.org). The sign-in screen still shows the address you typed, and a bare username uses that server name. Both apps refuse to sign in to a server without simplified sliding sync (MSC4186) and say so, instead of creating a device that never syncs. An existing session on such a server stops retrying sync every 5 seconds, and iOS shows the same message in its connection banner instead of "Connecting…". iOS explains that servers offering only single sign-on are not supported yet. A login response that names a different homeserver URL no longer fails sign-in with an identity mismatch.
- Approvals (iOS): Approve once on a notification asks you to unlock your iPhone first. To approve from the Lock Screen and Apple Watch without unlocking, turn on Settings → Notifications → Approve once without unlocking (off by default). Deny still works without unlocking. The settings text now describes what each setting actually does.
- Approvals: urgent approval alerts are harder to abuse. Once you list your agents in Agent Notifications, only those accounts can raise them; a prompt from anyone else arrives as an ordinary message. Without a list, approval alerts follow the room's notification settings, so a muted room stays silent. A room can raise at most three urgent approval alerts per minute; more arrive as ordinary messages. Desktop approval alerts name the room.
- Approvals: after you approve or deny in the timeline, the approval card shows your decision and when you made it instead of the buttons, on iOS and desktop and on every device. On iOS your own ✅ stays visible in the reactions below the card; a reaction whose count changed no longer disappears.
- iOS: the Approvals inbox refreshes every few seconds while open, so a new request appears without pulling to refresh. In Notifications, approval requests are labeled Approval instead of Mention.
- iOS: opening a room, tapping a notification or sending while Synara is coming back from the background waits for the connection to reopen instead of failing with "Messages are temporarily unavailable" or "Sign in again". A failed room-list refresh keeps your rooms and unread badges instead of showing "No Rooms".
- iOS: pulling down Control Center or Notification Center, a system alert, a permission prompt or a glance at the app switcher no longer disconnects Synara; it disconnects only when it goes to the background.
- iOS: after switching between Wi-Fi and cellular, or turning on Low Data Mode, Synara reconnects at once instead of receiving messages up to a minute late while showing "Connected". Network changes are handled one at a time and restart only the sync connection.
- iOS: restoring a session never reports another account's live session as already restored.
- Timeline: busy rooms cost less to keep open, on desktop and iOS. Read receipts moving between other people no longer re-send unchanged messages, reaction details are looked up once instead of on every update, and iOS refreshes a room by copying the rows it already has instead of rebuilding every message (about 40 times less work for a 300-message window), and only when that room's own timeline changed. iOS closes a room's timeline when you leave it, and stops updating one nobody has read for 10 minutes.
- Desktop: a timeline refresh that lands while new messages are arriving no longer shows a message twice or puts an edit or deletion on the wrong message, and the same holds right after a room's permissions change.
- iOS threads: a reply you send in a thread shows once instead of twice, and goes through the same send queue as room messages, with the same sending state, Retry and Discard. Threads open at their newest reply and follow your own replies, show messages the way the room does, and offer the same message actions as the room (Reply, Edit, React, Redact). A reply in a thread stays in that thread.
- iOS: coming back to a room (from a full-screen thread, another tab or Settings) keeps your place instead of reloading the room. The "Unread messages" divider stays where it was until you leave the room, even after the room is marked read, and never sits above a message you sent. A room that failed to load tries again by itself when the connection is back or when the room appears in your room list, and a room opened while Synara is reconnecting says it is waiting to reconnect instead of "temporarily unavailable".
- iOS timeline: messages from different days are separated by day headers (Today, Yesterday, the weekday, or the date). Joins, leaves and room changes such as a new name or topic show as small centered lines instead of chat bubbles from the person who made them. A message Synara can't display says so instead of naming its event type, and the approval card no longer shows developer wording.
- iOS: agent rooms are dark only inside the conversation; the rest of the app, the status bar and the tab bar keep following the system appearance.
- iOS: a notification that carries only a message id (no room) opens the right room at that message, or its thread, once the message has synced, instead of the Notifications tab.
- Timeline: scrolling to the bottom of a thread no longer shows a loading error, and a message opened from further back says when it has reached the newest messages, from the server's answer rather than by guessing (iOS stops asking sooner). Jump to latest that fails to load keeps the room you were reading instead of losing it. After following a room back to its latest messages, the unread marker no longer points at where you opened it. In encrypted rooms with many undecryptable messages, rooms, reactions and Mark as Read no longer start failing after a long session.

## [2.2.6] - 2026-10-12

- Approvals: Approve once and Deny on agent approval notifications work from the iPhone lock screen and from Apple Watch. They run in the background without unlocking or opening Synara; Synara opens its stores for one bounded decision and closes them before suspension, and shows "Approval not sent" if the decision could not be sent. Approve always remains in-app only.
- macOS: the first approval after launch no longer arrives without Approve and Deny buttons; Synara waits for the notification category to register before posting.
- Desktop: approval alerts are no longer withdrawn when the agent room is selected and the window gains focus.
- Linux: approval alerts say to open Synara when the notification server cannot show action buttons.
- Dependencies: update quinn-proto to 0.11.19 for RUSTSEC-2026-0338 through RUSTSEC-2026-0342.

## [2.2.5] - 2026-10-11

- iOS: threads work. Opening a thread loads its replies from the thread itself instead of showing only the first message, earlier replies load on request, and sent replies stay visible. Messages have Reply in Thread. Settings → Appearance → Thread Display matches desktop: Inline (the default) expands the latest replies beneath the first message, Sheet opens the thread over the room, and Full Screen opens it on its own screen. Opening a thread no longer resets the room's timeline.

## [2.2.4] - 2026-10-09

- Sign-in: the Synara mark is centered over the name, the "y" in Synara is no longer clipped, and the background grid no longer moves.

## [2.2.3] - 2026-10-09

- Sign-in: redesigned login, registration and password reset with an animated Synara mark, a brand panel and a glass card that follow your light or dark system theme and stack on narrow windows. Changing the homeserver no longer makes the username and password fields blink or clears what you typed: the form stays in place, a status line under the server field shows Connecting, Connected or a short error with Retry, and Sign in stays disabled until the new server is ready.
- Approvals: agent approval notifications on macOS, Linux and iOS show what is being approved, for example "forge: find -delete" with the requested command below it, alongside Approve once and Deny. Secret-like values are redacted. Turn off Show Approval Details in notification settings to show only the room or agent name. This setting is separate from message previews.

## [2.2.2] - 2026-10-09

- Settings: search finds individual settings, not just page names. Results show the setting and its description grouped by page; choosing one opens the page and scrolls to that row. Room and Space settings search their own rows too.
- Threads: Inline is now the default thread display. A choice made in Settings → Appearance is kept; until you choose, the default applies.

## [2.2.1] - 2026-10-08

- Linux: the title strip with Minimize, Maximize and Close renders and drags again, and the three controls share one size.
- iOS: going back to the room list after opening a room from a notification shows the rooms instead of loading placeholders forever.
- Timeline: a normal, always-visible scrollbar replaces the 7-day date rail, which covered the scrollbar and placed its dots by time rather than scroll position; hovering a message shows a steady tint with no border, lift, shadow or fade.
- Threads: Settings → Appearance → Thread Display opens threads in full view (as before), in a resizable side panel beside the live room, or inline beneath their first message.
- Settings: App, Room and Space settings share one redesigned layout: a larger dialog, grouped navigation with settings search, a centered column with one card per section, aligned controls and a Danger Zone for room upgrades.
- Menus: room and space context menus use one flat item style, so Invite matches the other items.
- Fix the error screen when opening the room Call menu while calls are unavailable, and the crash when opening a space.

## [2.2.0] - 2026-10-08

- Sessions: desktop and iOS share one Core session lifecycle; a rejected session retires locally without contacting the homeserver, logout is bounded and race-free, waits briefly for the key backup and warns on the last device, and a closed command gate shows as Connection Lost with static diagnostic ids in the log.
- Mark as Read: targets the newest event including thread replies, always writes the fully-read marker and private receipt, and reports failures.
- Messages: unsent messages show sending or failed with Discard and Retry; still-queued sends stay visibly queued and recoverable failures resend once sync recovers; reactions work in thread and permalink views and redactions go through the send queue.
- Sync: shows Reconnecting until a sync response arrives, restarts immediately after an expired sliding-sync session, re-applies open-room subscriptions, recovers a stopped sync service, and no longer restarts a healthy sync when the window is shown again.
- Encryption: authenticity shields on messages and identity-change warnings above the composer on desktop and iOS; QR verification asks for confirmation; room-key forwarding is off and history comes from the encrypted key backup, which now downloads on decryption failure; backup setup always shows the recovery key; late-arriving keys still produce notifications.
- Linux: closing the window hides Synara to the tray; the borderless window drags from the title strip on Wayland and stays undecorated across relaunch.
- Room list and badges: Core owns ordering, favorites, unread attention and badge totals for desktop and iOS (the desktop badge no longer double-counts rooms inside spaces), pushes room-list updates as events instead of polling, and evaluates room permissions with the room version's rules.
- Composer and rendering: desktop markdown renders through Core like iOS; Core sanitizes incoming message HTML before either app renders it.
- Desktop: the renderer runs on typed native modules instead of a js-sdk-shaped client; space order and pins, recent emoji and Developer Tools account data persist again, mutual rooms, local addresses and alias checks load natively, room upgrade opens the replacement room, `/delete` uses bounded bulk redaction, and `/ignore`, `/unignore` and jump-to-date work again.
- Performance: media is cached in memory and the encrypted store with thumbnails for avatars and token-refresh-safe downloads; polling slows while the window is hidden; unchanged timeline snapshots are skipped; idle timelines are evicted; startup probes no longer block the first paint; quit stops sync within 2 seconds.
- Internals: every command is a typed Core method called directly by desktop and iOS, the UniFFI boundary is proc-macro only with a pinned Swift API surface, closed status fields reach Swift as enums, and the renderer's wire types are generated from Rust.
- CI and release: a fast pull-request gate with parallel Rust test shards, a nightly heavy suite, warm release caches, release reuse of the proven CI gate and notarization retries.

## [2.1.46] - 2026-10-06

- Sessions: stop rejected refresh-token recovery loops, present expired-session guidance, and keep homeserver capability checks anonymous.
- Persistence: track failed credential saves, retry local writes under the active session lease, and retain bounded diagnostics while preserving encrypted history when retiring rejected credentials.
- iOS: distinguish device verification from decryption failures, connect Retry Decryption to the Core-owned live timeline, and avoid unmeasured session-wide decryption counts.
- iOS: update the Support address to synara-support@whyland.com, reduce space above Favorites, and join the composer background to the keyboard.
- Dependencies: integrate the reviewed renderer updates and patch Seroval and source-map-js advisories.

## [2.1.45] - 2026-10-05

- Desktop: let restored sessions start syncing without a successful homeserver metadata probe; improve native sync startup and connection Retry progress and failure handling.
- Dependencies: integrate the Rust and JavaScript dependency updates, including ESLint 10 and Prettier 3, and retire the frontend asset-copy dependency.
- Builds: adopt pinned Kache 1.0 compiler caching for local and CI Rust builds, narrow compilation inputs, preserve unchanged Apple bindings/frameworks and reuse bounded native Xcode compilation caches.
- CI: designate trusted cache writers and compatible cache families, add unsigned release-cache seeds and an isolated benchmark, and verify the Kache daemon lifecycle before compilation.

## [2.1.44] - 2026-10-04

- Notifications: add account-synced agent preferences for Tool activity, Commentary, and final/other responses while preserving verified approval prompts; expose controls on desktop and iOS.
- Apple: strengthen authenticated preview resolution and safe fallback presentation, add privacy-safe diagnostics Copy/Share, and use Time Sensitive delivery for eligible approvals with refreshed signing profiles. Critical alerts and complete background filtering remain conditional on Apple's restricted entitlements.
- Desktop: make Direct Messages unread badges consistent with native room classification, room lists, Mark as Read, and navigation; remeasure live edited Hermes Markdown to correct stale timeline row offsets.
- Release: validate and embed the macOS Developer ID notification profile, verify packaged signatures and capabilities, and extend Chromium/WebKit, Swift, Core, and signing regressions.

## [2.1.43] - 2026-10-02

- Matrix: consolidate shared session, encryption recovery, notifications, and timeline behavior in the Rust core used by desktop and iOS; strengthen cancellation and account-generation ownership.
- Security: keep recovery keys and passphrases out of generic command payloads and diagnostics, and make session wipe and restoration authoritative in the native store.
- Desktop: retire legacy Matrix JavaScript, IndexedDB deletion, unused call/PWA scaffolding, and obsolete runtime polyfills while retaining the React interface.
- Dependencies: consolidate the four Rust packages into one workspace and lockfile, update supported Rust and frontend dependencies, and record the disposition of all four dependency-update PRs.
- iOS: harden notification approval, deduplication, receipt handling, and extension isolation; preserve composer text and timeline position during updates.
- Validation: expand SDK-backed recovery/session fixtures, cross-device Synapse coverage, async bridge checks, browser runtime checks, and Apple archive validation.

## [2.1.42] - 2026-09-26

- Prepare the public project README with desktop, iPhone, and iPad interface images captured from disposable sample accounts, plus contribution and security guidance.
- iOS: bring composer formatting closer to desktop, preserving every line of pasted quotes and exposing Markdown controls in the native input.
- iOS: keep the compact composer immediately above the home indicator and dismiss the Connected recovery banner after 12 seconds even with repeated sync updates.
- Desktop: load the packaged React interface inside Tauri and tighten Matrix media, local file, and secret-storage handling.

## [2.1.41] - 2026-09-23

- iOS: keep typed and pasted composer text legible in dark appearance, and return to the live tail after sending from unread history. Keep Later navigation reliable on iPhone and in the iPad split canvas.
- iOS: use a compact stacked conversation canvas on iPhone and in narrow iPad windows, and a regular-width list-plus-conversation split on iPad. Resizing keeps the same room selected; the split canvas keeps the tab bar visible and drops the back-to-list chevron because the room list stays visible.
- Linux: show a red unread dot in the small panel icon, and include message spacing in each measured timeline row to prevent adjacent messages from overlapping. Numeric dock badges still depend on the desktop's launcher support.
- Widgets: include a self-hostable sample agent priority and status widget with a token-protected update endpoint.
- Notifications: verify the iOS notification service extension's production feature isolation and archive exports.
- Matrix: preserve restricted space hierarchy eligibility rooms and explain invite or membership requirements in the desktop space browser.
- Profile settings: show clear retry guidance when the homeserver rate-limits display-name or avatar changes on desktop and iOS.
- Integration: update the disposable Synapse baseline to 1.161.0 and add a version-12 smoke script for a future disposable 1.162 server.

## [2.1.40] - 2026-09-18

- Center date-rail ticks on the 2px spine and keep live bottom on the latest
  mark.
- Keep grouped messages still when the hover timestamp appears. The time fades
  in over the right gutter instead of translating the row.
- Linux: closing the window keeps the dash running indicator and restores the
  existing session instead of opening a second copy. The notification-area
  icon paints a Slack-style unread count, and Ubuntu Dock / Dash to Dock / KDE
  receive the Unity launcher badge.
- macOS: notification pings play through NSSound so they no longer appear as
  Now Playing media on the menu bar and Touch Bar.
- Pin Matrix Rust SDK 0.19.1 (`matrix-sdk`, `matrix-sdk-ui`,
  `matrix-sdk-crypto`, `matrix-sdk-sqlite`). The patch only ships the rustdoc
  Mermaid script that docs.rs needs; client behavior is unchanged.

## [2.1.39] - 2026-09-17

- Render markdown attachments with lists, GFM tables, task prefixes, strikethrough,
  and a Copy control. The native date rail shows a 7-day calendar of bubbles,
  hover date+time chips that sit above the messages, and older-tick jumps through
  `timestamp_to_event`.
- Keep historical loading chrome at the top of the timeline. Wheel past the
  newest message no longer starts forward pagination or strobes
  “Loading newer messages”. First paint measures history edges without waiting
  for a scroll event.
- Keep reaction-picker search results when pack lists are replaced, and hide the
  emoji catalog while a query is active. Inbox Notification Messages reload
  without wiping groups on joined-room identity ticks; room-list polls reuse
  the previous snapshot when ids and unreads are unchanged.
- Put Widgets under Settings → General on this device only. The room header
  Search works in encrypted rooms. Call chrome reports MatrixRTC status and
  does not pretend Start/Join is available.
- Browse in-room Media and Files over the last 7 days without a search keyword,
  walking the event cache and `/messages` instead of homeserver `/search`.
- Keep desktop dock/tray and iOS app-icon badges on the unread summary. Linux
  shows the count in the tray title when Unity dock badges are a no-op. iOS no
  longer zeros the icon on foreground or notification tap. The iOS Rooms tab
  badge omits invited rooms the same way the icon does; invites stay on Inbox.
- Quiet settings, room, and space left nav. Invite matches other menu rows
  instead of reading as selected. Composer plus and GIF/poll menus dismiss on
  outside click. Native Appearance no longer shows a Message Layout card or
  other-client names.
- Stop exporting `MACOSX_DEPLOYMENT_TARGET` from the SynaraCore generator so
  Xcode 27 host proc-macros can dlopen.

## [2.1.38] - 2026-09-17

- Adopt Matrix Rust SDK 0.19.0 (`matrix-sdk`, `matrix-sdk-ui`,
  `matrix-sdk-crypto`, `matrix-sdk-sqlite`) and Ruma 0.17. Pin the desktop
  toolchain to Rust 1.96 (upstream MSRV). Enable `rustls-aws-lc-rs` because
  0.19.0 no longer installs a rustls provider when default features are off.
  Map room-list subscriptions onto `set_room_subscriptions`, and write
  presence through `Client::set_presence` so sliding sync keeps the chosen
  state. Apply Dependabot rust-updates in `src-tauri`: tauri 2.11.5,
  getrandom 0.4.3, reqwest 0.13.5.
- Recover remote reaction annotation ids from the room event cache and a lazy
  `/relations` fetch so the native viewer can redact other senders. Self
  unreact still uses `Timeline::toggle_reaction` and does not wait on a missing
  id. After a recoverable RoomSendQueue failure, re-enable the queue so later
  sends are not stuck behind a silent disable. Pin panel, last-message
  pagination, and event-cache subscribe use the 0.19 cache APIs.
- Jump the native room date rail across the whole room with
  `timestamp_to_event` / `get_event_by_timestamp` Forward. Skip the center
  date chip while the rail is shown. Generic file chips download through
  `matrix_media_download`; markdown attachments open an in-client preview
  with Download still available.
- Open native thread timelines from the chip and Reply in thread. Live
  timelines use `TimelineFocus::Live` with `hide_threaded_events`. A room
  Threads panel lists threads; mark-read on a thread stream sends a threaded
  private receipt and overlays unread counts without clearing the room unread
  flag from an empty thread.
- Fill joined DM avatars from the peer MXC, subscribe to own-profile updates,
  show honest MSC1763 retention copy, and apply a global MediaRetentionPolicy.
  Link unfurls use `get_media_preview` for a single safe http(s) URL and skip
  encrypted or unknown rooms.
- Discover MatrixRTC transports after login and show live-call chrome without
  Join/Leave or widgets. MSC4426 user status and in-call fields are
  display-only; writes fail closed and `set_call` / automatic call status stay
  unused.
- Attach an MSC3814 dehydrated-device backup Olm catcher after session
  install; stop it on logout without deleting backup rows.
- Offer show-QR device verification with SAS fallback on hosts that can render
  the QR SVG. iOS stays SAS-only. QR login / MSC4388 is not in this release.
- Host experimental widgets in an isolated default-off webview with a
  deny-by-default capability filter. Compile-in of `experimental-widgets` is
  not user enablement.
- Replace native `/search` with the 0.19 EncryptedDirectory local index on
  desktop (`search-index` feature). Off disables product search with no
  Client-Server fallback. NSE and iOS keep typed `/search` and must not pull
  Tantivy.
- Enable experimental MSC4362 encrypted state events. The account setting
  gates create/opt-in only; already flagged rooms still decrypt. Call rooms
  never opt in.
- Offer experimental X.509 identity verification beside SAS, default off.
  CA-trusted peers get a distinct `verified_by_certificate` chip.
- Enable automatic room-key forwarding on desktop and iOS full-app. NSE stays
  off.

## [2.1.37] - 2026-09-15

- Persist decided agent approvals in `in.synara.agent_approval_history` account
  data so the Approvals Recent tab survives restarts and syncs across devices.
  Recent prefers an inbox `decision` over account data; a decided inbox row
  without a named decision does not inherit a homeserver-injected label.
  History summaries redact path-like args and secret-like assignments. History
  writes fetch-confirm and retry because Matrix global account data has no
  `If-Match`. AlreadyDecided taps persist history only when an existing own
  terminal reaction matches the requested action.
- Remove the redundant empty "Full approval prompt" details block from approval
  cards; the Command block and the no-command fallback remain.
- Expose decided agent-approval history to iOS through UniFFI
  `agent_approval_history_snapshot` and show a Recent list that unions Core
  inbox decided/expired items with synced account data using the same
  inbox-proof conflict rules as desktop. Remove the redundant iOS "Full
  approval prompt" block; the Command block and no-command fallback remain.
- Dismiss Linux OS notifications when a room is focused or an agent-approval
  decision is submitted (`desktop_dismiss_notifications`). macOS remains a no-op
  for this command.
- Make native room-timeline scrolling smoother: cache measured row heights,
  reserve media space, coalesce view deltas, and stick to the live tail in the
  same layout pass. Follow-live releases on a focus/`scrollIntoView` move away
  from the tail, prepend anchoring is never overwritten by an in-flight
  programmatic scroll, a parked history row is held steady when rows above it
  re-measure or when a date separator or reaction lands on the parked row
  itself, a scripted move off the live tail is not yanked back, same-frame
  deltas apply in revision order (keeping a successful prefix on a true gap),
  and the measured-size cache is keyed by row content so edits do not reuse
  stale heights. Home drops follow-live so a keyboard move into history is
  not yanked back toward the live tail.
- Bump rustls to 0.23.45 in the desktop and workspace lockfiles
  (RUSTSEC-2026-0285).
- Promote the composer action recipe to a shared `quietActionButton` export in
  `Depth.css.ts` (`RoomComposer.css.ts` re-exports it as `ComposerAction`, no
  visual change) and restyle the message formatting toolbar and message-editor
  Aa/emoji buttons to match the composer exactly: transparent at rest,
  tint + edge light on hover, stronger tint when pressed.
- Restyle Explore Community controls to the quiet style: transparent filter,
  protocol, and page-limit chips (selection reads as a quiet raised/pressed
  state, no green fill), quiet Add Server dialog buttons, floating menus, and
  flat info/error cards with a Critical border instead of a saturated red block.
- Fix the Personal Notes tab strip being covered by the textarea: folds `Box`
  resets `min-height: 0`, so the composer form and tab strip shrank inside the
  height-constrained panel. Set `flexShrink: 0` on the composer card, kind
  switch, and textarea so they keep their intrinsic height; only the notes
  list scrolls. Remove the gradient fold from resting notes cards and
  quiet the Note/ToDo segmented control and Add action.
- Sweep desktop Settings controls to the quiet style via a shared
  `SettingsQuietControl` helper: transparent rests, `aria-pressed` selection
  states, Soft fill only for each page's primary action, Critical color kept
  for danger actions at a quiet rest.
- Add a "Support this project" button (Sponsors page) to the Welcome page and
  Settings › About, with `.github/FUNDING.yml` for GitHub's Sponsor button.
- Restore Explore Community browsing of other homeservers: classified
  federation/network/server errors replace the generic "unavailable"
  message, unknown public-room types no longer fail the whole page, and
  keyword search, All/Spaces/Rooms filters, and page limit still pass
  through to the remote directory.
- Show richer Settings > Devices session details: per-tile trust chips,
  a This device marker, and expanded identity-key fingerprint / first-seen
  fields from the native device snapshot. The trust vocabulary now separates
  cross-signed (`verified`) from direct-SAS (`verified_locally_only`)
  sessions; both desktop and iOS render either as Verified. Dehydrated
  sessions read as Backup device rather than Not encrypted. iOS session rows
  now show the same identity-key / first-seen / cross-signed details (still
  without last-seen IP). Oversized homeserver display names and last-seen IPs
  are dropped in Core before they reach the snapshot wire.

## [2.1.36] - 2026-09-13

- Add an Approvals center (rail and `/approvals/` page) backed by Core
  `matrix_agent_approvals_list`. Decisions stay on
  `matrix_agent_approval_decide`.
- Replace the JavaScript inbox-notification stub with native
  `matrix_inbox_notifications` on desktop and iOS.
- Idle room-list approval badges use the latest event only, without farming a
  timeline per room. Opening Approvals runs bounded discovery (32 observers, 30s
  lease, spaces skipped). Rail `!` alarms only when a room is incomplete or
  errored, and leaving Approvals drops leftover discovery gaps.
- Opening a room waits briefly for inbox pagination, then fail-opens so a stuck
  inbox page cannot fail the user's room. Encrypted last messages appear on the
  idle approval list only after a local decrypt succeeds.
- Polish the desktop room list and composer: virtualizer scroll-margin, avatar
  object-URL cache, and composer/menu CSS.
- Keep a missing room as NotFound for both timeline-open diagnostics.

## [2.1.35] - 2026-09-11

- Show Element-like room member options on desktop and iOS using native
  membership snapshots instead of `room.getMember()`. Kick, ban, unban,
  invite, knock accept/deny, and power-level writes stay hidden until the native
  member list is ready, and a failed snapshot shows an error instead of offering
  moderation.
- Add timeline Copy All (sanitized HTML), partial text selection on iOS, code
  block copy, and desktop Copy Message. The composer pastes HTML as attributed
  text and projects markdown for send, so markup characters are not shown in
  the compose box.
- Restart Matrix sync after Linux sleep via `SyncIntent::Resume` when the SDK
  still reports Running on a dead long-poll. A short hide no longer leaves a
  stale `hiddenAtMs` that could recover on a later focus.
- Fix unread badges and agent-approval alerts: subscribe every joined room for
  client-side unread math, pulse the iOS room list after decrypt/receipts,
  retry encrypted observations so approvals can fire as Time Sensitive alerts
  (now on by default), and bump pending-approval unread only while SDK counts
  are still non-zero.
- Count desktop tray approvals from Core's raw last-message classifier
  instead of the collapsed preview, and keep Cancel Invite on the invite alert.
- Let native timeline message bodies use the available row width instead of a
  672px cap.
- Compare iOS composer bold wrapping to the composer base font so Accessibility
  Bold Text does not wrap every typed run as `**text**`.

## [2.1.34] - 2026-09-10

- Stop Room Settings > Developer Tools from crashing on native rooms. The page
  assumed a js-sdk `currentState.events` map that native room projections do
  not provide, so opening the panel threw `undefined is not an object
(evaluating 'i.events.forEach')` before Room ID could be copied. Room ID is
  now always visible on that page. Settings panels and routes also render a
  closeable error screen instead of React Router's default "Unexpected
  Application Error" page.

- Make iOS device verification sheets fit their content, with a compact emoji
  comparison and consistent full-width buttons throughout the flow. Large text
  layouts remain scrollable.
- Acknowledge a completed iOS verification flow once so buffered completion
  updates cannot reopen it after Done or swipe dismissal. Report dismissal
  failures for retry and keep newer verification requests separate.
- Show "Device verified" only when the SDK confirms this device's trust;
  finishing the code comparison alone shows "Verification complete."

## [2.1.33] - 2026-09-10

- Repair verification after signing out and back in with a retained encryption
  store. Publish the device's original signed public keys when missing on the
  homeserver, verify the readback, and refresh the account identity before
  requesting verification from another session. Failed or conflicting key
  checks stop before a verification request is sent.
- Refresh device trust, recovery, and backup status when Matrix SDK secret
  sharing completes, so desktop Settings reflects verification completion
  without reopening the page or restarting. The native repair is shared with iOS.

## [2.1.32] - 2026-09-09

- Make desktop "Jump to Last Read" work. Core now resolves a last-read marker
  that is not itself a renderable row (reaction, edit, hidden event) to the
  nearest rendered predecessor when opening a focused timeline, and the
  renderer adopts that anchor instead of failing with "The last-read message
  is not available in this context". The control moves to the bottom-right
  cluster beside Jump to latest as a matching pill, and is shown only while
  Core reported an unread frontier the presenter has not yet placed; it is
  cleared on placement, on an explicit Jump to latest, or on room change.
- Bind the desktop timeline's scroll and input listeners once per mounted
  viewport instead of re-subscribing on every render. The per-render swap
  raced the virtualizer's synchronous re-render at the start of a gesture and
  dropped the first scroll event, so a single wheel step away from the live
  tail was snapped back by the next snapshot. A scroll that ends at the bottom
  (the virtualizer re-measuring rows above the viewport) no longer releases
  follow-live for a non-live position; only leaving the bottom does. The e2e
  harness now applies the folds theme so overlay geometry matches the shipped
  app.
- Load the desktop Settings > Devices snapshot whenever a session is present.
  The query was gated on a session-generation marker the desktop session store
  never populates, so verification stayed spinning, Retry was a no-op,
  Encryption Backup read Disconnected, and the Current device list was empty.
  Load failures now render a retryable error instead of a false "could not
  check eligible verified sessions" prompt.
- Restyle desktop Settings: remove the gradient fold and inner edge from every
  settings card so resting surfaces match the rest of the app, move theme,
  color, text, zoom, and message layout controls to a dedicated Appearance
  page, add a Storage group in General for the native session store and
  Clear Cache, drop the duplicate maintenance actions from About, and phrase
  media loading positively.
- Inject the live app environment into iOS sheets. The Account sheet resolved
  the signed-out default environment, so it read "Not signed in" and its
  Settings and Log Out actions targeted a router nobody observed.
- Move the iOS "Jump to Last Read" control to the bottom-right cluster above
  Jump to Latest as a pill, capture the pending marker once at open (only when
  the room had unread and the marker was outside the opening window), and let
  that marker alone own visibility.
- Reset the iOS signed-in startup gate on sign-out. Re-login reuses the crypto
  device, so the same `user|device` identity was still considered prepared,
  `matrix.start` was skipped, and the room list stayed empty until the app was
  force-quit.

## [2.1.30] - 2026-09-08

- Decide desktop message notifications from the SDK-evaluated push actions of
  the exact observed event inside Core. Room mode, account defaults, mentions,
  keywords, room-mention power levels, the suppress-edits override, and rule
  ordering now have one owner; the renderer submits only room and event
  identity with privacy-filtered strings and can no longer supply a mode,
  highlight, sender, or encryption verdict.
- Report the desktop OS delivery result back to Core. The renderer waits for
  the notification call and acknowledges with `delivered` or `failed` instead
  of swallowing errors; Core counts receipts per session and releases a failed
  candidate without retrying it. The notification sound now follows the
  account's push rules (one-to-one rooms, mentions, keywords, and rules with a
  sound tweak) and stays silent when the OS refuses the notification. The
  desktop `matrix_notification_pending_snapshot` command now returns the
  delivery ledger alongside the pending candidates instead of dropping it at
  the shell boundary.
- Push desktop notification observations from Core to the renderer. Core now
  emits one `matrix-notification-observed` event per live message-like event
  the SDK sync delivers (other senders only, no edits, no replayed history),
  and the message and agent-approval notifiers subscribe to it. The renderer's
  timeline scans, `Room.timeline` listener, and `SYNCING` gate are removed;
  on the native client they never fired, so no live message reached Core's
  decision owner before this change.
- Use one modern macOS UserNotifications client for bundled-app permission,
  submission receipts, presentation, and action responses. A denied setting
  produces one failed receipt without posting or retrying; click handling is
  independent of OS acceptance. Unbundled development retains its legacy path.
- Check both IPv4 and IPv6 before selecting the packaged asset port, preventing
  a second running app from loading the first app's frontend.
- Keep late delivery acknowledgements and event-fetch results within their
  original login session. Approval prompts now use Core decision and receipt
  accounting while preserving native actions and avoiding duplicate sounds.
- Add an iOS compile gate to CI for Swift/FFI changes on feature pull requests
  that skip the simulator lane; labels and release branches keep the full lane.

## [2.1.29] - 2026-09-06

- Restore last-read on room entry, keep Jump to Last Read when the marker is
  missing, and follow new messages only at the real live tail. Sending a
  message returns to latest; later pagination must not jump the viewport.
- Pair the visible timeline tail with the receipt frontier so focused rooms
  clear unread automatically when newest messages are actually painted.
- Suppress iOS alerts for replacement edits, keep notification taps on the
  real room and event, and classify NSE vault failures separately from
  preview or verification problems.
- Restore native macOS titlebar drag, keep Linux maximize chrome in sync, and
  quiet iOS room-list plus desktop Settings depth.
- Fail closed on NSE production-feature and archive checks so a dev fixture
  cannot look like a full-Core leak.

## [2.1.28] - 2026-09-05

- Resume iOS automatic read acknowledgement when returning to a visible room.
  Track the delivered scene activity state so an older SwiftUI environment
  snapshot cannot suppress the write. Background and offscreen events remain
  unread until viewed.
- Add a live, two-account simulator probe with server read-marker, private
  receipt, notification-count, and room-list readback.

## [2.1.27] - 2026-09-05

- Fix desktop automatic read acknowledgement when Core changes placement at
  the same message revision or an unread room fits without scrolling.
- Avoid redundant iOS timeline snapshots and unnecessary anchor repositioning.
- Preserve iOS room invalidations across temporary read-marker streams.
- Allow local iOS sign-out after failed restore or remote push cleanup; revoke
  the current server session when reachable and remove persisted authentication.

## 2.1.26 - 2026-09-04

- Added integrated Linux window controls, a macOS overlay titlebar, and more
  consistent room, member, and message-search surfaces.
- Limited automatic follow-live to the actual Core live provider with an exact
  painted-tail observation; historical windows retain explicit Jump to latest.
- Removed iOS implicit historical-window replacement and unseen-tail read
  acknowledgement introduced during desktop UX integration.
- Routed desktop notification candidates through Core, bounded recent-event
  deduplication, and made rejected candidate conflicts atomic.
- Honored explicit Matrix mention metadata instead of legacy body matching.
- Restricted privileged desktop document navigation to the application origin.
- Closed CI filename-based validation bypasses and retained simulator gates for
  release branches and explicit iOS opt-ins.
- Updated Rust and JavaScript dependencies and included TestFlight export-retry
  and exact-tag release validation improvements.

## 2.1.25 - 2026-09-03

- Moved timeline reactions, poll voting, reports, encrypted-room forwarding,
  call declines, and their authoritative readback policies through the shared
  Rust ownership path across desktop and iOS.
- Prevented a committed reaction from appearing to fail while its projected
  aggregation catches up, keeping duplicate taps from accidentally reversing
  the successful action.
- Added a shared Matrix and Hermes message-format corpus with stricter
  cross-client sanitization for spoilers, inline media, tables, code, replies,
  and other semantic message relationships.
- Hardened iOS notification preview and HTTP-pusher lifecycle handling,
  including bounded logout cleanup, privacy-safe extension diagnostics, and
  time-sensitive agent-approval classification.
- Preserved actionable encrypted-forwarding errors on iOS and kept verification
  eligibility unknown when neither the homeserver nor local device store can
  provide authoritative evidence.
- Added bounded authenticated Matrix media download and decrypt paths with
  explicit ciphertext/plaintext memory limits and honest diagnostic behavior.
- Improved composer formatting controls, compact iOS room-list rhythm, and
  desktop Inbox handling for valid empty and error-shaped homeserver responses.
- Split iOS unit and UI validation into explicit arm64 simulator lanes while
  retaining full UI coverage for main, release, nightly, and opted-in PR runs.
- See [`docs/releases/v2.1.25.md`](docs/releases/v2.1.25.md) for details.

## 2.1.24 - 2026-09-01

- Improved rich-message readability across desktop and iOS with semantic
  treatments for emphasis, inline code, code blocks, tables, and spoilers.
- Added theme-aware rich-text roles for light, dark, Silver, Butter, and custom
  themes without changing the underlying Matrix message content.
- Enforced accessible authored-color presentation at 4.5:1 contrast normally
  and 7:1 when Increased Contrast is enabled.
- Matched native desktop, compatibility HTML, Prism, and iOS attributed-text
  rendering while preserving safe formatted selection and copying.
- Kept concealed spoiler descendants out of the interface until an accessible
  pointer or keyboard reveal, then resolved colors against the painted surface.
- See [`docs/releases/v2.1.24.md`](docs/releases/v2.1.24.md) for details.

## 2.1.23 - 2026-08-31

- Displayed the authenticated Matrix homeserver in the desktop Home header and
  extended quiet dimensionality to the interactive desktop navigation rail.
- Hardened Inbox response parsing so malformed server responses produce a
  controlled error instead of an `e.forEach` crash.
- Repaired automatic read-state submission around the SDK-owned latest event,
  including hidden state events, focus changes, manual unread state, and exact
  private/read-marker receipt targeting.
- Preserved safe rich-text structure when pasting into desktop composers,
  including lists, links, and emphasis, while bounding and sanitizing HTML.
- Added accessible iOS message text selection with formatted clipboard output,
  complete action icons, safe spoiler concealment, and explicit reveal control.
- Added complete Linux icon sizes and packaging contracts plus diagnostics for
  WebKitGTK acceleration policy and software-rendering overrides.
- See [`docs/releases/v2.1.23.md`](docs/releases/v2.1.23.md) for details.

## 2.1.22 - 2026-08-30

- Centered the macOS and Linux message composer controls optically within the
  input bar, including the add, formatting, emoji, and send actions.
- Removed persistent room-row separators while preserving the restrained hover
  and selected-room depth hierarchy.
- Reworked Personal Notes controls and surfaces so tabs are no longer clipped
  and the panel follows the shared quiet-dimensional design language.
- Expanded accessible quiet depth to room-header and Home navigation actions.
- Made timestamp reveal trackpad/touch-owned on desktop so ordinary mouse text
  selection cannot drag messages; touch and pen behavior remains available on
  Linux-capable hardware.
- See [`docs/releases/v2.1.22.md`](docs/releases/v2.1.22.md) for details.

## 2.1.21 - 2026-08-30

- Proved cold-restart room restoration from the production encrypted SQLite
  Matrix SDK stores while the homeserver is offline, and added bounded,
  privacy-safe timeline recovery that preserves the last readable messages.
- Corrected attachment filenames, captions, mentions, reply/thread relations,
  transaction IDs, 32 MiB limits, and partial-retry behavior across Rust,
  UniFFI, Swift, Tauri, and TypeScript.
- Made attachment retries assign composer text exactly once, including edits,
  cleared text, and text introduced after a partial attachment-only send.
- Removed unsupported outgoing sticker-send controls and APIs while preserving
  incoming stickers, reactions, custom emoji, and existing image-pack metadata.
- Kept iOS room titles and compact status readable at Accessibility XL, with a
  combined VoiceOver description and UI regression coverage.
- See [`docs/releases/v2.1.21.md`](docs/releases/v2.1.21.md) for details.

## 2.1.20 - 2026-08-28

- Rebuilt current-device verification around the Matrix SDK OwnIdentity route,
  authoritative current-device trust, tri-state eligibility, and Rust-owned SAS
  acceptance for either participant's start direction.
- Added a repeatable live proof that begins with a fresh unverified device,
  compares identical SAS values, reaches `Done`, reads the exact device as
  verified, and preserves that result after rebuilding the same crypto store.
- Fixed desktop and iOS verification presentation so peer-row trust cannot be
  mistaken for current-device cross-signing, and hardened the iOS proof against
  `Unverified`/`Verified` substring false positives.
- Kept iOS messages in one leading reading column, automatically returned
  revealed timestamps after 2.5 seconds, and restored authenticated avatar
  hydration for live and pending messages without racing room changes.
- Refined desktop dimensionality into a quieter hierarchy: flat resting message
  planes, restrained selected/hover depth, accessible high-contrast boundaries,
  and reduced-motion-safe transitions.

## 2.1.19 - 2026-08-28

- Corrected the iOS depth rollout so its hierarchy is visibly present on the
  actual room rows, standard and formatted messages, avatars, reactions,
  message actions, and composer rather than only on isolated controls.
- Added restrained light- and dark-mode surface edges and shadows, with
  boundary-first behavior for Increase Contrast and reduced shadow emphasis
  when Reduce Transparency is enabled.
- Added a discoverable 44-point message-actions control while preserving
  grouped-message timestamp reveal and native vertical room-history scrolling.
- Added simulator screenshot, typing, scroll ownership, bottom-content
  reachability, Dynamic Type, and semantic depth regression coverage.

## 2.1.18 - 2026-08-27

- Added a restrained, cross-client depth system for room rows, composers,
  message actions, approval cards, pickers, avatars, and message surfaces,
  preserving contrast, hierarchy, reduced-motion behavior, and touch clarity.
- Rebuilt iOS grouped-message timestamp reveal around a timeline-owned,
  direction-locked gesture so vertical room-history scrolling remains native
  and responsive while a deliberate leftward swipe reveals message time.
- Bound timestamp gestures to exact diffable-snapshot revisions and explicitly
  reconfigured stable identifiers when visible content changes, preventing
  stale gestures or cells from surviving live timeline updates.
- Added focused viewport policy, snapshot repaint, accessibility, and iOS UI
  regressions, including coordinate-level proof of both vertical scrolling and
  visible timestamp reveal.

## 2.1.16 - 2026-08-27

- Refreshed the existing Synara wing-and-network app icon for stronger visual
  mass and small-size legibility across iOS, macOS, Linux, and Windows.
- Added platform-correct iOS, ICNS, ICO, desktop PNG, and Linux symbolic assets,
  including Debian and Arch symbolic-icon installation.
- Added deterministic icon generation, a reviewed asset manifest, and a
  fail-closed icon-only CI path that preserves packaging/signing release proof
  without repeating unrelated hour-plus runtime suites.
- See [`docs/releases/v2.1.16.md`](docs/releases/v2.1.16.md) for details.

## 2.1.15 - 2026-08-27

- Fixed the remaining iOS suspension crash by making foreground authority a
  prerequisite for constructing the active app shell or opening Matrix stores,
  including cold notification/background launches and rapid scene transitions.
- Deferred notification approval handling until the app is active and Matrix
  has resumed, while keeping Approve and Deny actions visibly foregrounded.
- Added reusable TestFlight crash diagnostics and lifecycle regressions for
  cold-background startup, duplicate suspension callbacks, and session relogin.
- Fixed the macOS message composer so its text follows the selected Appearance
  > Message text tone just like the timeline.
- See [`docs/releases/v2.1.15.md`](docs/releases/v2.1.15.md) for details.

## 2.1.14 - 2026-08-26

- Fixed the iOS background `RUNNINGBOARD 0xdead10cc` termination by stopping
  Matrix sync, draining in-flight store work, and closing every retained SQLite
  connection before suspension; foreground activation reopens the stores before
  restarting sync through the same serialized native lifecycle route.
- Started iOS store quiescence at `applicationWillResignActive` so the complete
  persistence boundary begins before UIKit can suspend the process.
- Fixed the desktop message-text appearance setting across both timeline
  renderers and made Bright use true white in dark mode (true black in light
  mode), with distinct AAA-readable Soft and Balanced choices.
- See [`docs/releases/v2.1.14.md`](docs/releases/v2.1.14.md) for details.

## 2.1.13 - 2026-08-26

- Fixed an iOS foreground room crash by serializing and coalescing diffable
  timeline snapshots and replacing run-loop-pumping HTML entity decoding with
  deterministic decoding.
- Improved typing responsiveness in busy rooms by coalescing native timeline
  invalidations, bounding streams to newest state, suppressing unchanged
  snapshots, removing synthetic refreshes, and avoiding redundant full-text
  composer layout work.
- Added privacy-safe performance signposts plus unit and UI regressions for
  snapshot reentrancy, formatted HTML, and exact long-paragraph input during
  rapid live updates.
- See [`docs/releases/v2.1.13.md`](docs/releases/v2.1.13.md) for details.

## 2.1.12 - 2026-08-25

- Fixed iOS background suspension crashes by stopping and awaiting the retained
  native Matrix SyncService under a bounded UIKit background assertion before
  suspension, then serializing foreground restart through the same session
  owner. Rapid foreground/background transitions cannot restart sync while the
  app is backgrounded.
- Added privacy-safe iOS lifecycle signposts for native sync pause, background
  assertion expiry, suppressed stale resume, and foreground restart.
- See [`docs/releases/v2.1.12.md`](docs/releases/v2.1.12.md) for details.

## 2.1.11 - 2026-08-25

- Rich Matrix messages now use semantic, sanitized renderers across iOS and
  desktop, preserving exact code whitespace, nested and ordered lists, tables,
  details, spoilers, headings, colors, links, media fallbacks, and readable
  malformed-content fallbacks without reparsing HTML as Markdown.
- Hermes approval prompts are classified in the shared native core, surfaced as
  time-sensitive notification actions on iOS and distinctive desktop alerts,
  and resolved through the native Matrix reaction path with expiry, existing-
  decision, authentication, and cold-launch safeguards.
- Device verification is proven through the complete Matrix SAS operating path:
  both clients exchange identical emoji/decimal values, confirm trust, and read
  the peer back as Verified after relaunch.
- Opt-in iOS notification previews more reliably retain decrypted message
  content when available while preserving privacy-safe fallback text.
- Added a Pop!\_OS/Debian-family APT repository path that publishes the tagged
  `.deb` and signed flat repository metadata alongside every production
  release.
- Updated Linux update guidance to cover APT as well as pacman/paru, with
  focused repository generation and signature verification tests.
- See [`docs/releases/v2.1.11.md`](docs/releases/v2.1.11.md) for details.

## 2.1.10 - 2026-08-25

- Room lists across iOS, macOS, and Linux now use compact channel markers and
  Slack-like `# Room Name` labels instead of decorative room avatars.
- Cross-client semantic colors, spacing, and typography soften dark-mode
  contrast while preserving readable hierarchy in timelines, tables, replies,
  settings, and composers.
- iOS room notes can be reordered by dragging, and message composition avoids
  unnecessary synchronous layout work for more responsive typing.
- iOS surfaces, Dynamic Type composer sizing, notification disclosure rows,
  and final-list-item clearance are hardened across light and dark appearance.
- Desktop room-note ordering and conflict handling now match the mobile
  behavior, with richer formatted-message and code presentation.
- See [`docs/releases/v2.1.10.md`](docs/releases/v2.1.10.md) for details.

## 2.1.9 - 2026-08-24

- iOS room timelines now include Notes backed by shared Matrix account data,
  bringing synced room notes to the mobile client.
- Opt-in iOS notification previews can show encrypted message content through
  a bounded, cancellation-safe notification service extension, with safe
  fallback text when content cannot be resolved.
- The notification extension uses a purpose-built native core instead of
  linking the full application core, with arm64, linkage, and size guards in
  pull-request and release CI.
- Device-verification session restore/reset preserves durable SAS trust
  readback, and Settings retains clearance above the floating tab bar.
- See [`docs/releases/v2.1.9.md`](docs/releases/v2.1.9.md) for details.

## 2.1.8 - 2026-08-23

- Device verification now targets the exact newly signed-in session, refreshes
  the device list before selection, and retains verified trust after relaunch.
- Direct SAS trust is projected with the Matrix SDK's complete device trust
  result instead of the narrower cross-signing-only state.
- The paired iOS diagnostic proves both devices exchange the same SAS values,
  confirm, and read each other back as Verified after both apps relaunch.
- iOS Rooms and Settings use continuous opaque light-mode surfaces through the
  safe areas, removing the gray veil and mismatched top and bottom colors while
  retaining floating-tab-bar clearance.
- See [`docs/releases/v2.1.8.md`](docs/releases/v2.1.8.md) for details.

## 2.1.7 - 2026-08-23

- Device verification now completes through the real Matrix SAS operating path:
  requests reach both clients, both sides exchange matching emoji or decimal
  codes, confirmation completes, and the verified trust state refreshes.
- iOS verification stays explicitly user-driven, keeps the comparison visible
  while state changes arrive, and identifies the exact session being verified.
- iOS Settings content now clears the floating tab bar, including the final rows
  in Dark Mode, and uses more readable message typography and table treatment.
- macOS message content has more comfortable measure, spacing, hierarchy, and
  alternating table rows for easier scanning in long technical messages.
- Matrix room and event routes are encoded consistently across desktop hosts.
- See [`docs/releases/v2.1.7.md`](docs/releases/v2.1.7.md) for details.

## 2.1.6 - 2026-08-22

- Native Core now owns homeserver push rules, 3PID/email, ignored users,
  per-room notification mode (including Default), user-directory search,
  message search, join-rule write, and presence SET. Linux uses the same
  Tauri commands as macOS.
- iOS can upload an avatar, send attachments, download plain `mxc://`
  media, register HTTP pushers, restore backup from a recovery key, and
  set Online/Away/Offline presence through product FFI rather than
  leftover stubs.
- Desktop Account can set presence; invite search uses the user
  directory; notifications and ignored-user/email settings write through
  Core. Leftover SharedCore FFI stays fail-closed.
- See [`docs/releases/v2.1.6.md`](docs/releases/v2.1.6.md) for details.

## 2.1.5 - 2026-08-22

- Settings → Devices no longer hangs on a spinner while identity lookup waits
  on `/keys/query`. Linux uses the same desktop path; iOS Security uses the
  same bounded Core lookup.
- iOS logout keeps the per-account crypto store so the next password sign-in
  reuses the same Matrix device instead of minting a new session.
- iOS Account → Sessions lists devices and can force-sign-out others with the
  account password. Desktop Settings → Devices → Others already had this.
- See [`docs/releases/v2.1.5.md`](docs/releases/v2.1.5.md) for details.

## 2.1.4 - 2026-08-21

- iOS failed sends retry from a chip, offline text queues until connected,
  and failed local echoes can be edited on the same pending id.
- Long-press Copy copies the plain message body; text selection is a
  best-effort substring path.
- Connection Lost holds through short SDK blips instead of bouncing on
  1–2s Offline gaps.
- Desktop unread badges and Mark as Read use native receipts, typing
  shifts the last message up, and timeline rows use a full-width surface.
- See [`docs/releases/v2.1.4.md`](docs/releases/v2.1.4.md) for details.

## 2.1.3 - 2026-08-20

- Completed the native SAS device-verification path: incoming requests stay
  on the app chrome, Confirm waits for emoji or decimal codes, and trust
  refreshes without a restart. iOS cannot swipe away an in-progress SAS
  sheet, and Security settings hide Verify This Device once verified.
- Tightened desktop chrome so the chat pane matches the rest of the window,
  title-cased Rooms, and made Favorites and Rooms sort independently.
- Removed Twitter Emoji from Appearance, showed the real theme accent
  instead of unused mint, and held Connection Lost until Offline lasts.
- Jump to latest stays available until the loaded window is the live tail.
- See [`docs/releases/v2.1.3.md`](docs/releases/v2.1.3.md) for details.

## 2.1.2 - 2026-08-20

- Highlighted fenced chat code with Prism tokens, a matching line-number
  gutter, and a distinct panel on the desktop native timeline. iOS code
  blocks now draw a line-number gutter.
- Replaced the Recent (24h) room rail with Matrix favorites and a sortable
  rooms list.
- Restored iOS live sync after TestFlight updates, showed connection status,
  and kept composer attachments as drafts until Send.
- Added Discord-like dark/light stacked chrome with a user base color, and
  made Settings lead with native verification, the server avatar, and honest
  controls.
- Release reuses a proven Quality gate for the tagged SHA, skips iOS and
  Synapse on version-only bumps, and publishes the GitHub Release without
  waiting on TestFlight.
- See [`docs/releases/v2.1.2.md`](docs/releases/v2.1.2.md) for details.

## 2.1.1 - 2026-08-19

- Reused leftover local crypto devices on password login so a prior interrupted
  session no longer fails with Olm unavailable on macOS and Linux.
- Opened unread rooms on the live timeline at the newest receipt instead of a
  stale fully-read marker, and made Jump to latest follow the live tail.
- Tightened desktop room-list chips to two-letter initials and a muted palette,
  and rendered timeline avatars, display names, timestamps, grouped sends, and
  formatted message bodies.
- See [`docs/releases/v2.1.1.md`](docs/releases/v2.1.1.md) for details.

## 2.1.0 - 2026-08-18

- Hardened native credential custody, remote transport policy, bounded media
  and file operations, diagnostic redaction, and notification privacy across
  iOS, macOS, and Linux.
- Removed retired renderer session credentials and repaired startup recovery
  so an unrestorable native session can be cleared without an insecure fallback.
- Added versioned GitHub and TestFlight release notes for coordinated client
  releases. See [`docs/releases/v2.1.0.md`](docs/releases/v2.1.0.md) for details.

- Bumped coordinated macOS, Linux, and iOS release metadata to `1.2.59` after
  the `1.2.58` iOS build was promoted to internal TestFlight testing.
- Added opt-in, privacy-filtered desktop diagnostics for performance, session
  persistence, room state and positioning, recent-room organization, and an
  on-screen performance overlay, with bounded local retention and manual
  export controls.
- Stabilized iOS unread/read-marker positioning and variable-height timeline
  anchor restoration by deferring placement until layout settles and using
  view-space cell geometry consistently.
- Restored Tauri app-bundle notarization credentials to the production macOS
  build and made PR/exact-tag validation fail early when the updater,
  notarization, signing, metadata, or distributable-verification contract is
  incomplete.
- Bumped coordinated macOS, Linux, and iOS release metadata to `1.2.34`.
- Activated native desktop spell checking when the shared composer receives
  focus: macOS now enables AppKit continuous spell checking on the active
  webview text responder, while Arch and Debian packages declare the English
  dictionary support required by WebKitGTK.
- Allowed the packaged Tauri localhost webview origin to use the main desktop
  capability set, restoring the native IPC path used by system-browser link
  opening, clipboard image reads, native file drops, desktop events, and update
  checks in release builds.
- Added a release-readiness regression check that fails if the packaged
  localhost webview origin loses native desktop capability access.
- Added native spellcheck language, autocorrect, and capitalization hints to the
  shared Slate editor while preserving the existing composer spellcheck toggle.
- Bumped shared Synara app version metadata to `1.2.22` for packaged
  macOS/Linux update validation.
- Added the user-facing desktop updater layer: Settings/About update checks,
  macOS background prompts with install/relaunch, a macOS app menu update
  command, and Linux package-manager guidance for GitHub-release version checks.
- Tightened release-updater readiness checks to require install-capable updater
  permissions and process relaunch support.
- Added production release automation for a GitHub Release-backed `synara`
  pacman repository so Arch-family Linux updates are package-manager-owned via
  `paru -Syu` / `pacman -Syu` after one-time repo setup.
- Fixed macOS updater metadata generation for GitHub Actions' downloaded
  `macos-updater-artifacts` directory layout.
- Bumped shared Synara app version metadata to `1.2.21`.
- Fixed production macOS release validation by notarizing and stapling the DMG
  after Tauri signs and bundles it.
- Hardened pacman repository release publication by using the GitHub release
  event tag explicitly, setting `GH_REPO` for no-checkout `gh` commands, and
  verifying downloaded repository assets before upload.
- Added a macOS release preflight that validates the Tauri updater private key
  password before the expensive signed/notarized package build.
- Fixed the macOS release workflow bundle set so updater-enabled `.app`
  artifacts remain available for signature verification and updater metadata.
- Revised production release automation so macOS remains Tauri-updater managed
  while Linux no longer publishes AppImage self-update metadata for the current
  release goal.
- Added release-branch Arch/CachyOS pacman package artifact generation for `synara-desktop-bin` package smoke.
- Fixed room re-entry after Jump to Latest by persisting live-tail bottom snapshots and allowing them to override stale unread/read-marker state only when no newer live-tail event has arrived.
- Hardened desktop external-link opening by mounting the interceptor at the app shell, using capture-phase link interception, surfacing native opener failures in desktop diagnostics, and making the injected Tauri bridge fail explicitly when IPC is unavailable.
- Added release-branch CI triggers for core CI, desktop package smoke, and iOS skeleton validation.
- Added a build-and-release runbook and linked it from the README documentation index.
- Added a release-branch CI and controlled client-update publication plan.
- Configured GitHub Actions updater signing secrets and updater public endpoint variables for future signed release workflow validation.
- Updated production-readiness plans with 2026-06-30 smoke feedback: desktop launch works, link opening fails on macOS/Linux, Timeline behavior is tentatively improved, and updater work remains deferred.
- Documented the 2026-06-29 macOS desktop non-launch postmortem and updated validation guardrails for updater config and Timeline helper changes.
- Bumped shared Synara app version metadata to `1.2.20`.
- Tightened the release-updater gate so signed metadata evidence must come from the generated updater metadata workflow.
- Added root-level macOS workstation handoff and deferred GitHub Release updater project plan documents.
- Added regression coverage proving release-time updater config materialization satisfies the strict release-updater readiness inspector.
- Added release workflow updater-channel configuration from GitHub repository variables before strict updater validation and packaging.
- Added release workflow generation and upload of static signed updater metadata from Linux and macOS updater artifacts.
- Tightened the release-updater gate so updater signature sidecars and signed updater metadata uploads are validated independently.
- Added a `check:production-smoke` gate that keeps production smoke checklist cases, signoff rows, preflight commands, and macOS/iOS queue linkage intact.
- Added a consolidated production smoke checklist covering evidence rules, macOS/Linux desktop smoke, Timeline Resurrection cases, iOS Xcode/simulator validation, and updater release smoke.
- Extracted desktop save/drop file-transfer commands, transfer-session state, drag/drop allowlist lifecycle, and tests into `desktop_file_transfer.rs`.
- Extracted desktop integration status DTOs, Linux/KDE/session/portal probes, and tests into `desktop_integration.rs`.
- Extracted desktop notification payload validation, permission commands, route-click dispatch, and tests into `desktop_notifications.rs`.
- Extracted desktop agent-action payload sanitization, local copy/open handling, event emission, and tests into `desktop_agent_actions.rs`.
- Extracted desktop tray/menu state, badge clamping, DND dispatch, and tray tests into `desktop_tray.rs`.
- Extracted desktop global shortcut config, registration lifecycle, integration status, plugin factory, and tests into `desktop_shortcuts.rs`.
- Extracted desktop keyring session persistence flow and error-sanitization tests into `desktop_session_store.rs`, leaving Tauri session commands in `desktop.rs`.
- Moved desktop secret-store platform probes, status caches, credential identity constants, and live probe tests into `desktop_secret_store.rs`, leaving `desktop.rs` focused on command/session storage flow.
- Extracted desktop secret-store status, backend classification, and stable reason/error-code contracts from `desktop.rs` into a focused Rust module with direct tests.
- Extracted desktop session-envelope validation and expiry policy from `desktop.rs` into a focused Rust module with direct tests.
- Aligned the published desktop release workflow with the signed updater gate by exposing Tauri updater signing secrets, removing release-time updater artifact suppression, and uploading generated updater signature artifacts.
- Extracted desktop file-transfer policy helpers from `desktop.rs` into a focused Rust module with direct tests.
- Extracted desktop text and route sanitization helpers from `desktop.rs` into a focused Rust module with direct tests.
- Stabilized the localhost-port Rust test so the validation gate still passes when the preferred dev port is already occupied.
- Split desktop URL safety helpers out of `desktop.rs` into a focused Rust module with direct policy tests.
- Extracted room timeline opening/window/unread helpers from `RoomTimeline.tsx` into a tested timeline utility.
- Extracted Matrix linked-timeline helpers from `RoomTimeline.tsx` into a tested shared timeline utility.
- Removed the commented legacy Jotai `sessionsAtom` implementation now that session bootstrap and persistence own the active session flow.
- Replaced the `/home/join/` route stub with the existing join-address prompt flow and shared room-link URL construction.
- Added desktop Tauri updater plugin scaffolding with check-only frontend permission while production updater metadata remains release-gated.
- Added a release-updater readiness checker and wired published desktop releases to fail until signed updater artifacts, metadata, plugin wiring, and release signing secrets are configured.
- Hardened desktop composer drag/drop detection so file payloads that expose `files` or file items without a `Files` type marker still activate the upload drop zone.
- Improved desktop composer clipboard image paste so image-like native clipboard payloads are uploaded before rich-text insertion, with a rich-text fallback when the native image read yields no file.
- Routed desktop external link opens through the native `desktop_open_external_url` bridge across Hermes cards, profile/server actions, account-management links, auth/info anchors, and agent actions without unsafe desktop `window.open` fallback.
- Added a cross-platform timeline open-focus contract and expanded iOS focus-policy coverage for read-marker and jump-latest behavior.
- Added a desktop timeline viewport restore policy so unread rooms and stale historical anchors no longer override live/read-marker opening.
- Added Codex-Orchestrator-v2 persistent harness artifacts for production-readiness tracking.
- Expanded the living production-readiness backlog to cover the full KB Section 7 recommendation set and reconciliation constraints.
- Fixed existing Prettier drift in timeline, notification, app-link, and timeline lifecycle files so the formatting gate passes.

## 1.0.4 - 2026-05-18

- Fixed room timeline viewport restoration when leaving and returning to a channel after scrolling into history.
- Added explicit saved-anchor restore handling so historical restores load around the saved event before normal pagination resumes.
- Prevented initial bottom pinning and generic pagination from overwriting an in-progress historical viewport restore.
- Updated displayed client version to match the packaged app version.
