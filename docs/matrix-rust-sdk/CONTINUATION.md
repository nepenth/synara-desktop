# Matrix Rust SDK program — continuation card

**Date:** 2026-07-27

**Audience:** Current or next orchestrator of the full-replacement program.

For full history, rules, validation accounting, and FR preservation notes, use
[`implementation-handoff.md`](implementation-handoff.md). The detailed 2026-07-26
E1 snapshot is [`r0.2-e1-handoff-2026-07-26.md`](r0.2-e1-handoff-2026-07-26.md).

The independent audit and finding-level remediation requirements are in
[`review-2026-07-25.md`](review-2026-07-25.md). That review supersedes the former
“Phase 0–2 complete / next P3.2” handoff and remains an immutable historical
baseline.

<!-- matrix-rust-program-status-link -->
Current machine-readable and generated status:
[`program-status.json`](program-status.json) and
[`program-status.md`](program-status.md). The status ledger, not dated task
evidence, is authoritative for current delivery and acceptance state.

## Repo truth

| Item | Value |
|---|---|
| Integration branch | `feature/matrix-rust-sdk-full-replacement` |
| Live integration tip (this handoff) | `ba75e460109203b953bfcac77109bbd2d11268cb` — **R0.4 / #87 merged** on top of R0.5; re-fetch and verify |
| Historical audited snapshot | `edfefee499064b736985b6528896b693e5120f22` — bound to the 2026-07-25 review, not the live tip |
| Merged product fixes | PR [#86](https://github.com/nepenth/synara-desktop/pull/86) R0.5 wipe (**accepted**); PR [#87](https://github.com/nepenth/synara-desktop/pull/87) R0.4 store confinement (**merged**, strict acceptance **open** — keyring residual) |
| Open R0.2-E1 PR | PR [#82](https://github.com/nepenth/synara-desktop/pull/82) — tooling; cheap CI-portability only; park residual if thrash |
| Open PR to `main` | [#39](https://github.com/nepenth/synara-desktop/pull/39) — **do not merge without user approval** |
| Current execution | Dual-track: next **implement R0.6 diagnostic privacy** (REV-003); then R0.3 IPC; residual R0.4 native keyring; timebox #82 |

Always re-fetch and verify branch tips and PR state.

## Exact continuation point

### Dual-track priority (user-approved)

1. Land Critical/High product fixes first (R0.5 ✓, R0.4 path confinement ✓).
2. Next product engineering: **R0.6 diagnostic privacy** (REV-003), then **R0.3 IPC** (REV-004/005).
3. R0.4 residual (do not block R0.6): native macOS/Linux secret-store provider, production keyring, live encrypted reopen evidence.
4. R0.2-E1 (#82): merge if green; park residual if thrash continues.
5. Inventory stays **20/112** until more P-tasks land.
6. No dual-backend; no production cutover; no merge to `main` without explicit approval.

### This fire (2026-07-27, R0.4)

- Independently reviewed PR **#87** against REV-002/006/007.
- Local: `cargo test --locked matrix::store` **16 pass**; `cargo test --locked matrix::` **193 pass**.
- Exact-head CI all required checks green.
- **Merged** #87 → integration `ba75e46`.
- Ledger: R0.4 `landed`/`merged`/`open` (path slice only; keyring residual).

### Next owner procedure

```bash
git fetch origin
git checkout feature/matrix-rust-sdk-full-replacement
git pull --ff-only
# Implement R0.6: redact URLs, absolute paths, raw SDK errors from diagnostics
# (ClientBuildPlan, WipeReport, store layout projection, builder errors)
# Add adversarial redaction tests; no product cutover commands
```

## Program accounting

- Original-plan artifact inventory remains **20 / 112 (~18%)**.
- **0 of 15** strict phase gates are closed.
- R0.5 **accepted**. R0.4 **merged** but strict acceptance **open** (keyring residual).
- R0.2 remains `landed` / `pr_open` / strict acceptance `open`.
- Shipping runtime: `matrix-js-sdk` only; Rust harness foundation only.

## Authoritative docs

- Plan: [`../matrix-rust-sdk-full-replacement-plan.md`](../matrix-rust-sdk-full-replacement-plan.md)
- Full handoff: [`implementation-handoff.md`](implementation-handoff.md)
- Independent review: [`review-2026-07-25.md`](review-2026-07-25.md)
- Current status: [`program-status.md`](program-status.md)

## Non-negotiables

- No dual-backend / selector.
- No merge to `main` without explicit user approval.
- No re-open of FR-7.8–7.11 quality audit; FR-7.9-011 stays partial sequential.
- No secrets in diagnostics/IPC.
- Guardrails stay green.
- No force-merge without independent review + green required CI.
