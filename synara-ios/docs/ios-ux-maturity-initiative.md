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

## Phase 2 — Feel like iOS chat

- [ ] Tab bar badges (Notifications required, Rooms optional)
- [ ] Notifications tab sections: Mentions, Invites, Agent pending, Unread rooms (collapsed)
- [ ] Later: room display names, swipe-to-complete, due-date urgency colors
- [ ] Room list swipe actions: mark read, mute, favorite, leave

## Phase 3 — Feel like Synara

- [ ] Typography tokens: messageBody, messageMeta, roomPreview, chipLabel, composerPlaceholder
- [ ] SynaraMessageBubble primitive (own/other, grouped, agent, encrypted)
- [ ] Room list MXC avatars in RoomAvatarTile
- [ ] Composer polish: tokenized radius, keyboard inset animation, send haptic
- [ ] Haptics: send, invite accept, agent approve, failed send, filter selection
- [ ] Agent inbox filter on room list; pending approval chip on rows
- [ ] Agent card approve/reject styling
- [ ] Dismissible crypto banner (only when action needed)
- [ ] Spaces: unread per space, collapse headers

## Orchestration

Main agent assigns work → sub-agent implements → main agent reviews → fix loop until phase complete → commit → next phase.