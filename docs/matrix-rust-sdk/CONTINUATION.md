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
| Live integration tip (this handoff) | `67f27f283255ef013e01b7bc596e337fea080f57` — **R0.7 slice 3 / #100 merged** (composed encrypted store lifecycle) on #98/#96 + status; re-fetch and verify |
| Historical audited snapshot | `edfefee499064b736985b6528896b693e5120f22` — bound to the 2026-07-25 review, not the live tip |
| Merged product fixes | PR [#86](https://github.com/nepenth/synara-desktop/pull/86) R0.5 wipe (**accepted**); PR [#87](https://github.com/nepenth/synara-desktop/pull/87)+[#94](https://github.com/nepenth/synara-desktop/pull/94) R0.4 store confinement + native keyring (**accepted**); PR [#89](https://github.com/nepenth/synara-desktop/pull/89) R0.6 diagnostic privacy (**accepted**); PR [#91](https://github.com/nepenth/synara-desktop/pull/91)+[#92](https://github.com/nepenth/synara-desktop/pull/92) R0.3 IPC wire freeze (**accepted**); PR [#96](https://github.com/nepenth/synara-desktop/pull/96)+[#98](https://github.com/nepenth/synara-desktop/pull/98)+[#100](https://github.com/nepenth/synara-desktop/pull/100) R0.7 live CS + loopback login-types + composed store lifecycle (**merged**, strict acceptance **open** — authenticated live sync residual) |
| Parked R0.2-E1 PR | PR [#82](https://github.com/nepenth/synara-desktop/pull/82) — **draft / parked** (2× CI `v2 exceeded 512 MiB` residual; do not thrash) |
| Open PR to `main` | [#39](https://github.com/nepenth/synara-desktop/pull/39) — **do not merge without user approval** |
| Current execution | Dual-track: next **R0.8** evidence slice or authenticated disposable-Synapse residual; P3.2 blocked by R0.2 / R0.7 residual / R0.8; parked #82 |

Always re-fetch and verify branch tips and PR state.

## Exact continuation point

### Dual-track priority (user-approved)

1. Land Critical/High product fixes first (R0.5 ✓, R0.4 ✓, R0.6 ✓, R0.3 ✓).
2. R0.7 slices landed: live CS transports (**#96**) + loopback login-types (**#98**) + composed encrypted store lifecycle (**#100**).
3. Remaining R0 blockers for P3.2: R0.2 (parked E1), R0.7 residual (authenticated live sync vs Synapse + formal accept), R0.8 (acceptance evidence).
4. R0.2-E1 (#82): **parked** on identical 512 MiB isolation-benchmark residual; resume only with a deliberate memory-bound fix.
5. Inventory stays **20/112** until more P-tasks land.
6. No dual-backend; no production cutover; no merge to `main` without explicit approval.

### This fire (2026-07-27, R0.7 slice-3 merge + status handoff)

- Independently reviewed and merged PR **#100** (real `SdkClientHandle` open → Ready → logout → reopen → wipe; no production login/sync APIs).
- Local on #100: `cargo test --locked matrix::` **223 pass**; boundaries + guardrails pass.
- Exact-head CI required gates green on #100 `06505ed`.
- **Merged** #100 → integration `67f27f2`.
- Ledger: R0.7 remains `landed` / `merged` / strict **`open`** (do **not** accept fully — authenticated live sync residual + R0.8 remain).
- Status docs catch-up this PR; #82 remains parked draft.

### Next owner procedure

```bash
git fetch origin
git checkout feature/matrix-rust-sdk-full-replacement
git pull --ff-only
# Dual-track next: R0.8 acceptance evidence, or authenticated disposable-Synapse
# residual (login/sync — careful: P3.2 territory). Do not fully accept R0.7 yet.
# Resume #82 only with memory-bound fix. No cutover; no dual-backend; no main merge.
```

## Program accounting

- Original-plan artifact inventory remains **20 / 112 (~18%)**.
- **0 of 15** strict phase gates are closed.
- R0.1 / R0.3 / R0.4 / R0.5 / R0.6 **accepted**. R0.2 remains `landed` / `pr_open` (parked draft) / strict acceptance `open`.
- R0.7 `landed` / `merged` / strict acceptance **`open`** (slices 1–3). R0.8 not started.
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
