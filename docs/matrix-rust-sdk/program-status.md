# Matrix Rust SDK replacement — historical program status

> Generated from `program-status.json` by `scripts/check-matrix-rust-sdk-program-status.mjs`.
> Do not hand-edit. This is a frozen migration-program snapshot, not current product architecture.
> The replacement has landed; see [the codebase knowledge base](../../CODEBASE_KNOWLEDGE_BASE.md) and [the 2026-08-17 local proof](../shared-native-core/15-2026-08-17-local-proof.md).

As of: 2026-08-08

Integration branch: `feature/matrix-rust-sdk-full-replacement`

Audited snapshot: `bf1b565eb6163ded095e413ef88e3b22d7f21c01`

## Original-plan foundation queue

- Active task: **None**
- Next task: **None**
- Blocked tasks: None

## Original-plan inventory and runtime at this snapshot

- Landed original task artifacts: **74 / 112**
- Release/main Matrix client: `matrix-js-sdk-only`
- Rust SDK state: `harness-foundation-only`
- Dual backend: `false`
- Cutover state: `not_started`

These fields describe the historical audited snapshot and must not be used as current release/main claims.

## Full-vertical product execution at this snapshot

- Policy: `full-vertical-delete-per-vertical`
- Integration product state: `between-slices-paused`
- Active slice: **None**
- Wired / deletion open: None
- Completed under full policy: `V-CRYPTO.1`, `V-CRYPTO.2`, `V-CRYPTO.3`, `V-CRYPTO.4`, `V-CRYPTO.5`, `V-CRYPTO.6`, `V-CRYPTO.7`, `V-AUTH.1`, `V-ROOMS.1`, `V-ROOMS.3`, `V-ROOMS.4`, `V-SEND.1`, `V-SEND.2`, `V-ROOMS.5`, `V-SEND.3`
- Next slices: `V-TIMELINE.1` → `V-AUTH.2` → `V-AUTH.3` → `V-AUTH.4` → `V-ROOMS.2` → `V-SEND.5`
- Held PRs: #221, #240, #109, #193, #196, #198, #199, #201, #203, #204, #207, #208, #209
- Completion evidence: negative capability-owner/file deletion delta; global direct-import delta recorded and non-increasing
- matrix-js-sdk inventory: **0 files / 0 import lines current**; baseline **232 / 292**

## Phase gates

| Phase | Recorded tasks | Accepted tasks | Remediation blockers | Strict acceptance | Gate |
|---:|---:|---:|---|---|---|
| 0 | 7/7 | 0/7 | `R0.2`, `R0.8` | `open` | `open` |
| 1 | 6/6 | 0/6 | `R0.1`, `R0.3`, `R0.8` | `open` | `open` |
| 2 | 6/6 | 0/6 | `R0.4`, `R0.5`, `R0.6`, `R0.7`, `R0.8` | `open` | `open` |
| 3 | 8/8 | 0/8 | `R0.7`, `R0.8` | `open` | `open` |
| 4 | 8/8 | 0/8 | None | `not_reviewed` | `open` |
| 5 | 10/10 | 0/10 | None | `not_reviewed` | `open` |
| 6 | 8/8 | 0/8 | None | `not_reviewed` | `open` |
| 7 | 7/7 | 0/7 | None | `not_reviewed` | `open` |
| 8 | 8/8 | 0/8 | None | `not_reviewed` | `open` |
| 9 | 5/6 | 0/6 | None | `not_reviewed` | `open` |
| 10 | 1/7 | 0/7 | None | `not_reviewed` | `open` |
| 11 | 0/10 | 0/10 | None | `not_reviewed` | `open` |
| 12 | 0/7 | 0/7 | None | `not_reviewed` | `open` |
| 13 | 0/8 | 0/8 | None | `not_reviewed` | `open` |
| 14 | 0/6 | 0/6 | None | `not_reviewed` | `open` |

0 of 15 strict phase gates are closed.

## Mandatory remediation

