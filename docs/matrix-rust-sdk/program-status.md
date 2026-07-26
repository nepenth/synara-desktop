# Matrix Rust SDK replacement — current program status

> Generated from `program-status.json` by `scripts/check-matrix-rust-sdk-program-status.mjs`.
> Do not hand-edit. Dated task evidence and the independent review remain historical records.

As of: 2026-07-25

Integration branch: `feature/matrix-rust-sdk-full-replacement`

Audited snapshot: `edfefee499064b736985b6528896b693e5120f22`

## Current execution

- Active task: **R0.1**
- Next task: **R0.2**
- Blocked tasks: **P3.2** (by `R0.1`, `R0.2`, `R0.3`, `R0.4`, `R0.5`, `R0.6`, `R0.7`, `R0.8`)

## Inventory and runtime

- Landed original task artifacts: **20 / 112**
- Shipping Matrix client: `matrix-js-sdk-only`
- Rust SDK state: `harness-foundation-only`
- Dual backend: `false`
- Cutover state: `not_started`

## Phase gates

| Phase | Recorded tasks | Accepted tasks | Remediation blockers | Strict acceptance | Gate |
|---:|---:|---:|---|---|---|
| 0 | 7/7 | 0/7 | `R0.2`, `R0.8` | `open` | `open` |
| 1 | 6/6 | 0/6 | `R0.1`, `R0.3`, `R0.8` | `open` | `open` |
| 2 | 6/6 | 0/6 | `R0.4`, `R0.5`, `R0.6`, `R0.7`, `R0.8` | `open` | `open` |
| 3 | 1/8 | 0/8 | `R0.7`, `R0.8` | `open` | `open` |
| 4 | 0/8 | 0/8 | None | `not_reviewed` | `open` |
| 5 | 0/10 | 0/10 | None | `not_reviewed` | `open` |
| 6 | 0/8 | 0/8 | None | `not_reviewed` | `open` |
| 7 | 0/7 | 0/7 | None | `not_reviewed` | `open` |
| 8 | 0/8 | 0/8 | None | `not_reviewed` | `open` |
| 9 | 0/6 | 0/6 | None | `not_reviewed` | `open` |
| 10 | 0/7 | 0/7 | None | `not_reviewed` | `open` |
| 11 | 0/10 | 0/10 | None | `not_reviewed` | `open` |
| 12 | 0/7 | 0/7 | None | `not_reviewed` | `open` |
| 13 | 0/8 | 0/8 | None | `not_reviewed` | `open` |
| 14 | 0/6 | 0/6 | None | `not_reviewed` | `open` |

0 of 15 strict phase gates are closed.

## Mandatory remediation

| ID | Task | Artifact state | Integration state | Strict acceptance |
|---|---|---|---|---|
| R0.1 | Quality and metadata recovery | `in_progress` | `pr_open` | `open` |
| R0.2 | Governance and Phase 0 evidence | `not_started` | `not_submitted` | `not_reviewed` |
| R0.3 | IPC wire-contract correctness | `not_started` | `not_submitted` | `not_reviewed` |
| R0.4 | Store confinement and key management | `not_started` | `not_submitted` | `not_reviewed` |
| R0.5 | Safe destructive lifecycle | `not_started` | `not_submitted` | `not_reviewed` |
| R0.6 | Diagnostic privacy | `not_started` | `not_submitted` | `not_reviewed` |
| R0.7 | Live Phase 2 and P3.1 adapters | `not_started` | `not_submitted` | `not_reviewed` |
| R0.8 | Acceptance reports and CI evidence | `not_started` | `not_submitted` | `not_reviewed` |

## Recorded original task artifacts

`landed` and `merged` describe inventory and Git delivery only. They do not imply strict acceptance.

| ID | Artifact state | Integration state | Strict acceptance | Phase gate |
|---|---|---|---|---|
| P0.1 | `landed` | `merged` | `open` | `open` |
| P0.2 | `landed` | `merged` | `open` | `open` |
| P0.3 | `landed` | `merged` | `open` | `open` |
| P0.4 | `landed` | `merged` | `open` | `open` |
| P0.5 | `landed` | `merged` | `open` | `open` |
| P0.6 | `landed` | `merged` | `open` | `open` |
| P0.7 | `landed` | `merged` | `open` | `open` |
| P1.1 | `landed` | `merged` | `open` | `open` |
| P1.2 | `landed` | `merged` | `open` | `open` |
| P1.3 | `landed` | `merged` | `open` | `open` |
| P1.4 | `landed` | `merged` | `open` | `open` |
| P1.5 | `landed` | `merged` | `open` | `open` |
| P1.6 | `landed` | `merged` | `open` | `open` |
| P2.1 | `landed` | `merged` | `open` | `open` |
| P2.2 | `landed` | `merged` | `open` | `open` |
| P2.3 | `landed` | `merged` | `open` | `open` |
| P2.4 | `landed` | `merged` | `open` | `open` |
| P2.5 | `landed` | `merged` | `open` | `open` |
| P2.6 | `landed` | `merged` | `open` | `open` |
| P3.1 | `landed` | `merged` | `open` | `open` |
