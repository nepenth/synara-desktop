# Delivery layers — how to read progress

| Field | Value |
| --- | --- |
| Date | 2026-07-28 |
| Related | [program-status.md](program-status.md), [PROGRESS.md](PROGRESS.md), [cutover-operating-model.md](cutover-operating-model.md) |

## Three layers (do not collapse)

| Layer | What “landed” means | How to measure |
| --- | --- | --- |
| **L1 — Harness foundation** | Pure Rust types, indexes, state machines, unit tests under `src-tauri/src/matrix/` | `program-status` **landed/merged** + tip `pN.M-*.md` docs |
| **L2 — Live host wiring** | Real matrix-sdk client, sync, send, crypto live paths + Tauri IPC commands | Not yet a single counter; look for live adapters / IPC commands beyond harness |
| **L3 — Product cutover** | UI uses IPC only; js-sdk not owner; sole Matrix owner | `product_runtime.matrix_client_sdk == matrix-rust-sdk-only` + cutover complete |

**Today:** L1 is large (**~74/112** plan tasks with foundations). **L2 is partial/early. L3 is not started** (product still `matrix-js-sdk-only`).

Strict phase-gate **acceptance** is a fourth axis: landed ≠ accepted (still **0/15** gates closed).

## Anti-patterns

- Quoting inventory without opening `program-status.md`
- Treating foundation PRs as “client is ready”
- Leaving `program-status.json` stale after product merges (CI drift guard fails when tip docs lack records)

## Update rule

Every product task merge that adds `docs/matrix-rust-sdk/pN.M-*.md` **must** update `program-status.json` in the same PR or an immediate ledger PR.


## Current priority (D0)

**L2 dogfood vertical over new L1.** See [d0-dogfood-epic.md](d0-dogfood-epic.md).

Primary scoreboard: product `matrix-js-sdk` import count ↓ and “can log in via Rust on this branch.”
