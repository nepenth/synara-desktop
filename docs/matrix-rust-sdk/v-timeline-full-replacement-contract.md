# V-TIMELINE — full native replacement contract

| Field | Value |
| --- | --- |
| Status | Design gate; no cutover or acceptance claimed |
| Owner | Managed Rust `matrix-sdk-ui::Timeline` plus an SDK-neutral presenter |
| Policy | Physical JS deletion follows a complete operating path, never a shell swap |

## Decision

The existing `RoomTimeline.tsx` is one Matrix-owned operating path. Deleting it
in exchange for the current flat native snapshot would remove retained user
behavior, so it is not a valid V-TIMELINE.1 completion. The uncommitted
virtualized-timeline candidate must not be merged as a replacement.

The previously checked-in desktop selector for that flat renderer is withdrawn:
until the complete native presenter exists, `RoomTimeline.tsx` remains the sole
active timeline presenter. A logged-in desktop session must not select an
incomplete native text shell and then fall back to the legacy owner.

V-TIMELINE.1 through V-TIMELINE.5 may be delivered in dependency-ordered
native slices, but the JS timeline owner is deleted only when every behavior it
continues to own has a native read/action path. There is no native/JS presenter
fallback for a logged-in desktop session.

## Required operating path

```text
room or event-link open
  → native timeline open(room, initial position)
  → Rust-owned SDK timeline / pagination / crypto projection
  → bounded TimelineViewSnapshot or native delta stream
  → SDK-neutral virtualized React presenter
  → typed native action command
  → authoritative native readback or delta
```

The routed `eventId` must select a focused/event-context opening path; silently
dropping it is an ownership defect. The presenter must also preserve viewport
restore, jump-to-latest/unread, pagination state, day/unread rows, loading and
error states.

Implementation foundation: `matrix_timeline_open` now accepts a versioned
typed native request with either `live_bottom` or `focused(eventId)` position
and returns that position in authoritative Rust readback. This fixes the first
owner-boundary loss without activating a new presenter. Native unread and
restored-viewport positions remain pending with their read-frontier/viewport
owners; they must not be silently mapped to live bottom.

## Native boundary

The current `{ eventId, sender, type, body, timestamp }` item projection is
insufficient. The target boundary is one bounded product DTO, not an SDK event
graph:

```text
TimelineViewSnapshot
  sessionGeneration, roomId, revision
  position: live_bottom | unread | focused | restored (+ target event when needed)
  pagination: backward / forward state
  readState: own frontier + unread anchor
  rows: TimelineRow[]
  actionCapabilities: per-room and per-row booleans
```

`TimelineRow` is an exhaustive, versioned product union. It includes normalized
message/rich body and relation state, sticker/media handles, poll, membership,
state, call, redacted, encrypted-unavailable, and bounded unknown/developer
rows; it also includes presentation rows for dates, unread marker, read marker,
intro, and pagination. It never includes a `MatrixClient`, `Room`, `MatrixEvent`,
raw event graph/content, ciphertext, media bytes, tokens, or SDK/Ruma types.

## Actions and sequencing

The presenter invokes narrow native commands rather than a generic event API.
The required sequence is:

1. Define the snapshot/delta, focus, pagination, and viewport contract.
2. Re-home actions before retaining their affordance: reactions (V-SEND.2),
   reply/edit/forward/rich sends, receipts/read markers (V-TIMELINE.3),
   redact/report/pin, account-data-backed notes/later, and media resolution.
3. Deliver the SDK-neutral virtualized renderer on the native rows. Live
   updates ultimately use Rust timeline subscriptions/deltas; polling is not a
   final live-update owner.
4. Delete `RoomTimeline.tsx`, its CSS, its JS timeline listeners/pagination/
   context helpers, JS event synthesis, and tests used exclusively by the
   former operating path. Shared utilities remain until their other consumers
   are re-homed.

The existing focused UTD readback bridge is a V-CRYPTO.6 dependency, not the
native timeline presenter. It remains only until the full timeline DTO and
presenter replace the legacy row path; it must be deleted in that cutover and
must not be used as a model for a new native/JS hybrid route.

The V-ROOMS opaque avatar protocol is a useful narrow media pattern, but it
does not close general timeline media delivery/cache lifecycle requirements.

## Acceptance evidence

Completion requires retained-behavior proof for focused links, live and unread
opens, bidirectional pagination, viewport restoration, rich/relation/media/
state rows, and each still-visible action. Record the JS file/import deletions
and generated inventory delta. A passing typecheck, a slim text renderer, or a
virtualized shell alone is not acceptance evidence.
