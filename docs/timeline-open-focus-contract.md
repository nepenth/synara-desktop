# Timeline Open Focus Contract

Date: 2026-06-29

This contract defines how Synara opens a Matrix room timeline across desktop and
iOS. It is part of the Timeline Resurrection epic and exists to prevent SDK
read-marker state, local viewport memory, and focused timeline APIs from fighting
over scroll position.

## Sources

- Matrix Client-Server API: `m.fully_read` is room account data written through
  read-marker flows; it is not a persistent viewport command.
- `matrix-js-sdk`: `MatrixClient.getLatestTimeline(...)` fetches a latest-event
  timeline window and should back explicit jump-to-latest behavior.
- Matrix Rust SDK: live timelines and event-focused timelines are distinct
  `TimelineFocus` modes; read-marker focus is useful for initial context but is
  not equivalent to a live room stream.
- Local audit: `docs/matrix-sdk-lifecycle-semantics-audit.md`.

## Ownership Rules

1. SDK state owns timeline data windows, read markers, and read receipts.
2. UI state owns mounted scroll position.
3. A read marker may seed initial room placement only.
4. A saved local viewport may seed initial room placement only when it is fresh
   and there is no unread state.
5. After mount, receipt/account-data/unread-count changes must not move the
   viewport.
6. Explicit user commands can move the viewport:
   - open event/thread uses an event-focused timeline/window;
   - jump to latest uses the latest live timeline/window;
   - load older history preserves the current visible anchor.
7. If server unread/read-marker state or the loaded live slice is stale during
   first paint, the client must prefer a conservative jump-to-latest affordance
   over restoring unrelated old history.

## Initial Open Matrix

| Scenario | Desktop Behavior | iOS Behavior | Required Outcome |
|---|---|---|---|
| Fully read room, no saved viewport | Open latest live window and pin bottom. | Open live timeline and scroll bottom. | User sees the live end without history traversal. |
| Fully read room, saved bottom viewport | Restore bottom/live-end state. | Open live timeline and scroll bottom. | User sees the live end. |
| Fully read room, fresh saved historical viewport | Restore visible event anchor without treating it as unread. | N/A until iOS persists room viewport anchors. | Desktop returns to recent local reading position; iOS stays live until persisted viewport exists. |
| Fully read room, stale saved historical viewport | Ignore historical anchor and open live end. | Open live timeline and scroll bottom. | Old history cannot hijack room open. |
| Room with unread notification count or unread anchor | Ignore saved historical viewport; open around unread/read-marker context when available. | Use `m.fully_read` for initial focused load only. | User lands near unread context and sees jump-to-latest when not at live end. |
| Room with one new event after old local history visit | Ignore old local history anchor. | Use read marker for initial context only, then live updates stream live. | Client does not scroll through days-old history before showing the relevant new/live area. |
| Read marker or notification state lags the newest server event | Do not restore unrelated old history; expose jump-latest when not confidently at live end. | Keep read-marker focus initial-only and stream live updates after mount. | Stale sync state cannot make old local viewport memory look current. |
| Explicit event route | Open event-focused context and highlight target. | Open event-focused context and highlight target. | Route target wins over unread, read marker, and saved viewport. |
| Jump latest from historical/focused window | Fetch/rebind latest timeline window, then pin bottom. | Load latest timeline and restart live stream focus. | User reaches true current live end. |
| Timeline reset or sync gap while pinned | Reattach to new live tail and continue following bottom. | Live timeline stream remains live-focused unless route is explicit event/thread. | No blank viewport or stale focused stream. |
| Timeline reset or sync gap while reading history | Preserve current visible anchor if possible. | Preserve current loaded list position where SwiftUI can; show jump-to-latest if no longer at live end. | SDK lifecycle changes do not surprise-scroll the user. |

## Desktop Enforcement

- `RoomTimeline` stores process-local viewport snapshots with `updatedAtMs`.
- `shouldRestoreRoomTimelineViewport(...)` rejects non-bottom snapshots when:
  - the room has unread state;
  - the snapshot has no finite timestamp;
  - the snapshot is older than `ROOM_TIMELINE_VIEWPORT_RESTORE_TTL_MS`.
- `roomHaveUnread(...)` is consulted synchronously so a loaded live slice can
  block stale historical restore before Jotai unread state settles.
- Mounted auto-read paths use loaded-tail receipts only after the UI has proven
  live-bottom visibility.
- Explicit jump-latest uses `getLatestTimeline(...)`.

## iOS Enforcement

- `RoomTimelineFocusPolicy.initialLoadFocus(...)` allows `m.fully_read` to seed
  the first load only when there is no explicit focused route.
- `RoomTimelineFocusPolicy.updateStreamFocus(...)` ignores read-marker focus for
  mounted room updates; normal rooms stream live.
- Explicit focused routes keep event focus until jump-latest passes an override
  of `nil`, which returns the stream to live.
- `RoomTimelineView.jumpToLatest(...)` clears `initialReadMarkerEventID`, restarts
  live stream focus, and calls `loadLatestTimeline(...)`.

## Smoke Checklist

Before marking Timeline Resurrection complete, run these against desktop and iOS:

1. Fully read room with no unread and no saved viewport opens at live end.
2. Fully read room after visiting old history reopens at live end once the saved
   historical anchor is older than `ROOM_TIMELINE_VIEWPORT_RESTORE_TTL_MS`
   (10 minutes on desktop).
3. Room with one new message after visiting old history opens near unread/new
   context, not at the old history anchor.
4. Room opened at read marker shows a jump-to-latest affordance when latest is
   outside the focused/read-marker window.
5. Jump latest from a focused/read-marker window reaches the true current latest
   event after an external sender posts a new message.
6. Open while notification/read-marker state lags the newest server event does
   not restore unrelated old history and still exposes a clear jump-latest path.
7. Live append while pinned follows bottom and marks read after the visible delay.
8. Live append while scrolled up preserves the current visible anchor.
9. SDK timeline reset/sync gap while pinned reattaches to live tail.
10. SDK timeline reset/sync gap while scrolled up preserves the visible anchor or
   keeps a clear jump-to-latest affordance.
