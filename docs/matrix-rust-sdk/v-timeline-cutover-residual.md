# V-TIMELINE — cutover residual map after #240 / #284

| Field  | Value                                                                                          |
| ------ | ---------------------------------------------------------------------------------------------- |
| Status | Cutover **approved**; residual map only — no product code in this PR                            |
| Tip    | `0bfcd2b53c8476f795cc9e49b3d4cbbc0d4cad40` (merge of [#284](https://github.com/nepenth/synara-desktop/pull/284) CI parallel Validate, on top of [#240](https://github.com/nepenth/synara-desktop/pull/240) V-TIMELINE contract) |
| Policy | [full-vertical-policy.md](full-vertical-policy.md); [cutover-operating-model.md](cutover-operating-model.md) |
| Owner  | Managed Rust `matrix-sdk-ui::Timeline` + SDK-neutral presenter (`NativeTimelinePresenter`)       |

## 1. Tip measured

- **Branch:** `matrix-rust/v-timeline-cutover-residual-map`
- **HEAD:** `0bfcd2b53c8476f795cc9e49b3d4cbbc0d4cad40`
- **History:** `0bfcd2b5` = merge #284 (CI parallel Validate) → `7030cacc` merge tip after #240 into CI parallel Validate → `9f2d033d` merge #240 (V-TIMELINE contract).
- **Base:** `feature/matrix-rust-sdk-full-replacement` (0 ahead / 0 behind at measurement).

## 2. Product path today

`RoomView` still mounts the JS `RoomTimeline` as the **sole active timeline presenter**; the native
`NativeTimelinePresenter` exists but is **unselected** (no route mounts it).

- `synara/src/app/features/room/RoomView.tsx` — imports `RoomTimeline` (line 12) and renders
  `<RoomTimeline key={roomId} room={room} eventId={eventId} … />` (lines 96–104). This is the only
  active timeline mount.
- `synara/src/app/features/room/RoomTimeline.tsx` — the JS timeline operating path (Matrix-owned
  `getUnfilteredTimelineSet`, pagination, viewport restore, JS event synthesis, media, actions).
- `synara/src/app/features/room/NativeTimelinePresenter.tsx` — the SDK-neutral virtualized presenter
  (`@tanstack/react-virtual`). **Not selected**: grep for `NativeTimelinePresenter` finds only the
  definition plus comments in `nativeTimelineAction.ts`, `nativeTimelineActions.ts`,
  `nativeComposerDraftOwner.ts`, `nativeLaterOwner.ts`, `nativeRoomNotesOwner.ts` — no mount site.
- `synara/src/app/features/room/nativeTimelineView.ts` — `useNativeTimelineView` hook; header comment
  (lines 398–401) states it is "deliberately not an activation switch: until the full presenter and
  action/media routes exist, RoomTimeline remains the active owner."

Cutover is **approved** (user: full-steam js-sdk replacement; select presenter + delete RoomTimeline
allowed; break/fix-forward OK). This map measures the remaining slices; it does not recommend HOLD.

## 3. Remaining cutover slices

| ID | Slice | Path | Current owner | Native gap | Done when |
| -- | ----- | ---- | ------------- | ---------- | --------- |
| V-TIMELINE.C1 | Select presenter in RoomView | `synara/src/app/features/room/RoomView.tsx` | `RoomTimeline` (JS) | `NativeTimelinePresenter` exists but unmounted; no selection switch | `RoomView` mounts `NativeTimelinePresenter` (native open/readback) as the sole active timeline; no JS fallback route |
| V-TIMELINE.C2 | Delete RoomTimeline + dead imports | `synara/src/app/features/room/RoomTimeline.tsx`, `RoomTimeline.css.ts` | `RoomTimeline` (JS) | Presenter unselected, so JS owner still required | `RoomTimeline.tsx`/`.css.ts` deleted; JS timeline listeners/pagination/context helpers, JS event synthesis, and tests used only by the former path removed; shared utilities retained only for other consumers; import delta recorded |
| V-TIMELINE.C3 | Stream delta binding gaps | `synara/src/app/features/room/nativeTimelineView.ts` (`useNativeTimelineView`) | Native stream (`matrix-timeline-view-updated`) | Binding implemented on the unselected presenter: registers listener before open, keeps exact returned `streamId`, rejects revision gaps / malformed ops, aborts with session registry. No gap found at tip | No residual gap — binding is complete; re-verify live authenticated viewport proof after C1 |
| V-TIMELINE.C4 | Media / render parity gaps | `synara/src/app/features/room/NativeTimelinePresenter.tsx` (`NativeTimelineMedia`), `nativeTimelineView.ts` (`nativeTimelineMediaSrc`) | Native media-handle registry + `synara-media` protocol | Image/audio/video/sticker/file render via opaque handles + `convertFileSrc(handle, "synara-media")`; parity with legacy `RenderMessageContent`/`Image`/`ImageViewer` unproven on the selected path | Selected presenter renders every retained media/sticker row with native handles; legacy JS media components no longer needed for timeline rows |
| V-TIMELINE.C5 | Pins / notes / jump residual | `NativeTimelinePresenter.tsx` (pin/unpin, Later/notes, `controller.jumpLatest()`), `nativeTimelineAction.ts`, `nativeTimelineActions.ts`, `nativeLaterOwner.ts`, `nativeRoomNotesOwner.ts` | Native `matrix_timeline_pin`/`unpin`, `matrix_later_*`, `matrix_room_notes_*`, `matrix_timeline_jump_latest` | Pin/Unpin gated from stream `pinnedEventIds`; Later/notes via native owners; jump-to-latest is a stream-addressed native command. All wired on the unselected presenter; live authenticated proof unclaimed | Selected presenter surfaces pin/unpin, Later/notes, and jump-to-latest with native owners and live proof; no JS `setAccountData`/pin fallback |

## 4. Native IPC already present (`matrix_timeline_*`)

From `src-tauri/src/lib.rs` (invoke_handler block, lines 425–446) — grep only, no product.rs reads:

| Command | Purpose |
| ------- | ------- |
| `matrix_timeline_open` | Open native timeline view (typed position: live_bottom / unread / focused / normal restore) |
| `matrix_timeline_close` | Close the opened stream |
| `matrix_timeline_jump_latest` | Stream-addressed jump-to-latest (fresh live-bottom open readback) |
| `matrix_timeline_snapshot` | Bounded `TimelineViewSnapshot` readback |
| `matrix_timeline_paginate` | Bidirectional pagination on the stream |
| `matrix_timeline_set_read_state` | Native read/unread-frontier command |
| `matrix_timeline_event_readback` | Event readback (V-CRYPTO.6 bridge dependency) |
| `matrix_timeline_reaction_toggle` | Reaction toggle (V-SEND.2) |
| `matrix_timeline_edit_text` | Rich edit (plain + HTML body) |
| `matrix_timeline_redact` | Redact |
| `matrix_timeline_forward_text` | Text/quote forward |
| `matrix_timeline_report` | Report |
| `matrix_timeline_pin` / `matrix_timeline_unpin` | Pin / unpin (gated by projected `pinnedEventIds`) |
| `matrix_timeline_forward_media` | Media/sticker forward |
| `matrix_timeline_poll_vote` | Poll vote |
| `matrix_timeline_call_decline` | Call decline |

Related non-`timeline` owners used by the presenter: `matrix_composer_{set,clear,get}_reply_draft`,
`matrix_reaction_{ensure,redact}`, `matrix_send_text`, `matrix_send_attachment`, `matrix_send_sticker`,
`matrix_send_poll`, `matrix_poll_respond`, `matrix_later_*`, `matrix_room_notes_*`.

## 5. Explicit non-goals

- **#39** — umbrella merge to `main`; still gated on explicit user approval. Not part of this cutover.
- **dual_backend** — `false` (forbidden forever). No native/JS presenter fallback for a logged-in
  desktop session; no dual-backend flag.
- **Inventing a dual flag** — no new "dual backend" / "native vs JS" runtime switch is introduced.
  Selection is a single cutover (C1), not a toggle.

## 6. Self-eval confidence

- **High** on tip SHA, product path (RoomView → RoomTimeline; NativeTimelinePresenter unselected),
  and the `matrix_timeline_*` command inventory (from lib.rs grep).
- **Medium** on C3/C4/C5 "done when" wording: the native binding and action/media routes are present
  on the unselected presenter, but live authenticated viewport proof and full render parity are
  unclaimed until C1 selects the presenter. These are framed as verification gates, not claims.
- **Medium** on C2 deletion scope: `RoomTimeline.tsx` has many shared imports; exact dead-import
  list requires the C1 selection to be measured. Recorded as a scope, not an exhaustive file list.
