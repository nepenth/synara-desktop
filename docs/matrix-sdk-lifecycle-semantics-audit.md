# Matrix SDK Lifecycle Semantics Audit

Date: 2026-06-22

> **Historical audit with still-relevant behavioral contracts.** SDK ownership
> has since moved to the shared Rust core. Viewport, receipt, subscription, and
> lifecycle rules below remain design evidence; current ownership is documented
> in [the codebase knowledge base](../CODEBASE_KNOWLEDGE_BASE.md).

## Purpose

This audit covers a class of bugs where Synara uses Matrix SDK APIs directly but
still violates SDK lifecycle semantics. The raw REST boundary check catches
unapproved `/_matrix` calls; it does not catch mistakes such as treating read
receipts as viewport commands, retaining live timeline references across SDK
timeline resets, or assuming a focused event timeline is equivalent to the live
room timeline.

## Timeline Viewport Contract

The timeline viewport is owned by the client UI. Matrix SDK state informs the
data window, unread markers, and receipts, but it must not continuously drive
scroll position after the room has mounted.

- Read marker / fully-read / read receipt: initial placement only when entering
  a room or opening an explicit unread target.
- Current viewport: local UI state, represented by a visible event anchor plus
  offset when not at the live end.
- Live events: append and auto-follow only when the user is pinned to bottom.
- Jump to latest: fetch or rebind a latest SDK timeline/window, then scroll to
  the newest loaded event.
- Timeline reset or sync gap: preserve the current viewport anchor unless the
  user is pinned to bottom; if pinned, reattach to the new live timeline and
  continue following bottom.
- Focus changes, receipt/account-data updates, and unread count changes must not
  reposition an already-mounted room.

See `docs/timeline-open-focus-contract.md` for the cross-platform open-room
focus matrix used by the Timeline Resurrection epic.

## Confirmed SDK Semantics

Desktop uses `matrix-js-sdk`. Its `EventTimelineSet` docs state that the live
timeline is special and may stop being the live timeline after a sync gap. Code
must not treat retained live timeline references as stable ownership of the live
room tail. When the SDK emits `TimelineReset` or `TimelineRefresh`, UI code must
decide whether to preserve the current viewport or rebind to the new live tail.

Desktop `matrix-js-sdk` also exposes `getLatestTimeline(timelineSet)`, which
fetches the latest room event via `/messages` and constructs context around it.
Jump-to-latest should use that path or equivalent SDK support instead of merely
scrolling to the end of the currently loaded local slice.

iOS uses Matrix Rust SDK timelines. `Room.timelineWithConfiguration(focus:
.event(...))` is a focused context timeline, not the same thing as the live room
timeline. Read-marker focused timelines are valid for initial placement, but
open-room update streams should return to the live room timeline unless the route
itself is an explicit focused event/thread route.

## Findings

### Finding 1: Desktop Timeline Viewport Ownership

Status: fixed in `8baaf28`; hardened in `fc9fc66`.

`RoomTimeline` mixed saved viewport restoration, read-receipt/unread scrolling,
live-end pinning, and SDK timeline refresh handling. A mounted room could be
repositioned by unread/read-marker updates or focus changes, and jump-to-latest
could stop at the end of the currently loaded local slice.

Remediation:

- Read-marker updates no longer command scroll after initial placement.
- Focus regain only marks visible bottom content read; it does not jump to the
  read marker.
- Jump-to-latest calls `getLatestTimeline(...)` and then pins to the live end.
- SDK `TimelineReset` is tracked so empty replacement live timelines do not blank
  the viewport.
- Live events reattach to a new live timeline only when the user is pinned to
  bottom or the viewport is empty.
- Stale non-bottom local viewport anchors no longer override unread room opens;
  desktop now restores saved historical anchors only when they are fresh and the
  room has no unread state.

### Finding 2: iOS Read-Marker Focus Leaked Into Live Updates

Status: fixed in this pass.

`RoomTimelineView.startTimelineUpdates` defaulted to
`focusedEventID ?? initialReadMarkerEventID`. That meant a normal room opened at
the read marker could keep streaming a read-marker focused timeline instead of
the live room timeline. The read marker should influence initial placement only.

Remediation:

