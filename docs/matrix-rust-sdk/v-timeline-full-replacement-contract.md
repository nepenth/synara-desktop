# V-TIMELINE — full native replacement contract

| Field  | Value                                                                      |
| ------ | -------------------------------------------------------------------------- |
| Status | Design gate; no cutover or acceptance claimed                              |
| Owner  | Managed Rust `matrix-sdk-ui::Timeline` plus an SDK-neutral presenter       |
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
typed native request with `live_bottom`, `unread`, `focused(eventId)`, or
`normal` (optional viewport restore hint) position and returns that position
in authoritative Rust readback. An unread open obtains the native room's
unread signal and `m.fully_read` frontier, then opens SDK event context at
that frontier and returns the anchor for the future presenter to place the
first-unread row after it. It rejects missing unread or frontier state rather
than silently mapping to live bottom. Normal open restore uses the typed
hint plus TTL / live-tail matching; jump-to-latest rebinds through
`matrix_timeline_jump_latest`. Presenter-local scroll-offset application for
restored anchors remains pending with the selected virtualized owner.

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
message/relation (including SDK-sanitized `formatted_body` and text/notice/emote
labels), poll, membership/profile/state summaries, call, redaction, sticker/
media handles, and UTD row shapes. Reaction ownership is calculated from the
active native user; row `react` opens when V-SEND.2 commands are on tip and the
unselected presenter/RoomTimeline consume `matrix_timeline_reaction_toggle`.
Poll/call action commands exist; presenter selection and live authenticated
viewport proof still remain. Body drafts remain local. The flat `NativeTimelineSnapshot` remains only for the explicitly
temporary V-CRYPTO.6 bridge until those owners replace it.

The managed SDK timeline now also owns an ordered subscription from the same
pre-snapshot boundary and emits `matrix-timeline-view-updated` batches. Each
batch contains only the versioned product row operations and a monotonic native
revision plus the exact opaque stream ID returned by the open readback; it is
aborted with the session timeline registry. The unselected renderer-side
bridge registers its event listener before `matrix_timeline_open`, keeps only
the exact returned stream, and rejects revision gaps or malformed operations
instead of fetching through the JS timeline. The unselected virtualized
presenter consumes only that product DTO and invokes only capability-gated
native pagination/read/jump-latest commands. It is not an active presenter or a
fallback route: complete retained action ownership and live authenticated
viewport proof remain absent. The initial unread/read-frontier open is
native-owned. Normal opens now also carry a typed local viewport restore hint
(`at_bottom`, `live_tail_event_id`, `updated_at_ms`, `restored_anchor_event_id`)
and resolve placement with legacy-compatible precedence: unread beats
historical restore, while an exact live-tail at-bottom match may keep live
bottom. Jump-to-latest is a stream-addressed native command that closes the
prior stream and returns a fresh live-bottom open readback. Live read-frontier and pagination-state
signals now travel on the same `matrix-timeline-view-updated` stream as row
ops: optional `readState` / `pagination` metadata (including metadata-only
batches) are projected from room-info, own-receipt, and live back-pagination
status subscriptions. Restored scroll-offset application remains presenter-
local until the selected virtualized owner lands.

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

| Visible timeline capability | Required native owner before final cutover         | Current status                                                                                                                                                                                                                                                                 |
| --------------------------- | -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Reactions                   | V-SEND.2 typed toggle/ensure/redact commands       | Merged on tip via [#239](https://github.com/nepenth/synara-desktop/pull/239); row `react` capability open for remote message/sticker/poll; RoomTimeline + unselected presenter invoke `nativeReactionOwner`                                                                                                                                  |
| Plain-text reply            | Native send plus native reply draft/composer state | Transport via `matrix_send_text`/`reply_to`; `matrix_composer_{set,clear,get}_reply_draft` owns the reply target with typed readback; RoomTimeline/RoomInput consume that owner on desktop. Body drafts remain local Slate/localStorage                                        |
| Rich send, edit, forward    | Typed send/edit/forward DTO commands               | `matrix_send_text` accepts optional `formattedBody`; edit accepts optional HTML; text forward and media/sticker forward (`matrix_timeline_forward_media`) exist with typed readback                                                                                            |
| Redact, report, pin         | Typed room-event action commands                   | `matrix_timeline_redact`, `matrix_timeline_report`, `matrix_timeline_pin`, and `matrix_timeline_unpin` exist with typed readback                                                                                                                                               |
| Mark read/unread, receipts  | Native receipt/read-frontier command and readback  | Stream-addressed private `m.read` / unread-flag command and snapshot readback exist; unread opening uses the native `m.fully_read` frontier; normal open restore policy and jump-to-latest are native; live frontier and pagination metadata now emit on the view-delta stream |
| Save/later/notes/reminders  | Typed account-data commands and snapshot           | Pending                                                                                                                                                                                                                                                                        |
| Media/sticker image display | Bounded native media-handle resolver               | Wired on the unselected presenter via stream/session-bound opaque handles and the shared `synara-media` protocol; selection still deferred                                                                                                                                     |
| Poll vote and call controls | Typed poll/call commands with capability readback  | `matrix_timeline_poll_vote` and `matrix_timeline_call_decline` exist with typed readback; row capabilities expose `vote` / `decline_call`; PollContent prefers the native vote owner on desktop                                                                                                                                               |

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

The implemented registry binds each random 256-bit-suffix handle to session
generation, exact opened stream, and source item; caps entries; rejects unknown
or revoked handles; and revokes them on ordered diffs, explicit stream close,
or session drop. No MXC URI, encryption descriptor, download URL, media bytes,
or credential enters `TimelineViewSnapshot`, a delta batch, or a Tauri command
payload. The unselected presenter forms image/file/audio/video/sticker URLs
only with `convertFileSrc(handle, "synara-media")`.

### Cross-vertical protocol ownership gate

`V-ROOMS.1` established the application's sole `synara-media` URI-protocol
registration for opaque invite-avatar capabilities. `V-TIMELINE` extends that
same native protocol owner rather than registering a second handler. The
shared resolver dispatches by validated handle type before lookup, resolves
timeline sources through `MatrixAuthState`, downloads/decrypts with the SDK,
and returns only byte-validated allowlisted MIME types with `no-store` and
`nosniff`.

This media slice does not select the native presenter, delete
`RoomTimeline.tsx`, or claim V-TIMELINE completion.

## Acceptance evidence

Completion requires retained-behavior proof for focused links, live and unread
opens, bidirectional pagination, viewport restoration, rich/relation/media/
state rows, and each still-visible action. Record the JS file/import deletions
and generated inventory delta. A passing typecheck, a slim text renderer, or a
virtualized shell alone is not acceptance evidence.
