# Codex parallel work protocol

| Field | Value |
| --- | --- |
| Date | 2026-07-28 |
| CLI | `codex` (Codex CLI; non-interactive: `codex exec`) |
| Default model | **`gpt-5.6-sol`** |
| Default effort | **`medium`** (`model_reasoning_effort`) |
| Config (local) | `~/.codex/config.toml` already sets both |
| Related | [`minimax-parallel-work.md`](minimax-parallel-work.md), [`cutover-operating-model.md`](cutover-operating-model.md), [`program-status.md`](program-status.md), [`PROGRESS.md`](PROGRESS.md) |

## Role split

| Agent | Role |
| --- | --- |
| **Grok** (orchestrator) | Task packets, merge order, CI babysit, policy, **ledger accuracy**, final code review, tip-merge strategy |
| **Codex** (`gpt-5.6-sol`) | Parallel **implementer** for bounded foundation slices while Grok reviews/merges other work |
| **MiniMax-M3** | Free parallel **text** (graphs, checklists, inventory notes) — not sole code owner |

Codex may write code and open-ready branches. **Grok (or human) reviews and merges.** Codex must not merge to integration/main or claim phase-gate acceptance.

## Credit / availability discipline

- Paid Codex credits may **exhaust in a few hours** under multi-task load. That is expected and fine.
- If CLI/API returns **quota / credit / rate-limit / auth** errors:
  1. **Report once** in the orchestrator status (do not spam).
  2. **Stop dispatching new Codex packets** until credits return or the user says so.
  3. Continue Grok + MiniMax + CI babysit without Codex.
- Prefer **medium** effort and **tight task packets** to stretch credits.
- Escalate to **`high`** only for high-blast-radius work (crypto, scroll/timeline policy, session/cutover). Do **not** burn high on boilerplate.
- Cap concurrent Codex workers at **2** (3 max) so we do not pile five full CI suites onto runners.

## Default model policy

| Setting | Use when |
| --- | --- |
| `gpt-5.6-sol` + **medium** | Default product foundation slices, tests, clippy-clean scaffolding |
| `gpt-5.6-sol` + **high** | Crypto/session edges, timeline scroll/position policy, cutover-sensitive design |
| lower / faster effort | Mechanical fmt-only, mod order, trivial test fills (or MiniMax for pure text) |

Do not thrash model IDs mid-stack; consistency beats micro-optimizing every PR.

### Example non-interactive invoke

```bash
codex exec \
  -C /Users/nepenthe/git_repos/synara_project/synara-desktop \
  -m gpt-5.6-sol \
  -c model_reasoning_effort=\"medium\" \
  "$(cat /path/to/packet.md)"
```

Global config already defaults model + medium; pass flags only to override.

## Hard rules (paste into every Codex packet)

1. Integration sole target: `feature/matrix-rust-sdk-full-replacement` (never merge umbrella #39 / `main` without explicit user approval).
2. **`dual_backend = false` forever**; no JS+Rust concurrent Matrix clients for one session.
3. Clean-break re-login / wipe OK; no elaborate JS→Rust token/crypto migration.
4. Product runtime remains **matrix-js-sdk only** until atomic sole-owner cutover; this work is **harness foundation** under `src-tauri/src/matrix/`.
5. **No tokens, recovery keys, secrets, or event plaintext** in logs, errors Display/Debug, DTOs, or diagnostics.
6. One branch / one task ID / one PR; prefer modules that do not thrash the same files as in-flight PRs.
7. Local bar before handoff: module unit tests green, `cargo fmt`, `cargo clippy -D warnings` for touched code, guardrails if relevant.
8. Add `docs/matrix-rust-sdk/pN.M-….md` task note. **Do not** edit `program-status.json` unless the packet explicitly says so (orchestrator owns ledger accuracy).
9. Do not tip-merge docs mid-flight onto green product CI stacks; leave tip-merge to orchestrator.

## Good Codex packets

| Kind | Examples |
| --- | --- |
| Pure projection / index foundations | Polls, account-data codecs, room profile DTOs, notification stream shapes |
| Clippy/fmt / mod.rs order fix on **one** red PR | Single-branch fix only |
| Tests + docs for a specified state machine | Match existing `relations` / `receipts` / `devices` style |
| Isolated residual | UTD helpers, search filters — if not already owned by an open PR |

## Avoid with Codex alone

- Merge decisions, force-push of shared branches, umbrella → main.
- Silent **program-status** inventiveness (counts must match reality).
- Dual-backend, runtime selector, or “temporary” JS crypto bridge.
- Five parallel product PRs all fighting `mod.rs` + package smoke.
- Claiming strict phase-gate close.

## Orchestrator loop

```text
while credits available and migration incomplete:
  babysit mergeable product PRs (Quality + Desktop package gates)
  if Codex credit error:
    log once; set codex_enabled=false; continue without Codex
  if codex_enabled and < 2 active Codex packets and free slice exists:
    write bounded packet (task id, files to touch, acceptance, out of scope)
    dispatch codex exec on a dedicated branch/worktree
    on completion: Grok review → fix → PR → CI
  if MiniMax down:
    log once; retry health check every few orchestrator fires; continue
  during long CI waits: MiniMax text jobs (feature graph / checklists)
```

## Packet template

```markdown
# Codex packet — <TASK_ID> <short title>

## Model
gpt-5.6-sol / medium  (or high if justified below)

## Goal
One-sentence outcome.

## Branch
matrix-rust/<task-slug> from origin/feature/matrix-rust-sdk-full-replacement

## In scope
- …

## Out of scope
- Production Tauri commands / product UI cutover
- dual_backend, token migration, secrets in logs
- program-status.json (orchestrator)

## Patterns to copy
- src-tauri/src/matrix/<similar>/
- docs/matrix-rust-sdk/p….md style of P5.6 / P6.2

## Acceptance
- [ ] unit tests for happy + edge paths
- [ ] cargo fmt + clippy -D warnings on matrix module
- [ ] task doc under docs/matrix-rust-sdk/
- [ ] register module in matrix/mod.rs with correct rustfmt order
- [ ] no secrets in Display/Debug/DTO

## Handoff
Push branch; summarize files + test commands; do not merge.
```

## Review bar (Grok)

Before merge:

1. Policy: dual_backend, secrets, clean-break.
2. Scope: foundation only unless packet said otherwise.
3. Tests real (not only helpers).
4. mod.rs / rustfmt order won’t fail CI.
5. Ledger update scheduled (same PR or immediate follow-up) so inventory cannot go stale.
