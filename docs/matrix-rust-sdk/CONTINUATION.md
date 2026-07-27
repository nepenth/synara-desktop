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
| Live integration tip (this handoff) | `9f585d146731af8419c157cf179a0f079d36901d` — **R0.7 slice 2 / #98 merged** (loopback live login-types) on #96 transports + #97 status; re-fetch and verify |
| Historical audited snapshot | `edfefee499064b736985b6528896b693e5120f22` — bound to the 2026-07-25 review, not the live tip |
| Merged product fixes | PR [#86](https://github.com/nepenth/synara-desktop/pull/86) R0.5 wipe (**accepted**); PR [#87](https://github.com/nepenth/synara-desktop/pull/87)+[#94](https://github.com/nepenth/synara-desktop/pull/94) R0.4 store confinement + native keyring (**accepted**); PR [#89](https://github.com/nepenth/synara-desktop/pull/89) R0.6 diagnostic privacy (**accepted**); PR [#91](https://github.com/nepenth/synara-desktop/pull/91)+[#92](https://github.com/nepenth/synara-desktop/pull/92) R0.3 IPC wire freeze (**accepted**); PR [#96](https://github.com/nepenth/synara-desktop/pull/96)+[#98](https://github.com/nepenth/synara-desktop/pull/98) R0.7 live CS transports + loopback login-types (**merged**, strict acceptance **open** — store/sync/wipe lifecycle residual) |
| Parked R0.2-E1 PR | PR [#82](https://github.com/nepenth/synara-desktop/pull/82) — **draft / parked** (2× CI `v2 exceeded 512 MiB` residual; do not thrash) |
| Open PR to `main` | [#39](https://github.com/nepenth/synara-desktop/pull/39) — **do not merge without user approval** |
| Current execution | Dual-track: next **R0.7 residual** (encrypted store open / sync readiness / logout / wipe against disposable Synapse) or **R0.8** evidence; P3.2 blocked by R0.2 / R0.7 residual / R0.8; parked #82 |

Always re-fetch and verify branch tips and PR state.

## Exact continuation point

### Dual-track priority (user-approved)

1. Land Critical/High product fixes first (R0.5 ✓, R0.4 ✓, R0.6 ✓, R0.3 ✓).
2. R0.7 slices landed: live CS transports (**#96**) + loopback real-HTTP login-types evidence (**#98**).
3. Remaining R0 blockers for P3.2: R0.2 (parked E1), R0.7 residual (store/sync/logout/wipe lifecycle + full live accept), R0.8 (acceptance evidence).
4. R0.2-E1 (#82): **parked** on identical 512 MiB isolation-benchmark residual; resume only with a deliberate memory-bound fix.
5. Inventory stays **20/112** until more P-tasks land.
6. No dual-backend; no production cutover; no merge to `main` without explicit approval.

### This fire (2026-07-27, R0.7 slice-2 merge + status handoff)

- Independently reviewed and merged PR **#98** (loopback CS stub exercises live `HttpLoginFlowTransport`; privacy-safe 5xx; opt-in Synapse env gate).
- Local on #98: `cargo test --locked matrix::` **222 pass**; boundaries + guardrails pass.
- Exact-head CI required gates green on #98 `cf4ca9f`.
- **Merged** #98 → integration `9f585d1` (on top of #96/`ac7a062` + #97 status).
- Ledger: R0.7 remains `landed` / `merged` / strict **`open`** (do **not** accept fully — store/sync/logout/wipe residual remains).
- Status docs catch-up this PR; #82 remains parked draft.

### Next owner procedure

```bash
git fetch origin
git checkout feature/matrix-rust-sdk-full-replacement
git pull --ff-only
# Dual-track next: R0.7 residual (encrypted store open / sync readiness /
# logout / wipe vs disposable Synapse) or R0.8 acceptance evidence.
# P3.2 only when R0.2/R0.7/R0.8 allow. Resume #82 only with memory-bound fix.
# No product cutover commands; no dual-backend; no main merge without approval
```

## Program accounting

- Original-plan artifact inventory remains **20 / 112 (~18%)**.
- **0 of 15** strict phase gates are closed.
- R0.1 / R0.3 / R0.4 / R0.5 / R0.6 **accepted**. R0.2 remains `landed` / `pr_open` (parked draft) / strict acceptance `open`.
- R0.7 `landed` / `merged` / strict acceptance **`open`** (slices 1–2 only). R0.8 not started.
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
