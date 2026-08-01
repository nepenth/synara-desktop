# V-TIMELINE — cutover residual map after C1/C2

| Field  | Value                                                                                          |
| ------ | ---------------------------------------------------------------------------------------------- |
| Status | **C1 done** (presenter selected); **C2 this PR** (RoomTimeline deleted). Residuals C3–C5 remain |
| Tip    | Stacked on [#285](https://github.com/nepenth/synara-desktop/pull/285) C1 presenter cutover head |
| Policy | [full-vertical-policy.md](full-vertical-policy.md); [cutover-operating-model.md](cutover-operating-model.md) |
| Owner  | Managed Rust `matrix-sdk-ui::Timeline` + SDK-neutral presenter (`NativeTimelinePresenter`)       |

## 1. Tip measured

- **Branch:** `matrix-rust/v-timeline-c2-delete-roomtimeline`
- **Base stack:** [#285](https://github.com/nepenth/synara-desktop/pull/285) C1 (`matrix-rust/v-timeline-cutover-presenter`) after tip-merge past [#286](https://github.com/nepenth/synara-desktop/pull/286).
- **Base:** `feature/matrix-rust-sdk-full-replacement` (via #285).

## 2. Product path today (after C1 + C2)

`RoomView` mounts **only** `NativeTimelinePresenter` as the sole active timeline presenter.
JS `RoomTimeline.tsx` / `RoomTimeline.css.ts` are **deleted** (this PR, V-TIMELINE.C2).

- `synara/src/app/features/room/RoomView.tsx` — imports `NativeTimelinePresenter` and renders
  `<NativeTimelinePresenter key={roomId} roomId={roomId} eventId={eventId} />`. Sole timeline mount.
- ~~`synara/src/app/features/room/RoomTimeline.tsx`~~ — **deleted** (C2).
- `synara/src/app/features/room/NativeTimelinePresenter.tsx` — SDK-neutral virtualized presenter
  (`@tanstack/react-virtual`). **Selected** (C1).
- `synara/src/app/features/room/nativeTimelineView.ts` — `useNativeTimelineView` hook; mounted via
  the selected presenter.
- Shared utilities retained for notifications residual (not full rewrite this PR):
  `getLatestRoomTimeline`, `getRoomTimelineOpenMode`, `shouldRestoreRoomTimelineViewport`, etc.

Cutover is **approved** (user: full-steam js-sdk replacement; dual_backend **false**). C1+C2 land
presenter selection + dead JS owner deletion; break/fix-forward OK.

## 3. Remaining cutover slices

| ID | Slice | Path | Current owner | Native gap | Done when |
| -- | ----- | ---- | ------------- | ---------- | --------- |
| V-TIMELINE.C1 | Select presenter in RoomView | `synara/src/app/features/room/RoomView.tsx` | **DONE** — `NativeTimelinePresenter` | — | `RoomView` mounts `NativeTimelinePresenter` as the sole active timeline; no JS fallback route |
| V-TIMELINE.C2 | Delete RoomTimeline + dead imports | `RoomTimeline.tsx`, `RoomTimeline.css.ts` | **DONE this PR** | — | Files deleted; allowlist drop; shared notification/open utilities retained; no broken imports |
| V-TIMELINE.C3 | Stream delta binding gaps | `synara/src/app/features/room/nativeTimelineView.ts` (`useNativeTimelineView`) | Native stream (`matrix-timeline-view-updated`) | Binding implemented: registers listener before open, keeps exact returned `streamId`, rejects revision gaps / malformed ops, aborts with session registry | Live authenticated viewport proof after C1 selection |
| V-TIMELINE.C4 | Media / render parity gaps | `NativeTimelinePresenter.tsx` (`NativeTimelineMedia`), `nativeTimelineView.ts` (`nativeTimelineMediaSrc`) | Native media-handle registry + `synara-media` protocol | Image/audio/video/sticker/file render via opaque handles; parity with legacy renderers unproven on the selected path | Selected presenter renders every retained media/sticker row with native handles |
| V-TIMELINE.C5 | Pins / notes / jump residual | `NativeTimelinePresenter.tsx`, `nativeTimelineAction.ts`, `nativeLaterOwner.ts`, `nativeRoomNotesOwner.ts` | Native pin/later/notes/jump owners | Wired on the selected presenter; live authenticated proof unclaimed | Live proof for pin/unpin, Later/notes, jump-to-latest |

## 4. Native IPC already present (`matrix_timeline_*`)

From `src-tauri/src/lib.rs` (invoke_handler block) — grep only, no product.rs reads:

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
  Selection was a single cutover (C1), not a toggle.
- **Rewriting notifications** off `getLatestRoomTimeline` — residual; not this PR's full rewrite.
- **product.rs** — out of scope for C2.

## 6. Self-eval confidence

- **High** on C1 product path (RoomView → NativeTimelinePresenter only) and C2 file deletion + allowlist drop.
- **Medium** on C3/C4/C5 live authenticated proof and full media/action parity (verification gates, not claims).
