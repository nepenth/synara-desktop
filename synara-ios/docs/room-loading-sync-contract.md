# Room Loading And Sync Contract

## Problem

The iOS client must feel immediate without presenting stale Matrix SDK cache data as the current room state. A cached room list or timeline slice is acceptable for first paint only; unread state, latest message previews, and "bottom of room" positioning must be corrected by live sync before the UI implies it is current.

## Target Behavior

### Room List

- Hydrate quickly from the local Matrix SDK store so the app is interactive immediately after launch.
- Start a live sync in the background when cached rooms are available.
- Refresh room rows in place after the background sync completes; do not replace the room list with a full-screen loading state.
- Treat unread count, mention count, latest preview, and latest timestamp as provisional until the post-cache refresh has run.
- Keep room rows tappable during refresh. Navigation must not wait for a room-level timeline load.

### Opening A Room

- Route into the room immediately after tap.
- Perform a bounded live sync before constructing the visible SDK timeline.
- If `m.fully_read` is available, position around that event and show the jump-to-latest affordance conservatively.
- If the focused/read-marker window does not include the true latest event, the UI must not imply the user is already at the bottom.
- The jump-to-latest control must fetch a fresh latest timeline window, then scroll to the newest loaded event.
- Older history is loaded only when the user scrolls upward or taps `Load older messages`.

### Longer-Term Implementation Direction

- Prefer SDK room-list/timeline update streams over repeated ad hoc `syncOnce` calls once the Swift bindings expose a stable API for this use case.
- Track per-room freshness state: `cached`, `syncing`, `fresh`, and `failed`.
- Add an explicit "loading newer messages" state for read-marker windows when many newer events exist below the marker.
- Add a live regression smoke that seeds a room, opens from stale cache, sends a new message externally, then verifies the iOS app refreshes the list preview and can jump to the new latest event.

## Current Guardrails

- Room list renders cached rows first, then subscribes to Matrix Rust SDK `SyncService` / `RoomListService` room-entry diffs.
- Room timeline load performs a bounded interactive sync before creating the initial timeline window.
- Open timelines subscribe to Matrix Rust SDK timeline diffs after initial load so incoming events update the visible room without leaving and re-entering.
- Read-marker focused timelines remain bounded and always expose Jump to Latest;
  they never walk all intervening history to reach the live end.
- Jump-to-latest reloads the latest timeline window instead of only scrolling within the existing focused slice.

## SDK Streaming Architecture

- A single Matrix Rust SDK `SyncService` is retained by `MatrixRustSDKClientStore` per restored/logged-in session.
- `MatrixRustSDKRoomListService.roomUpdates()` subscribes to `roomListService.allRooms().entriesWithDynamicAdapters(...)`, maps SDK `Room` snapshots into `RoomSummary`, and yields newest room-list state with buffering policy `bufferingNewest(1)`.
- `MatrixRustSDKTimelineService.timelineUpdates(...)` creates an SDK timeline, attaches a `TimelineListener`, applies `TimelineDiff` changes to an in-memory ordered item list, maps events into Synara `TimelineItem`s, and yields newest snapshots with buffering policy `bufferingNewest(1)`.
- SwiftUI views keep their pull-based initial load for fast deterministic first render and then consume streaming updates for freshness.
- Stream cancellation cancels SDK task handles so room-list/timeline listeners do not leak after navigation.
