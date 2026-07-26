# Matrix Rust SDK program — continuation card

**Date:** 2026-07-25
**Audience:** Current or next orchestrator of the full-replacement program.

For full history, rules, and FR preservation notes, use
[`implementation-handoff.md`](implementation-handoff.md). This card is the short
path only.

The independent audit and finding-level remediation requirements are in
[`review-2026-07-25.md`](review-2026-07-25.md). That review supersedes the former
“Phase 0–2 complete / next P3.2” handoff.

<!-- matrix-rust-program-status-link -->
Current machine-readable and generated status:
[`program-status.json`](program-status.json) and
[`program-status.md`](program-status.md). The status ledger, not dated task
evidence, is authoritative for current delivery and acceptance state.

## Repo truth

| Item | Value |
|------|--------|
| Branch | `feature/matrix-rust-sdk-full-replacement` |
| Audited integration tip | `edfefee499064b736985b6528896b693e5120f22` — always re-check with `git rev-parse origin/feature/matrix-rust-sdk-full-replacement` |
| Open PRs to integration at audit start | **None** |
| Open PR to `main` | [#39](https://github.com/nepenth/synara-desktop/pull/39) — **do not merge without user approval** |
| Current execution | See [`program-status.md`](program-status.md) |

## Current state and next work

Use [`program-status.md`](program-status.md). It is generated from the canonical
JSON ledger and is the only current inventory, gate, runtime, active-task, and
next-task summary.

## Validate after clone

```bash
git checkout feature/matrix-rust-sdk-full-replacement
git pull
npm run check:matrix-rust-guardrails
(cd src-tauri && cargo test --locked matrix::)
```

The two commands above pass at the audited tip (189 Matrix-filtered Rust tests;
0 guardrail findings), but they are not the full gate. Current known failures:

- Rust `cargo fmt --check` and strict clippy;
- TypeScript ESLint and Prettier;
- integration-range `git diff --check`;
- GitHub desktop validation on `edfefee`.

## Authoritative docs

- Plan: [`../matrix-rust-sdk-full-replacement-plan.md`](../matrix-rust-sdk-full-replacement-plan.md)
- Handoff: [`implementation-handoff.md`](implementation-handoff.md)
- Parity: [`feature-parity-traceability.md`](feature-parity-traceability.md)
- Migration UX (P0.7): [`migration-ux-decision.md`](migration-ux-decision.md)
- Independent review: [`review-2026-07-25.md`](review-2026-07-25.md)
- Current status: [`program-status.md`](program-status.md)

## Non-negotiables

- No dual-backend / selector
- No merge to `main` without explicit user approval
- No re-open of FR-7.8–7.11 quality audit; FR-7.9-011 stays partial sequential
- No secrets in diagnostics/IPC
- Guardrails stay green
- No P3.2 work until R0.1–R0.8 and the Phase 0–2/P3.1 gates are accepted
