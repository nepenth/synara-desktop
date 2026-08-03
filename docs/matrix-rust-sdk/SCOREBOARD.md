# Matrix Rust full-replacement — scoreboard

| Field                                                  | Value                                                                       |
| ------------------------------------------------------ | --------------------------------------------------------------------------- |
| Updated                                                | **2026-08-03** (pipeline pause / audit handoff)                             |
| Tip                                                    | `80af6ce7` on `feature/matrix-rust-sdk-full-replacement`                    |
| Production `matrix-js-sdk` import files (`synara/src`) | **124** (plan baseline was **220**; **96 removed**, ~**43.6%**)             |
| Allowlist `pathCount`                                  | **124** (matches `paths[]` length)                                          |
| Dual backend                                           | **false** (forbidden forever)                                               |
| Pipeline                                               | **PAUSED** — no daytime/overnight spawns until operator resume              |
| Burn board                                             | https://kb.whyland.com/go/synara-matrix-burn                                |
| Umbrella #39                                           | **Do not merge** without explicit user approval                             |

## Operator index — timeline live proofs

These are docs-only operator checklists for the selected native desktop path.
All three live proofs remain **Not confirmed** until an authenticated desktop
run passes the relevant checklist.

| Proof                         | Operator checklist                                                                 | Live proof        |
| ----------------------------- | ---------------------------------------------------------------------------------- | ----------------- |
| V-TIMELINE.C3 stream/delta    | [v-timeline-c3-stream-verify.md](v-timeline-c3-stream-verify.md)                   | **Not confirmed** |
| V-TIMELINE.C4 media/render    | [v-timeline-c4-media-render-verify.md](v-timeline-c4-media-render-verify.md)       | **Not confirmed** |
| V-TIMELINE.C5 pins/notes/jump | [v-timeline-c5-pins-notes-jump-verify.md](v-timeline-c5-pins-notes-jump-verify.md) | **Not confirmed** |

## Done (high level)

| Area | Evidence |
| ---- | -------- |
| Crypto vertical | V-CRYPTO done earlier |
| Core send | text/attachment/reaction/poll/thread/sticker/GIF native |
| Auth | SSO out; login/UIA/register/reset; fail-closed desktop |
| Rooms core | leave/join/create, hierarchy, DM, unread, typing |
| Timeline shell C1/C2 | NativeTimelinePresenter; C3–C5 live **Not confirmed** |
| Members/power (partial → residual-empty slices) | #375/#395/#405/#439/#450 powers; **#514** direct readers; **#516** tags READ; via-server **#519** |
| Presence | first slice **#458** + full residual **#515** |
| Directory | first **#461** + residual **#513** + visibility **#520** |
| Product extract | **#446** domain product_commands |
| Join-rule READ residual-empty | **#521** native snapshot + RoomPublish |
| Join-rule presentation DTO | **#522** |
| Long-tail residual-empty (type/presentation/orphan) | **#517–#538** (room-summary → RenderMessageContent MsgType) — see [pause handoff](pause-handoff-2026-08-03.md) |
| Importer burn (tip inventory) | **220 → 124** production files |

## In flight

| Item | Status |
| ---- | ------ |
| Daytime / overnight pipelines | **PAUSED** (scheduler cancelled 2026-08-03) |
| Open product PRs onto full-replacement base | **none** (after #538 land + docs freezes closed) |
| Stale tip-docs drafts #502–#512 | **Closed** as obsolete tip-SHA freezes |

## Left (finish-line order after resume)

1. **Long-tail importer burn (residual-empty only).** Prefer single-file type/presentation kills with allowlist+inventory honesty; refuse freestyle multi-module thrash. Hard leftovers include `RoomJoinRules.tsx` **writer**, `useMessageSearch`, `utils/room.ts`, timeline/media listeners, CallWidget media IPC, `initMatrix`/`cryptoStoreContinuity`, R-DEVTOOL.
2. **Members residual honesty** — via-servers closed (#519); enumeration/search/DM-peer may remain — re-open [v-rooms-members-read-residual.md](v-rooms-members-read-residual.md) only with tip-accurate residual list.
3. **V-TIMELINE.C3–C5 live proofs** — still **Not confirmed**; operator checklists only.
4. **V-SEND.R-DEVTOOL** — gated after C3–C5 or explicit reorder.
5. **CallWidget media config/download** — residual still open ([v-send-call-widget-residual.md](v-send-call-widget-residual.md)).
6. **V-BURN.1–.3** — HOLD until zero importers + drop npm criteria met.

**V-BURN remains HOLD.** `dual_backend` remains **forbidden**. [#39](https://github.com/nepenth/synara-desktop/pull/39) remains gated.

## How to resume

See [pause-handoff-2026-08-03.md](pause-handoff-2026-08-03.md) and `/tmp/synara-daytime-pipeline/PAUSE_HANDOFF.md`.
