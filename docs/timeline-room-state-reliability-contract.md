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

`m.fully_read`, public `m.read`, and private `m.read.private` are shared,
event-level Matrix read signals. None is a pixel viewport. A read-marker write
uses `POST /_matrix/client/v3/rooms/{roomId}/read_markers` and contains
`m.fully_read` plus exactly one receipt key—`m.read` or `m.read.private`—according
to the user's receipt privacy mode.

Each client exposes equivalent internal values:

```text
RoomReadState {
  fullyReadEventId?: EventId
  publicReceiptEventId?: EventId
  privateReceiptEventId?: EventId
  effectiveFrontierEventId?: EventId
  hasUnread: Boolean
  frontierSource: fullyRead | publicReceipt | privateReceipt |
                  currentLiveBottom | absent
  receiptPrivacy: public | private
}

TimelineMode = live | unread(markerEventId) | focused(eventId)
NavigationPhase = idle | loadingContext | rebindingLive |
                  settlingLayout | bottomConfirmed | error
```

Rules:

1. An explicit event route opens a bounded focused context and takes precedence.
2. The effective frontier is the newest comparable event among `m.fully_read`,
   the current user's unthreaded public receipt, and the current user's
   unthreaded private receipt. Ordering comes from the SDK timeline/event graph,
   never lexical event IDs or origin timestamps alone.
3. A room with unread state opens a bounded context around that effective
   frontier and places the first following visible event at the top of the
   viewport.
4. A room without unread state restores a valid current-session event/pixel
   viewport when the user deliberately left history; otherwise it opens live.
5. A candidate that is missing, purged, inaccessible, or outside the bounded
   live graph cannot make a known-newer receipt appear older. When the newest
   frontier cannot be established without walking linked history, fall back to
   live and display a non-blocking "Unread position unavailable" notice.
6. A marker or receipt change after mount updates state but never moves the
   viewport.
7. Viewport state is device-local and is invalidated on leave, purge,
   incompatible timeline reset, logout, or a newer explicit navigation intent.
8. Read writes are serialized per room, resolve the authoritative live-tail
   event at execution time, coalesce to the newest known event, and
   cannot regress when an older request completes late.
9. Custom unread state clears only after the server marker succeeds. Transient
   failure retries after sync resumes without claiming success in the UI.
10. Automatic read advancement requires an active app, the current live provider
    generation, and a live-tail sentinel continuously visible for one second.

## Room activity, Favorites, and sort

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

- Unfiltered live timeline events and local echoes update the store before room
  category rendering. The stored last-qualifying-activity timestamp is
  monotonic. Decryption and remote echo may refine the preview without removing
  the room or moving activity backward.
- Message-like bumping events count as activity. Back-pagination, reactions,
  receipts, typing, presence, ordinary state changes, redactions, and edits do
  not create new activity timestamps. An edit retains its original message's
  membership effect, and a redaction does not erase the fact that qualifying
  activity occurred.
- There is no 24-hour Recent partition. Joined rooms with Matrix `m.favourite`
  appear under Favorites; remaining joined rooms appear under Rooms (and iOS
  spaces/DMs). Every visible joined room appears in exactly one of those lists.
- One global sort applies to both Favorites and remaining rooms: recent activity
  (native `last_activity_ts` / SDK latest-event timestamp, missing ts last) or
  name. Sort preference is device chrome (`synara.roomListSort`, default recent)
  and does not sync via account data.
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
