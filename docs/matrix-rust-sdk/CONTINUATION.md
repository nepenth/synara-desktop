# Matrix Rust SDK program — continuation card

**Date:** 2026-07-26  
**Audience:** Next owner / orchestrator resuming the full-replacement program.

For full history, rules, and FR preservation notes, use
[`implementation-handoff.md`](implementation-handoff.md). This card is the short
path only.

## Repo truth

| Item | Value |
|------|--------|
| Branch | `feature/matrix-rust-sdk-full-replacement` |
| Tip (at handoff) | `87d955297cc3c5decfd81e22012e7c7701f6dd04` |
| Open PRs to integration | **None** |
| Open PR to `main` | [#39](https://github.com/nepenth/synara-desktop/pull/39) — **do not merge without user approval** |
| Uncommitted work expected | **None** on integration tip |
| Auto progress loop | **Paused** (not running) |

## Done vs next

| Band | Status |
|------|--------|
| Phase 0 evidence | **Complete** (P0.1–P0.7) |
| Phase 1 contracts/guardrails | **Complete** (P1.1–P1.6) |
| Phase 2 lifecycle/store harness | **Complete** (P2.1–P2.6) |
| Phase 3 auth | **P3.1 complete**; **next = P3.2** password/token login + device naming |
| Phases 4–14 | Not started |
| Product cutover | **Not started** — still `matrix-js-sdk` only |

**Approx. plan progress:** ~20 / ~112 tasks (**~18%**); 3 / 15 phases fully done (**20%** of phases).

## Validate after clone

```bash
git checkout feature/matrix-rust-sdk-full-replacement
git pull
npm run check:matrix-rust-guardrails
(cd src-tauri && cargo test --locked matrix::)
```

Expected (2026-07-26): matrix filter **189** tests pass; guardrails **0** findings.

## Authoritative docs

- Plan: [`../matrix-rust-sdk-full-replacement-plan.md`](../matrix-rust-sdk-full-replacement-plan.md)
- Handoff: [`implementation-handoff.md`](implementation-handoff.md)
- Parity: [`feature-parity-traceability.md`](feature-parity-traceability.md)
- Migration UX (P0.7): [`migration-ux-decision.md`](migration-ux-decision.md)

## Non-negotiables

- No dual-backend / selector  
- No merge to `main` without explicit user approval  
- No re-open of FR-7.8–7.11 quality audit; FR-7.9-011 stays partial sequential  
- No secrets in diagnostics/IPC  
- Guardrails stay green  
