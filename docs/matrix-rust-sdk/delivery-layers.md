# Delivery layers — how to read progress

| Field   | Value                                                                                                                        |
| ------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Date    | 2026-07-28                                                                                                                   |
| Related | [program-status.md](program-status.md), [PROGRESS.md](PROGRESS.md), [cutover-operating-model.md](cutover-operating-model.md) |

## Three layers (do not collapse)

| Layer                             | What “landed” means                                                                | How to measure                                                                                                   |
| --------------------------------- | ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| **L1 — Harness foundation**       | Pure Rust types, indexes, state machines, unit tests under `src-tauri/src/matrix/` | `program-status` **landed/merged** + tip `pN.M-*.md` docs                                                        |
| **L2 — Live host wiring**         | Real matrix-sdk client, sync, send, crypto live paths + Tauri IPC commands         | Not yet a single counter; look for live adapters / IPC commands beyond harness                                   |
| **L3 — Product vertical cutover** | UI uses IPC for a capability and the superseded JS owner is physically deleted     | [d0-residual-completion.md](d0-residual-completion.md): `wired` vs `done`, plus per-vertical importer/file delta |
| **L4 — Repository convergence**   | No JS client/imports/dependency remain anywhere; release cutover is complete       | V-BURN audit, guardrails, package removal, final product/release gates                                           |

**Today:** L1 is large (**~74/112** plan tasks with foundations). L2 is broad
enough to support the D0 native core and V-CRYPTO wiring. L3 is **in progress**:
D0 core paths and V-CRYPTO.1–.5 are complete, including physical deletion of
their superseded JS owners. V-CRYPTO.6 is a local automatic-recovery candidate
with its superseded owners deleted; live/UI proof and PR review remain open.
V-CRYPTO.7 is queued. L4 has not started.

Strict phase-gate **acceptance** is a fourth axis: landed ≠ accepted (still **0/15** gates closed).

## Anti-patterns

- Quoting inventory without opening `program-status.md`
- Treating foundation PRs as “client is ready”
- Treating a native conditional branch as product completion while its JS owner remains
- Leaving `program-status.json` stale after product merges (CI drift guard fails when tip docs lack records)

## Update rule

Every product task merge that adds `docs/matrix-rust-sdk/pN.M-*.md` **must** update `program-status.json` in the same PR or an immediate ledger PR.

## Current priority (D0)

**Full product verticals over new L1.** See [d0-dogfood-epic.md](d0-dogfood-epic.md).

Primary scoreboard: capability residual rows close only after live wiring,
parity, privacy, tests, and physical JS deletion. Each completed vertical must
record a negative capability-owner/file deletion delta. The repository-wide
direct `matrix-js-sdk` import count is recorded and non-increasing; a zero global
delta is honest when the deleted owner accessed the SDK indirectly.
