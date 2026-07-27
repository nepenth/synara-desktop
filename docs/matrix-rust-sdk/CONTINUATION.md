# Matrix Rust SDK program — continuation card

**Date:** 2026-07-26

**Audience:** Current or next orchestrator of the full-replacement program.

For full history, rules, today’s validation accounting, and FR preservation
notes, use [`implementation-handoff.md`](implementation-handoff.md). The detailed
2026-07-26 E1 snapshot is
[`r0.2-e1-handoff-2026-07-26.md`](r0.2-e1-handoff-2026-07-26.md).

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
| E1 base before this documentation handoff | `7ffd5885e456c2b99c42d834127bc1ec6b1956ce`; the live integration tip is this commit or later and must be re-read from `origin` |
| Historical audited snapshot | `edfefee499064b736985b6528896b693e5120f22` — bound to the 2026-07-25 review, not the live tip |
| CI prerequisite | PR [#83](https://github.com/nepenth/synara-desktop/pull/83) merged as `7ffd588`; validation checkout now fetches full Git history |
| Active task PR | PR [#82](https://github.com/nepenth/synara-desktop/pull/82), R0.2-E1, open at `8ded923c6846194b3332c85dce69614368882729` |
| Open PR to `main` | [#39](https://github.com/nepenth/synara-desktop/pull/39) — **do not merge without user approval** |
| Current execution | R0.2 in progress; E1 locally accepted but unmerged; E2 blocked |

Always re-fetch and verify the two branch tips and PR state. The SHAs above are
the exact handoff snapshot, not a promise that remote state has not advanced.

## Exact continuation point

R0.2-E1 implements the deterministic audit-normalization and traceability-v2
tooling in an exact 11-path scope. Independent local review accepted the content,
but plan acceptance requires a green, non-cancelled exact-head CI run and merge.
PR #82 therefore remains **pending**, not completed or merged.

The first E1 CI attempt exposed a shallow-checkout incompatibility because the
validator intentionally reads pinned repository history. PR #83 corrected the CI
checkout and passed all of its required jobs. Refreshed PR #82 CI then ran the
full repository script suite: **283 of 284 tests passed**. The sole primary
failure is test-fixture portability: `temporaryLocalGitClone` creates a commit
without first configuring repository-local `user.name` and `user.email`. The
downstream `Quality gate` failure follows from the failed desktop-validation job;
it is not a separate E1 behavior defect.

At this snapshot the focused helper fix has **not** been implemented. Make only
that bounded test-helper change, rerun the complete validation set, independently
review the new exact head, push it to PR #82, and wait for green exact-head CI.
Only then merge #82 into the integration branch.

After the merge, E2 is the next R0.2 slice: recover and commit the authoritative
119-row normalized audit and traceability-v2 artifact through the accepted E1
tooling. Do not reconstruct, paraphrase, or invent missing reviewed payloads.
E2 does not by itself complete R0.2 or close Phase 0.

## Resume recipe

```bash
git fetch origin
git checkout matrix-rust/r0.2-e-traceability-tooling
git pull --ff-only origin matrix-rust/r0.2-e-traceability-tooling
git rev-parse HEAD
git status --short
gh pr view 82 --json headRefOid,baseRefName,state,statusCheckRollup
```

Expected pre-fix head: `8ded923c6846194b3332c85dce69614368882729`.
Configure the cloned test repository’s local Git identity inside
`temporaryLocalGitClone`; do not rely on a developer’s global Git configuration
and do not broaden the exact 11-path E1 scope.

Then reproduce at minimum:

```bash
npm run test:matrix-rust-traceability-tooling
node --test scripts/__tests__/*.test.mjs
npm run check:matrix-rust-guardrails
npm run check:matrix-rust-governance
npm run check:quality-gates
git diff --check origin/feature/matrix-rust-sdk-full-replacement...HEAD
```

Also rerun the focused temporary-clone regression, exact-scope Prettier, and
`node --check` for all six production E1 scripts as recorded in the detailed E1
handoff. Review the complete base-to-head diff again before push or merge.

## Program accounting

- Original-plan artifact inventory remains **20 / 112 (~18%)**. R0 corrective
  work does not increment that metric.
- **0 of 15** strict phase gates are closed.
- R0.2 remains `in_progress` / `pr_open` / strict acceptance `open`.
- The shipping desktop runtime remains `matrix-js-sdk` only; the Rust SDK is
  still a harness foundation. There is no dual backend and no cutover.
- P3.2 remains blocked by every unaccepted R0 remediation.

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
- No E2 before E1 has green exact-head CI and is merged.
- No P3.2 work until R0.1–R0.8 and the Phase 0–2/P3.1 gates are accepted.
