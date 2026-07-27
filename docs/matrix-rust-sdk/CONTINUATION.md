# Matrix Rust SDK program — continuation card

**Date:** 2026-07-27

**Audience:** Current or next orchestrator of the full-replacement program.

For full history, rules, and FR notes use
[`implementation-handoff.md`](implementation-handoff.md).

**Canonical how-we-execute model:**
[`cutover-operating-model.md`](cutover-operating-model.md)
(capability vertical slices → atomic sole-owner cutover → burn down js-sdk →
merge to main with approval). Supersedes any older text that implies a runtime
SDK selector, dual production backends, or hard-blocking product slices on residual
R0 formal thrash.

**Live human progress log (refresh on GitHub while away):**
[`PROGRESS.md`](PROGRESS.md) —
https://github.com/nepenth/synara-desktop/blob/feature/matrix-rust-sdk-full-replacement/docs/matrix-rust-sdk/PROGRESS.md  
Orchestrators must update `PROGRESS.md` when PRs merge or priorities change.

<!-- matrix-rust-program-status-link -->
Current machine-readable and generated status:
[`program-status.json`](program-status.json) and
[`program-status.md`](program-status.md). The status ledger, not dated task
evidence, is authoritative for current delivery and acceptance state.

## Repo truth

| Item | Value |
|---|---|
| Integration branch | `feature/matrix-rust-sdk-full-replacement` |
| Live integration tip | Re-fetch; expected at/after `8b7d39e` (#110 P3.5). Verify with `git rev-parse origin/feature/matrix-rust-sdk-full-replacement` |
| Progress log | [`PROGRESS.md`](PROGRESS.md) — remote human monitor |
| Open PRs | [#112](https://github.com/nepenth/synara-desktop/pull/112) **P3.6** (product); [#111](https://github.com/nepenth/synara-desktop/pull/111) PROGRESS; [#109](https://github.com/nepenth/synara-desktop/pull/109) MiniMax |
| Open PR to `main` | [#39](https://github.com/nepenth/synara-desktop/pull/39) — **do not merge without user approval** |
| Product runtime | Still **`matrix-js-sdk` only** until atomic cutover; Rust = harness foundation / future sole owner |
| Dual backend | **`false` forever** — no selector |
| Execution model | Capability slices on branch → dogfood sole-owner cutover → delete js-sdk → main when approved |

Always re-fetch and verify branch tips and PR state.

## Exact continuation point

### Priority (user-approved 2026-07-27)

1. **Update [`PROGRESS.md`](PROGRESS.md)** on every merge / priority change.
2. **P3.2 + P3.5 landed.** **P3.6 session restore** open [#112](https://github.com/nepenth/synara-desktop/pull/112) (merge when CI green; rustfmt fix `78c61ea`).
3. Then sync + room list (P4.1 SyncService readiness → P4.2 room list) toward dogfood sole-owner flip.
4. **Clean-break:** re-login / wipe local Matrix dirs OK; no elaborate JS→Rust token/device migration.
5. **Do not** build dual-backend, runtime flags, or dual live clients.
6. Residual R0 formal work is secondary; fix real safety only.
7. Merge to `main` / #39 only with explicit user approval.

### Next owner procedure

```bash
git fetch origin
git checkout feature/matrix-rust-sdk-full-replacement
git pull --ff-only
npm run check:matrix-rust-guardrails
(cd src-tauri && cargo test --locked matrix::)
# Next: merge #112 when green; then P4.1 sync readiness. Keep PROGRESS.md current.
```

## Program accounting

- Original-plan artifact inventory: see [`program-status.md`](program-status.md) (sync after P3.5; human view in [`PROGRESS.md`](PROGRESS.md)).
- **0 of 15** strict phase gates closed (honest).
- Shipping runtime: `matrix-js-sdk` only until cutover.
- Rust: harness foundation growing toward sole owner (login + session persist landed).

## Authoritative docs

- **Progress log (remote):** [`PROGRESS.md`](PROGRESS.md)
- Operating model: [`cutover-operating-model.md`](cutover-operating-model.md)
- Plan: [`../matrix-rust-sdk-full-replacement-plan.md`](../matrix-rust-sdk-full-replacement-plan.md)
- Full handoff: [`implementation-handoff.md`](implementation-handoff.md)
- Migration UX (reauth / new device / no token continuity): [`migration-ux-decision.md`](migration-ux-decision.md)
- Independent review (historical baseline): [`review-2026-07-25.md`](review-2026-07-25.md)
- Machine status: [`program-status.md`](program-status.md)

## Non-negotiables

- No dual-backend / selector.
- No concurrent JS + Rust Matrix clients for one session.
- No merge to `main` without explicit user approval.
- No secrets in diagnostics/IPC.
- Guardrails stay green.
- Capability-first slices; atomic cutover; then js-sdk burn-down.
