# V-TIMELINE — cutover residual map after C1/C2

| Field  | Value                                                                                          |
| ------ | ---------------------------------------------------------------------------------------------- |
| Status | **C1 done** (#285) + **C2 done** (#289). C3–C5 live proof is **Not confirmed / optional Beta feedback**, not a completion or merge gate |
| Tip    | `abd736b3` on `feature/matrix-rust-sdk-full-replacement` (after #538 and the tip-policy refresh) |
| Policy | [full-vertical-policy.md](full-vertical-policy.md); [cutover-operating-model.md](cutover-operating-model.md); residual-empty policy below |
| Owner  | Managed Rust `matrix-sdk-ui::Timeline` + SDK-neutral presenter (`NativeTimelinePresenter`)       |

## 1. Tip measured

- **Branch:** `feature/matrix-rust-sdk-full-replacement` (docs-only policy refresh)
- **Base stack:** tip `abd736b3` — after #285 C1, #289 C2, #294 C3 checklist, #295 scoreboard, #296 V-SEND.R-FORWARD, and #538 residual-empty burns.
- **Base:** `feature/matrix-rust-sdk-full-replacement`.

## 2. Product path today (after C1 + C2)

`RoomView` mounts **only** `NativeTimelinePresenter` as the sole active timeline presenter.
JS `RoomTimeline.tsx` / `RoomTimeline.css.ts` are **deleted** (V-TIMELINE.C2, #289).

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
presenter selection + dead JS owner deletion; break/fix-forward and private Beta are accepted.

### Residual-empty completion policy

For a claimed file, branch completion requires the code to be on the measured
tip, focused unit/CI checks to pass, and no `matrix-js-sdk` import to remain in
that file. If the product path needs live Matrix state, native absence or
failure must fail closed; no JS fallback or backend selector is permitted.
HUMAN OPERATOR LIVE-PROOF is optional Beta feedback, not a completion or merge
gate. C3–C5 may therefore remain **Not confirmed** while their tip + unit/CI
engineering evidence is accepted. R-DEVTOOL may start without waiting for
C3–C5 live confirmation.

If a product change changes importer inventory, run
`npm run inventory:matrix-sdk-usage`, ratchet the allowlist `pathCount` and
`paths[]`, update inventory test floors for files, declarations, and buckets,
and update the P1.6 guardrail floors. This docs-only refresh changes no
production importers and keeps the tip inventory at 124 files / 124 paths.

## 3. Remaining cutover slices

| ID | Slice | Path | Current owner | Native gap | Done when |
| -- | ----- | ---- | ------------- | ---------- | --------- |
| V-TIMELINE.C1 | Select presenter in RoomView | `synara/src/app/features/room/RoomView.tsx` | **DONE #285** — `NativeTimelinePresenter` | — | `RoomView` mounts `NativeTimelinePresenter` as the sole active timeline; no JS fallback route |
| V-TIMELINE.C2 | Delete RoomTimeline + dead imports | `RoomTimeline.tsx`, `RoomTimeline.css.ts` | **DONE #289** | — | Files deleted; allowlist drop; shared notification/open utilities retained; no broken imports |
| V-TIMELINE.C3 | Stream delta binding gaps | `synara/src/app/features/room/nativeTimelineView.ts` (`useNativeTimelineView`) | Native stream (`matrix-timeline-view-updated`) | Binding implemented: registers listener before open, keeps exact returned `streamId`, rejects revision gaps / malformed ops, aborts with session registry | **Branch-complete when tip + focused unit/CI + residual-empty/fail-closed evidence pass.** Live desktop proof remains optional Beta feedback and **Not confirmed**; see [v-timeline-c3-stream-verify.md](v-timeline-c3-stream-verify.md) (#294) |
| V-TIMELINE.C4 | Media / render parity gaps | `NativeTimelinePresenter.tsx` (`NativeTimelineMedia`), `nativeTimelineView.ts` (`nativeTimelineMediaSrc`) | Native media-handle registry + `synara-media` protocol | Image/audio/video/sticker/file render via opaque handles; native path is fail-closed where live state is required | **Branch-complete when tip + focused unit/CI + residual-empty/fail-closed evidence pass.** Live authenticated render proof is optional Beta feedback and **Not confirmed**; see [v-timeline-c4-media-render-verify.md](v-timeline-c4-media-render-verify.md) |
| V-TIMELINE.C5 | Pins / notes / jump residual | `NativeTimelinePresenter.tsx`, `nativeTimelineAction.ts`, `nativeLaterOwner.ts`, `nativeRoomNotesOwner.ts` | Native pin/later/notes/jump owners | Wired on the selected presenter; native absence/failure must fail closed | **Branch-complete when tip + focused unit/CI + residual-empty/fail-closed evidence pass.** Live pin/unpin, Later/notes, and jump proof is optional Beta feedback and **Not confirmed**; see [v-timeline-c5-pins-notes-jump-verify.md](v-timeline-c5-pins-notes-jump-verify.md) |

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

## 5a. Related landed / in-flight (historical context; current truth is tip `abd736b3`)

- **V-SEND.R-FORWARD #296 — LANDED** at tip. Legacy `MessageForwardItem` + `utils/forward.ts` deleted;
  native `matrix_timeline_forward_*` is the sole forward path. Production import files **163**.
- **V-SEND.R-PACK-READ #297 — NOT merged** (open at this tip). Native image-pack snapshot get +
  `useImagePacks` fail-closed; do **not** claim it landed until the tip shows it. It owns
  `image_packs.rs`, `nativeImagePack*`, `useImagePacks.ts`, and the `product.rs` pack commands —
  out of scope here.

## 6. Self-eval confidence

- **High** on C1 product path (RoomView → NativeTimelinePresenter only) and C2 file deletion + allowlist drop.
- **Medium** on optional C3/C4/C5 live Beta feedback and full media/action parity; live proof is not a merge gate.
