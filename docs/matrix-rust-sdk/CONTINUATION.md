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
| Live integration tip (this handoff) | `447cbdcdd43a26775db32d8d62d6929d8a5c09b9` — **R0.2-E1 / #82 merged** (traceability tooling + memory-bound RSS fix) on R0.8 slice-1 + R0.7; re-fetch and verify |
| Historical audited snapshot | `edfefee499064b736985b6528896b693e5120f22` — bound to the 2026-07-25 review, not the live tip |
| Merged product fixes | PR [#86](https://github.com/nepenth/synara-desktop/pull/86) R0.5 wipe (**accepted**); PR [#87](https://github.com/nepenth/synara-desktop/pull/87)+[#94](https://github.com/nepenth/synara-desktop/pull/94) R0.4 store confinement + native keyring (**accepted**); PR [#89](https://github.com/nepenth/synara-desktop/pull/89) R0.6 diagnostic privacy (**accepted**); PR [#91](https://github.com/nepenth/synara-desktop/pull/91)+[#92](https://github.com/nepenth/synara-desktop/pull/92) R0.3 IPC wire freeze (**accepted**); PR [#96](https://github.com/nepenth/synara-desktop/pull/96)+[#98](https://github.com/nepenth/synara-desktop/pull/98)+[#100](https://github.com/nepenth/synara-desktop/pull/100)+[#102](https://github.com/nepenth/synara-desktop/pull/102) R0.7 live CS + login-types + composed store + stale-gen/wrong-key residuals (**merged**, strict acceptance **open**); PR [#82](https://github.com/nepenth/synara-desktop/pull/82) R0.2-E1 traceability tooling (**merged**, R0.2 strict **open** — E2 + Phase 0 evidence residuals remain) |
| Open PR to `main` | [#39](https://github.com/nepenth/synara-desktop/pull/39) — **do not merge without user approval** |
| Current execution | Dual-track: next **R0.2-E2** (generate/commit normalized audit+v2 with E1 tooling) or R0.7 authenticated residual / R0.8 accepting re-issue; P3.2 blocked by R0.2 residual / R0.7 residual / R0.8 |

Always re-fetch and verify branch tips and PR state.

## Exact continuation point

### Dual-track priority (user-approved)

1. Land Critical/High product fixes first (R0.5 ✓, R0.4 ✓, R0.6 ✓, R0.3 ✓).
2. R0.7 slices landed: live CS transports (**#96**) + loopback login-types (**#98**) + composed encrypted store lifecycle (**#100**) + stale-gen/wrong-key residuals (**#102**).
3. R0.2-E1 landed via **#82** (memory-bound RSS fix: phase-isolated children + incremental 512 MiB budget).
4. Remaining R0 blockers for P3.2: R0.2 (E2 + Phase 0 evidence residuals; strict open), R0.7 residual (authenticated live sync), R0.8 (accepting formal re-issue).
5. Inventory stays **20/112** until more P-tasks land.
6. No dual-backend; no production cutover; no merge to `main` without explicit approval.

### This fire (2026-07-27, R0.2-E1 merge after memory-bound fix)

- Unparked **#82** with deliberate memory-bound residual fix (not thrash):
  sequential dual-render digests; separate audit/v2 isolated children; 512 MiB
  budget on **incremental** RSS during measured ops.
- Merged integration tip onto E1; exact-head CI green on `74af433`.
- **Merged** #82 → integration `447cbdc`.
- Ledger: R0.2 → `landed` / `merged` / strict **`open`** (do **not** accept —
  E2 normalized artifacts + Phase 0 evidence residuals remain).
- Inventory still 20/112. No phase-gate close.

### Next owner procedure

```bash
git fetch origin
git checkout feature/matrix-rust-sdk-full-replacement
git pull --ff-only
# Dual-track next: R0.2-E2 (recover 119-row payloads → generate/commit audit+v2
# with accepted E1 tooling; do not invent missing payloads),
# or R0.7 authenticated disposable-Synapse residual (P3.2 allowlist careful),
# or R0.8 accepting formal re-issue when residuals clear.
# No false gate close. No cutover; no dual-backend; no main merge.
```

## Program accounting

- Original-plan artifact inventory remains **20 / 112 (~18%)**.
- **0 of 15** strict phase gates are closed.
- R0.1 / R0.3 / R0.4 / R0.5 / R0.6 **accepted**.
- R0.2 `landed` / `merged` (E1 tooling) / strict acceptance **`open`** (E2 + evidence residuals).
- R0.7 `landed` / `merged` / strict acceptance **`open`** (slices 1–4).
- R0.8 `landed` / `merged` (slice 1 residual formal reports) / strict acceptance **`open`**.
- Shipping runtime: `matrix-js-sdk` only; Rust harness foundation only.

## R0.8 formal report pointers

- [`r0.8-phase-gate-readiness-inventory.md`](r0.8-phase-gate-readiness-inventory.md)
- [`phase-0-formal-acceptance-report.md`](phase-0-formal-acceptance-report.md) — **not_accepted**
- [`phase-1-formal-acceptance-report.md`](phase-1-formal-acceptance-report.md) — **not_accepted**
- [`phase-2-formal-acceptance-report.md`](phase-2-formal-acceptance-report.md) — **not_accepted**
- [`p3.1-task-acceptance-report.md`](p3.1-task-acceptance-report.md) — **not_accepted**

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
