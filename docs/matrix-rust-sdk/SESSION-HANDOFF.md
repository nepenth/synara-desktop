# Session handoff — Matrix Rust full replacement (Grok-only)

| Field | Value |
| --- | --- |
| Written | 2026-07-31 (America/New_York) |
| Audience | Next orchestrator / implementation agent (Grok Build) |
| Integration branch | `feature/matrix-rust-sdk-full-replacement` |
| Tip at handoff | `b558344` — Merge PR [#253](https://github.com/nepenth/synara-desktop/pull/253) (V-SEND.4 rich messages) |
| Prior tip (Cursor wrap) | `88ed143` — #250 V-SEND.3 polls |
| Execution | **Grok-only** (grok-4.5). GPT-5.6 / Codex API and Cursor plan usage are exhausted — **do not** dispatch external Codices or parallel Cursor streams |
| Scheduler | None active (previous 4m Grok loop was failing idle; leave off unless re-created carefully) |

Live trackers:

- [CONTINUATION.md](CONTINUATION.md)
- [PROGRESS.md](PROGRESS.md)
- [d0-residual-completion.md](d0-residual-completion.md)
- [full-vertical-policy.md](full-vertical-policy.md)
- [operating-path-contract.md](operating-path-contract.md) (if present)

## Goal

Complete Synara desktop Matrix client replacement as **serial, product-visible full verticals**:

```text
React UI → versioned Tauri IPC + Synara DTOs → live Rust matrix-sdk
```

**Complete replacement only.** No dogfood minima, no residual plateaus, no dual backend, no concurrent JS/Rust clients for one session. **Physical deletion** of the superseded JS owner is part of each vertical.

## Never-main / never-#39

- Work only on `feature/matrix-rust-sdk-full-replacement` (and PRs into it).
- Umbrella [#39](https://github.com/nepenth/synara-desktop/pull/39) → `main`: **never merge without explicit user approval**.

## Worktrees

Prefer `/private/tmp/synara-codex-*` (or `/tmp/synara-codex-*`) worktrees. Many historical clones exist; for in-flight product:

| Path | Branch / PR | Status |
| --- | --- | --- |
| `/private/tmp/synara-codex-v-send-4-rich-messages` | #253 | **Merged** via tip `b558344` |
| `/private/tmp/synara-codex-v-rooms-2b-hierarchy` | #254 | **Rebased** onto tip; head `e3a0b3d`; waiting CI |
| `/private/tmp/synara-codex-v-timeline-contract` | #240 | **HOLD** incomplete contract |
| `/private/tmp/synara-codex-docs-grok-handoff` | this docs PR | tip sync + handoff |

## What landed (high level, ~48h + session)

### Tip now

```text
b558344 Merge pull request #253 from nepenth/matrix-rust/v-send-4-rich-messages
```

### Merged (sample of recent integration)

| PR | Meaning |
| --- | --- |
| #236–#237 area | V-CRYPTO.6–.7 (UTD recovery, devices) earlier |
| #238 | V-AUTH.1 SSO removal |
| #241 | V-ROOMS.1 invites |
| #244 | CI guardrails harness |
| #245 | V-ROOMS.3 unread |
| #246 | V-ROOMS.4 typing |
| #239 | V-SEND.2 reactions |
| #247 | V-ROOMS.2a parents |
| #248 | V-SEND.1 attachments |
| #249 | V-ROOMS.5 m.direct read |
| #251 | V-ROOMS.5w m.direct writers |
| #243 | docs tracking |
| #252 | V-ROOMS.5r m.direct users |
| #250 | V-SEND.3 polls |
| **#253** | **V-SEND.4 rich composer messages** (merged this Grok session) |

### Inventory (approx on tip)

- ~**205** files under `synara/src` still reference `matrix-js-sdk` (measure before each PR).
- Baseline ledger still references historical **232 / 292**; treat measured deltas per PR as truth.

## Open PR disposition (after this handoff)

| PR | Disposition | Next action |
| --- | --- | --- |
| **#254** V-ROOMS.2b hierarchy | **Active** — rebased `e3a0b3d`, draft | Wait required CI green → undraft → `gh pr merge --merge` |
| **#240** V-TIMELINE contract | **HOLD-merge** | Keep implementing toward full contract; **do not** select NativeTimelinePresenter or delete `RoomTimeline.tsx` until complete + runtime proof; then merge |
| **#255** Cursor docs handoff | Superseded by this tip-sync | Close or ignore after this PR lands |
| **#221** D0.6 plateau | **HOLD forever as-is** | Zero-deletion plateau; not complete |
| L1 #109, #193–#209 | **Parked** | Do not merge while residual queue active |
| **#39** | **HOLD** | No main merge without approval |

## Serial queue (binding)

1. **Finish #254** (CI → undraft → merge).
2. **Continue #240** V-TIMELINE full vertical (implement until closable; merge only then).
3. Remaining residual rows from [d0-residual-completion.md](d0-residual-completion.md) (V-SEND.5 threads, remaining V-ROOMS, etc.).
4. **V-BURN** last (import zero + drop npm dependency).
5. Only then media/widgets/notifications/calls as new full verticals.

## “Hold” clarification (user-corrected)

- **#253 / #254** were never plan-forbidden — only paused for usage wrap-up. **Finish them.**
- **#240** “hold” means **do not merge incomplete**, not abandon. Keep serial implementation until the timeline contract is actually closable.

## Hard constraints

1. Full vertical = native ownership **+** physical JS owner deletion.
2. No dual_backend / no runtime selector / no live JS Matrix client after native session selected.
3. Secrets: one-way command inputs only; never in IPC returns, events, logs.
4. Prefer `gh pr merge --merge` into integration; draft until green.
5. Grok-only execution; no Codex/Cursor credit burn.

## Prior Grok loop failures (context)

Many “orchestrator” background failures (~1s, 0 tools) were **session/scheduler thrash** while waiting on CI / quota, not product CI red. Scheduler is currently **empty** — do not re-enable a 4-minute loop unless it is proven healthy; prefer explicit serial work.

## Resume prompt (copy-paste)

```text
Resume Matrix Rust full-replacement on feature/matrix-rust-sdk-full-replacement.

Execution: Grok-only (this environment). No Codex API / no Cursor plan burn.
Read first: docs/matrix-rust-sdk/SESSION-HANDOFF.md, CONTINUATION.md, PROGRESS.md,
d0-residual-completion.md, full-vertical-policy.md.

Tip should be at or after b558344 (#253 V-SEND.4 merged).
Serial next: #254 V-ROOMS.2b (rebased e3a0b3d) — green CI → undraft → merge.
Then #240 V-TIMELINE — implement to full contract; do NOT merge until presenter
cutover + RoomTimeline deletion + runtime proof are complete.
Never merge #221 plateau, L1 parked PRs, or #39/main without explicit approval.
Work under /private/tmp/synara-codex-* worktrees; keep tracking docs accurate.
```
