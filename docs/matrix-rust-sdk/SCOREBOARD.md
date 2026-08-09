# Matrix Rust full-replacement — scoreboard

| Field                                                  | Value                                                                                                                                                                                                                                  |
| ------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Updated                                                | **2026-08-10** (tip `TBD` · **0 importers, js-sdk removed**) |
| Tip                                                    | `TBD` (full-removal slice) on `feature/matrix-rust-sdk-full-replacement` |
| Production `matrix-js-sdk` import files (`synara/src`) | **0** — plan baseline **220**; **220 removed, 100%** |
| Allowlist `pathCount`                                  | **0** (empty = full ban; any new importer fails) |
| Test `matrix-js-sdk` import files (`synara/src`)             | **0** (burned in #595; fixtures migrated to probed literals)  |
| Dual backend                                           | **false** (forbidden forever)                                                                                                                                                                                                          |
| Pipeline                                               | **GREEN** — Quality gate + Desktop package gate pass at tip; 0 open PRs |
| Operating model                                        | **prime-agent orchestrator + `deepseek-v4-flash-0731` sub-agents (max 2 concurrent)** — locally hosted, only configured model; public-repo hygiene + UI/UX fidelity always on ([operating-instructions.md](operating-instructions.md)) |
| Burn board                                             | https://kb.whyland.com/go/synara-matrix-burn                                                                                                                                                                                           |
| Umbrella #39                                           | **Do not merge** without explicit user approval                                                                                                                                                                                        |
| V-BURN                                                | **REACHED (full)** — 0 importers; allowlist 0; **`matrix-js-sdk@42.0.0` fully removed from package.json + lockfile** (dep, devDep, two-client tooling harness + CI job all retired); no js-sdk anywhere in the repo tree. matrix-widget-api stays (V-CALL separate slice); #39 still operator-gated |

## Current burn policy

The HUMAN OPERATOR LIVE-PROOF step is **not a completion or merge gate** for
residual-empty `matrix-js-sdk` burns on this feature branch. A claimed file is
engineering-complete for branch purposes when the code is on the measured tip,
focused unit/CI checks pass, and the file has no remaining `matrix-js-sdk`
import. A native product path that needs live Matrix state must fail closed;
fix-forward and private Beta are accepted. C3–C5 desktop sessions remain
useful optional Beta feedback and may remain **Not confirmed**, but they do not
hold a residual-empty burn or merge.

Inventory honesty remains mandatory when a product change changes importers:
regenerate with `npm run inventory:matrix-sdk-usage`, ratchet the allowlist
`pathCount` and `paths[]`, and update the inventory test floors for production
files, declarations, and buckets plus the P1.6 guardrail floors. F6c-2c completes the importer burn: current tip inventory is **0** production files / **0** allowlist paths; the production allowlist is now empty (full ban) after the #624 importer drop + dependency removal.

## Operator index — timeline live proofs

These are docs-only operator checklists for the selected native desktop path.
All three live proofs remain **Not confirmed** because no authenticated desktop
run is recorded at this tip. That status is optional Beta feedback, not a
completion or merge hold.

| Proof                         | Operator checklist                                                                 | Live proof        |
| ----------------------------- | ---------------------------------------------------------------------------------- | ----------------- |
| V-TIMELINE.C3 stream/delta    | [v-timeline-c3-stream-verify.md](v-timeline-c3-stream-verify.md)                   | **Not confirmed** |
| V-TIMELINE.C4 media/render    | [v-timeline-c4-media-render-verify.md](v-timeline-c4-media-render-verify.md)       | **Not confirmed** |
| V-TIMELINE.C5 pins/notes/jump | [v-timeline-c5-pins-notes-jump-verify.md](v-timeline-c5-pins-notes-jump-verify.md) | **Not confirmed** |

## Done (high level)

| Area                                                | Evidence                                                                                                                             |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| Crypto vertical                                     | V-CRYPTO done earlier                                                                                                                |
| Core send                                           | text/attachment/reaction/poll/thread/sticker/GIF native                                                                              |
| Auth                                                | SSO out; login/UIA/register/reset; fail-closed desktop                                                                               |
| Rooms core                                          | leave/join/create, hierarchy, DM, unread, typing                                                                                     |
| Timeline shell C1/C2                                | NativeTimelinePresenter; C3–C5 code/unit/CI may be engineering-complete while live proof remains **Not confirmed**                   |
| Members/power (partial → residual-empty slices)     | #375/#395/#405/#439/#450 powers; **#514** direct readers; **#516** tags READ; via-server **#519**                                    |
| Presence                                            | first slice **#458** + full residual **#515**                                                                                        |
| Directory                                           | first **#461** + residual **#513** + visibility **#520**                                                                             |
| Product extract                                     | **#446** domain product_commands                                                                                                     |
| Join-rule READ residual-empty                       | **#521** native snapshot + RoomPublish                                                                                               |
| Join-rule presentation DTO                          | **#522**                                                                                                                             |
| Long-tail residual-empty (type/presentation/orphan) | **#517–#538** (room-summary → RenderMessageContent MsgType) — see [pause handoff](pause-handoff-2026-08-03.md)                       |
| Importer burn (tip inventory)                       | **220 → 0** production files (#546→#624 stack); test import files 10→**0** (#595); allowlist **1 → 0** (F6c-2c) |
| Live-proof gate policy                              | **#544** — not a residual-empty merge gate                                                                                           |
| Live-proof-held residual stack                      | **#546** landed (chrome/typing/notes/reaction/NativeEventContent/reactions)                                                          |
| Desktop Beta package smoke                          | Actions run [30821912637](https://github.com/nepenth/synara-desktop/actions/runs/30821912637) @ `57ab9e64` (artifacts, not Releases) |

## In flight

| Item                                        | Status                                          |
| ------------------------------------------- | ----------------------------------------------- |
| Daytime / overnight pipelines               | **BURN COMPLETE (dormant-green)** — all gates green at tip; 0 open PRs |
| Open product PRs onto full-replacement base | **none** — this docs-only tracking update |
| Stale tip-docs drafts #502–#512             | **Closed** as obsolete tip-SHA freezes          |

## Left (finish-line order after resume)

1. **Long-tail importer burn (residual-empty only).** Prefer single-file type/presentation kills with allowlist+inventory honesty; refuse freestyle multi-module thrash. Hard leftovers include `RoomJoinRules.tsx` **writer**, `useMessageSearch`, `utils/room.ts`, timeline/media listeners, CallWidget media IPC, `initMatrix`/`cryptoStoreContinuity`, R-DEVTOOL.
2. **Members residual honesty** — via-servers closed (#519); enumeration/search/DM-peer may remain — re-open [v-rooms-members-read-residual.md](v-rooms-members-read-residual.md) only with tip-accurate residual list.
3. **V-TIMELINE.C3–C5 live proofs** — still **Not confirmed**; optional Beta feedback, not a merge gate. Branch completion is tip + focused unit/CI + residual-empty/fail-closed evidence.
4. **V-SEND.R-DEVTOOL** — may start without waiting for C3–C5 live confirmation; retain the native, fail-closed implementation gate.
5. **CallWidget media config/download** — residual still open ([v-send-call-widget-residual.md](v-send-call-widget-residual.md)).
6. **V-BURN.1–.3** — HOLD until zero importers + drop npm criteria met.

**V-BURN reached** (0 importers + npm dep removed). `dual_backend` remains **forbidden**. [#39](https://github.com/nepenth/synara-desktop/pull/39) remains gated on explicit operator approval.

## How to resume

See [pause-handoff-2026-08-03.md](pause-handoff-2026-08-03.md) and
[operating-instructions.md](operating-instructions.md). Execution runs through
**this agent harness with its locally hosted model** — orchestrator + bounded
sub-agents, ≤2–3 concurrent, no external model APIs. Public-repo hygiene and
the UI/UX high-fidelity mandate apply to every slice. Preferred next slice is
item 1 in the "Left" list above (long-tail residual-empty importer burn).