- Default update stream focus is now `focusedEventID` only.
- Explicit jump-to-latest still forces live stream focus with an override.
- Initial load can still use `initialReadMarkerEventID` to position the first
  visible window.

Regression coverage:

- `TimelineServiceTests` covers that read markers can seed initial load focus
  but do not become mounted-room update stream focus.

### Finding 3: Desktop Read Helpers Use Cached Live Timeline Slices

Status: fixed in follow-up sweep.

`synara/src/app/utils/notifications.ts` sends read receipts to the newest event
in `room.getLiveTimeline().getEvents()`. This is usually correct when called
from a mounted, live-end-pinned timeline, but it can be stale after a sync gap or
before latest rebind. `roomHaveUnread(...)` in `synara/src/app/utils/room.ts`
also derives unread state from the current live slice when SDK notification
counts are insufficient.

Risk:

- Mark-as-read can acknowledge an older live-slice event and leave the room
  appearing unread.
- Room unread state can be wrong when the local live timeline is partial.

Remediation:

- `markAsRead(...)` now defaults to resolving the latest SDK timeline before
  choosing the read receipt target.
- Mounted-room auto-read paths explicitly opt into `loaded-live-tail` mode when
  the UI has already proven it is pinned to the live end.
- `roomHaveUnread(...)` no longer infers unread state from a partial loaded
  slice unless that slice contains the read marker needed to order events.
- Tests cover latest-timeline receipt selection, loaded-tail receipt selection,
  and conservative unread inference.

### Finding 4: Desktop Room State Hooks Subscribe To Old RoomState References

Status: fixed in follow-up sweep.

`useRoomState` subscribes to the current `room.getLiveTimeline().getState(...)`
object once. `matrix-js-sdk` can replace the room state object when the live
timeline is reset. The SDK emits room current-state updates, but this hook does
not resubscribe on those lifecycle changes.

Risk:

- Room settings/state UI can stop updating after a limited sync or live timeline
  reset.

Remediation:

- Shared state helpers now read `room.currentState` rather than deriving current
  state from the live timeline object.
- `useRoomState` subscribes at the room level and handles
  `RoomEvent.CurrentStateUpdated`, so SDK state replacement after timeline
  refreshes cannot strand the hook on an old `RoomState`.
- Tests cover the current-state helper preference.

### Finding 5: Desktop Notification/Preview Scans Are Cached-Slice Best Effort

Status: hardened in follow-up sweep.

`useRoomLatestRenderedEvent`, notification rescans, and some reader/preview
helpers inspect `room.getLiveTimeline().getEvents()`. These are display or
best-effort notification paths, not viewport owners. They can miss events across
timeline gaps but do not directly reposition the user.

Remediation:

- Best-effort display paths now use an explicit `getLoadedLiveTimelineEvents`
  helper rather than scattered direct live-timeline calls.
- Latest-rendered-event and event-reader hooks refresh on SDK timeline
  reset/refresh events so they do not retain stale display state.
- Call widget timeline reads remain loaded-tail/local-cache reads by contract,
  but direct SDK calls have been routed through the same helper.

## Mechanical Guardrails

Current guardrail:

- `npm run check:matrix-boundaries` blocks new raw Matrix REST or `URLSession`
  usage outside approved exceptions.
- iOS `RoomTimelineFocusPolicy` tests cover initial-load focus versus mounted
  live-update stream focus.
- Desktop tests cover latest-room versus loaded-live-tail read receipt modes,
  conservative unread inference from partial live slices, and room current-state
  helper preference.
- `docs/timeline-open-focus-contract.md` defines the shared open-room focus
  matrix and smoke checklist.

Needed guardrails:

- A timeline viewport policy test matrix for desktop and iOS:
  - read marker changes after mount do not scroll
  - live append while pinned follows bottom
  - live append while scrolled up preserves anchor
  - SDK timeline reset while pinned reattaches to live tail
  - SDK timeline reset while scrolled up preserves anchor
  - jump latest from stale/focused timeline resolves latest SDK window
- A small SDK lifecycle review checklist for new Matrix UI features:
  - Is this API returning a stable object, a live object, or a focused context?
  - Can the SDK replace this object after sync gaps?
  - Does this state drive display only, or can it move viewport/read state?
  - Does an explicit user command need latest-server semantics?
