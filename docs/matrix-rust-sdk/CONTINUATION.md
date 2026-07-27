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

<!-- matrix-rust-program-status-link -->
Current machine-readable and generated status:
[`program-status.json`](program-status.json) and
[`program-status.md`](program-status.md). The status ledger, not dated task
evidence, is authoritative for current delivery and acceptance state.

## Repo truth

| Item | Value |
|---|---|
| Integration branch | `feature/matrix-rust-sdk-full-replacement` |
| Live integration tip | Re-fetch; expected at/after `447cbdc` (#82 E1 tooling). Verify with `git rev-parse origin/feature/matrix-rust-sdk-full-replacement` |
| Open product PR | [#107](https://github.com/nepenth/synara-desktop/pull/107) **P3.2** password/token login + device naming (merge when CI green) |
| Docs-only | [#106](https://github.com/nepenth/synara-desktop/pull/106) E1 status handoff — non-blocking |
| Open PR to `main` | [#39](https://github.com/nepenth/synara-desktop/pull/39) — **do not merge without user approval** |
| Product runtime | Still **`matrix-js-sdk` only** until atomic cutover; Rust = harness foundation / future sole owner |
| Dual backend | **`false` forever** — no selector |
| Execution model | Capability slices on branch → dogfood sole-owner cutover → delete js-sdk → main when approved |

Always re-fetch and verify branch tips and PR state.

## Exact continuation point

### Priority (user-approved 2026-07-27)

1. **Land P3.2** (#107) when CI green — password/token login + D-NEW-DEVICE names under `matrix/auth/`.
2. **Next vertical slices:** session secrets / restore (P3.5–P3.6) → sync + room list (P4.x) toward dogfood path.
3. **Clean-break:** re-login / wipe local Matrix dirs OK; no elaborate JS→Rust token/device migration.
4. **Do not** build dual-backend, runtime flags, or dual live clients.
5. Residual R0 formal work is secondary; fix real safety only.
6. Merge to `main` / #39 only with explicit user approval.

### Next owner procedure

```bash
git fetch origin
git checkout feature/matrix-rust-sdk-full-replacement
git pull --ff-only
npm run check:matrix-rust-guardrails
(cd src-tauri && cargo test --locked matrix::)
# Merge #107 when green; then P3.5/P3.6 or next slice per cutover-operating-model.md
```

## Program accounting

- Original-plan artifact inventory: see [`program-status.md`](program-status.md) (P3.2 open PR → 21/112 when ledger includes it).
- **0 of 15** strict phase gates closed (honest).
- Shipping runtime: `matrix-js-sdk` only until cutover.
- Rust: harness foundation growing toward sole owner.

## Authoritative docs

- Operating model: [`cutover-operating-model.md`](cutover-operating-model.md)
- Plan: [`../matrix-rust-sdk-full-replacement-plan.md`](../matrix-rust-sdk-full-replacement-plan.md)
- Full handoff: [`implementation-handoff.md`](implementation-handoff.md)
- Migration UX (reauth / new device / no token continuity): [`migration-ux-decision.md`](migration-ux-decision.md)
- Independent review (historical baseline): [`review-2026-07-25.md`](review-2026-07-25.md)
- Current status: [`program-status.md`](program-status.md)

## Non-negotiables

- No dual-backend / selector.
- No concurrent JS + Rust Matrix clients for one session.
- No merge to `main` without explicit user approval.
- No secrets in diagnostics/IPC.
- Guardrails stay green.
- Capability-first slices; atomic cutover; then js-sdk burn-down.
