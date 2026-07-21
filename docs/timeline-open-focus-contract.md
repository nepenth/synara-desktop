# Timeline Open Focus Contract

Date: 2026-06-29

> **Superseded behavior:** The cross-client implementation contract is now
> [timeline-room-state-reliability-contract.md](timeline-room-state-reliability-contract.md).
> In particular, normal opening is no longer defined as always-live when shared
> unread state exists. This document retains lower-level implementation history.

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
3. A read marker seeds a bounded unread context when shared unread state exists.
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

| Scenario                                                       | Desktop Behavior                                                                           | iOS Behavior                                                                                           | Required Outcome                                                                                  |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| Fully read room, no saved viewport                             | Open latest live window and pin bottom.                                                    | Open live timeline and scroll bottom.                                                                  | User sees the live end without history traversal.                                                 |
| Fully read room, saved bottom viewport                         | Restore bottom/live-end state.                                                             | Open live timeline and scroll bottom.                                                                  | User sees the live end.                                                                           |
| Fully read room, fresh saved historical viewport               | Restore visible event anchor without treating it as unread.                                | N/A until iOS persists room viewport anchors.                                                          | Desktop returns to recent local reading position; iOS stays live until persisted viewport exists. |
| Fully read room, stale saved historical viewport               | Ignore historical anchor and open live end.                                                | Open live timeline and scroll bottom.                                                                  | Old history cannot hijack room open.                                                              |
| Room with unread notification count or unread anchor           | Ignore saved historical viewport; open a bounded unread context.                           | Open the equivalent bounded unread context.                                                            | First event after the shared fully-read marker is placed at the top.                              |
| Room with one new event after old local history visit          | Ignore old local history anchor.                                                           | Use the bounded unread context until the user chooses live.                                            | Client does not scroll through days-old history before showing the relevant new/live area.        |
| Read marker or notification state lags the newest server event | Do not restore unrelated old history; expose jump-latest when not confidently at live end. | Keep the bounded unread provider until the user chooses live.                                          | Stale sync state cannot make old local viewport memory look current.                              |
| Explicit event route                                           | Open event-focused context and highlight target.                                           | Open event-focused context and highlight target.                                                       | Route target wins over unread, read marker, and saved viewport.                                   |
| Jump latest from historical/focused window                     | Fetch/rebind latest timeline window, then pin bottom.                                      | Load latest timeline and restart live stream focus.                                                    | User reaches true current live end.                                                               |
| Timeline reset or sync gap while pinned                        | Reattach to new live tail and continue following bottom.                                   | Live timeline stream remains live-focused unless route is explicit event/thread.                       | No blank viewport or stale focused stream.                                                        |
| Timeline reset or sync gap while reading history               | Preserve current visible anchor if possible.                                               | Preserve current loaded list position where SwiftUI can; show jump-to-latest if no longer at live end. | SDK lifecycle changes do not surprise-scroll the user.                                            |

## Desktop Enforcement

- `RoomTimeline` stores process-local viewport snapshots with `updatedAtMs`.
- `shouldRestoreRoomTimelineViewport(...)` rejects non-bottom snapshots when:
  - the room has unread state (`hasUnreadSignal` via
    `shouldGateViewportRestoreOnUnread`);
  - the snapshot has no finite timestamp;
  - the snapshot is older than `ROOM_TIMELINE_VIEWPORT_RESTORE_TTL_MS`.
- `roomHaveUnread(...)` is consulted synchronously so a loaded live slice can
  block stale historical restore before Jotai unread state settles.
- Initial unread placement uses a bounded marker-focused provider even when the
  marker is outside the initial live window. It must not traverse intervening
  history to reach the marker.
- Jump to Unread remains available when an unread target exists and is either
  outside the live chain or outside the currently rendered timeline window
  (`shouldShowJumpToUnread`), even if the marker is still in the live timeline
  chain.
- Mounted auto-read paths use loaded-tail receipts only after the UI has proven
  live-bottom visibility.
- Explicit jump-latest uses `getLatestTimeline(...)`.
- Normal live-end open also refreshes via `getLatestRoomTimeline(...)` when the
  room is pinned to live, without forcing a history walk.
- `TimelineWindow.range` is authoritative for row construction. Linked SDK
  history may be large, but no more than the configured 200-row window is built.
- Backward and forward pagination move the bounded range toward the requested
  edge and restore an event-ID/offset anchor after layout.
- Live-end pinning only records bottom state after virtualized geometry confirms
  the bottom. A timeout force-renders the bottom once and records failure if it
  still cannot be confirmed.
- Correlated diagnostics always emit to the native desktop log and mirror through
  `perfLog` when performance debug is enabled. Payloads contain a random room-open
  trace ID, counts, ranges, and state only.

### Remaining timeline risk

Bounded rendering and deterministic range movement are implemented. Remaining
risk is runtime geometry under unusually tall or late-loading message content,
plus SDK timeline replacement while the user is reading history. Treat the
large-room smoke cases and correlated traces as release gates until repeated
daily-use evidence is clean.

## iOS Enforcement

- `RoomTimelineFocusPolicy.initialLoadFocus(...)` selects a bounded unread focus
  when shared unread state and `m.fully_read` exist; explicit event routes retain
  higher priority.
- Mounted read-marker changes do not move the viewport. The active unread or
  focused provider remains detached until an explicit Jump to Latest succeeds.
- Explicit focused routes keep event focus until jump-latest passes an override
  of `nil`, which returns the stream to live.
- `RoomTimelineView` positions the first visible event following the fully-read
  marker at the top for unread rooms. Fully read rooms restore only a valid
  current-session viewport or use the live bottom.
- `RoomTimelineView.jumpToLatest(...)` clears marker presentation state, restarts
  live stream focus, and calls `loadLatestTimeline(...)`.
- All programmatic room scrolls pass through one cancellable coordinator. State
  updates select one action in priority order: pagination anchor, initial live
  end, focused event, pending jump-latest, or live append.
- Live append follows only an established live-end state. Bottom-sentinel layout
  churn cannot independently move the user into or out of history mode.
- Normal room open invalidates the cached live Matrix Rust SDK timeline before
  loading its bounded page; stream attachment does not paginate it again.
- Empty or failed refresh snapshots preserve the last rendered non-empty list.
- Release diagnostics use the `timeline` OSLog category with a random room-open
  trace ID and no room IDs, event IDs, message text, or server addresses.

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
11. Desktop: unread marker outside the initial live window opens a bounded unread
    context with the first following event at the top; it does not walk history.
12. iOS: unread normal open uses the same bounded marker context and never snaps
    to a different position after first stable placement; explicit links win.
13. Desktop: paginate backward and then forward through more than 200 linked
    events; each direction advances and the visible event retains its offset.
14. iOS: receive at least ten live messages while pinned; each update uses one
    scroll request and the timeline never becomes blank.
15. iOS: scroll into history while live messages arrive; the visible rows remain
    stable and Jump to Latest remains available.

See [timeline-diagnostics.md](timeline-diagnostics.md) for capture instructions.
