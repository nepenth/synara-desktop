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
intro, and pagination. Date separators carry their neutral SDK timestamp; the
SDK-neutral presenter formats it for its locale rather than Rust inventing a
machine-local day key. The boundary never includes a `MatrixClient`, `Room`,
`MatrixEvent`, raw event graph/content, ciphertext, media bytes, tokens, or
SDK/Ruma types.

The version-one Rust `TimelineViewSnapshot`/`TimelineViewRow` contract now
exists in `src-tauri/src/matrix/timeline/view.rs`, including opaque media
handles and explicit capability gates. `matrix_timeline_open` now returns this
versioned boundary from the managed SDK timeline, without activating a React
consumer. Its projection covers SDK virtual date/read/start rows plus safe
message/relation, poll, membership, profile/state, call, redaction, and UTD
row shapes. Reaction ownership is calculated from the active native user; its
action capability remains closed until V-SEND.2 is integrated. It deliberately
does not claim projection completion: stickers require the native media
resolver, and native unread, pagination/read-frontier, viewport, and presenter
ownership still remain. The flat
`NativeTimelineSnapshot` remains only for the explicitly temporary V-CRYPTO.6
bridge until those owners replace it.

The managed SDK timeline now also owns an ordered subscription from the same
pre-snapshot boundary and emits `matrix-timeline-view-updated` batches. Each
batch contains only the versioned product row operations and a monotonic native
revision plus the exact opaque stream ID returned by the open readback; it is
aborted with the session timeline registry. No React listener exists yet, so
this establishes the live-update owner without activating a partial presenter.
Native unread/read-frontier, pagination-state changes, and viewport restoration
still need their corresponding owner signals before final cutover.

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

### Current action cutover gates

The renderer may show an affordance only after the listed native owner exists.
The current product command surface has timeline read/pagination and plain-text
send-with-`reply_to` only; that transport capability is not parity for the
legacy composer or a rich reply/edit flow.

| Visible timeline capability | Required native owner before final cutover | Current status |
| --- | --- | --- |
| Reactions | V-SEND.2 typed toggle/ensure/redact commands | Candidate exists on its own branch; not integration evidence yet |
| Plain-text reply | Native send plus native reply draft/composer state | Transport input exists; UI owner pending |
| Rich send, edit, forward | Typed send/edit/forward DTO commands | Pending |
| Redact, report, pin | Typed room-event action commands | Pending |
| Mark read/unread, receipts | Native receipt/read-frontier command and readback | Stream-addressed private `m.read` / unread-flag command and snapshot readback exist; unread positioning/frontier signals still pending |
| Save/later/notes/reminders | Typed account-data commands and snapshot | Pending |
| Media/sticker image display | Bounded native media-handle resolver | Pending; invite-avatar handling is not general timeline media |
| Poll vote and call controls | Typed poll/call commands with capability readback | Pending |

No active desktop timeline presenter may retain these JS action paths as a
fallback once the native presenter is selected.

The existing focused UTD readback bridge is a V-CRYPTO.6 dependency, not the
native timeline presenter. It remains only until the full timeline DTO and
presenter replace the legacy row path; it must be deleted in that cutover and
must not be used as a model for a new native/JS hybrid route.

The V-ROOMS opaque avatar protocol is a useful narrow media pattern, but it
does not close general timeline media delivery/cache lifecycle requirements.

### Timeline media owner route

Timeline media is a separate V-TIMELINE owner, not an extension of invite
avatars or a webview `mxc://` conversion. The locked SDK exposes both plaintext
and encrypted attachment/sticker sources through its `MediaSource` abstraction;
the native registry must retain that SDK source and let the SDK obtain and
decrypt bytes. The webview receives only a bounded opaque handle and safe
metadata.

```text
SDK timeline event media source
  → session-scoped native handle registry
  → TimelineMediaHandle (opaque handle + safe metadata)
  → native URI/protocol resolver requests that handle
  → SDK media request/decryption/cache
  → bytes returned directly to the renderer
```

The registry must bind each handle to session generation and its source event,
cap its entries, reject unknown/revoked handles, and revoke handles when the
event disappears or the session ends. No MXC URI, encryption descriptor,
download URL, media bytes, or credential may enter `TimelineViewSnapshot`, a
delta batch, or a Tauri command payload. The presenter may render sticker or
attachment media only after this complete route and bounded resolver readback
exist; a per-kind URL workaround is not an acceptable partial replacement.

### Cross-vertical protocol ownership gate

`V-ROOMS.1` establishes the application's sole `synara-media` URI-protocol
registration for opaque invite-avatar capabilities. `V-TIMELINE` must extend
that one native protocol owner after the candidates are integrated; it must
not register a second handler for the same scheme or route an opaque timeline
handle through the invite-avatar store. The shared resolver dispatches only
after validating the handle in its typed, session-scoped native registry, then
uses that registry's retained source and media policy. This is a sequencing
gate, not a Matrix Rust SDK gap: the timeline registry foundation is currently
unattached, so timeline media remains pending until the shared owner has a
single authoritative readback for both capability classes.

## Acceptance evidence

Completion requires retained-behavior proof for focused links, live and unread
opens, bidirectional pagination, viewport restoration, rich/relation/media/
state rows, and each still-visible action. Record the JS file/import deletions
and generated inventory delta. A passing typecheck, a slim text renderer, or a
virtualized shell alone is not acceptance evidence.
