# Timeline and Room-State Reliability Contract

Status: implementation contract
Applies to: desktop, iOS, Matrix client integration, and release validation

This contract supersedes every older smoke expectation that normal room opening
always goes to the live end. It defines equivalent behavior across clients while
allowing platform-specific rendering implementations.

Normative protocol source: [Matrix read and unread markers](https://spec.matrix.org/latest/client-server-api/#read-and-unread-markers).
Reference implementations include Element Web's bounded timeline/scroll-panel
model and Element X iOS's live-versus-focused provider model.

## Read state and room opening

`m.fully_read` is a shared, event-level Matrix marker. It is not a pixel viewport
and must be written only through `POST /_matrix/client/v3/rooms/{roomId}/read_markers`.
A write contains `m.fully_read` and exactly one receipt key: `m.read` or
`m.read.private`, according to the user's receipt privacy mode.

Each client exposes equivalent internal values:

```text
RoomReadState {
  fullyReadEventId?: EventId
  hasUnread: Boolean
  markerSource: server | receiptFallback | absent
  receiptPrivacy: public | private
}

TimelineMode = live | unread(markerEventId) | focused(eventId)
NavigationPhase = idle | loadingContext | rebindingLive |
                  settlingLayout | bottomConfirmed | error
```

Rules:

1. An explicit event route opens a bounded focused context and takes precedence.
2. A room with unread state opens a bounded context around `m.fully_read` and
   places the first following visible event at the top of the viewport.
3. A room without unread state restores a valid current-session event/pixel
   viewport when the user deliberately left history; otherwise it opens live.
4. A missing, purged, or inaccessible marker falls back to live without walking
   history and displays a non-blocking "Unread position unavailable" notice.
5. A marker change after mount never moves the viewport.
6. Viewport state is device-local and is invalidated on leave, purge,
   incompatible timeline reset, logout, or a newer explicit navigation intent.
7. Read writes are serialized per room, coalesced to the newest known event, and
   cannot regress when an older request completes late.
8. Custom unread state clears only after the server marker succeeds. Transient
   failure retries after sync resumes without claiming success in the UI.
9. Automatic read advancement requires an active app, the current live provider
   generation, and a live-tail sentinel continuously visible for one second.

## Room activity and Recent 24h

Both clients maintain one immutable activity snapshot per visible joined room:

```text
RoomActivity {
  roomId: RoomId
  activityTimestampMs: Integer
  latestRelevantEventId?: EventId
  bumpStamp?: Integer
  revision: Integer
}
```

- Unfiltered live timeline events and local sends update the store before room
  category rendering. Decryption and remote echo may refine the preview without
  removing the room or moving activity backward.
- Message-like bumping events count as activity. Back-pagination, reactions,
  redactions, and edits do not create new activity timestamps.
- Use server/SDK bump data for ordering and change detection, and the relevant
  event timestamp for the 24-hour cutoff. Preserve the last valid timestamp when
  the SDK temporarily lacks a preview event.
- Recent and normal Rooms are one atomic partition of the same snapshot. Every
  visible joined room appears in exactly one partition.
- Recent sorts by descending activity, then case-insensitive room name, then room
  ID. Recompute at the next expiry boundary and on foreground, clock/time-zone
  change, reconnect, membership change, and timeline reset.
- A single-room update must map and publish that room, not remap the full list.

## Timeline providers and scrolling

- Maintain distinct live, unread, and focused providers. A live snapshot replaces
  stable server events; it never unions the full historical window. Preserve only
  unmatched local echoes during reconciliation.
- Load 50 live events initially and retain at most 300 stable server events plus
  unmatched local echoes. Focused/unread context loads at most 50 events on each
  side of the anchor and paginates only from explicit user demand.
- Room or mode changes cancel the prior listener and increment a generation.
  Ignore all late snapshots and scroll completions from stale generations.
- Jump to Latest transitions through `rebindingLive`, replaces the provider with
  a clean bounded live provider, waits for layout and live-tail confirmation, and
  only then hides the control, persists bottom state, or advances read state.
  Failure retains the previous context and returns to an actionable error state.
- Scroll ownership is either `stuckAtLiveBottom` or a stable event ID plus its
  measured viewport offset. Structural changes restore by measured relative
  delta after layout; absolute estimated `scrollTop` is not an anchor.
- Queue/coalesce structural snapshots during active drag or momentum. Pagination
  requires scroll idle, matching direction, acceptable velocity, and re-entry
  into its trigger zone.
- Late media, font, decryption, edit, reply, Dynamic Type, and rotation layout
  changes preserve the same event/offset unless the user issues newer input.

## Failure and privacy boundaries

- Sync gaps and provider resets preserve the visible anchor when possible and
  keep Jump to Latest available when live position is uncertain.
- Empty or failed refreshes retain the last non-empty view and never assert a
  bottom/read confirmation.
- Diagnostics may contain random per-open trace IDs, modes, phases, counts,
  ranges, durations, reason enums, and memory totals. They must never contain
  room/event/user identifiers, message content, server URLs, access tokens, or
  encryption material.

The measurable release budgets and evidence format are in
[timeline-room-state-acceptance.md](timeline-room-state-acceptance.md).
