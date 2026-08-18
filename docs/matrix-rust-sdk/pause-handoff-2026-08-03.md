# Pipeline pause handoff — 2026-08-03 (final)

> **Operating-model supersession:** this file is a **historical snapshot** of
> what was true at the 2026-08-03 pause. Go-forward execution runs through
> **this agent harness with its locally hosted model** — orchestrator + bounded
> sub-agents, ≤2–3 concurrent, no external model APIs — per
> [operating-instructions.md](operating-instructions.md). Any
> Grok/Codex/DeepSeek consumption or model-routing language below is
> **superseded**, as is the "no agent spawns until re-enable" hold (resume is
> authorized). Public-repo hygiene and the **UI/UX high-fidelity mandate**
> apply to every slice.

## Why paused (this stop)

Operator request: **pause Matrix Rust js-sdk replacement agent work** to conserve
weekly Grok + Codex usage. Wrap tracking, clean stale PRs/worktrees/caches, leave
**resume-ready**. No new implementers / DeepSeek / daytime bench fill until
explicit re-enable.

## Tip snapshot (authoritative)

| Field | Value |
| ----- | ----- |
| Branch | `feature/matrix-rust-sdk-full-replacement` |
| Tip SHA (short) | **`57ab9e64`** |
| Tip subject | `fix(matrix): residual-empty live-proof-held stack (124→114 importers) (#546)` |
| Production import files | **114** / baseline **220** (**106** removed, ~**48.2%**) |
| Allowlist pathCount | **114** (= `paths[]` length) |
| Inventory source | `docs/matrix-rust-sdk/desktop-sdk-usage.md` **on tip** (never a stale worktree) |
| Burn board | Retired; use the repository-local scoreboard and progress log |
| Open product PRs (base = full-replacement) | **none** after #546 (stale drafts closed) |
| Desktop Beta packages | **Built** (Actions artifacts, not GitHub Releases) — https://github.com/nepenth/synara-desktop/actions/runs/30821912637 @ `57ab9e64` |

## Policy (still in force)

| Control | State |
| ------- | ----- |
| `dual_backend` | **false** forever |
| main / umbrella #39 | **no merge** without explicit approval |
| V-BURN | **HOLD** until zero production importers + drop npm criteria |
| HUMAN live-proof | **not** a residual-empty merge gate (#544) |
| C3–C5 live checklists | Optional Beta feedback; may remain **Not confirmed** without holding burns |
| R-DEVTOOL | Allowed to start when residual work resumes (not gated on live proof) |
| Daytime / overnight agent loops | **OFF** for this pause |
| New Codex / DeepSeek spawns | **FORBIDDEN** until operator re-enables |

### Residual-empty acceptance (branch)

A claimed file is engineering-complete for the feature branch when:

1. Code is on the measured tip,
2. Focused unit/CI checks pass,
3. The file has no remaining `matrix-js-sdk` import,
4. Inventory honesty is regenerated and ratcheted when importers change.

Native paths that need live Matrix state must **fail closed**. Fix-forward and
private Beta are accepted.

## What landed this session (2026-08-03)

### Policy + product

| PR | Result |
| -- | ------ |
| **#544** | Docs: drop live-proof merge gates for residual-empty burns |
| **#546** | Residual-empty stack: RoomView chrome, typing/notes, Reaction/Reply, NativeEventContent, reactions path (+ delete `useRelations`) — **124→114** importers |
| #540–#543, #545, #547 | Superseded by #546 / closed as stale |

### Desktop Beta builds

| Artifact | Source |
| -------- | ------ |
| macOS `Synara.app` | Actions artifact `synara-macos-app` |
| Arch `synara-desktop-bin-*.pkg.tar.zst` | Actions artifact `synara-linux-arch-pkg` |
| Linux `.deb` | Actions artifact `synara-linux-deb` |

Run: https://github.com/nepenth/synara-desktop/actions/runs/30821912637 (workflow_dispatch Desktop Package Smoke; **not** a GitHub Release).
Retention ~7 days from build. Install: download via Actions UI or `gh run download 30821912637 -R nepenth/synara-desktop`.

## Hard residuals still open (not exhaustive)

- **~114** production `matrix-js-sdk` importers remain (feature/hook heavy)
- `RoomJoinRules` **writer**, `useMessageSearch`, `utils/room.ts` / matrix utilities
- Message path remainder (`Message.tsx` etc.), CallWidget media residual
- **R-DEVTOOL** (SendRoomEvent / StateEventEditor)
- Bootstrap: `initMatrix` + `cryptoStoreContinuity`
- **V-BURN** HOLD until importers → 0

Bucket sketch at tip (~desktop-runtime production): feature ~40, hook ~34,
component ~12, utility ~9, page ~7, plugin ~6, state ~4, client-lifecycle ~2.

## Inventory honesty rules (resume)

1. Always measure inventory from **`origin/feature/matrix-rust-sdk-full-replacement` tip**.
2. Ratchet **together**: `desktop-sdk-usage.{md,json}`, allowlist `pathCount` **and** `paths[]`, inventory test floors, p1.6 guardrail floors.
3. Concurrent PRs claiming the same tip floor are **not additive** — serial land + recompute.
4. Update the repository-local scoreboard after each merge.

## Resume checklist

1. Confirm tip + inventory:
   ```bash
   git fetch origin feature/matrix-rust-sdk-full-replacement
   git rev-parse --short origin/feature/matrix-rust-sdk-full-replacement
   git show origin/feature/matrix-rust-sdk-full-replacement:docs/matrix-rust-sdk/desktop-sdk-usage.md | grep 'Production import files'
   ```
2. Re-enable daytime **only** if operator asks (MODEL_ROUTING / BENCH_FILL / matrix-rust-full-vertical).
3. DeepSeek `deepseek-v4-flash-0731`: max context **393k**; no more than two concurrent workers on an approved local runner.
4. Prefer **one** product residual-empty land at a time; serial inventory.
5. Publish burn board after first spawn / merge.
6. Do **not** merge main / #39 without explicit approval.

## Local workspace note

Operator checkouts may be on residual side-branches with **stale local inventory**.
That is **not** tip. Always measure tip via `origin/feature/matrix-rust-sdk-full-replacement`.
