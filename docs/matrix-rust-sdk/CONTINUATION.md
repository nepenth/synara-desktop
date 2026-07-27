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
| Live integration tip (this handoff) | `9ab482bcc00998b3c9f62d227ccacfd42000b6cd` — **R0.5 / #86 merged**; re-fetch and verify |
| Historical audited snapshot | `edfefee499064b736985b6528896b693e5120f22` — bound to the 2026-07-25 review, not the live tip |
| Merged this fire | PR [#86](https://github.com/nepenth/synara-desktop/pull/86) — R0.5 transactional wipe (REV-001), merge `9ab482b` |
| Active product PR | PR [#87](https://github.com/nepenth/synara-desktop/pull/87) — R0.4 store confinement; CI was all-green on pre-#86 base — re-check mergeability after #86 |
| Open R0.2-E1 PR | PR [#82](https://github.com/nepenth/synara-desktop/pull/82) — tooling; cheap CI-portability only; park residual if thrash continues |
| Open PR to `main` | [#39](https://github.com/nepenth/synara-desktop/pull/39) — **do not merge without user approval** |
| Current execution | Dual-track: **R0.5 accepted/merged**; next **review/merge #87 (R0.4)**; then R0.6 → R0.3 → P3.2 when Critical/High residual allows |

Always re-fetch and verify branch tips and PR state. The SHAs above are the exact
handoff snapshot, not a promise that remote state has not advanced.

## Exact continuation point

### Dual-track priority (user-approved)

1. **Review/merge open product PRs** targeting integration (never `main`).
2. Prefer Critical/High product defects: R0.5 wipe ✓, **R0.4 store confinement next**, then R0.6 privacy, R0.3 IPC.
3. R0.2-E1 (#82): only cheap CI-portability fixes; merge if green; if CI thrash continues, park with residual and proceed with product engineering.
4. Do **not** block forever on formal R0.8 phase-gate theater before product Critical/High land.
5. Original-plan inventory stays **20/112** until more P-tasks land; R0 does not inflate that count.
6. No dual-backend; no production cutover; no merge to `main` without explicit approval.

### This fire (2026-07-27)

- Independently reviewed PR **#86** (R0.5 / REV-001) against wipe ordering requirements.
- Local: `cargo test --locked matrix::lifecycle` **19 pass**; `cargo test --locked matrix::` **191 pass**.
- Exact-head CI all required checks **green** (Validate desktop, Quality gate, Synapse, iOS, package smoke).
- **Merged** #86 → integration `9ab482b`.
- R0.5 ledger: `landed` / `merged` / **`accepted`** (product fix + tests + green CI). Phase 2 gate remains **open** (still blocked by R0.4/R0.6/R0.7/R0.8).

### Next owner procedure

```bash
git fetch origin
gh pr view 87 --json headRefOid,baseRefName,mergeable,mergeStateStatus,statusCheckRollup
# If #87 is behind integration after #86, rebase onto feature/matrix-rust-sdk-full-replacement
# Independently review diff vs REV-002/006/007; rerun focused store tests + guardrails
# Merge only on green non-cancelled required checks for the reviewed SHA
```

Then: R0.6 diagnostic privacy → R0.3 IPC wire freeze → timebox remaining R0.2 evidence → P3.2 when dual-track Critical/High residual allows.

## Program accounting

- Original-plan artifact inventory remains **20 / 112 (~18%)**. R0 corrective
  work does not increment that metric.
- **0 of 15** strict phase gates are closed.
- R0.5 is **accepted** on integration. R0.4 is `in_progress` / `pr_open`. R0.2 remains
  `landed` / `pr_open` / strict acceptance `open` (E1 unmerged).
- The shipping desktop runtime remains `matrix-js-sdk` only; the Rust SDK is
  still a harness foundation. There is no dual backend and no cutover.
- P3.2 remains blocked by unaccepted R0 remediations (R0.5 removed from that list).

## Authoritative docs

- Plan: [`../matrix-rust-sdk-full-replacement-plan.md`](../matrix-rust-sdk-full-replacement-plan.md)
- Detailed E1 handoff: [`r0.2-e1-handoff-2026-07-26.md`](r0.2-e1-handoff-2026-07-26.md)
- Full implementation handoff: [`implementation-handoff.md`](implementation-handoff.md)
- Parity: [`feature-parity-traceability.md`](feature-parity-traceability.md)
- Migration UX: [`migration-ux-decision.md`](migration-ux-decision.md)
- Independent review: [`review-2026-07-25.md`](review-2026-07-25.md)
- Current status: [`program-status.md`](program-status.md)

## Non-negotiables

- No dual-backend / selector.
- No merge to `main` without explicit user approval.
- No re-open of FR-7.8–7.11 quality audit; FR-7.9-011 stays partial sequential.
- No secrets in diagnostics/IPC.
- Guardrails stay green.
- No unnecessary E1 scope expansion beyond the 11-path set.
- No force-merge without independent review + green required CI.