| ID | Task | Artifact state | Integration state | Strict acceptance |
|---|---|---|---|---|
| R0.1 | Quality and metadata recovery | `landed` | `merged` | `accepted` |
| R0.2 | Governance and Phase 0 evidence | `landed` | `merged` | `open` |
| R0.3 | IPC wire-contract correctness | `landed` | `merged` | `accepted` |
| R0.4 | Store confinement and key management | `landed` | `merged` | `accepted` |
| R0.5 | Safe destructive lifecycle | `landed` | `merged` | `accepted` |
| R0.6 | Diagnostic privacy | `landed` | `merged` | `accepted` |
| R0.7 | Live Phase 2 and P3.1 adapters | `landed` | `merged` | `open` |
| R0.8 | Acceptance reports and CI evidence | `landed` | `merged` | `open` |

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
| P3.2 | `landed` | `merged` | `open` | `open` |
| P3.3 | `landed` | `merged` | `open` | `open` |
| P3.4 | `landed` | `merged` | `open` | `open` |
| P3.5 | `landed` | `merged` | `open` | `open` |
| P3.6 | `landed` | `merged` | `open` | `open` |
| P3.7 | `landed` | `merged` | `open` | `open` |
| P3.8 | `landed` | `merged` | `open` | `open` |
| P4.1 | `landed` | `merged` | `open` | `open` |
| P4.2 | `landed` | `merged` | `open` | `open` |
| P4.3 | `landed` | `merged` | `open` | `open` |
| P4.4 | `landed` | `merged` | `open` | `open` |
| P4.5 | `landed` | `merged` | `open` | `open` |
| P4.6 | `landed` | `merged` | `open` | `open` |
| P4.7 | `landed` | `merged` | `open` | `open` |
| P4.8 | `landed` | `merged` | `open` | `open` |
| P5.1 | `landed` | `merged` | `open` | `open` |
| P5.2 | `landed` | `merged` | `open` | `open` |
| P5.3 | `landed` | `merged` | `open` | `open` |
| P5.4 | `landed` | `merged` | `open` | `open` |
| P5.5 | `landed` | `merged` | `open` | `open` |
| P5.6 | `landed` | `merged` | `open` | `open` |
| P5.7 | `landed` | `merged` | `open` | `open` |
| P5.8 | `landed` | `merged` | `open` | `open` |
| P5.9 | `landed` | `merged` | `open` | `open` |
| P5.10 | `landed` | `merged` | `open` | `open` |
| P6.1 | `landed` | `merged` | `open` | `open` |
| P6.2 | `landed` | `merged` | `open` | `open` |
| P6.3 | `landed` | `merged` | `open` | `open` |
| P6.4 | `landed` | `merged` | `open` | `open` |
| P6.5 | `landed` | `merged` | `open` | `open` |
| P6.6 | `landed` | `merged` | `open` | `open` |
| P6.7 | `landed` | `merged` | `open` | `open` |
| P6.8 | `landed` | `merged` | `open` | `open` |
| P7.1 | `landed` | `merged` | `open` | `open` |
| P7.2 | `landed` | `merged` | `open` | `open` |
| P7.3 | `landed` | `merged` | `open` | `open` |
| P7.4 | `landed` | `merged` | `open` | `open` |
| P7.5 | `landed` | `merged` | `open` | `open` |
| P7.6 | `landed` | `pr_open` | `open` | `open` |
| P7.7 | `landed` | `pr_open` | `open` | `open` |
| P8.1 | `landed` | `merged` | `open` | `open` |
| P8.2 | `landed` | `merged` | `open` | `open` |
| P8.3 | `landed` | `merged` | `open` | `open` |
| P8.4 | `landed` | `merged` | `open` | `open` |
| P8.5 | `landed` | `merged` | `open` | `open` |
| P8.6 | `landed` | `merged` | `open` | `open` |
| P8.7 | `landed` | `merged` | `open` | `open` |
| P8.8 | `landed` | `merged` | `open` | `open` |
| P9.1 | `landed` | `merged` | `open` | `open` |
| P9.2 | `landed` | `pr_open` | `open` | `open` |
| P9.3 | `landed` | `pr_open` | `open` | `open` |
| P9.4 | `landed` | `pr_open` | `open` | `open` |
| P9.5 | `landed` | `pr_open` | `open` | `open` |
| P10.4 | `landed` | `pr_open` | `open` | `open` |
