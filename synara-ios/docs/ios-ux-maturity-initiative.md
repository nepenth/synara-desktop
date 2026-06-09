# iOS UX Maturity Initiative

Branch: `feature/ios-ux-maturity`

## Principles

1. Never show fake UI
2. Never block the whole screen on navigation — skeleton/shimmer + incremental content
3. Never make the user wait to see their own message — optimistic send

## Phase 1 — Stop feeling beta (complete)

- [x] Remove fake presence dots (room list header, timeline avatars)
- [x] Remove heuristic favorites; replace with user-starred or remove Favorites filter until backed by data
- [x] Wire timeline header search; remove non-functional phone icon
- [x] Attachment sheet: keep Photo/Video, File, Camera; remove unavailable stubs; clean grid UI
- [x] Inline media thumbnails + real MediaViewer via mediaLoader
- [x] Reply/edit composer banners show quoted snippet, not raw eventID
- [x] Optimistic send with sending/sent/failed bubble states + tap-to-retry
- [x] Skeleton loading for room list, timeline, notifications inbox
- [x] Scroll-triggered older-message pagination
- [x] Mark-read when latest messages visible ~1s
- [x] Pull-to-refresh on Rooms + Notifications

## Phase 2 — Feel like iOS chat (complete)

- [x] Tab bar badges (Notifications required, Rooms optional)
- [x] Notifications tab sections: Mentions, Invites, Agent pending, Unread rooms (collapsed)
- [x] Later: room display names, swipe-to-complete, due-date urgency colors
- [x] Room list swipe actions: mark read, mute, leave (favorite deferred until starred rooms API)

## Phase 3 — Feel like Synara (complete)

- [x] Typography tokens: messageBody, messageMeta, roomPreview, chipLabel, composerPlaceholder
- [x] SynaraMessageBubble primitive (own/other, grouped, agent, encrypted)
- [x] Room list MXC avatars in RoomAvatarTile
- [x] Composer polish: tokenized radius, keyboard inset animation, send haptic
- [x] Haptics: send, invite accept, failed send, filter selection
- [x] Agent inbox filter on room list; pending approval chip on rows
- [x] Agent card approve/reject styling
- [x] Dismissible crypto banner (only when action needed)
- [x] Spaces: unread per space, collapse headers, selected-space header

## Validation (2026-06-09)

- **202 unit tests passing** on iPhone 17 / iOS 26.5 simulator after review fixes.
- Post-review fixes: notifications caught-up logic, removed misleading global agent pending chips, agent rooms detected from latest agent event (not room name), Later/thread skeleton loading, tab badges only when count > 0, removed duplicate `loadRooms` for tab badges.

## Known follow-ups

- Per-room agent pending approval chips (needs inbox API).
- Favorite swipe / starred rooms (needs Matrix account data).
- Push gateway deployment (paused).
- Notifications inbox rows still omit room avatars (parity with room list).

## Orchestration

Main agent assigns work → sub-agent implements → main agent reviews → fix loop until phase complete → commit → next phase.