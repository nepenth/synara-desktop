# Program state

Last updated: 2026-09-01 (ROE-09 closed; ROE-06 started).
Integration branch: `feature/rust-ownership-residual-census`.
Base: `main` at `011cf39a`.

## Headline

ROE-01 and ROE-02 are closed as already owned. ROE-08 remains a human
implementation gate (D10). ROE-09 is closed as already owned. ROE-07
and ROE-03 are still researching. ROE-06 census of existing Core sort
helpers is started. No product work. No merge to `main`.

## Next actions

1. Review the ROE-07 and ROE-03 PRs when they exist.
2. Keep ROE-08 implementation stopped.
3. Do not start ROE-04/12 until the agent-policy cluster is idle.

## Active lanes

| Lane | Role | Branch | Status |
| --- | --- | --- | --- |
| Orchestrator | assign/merge | feature branch | active |
| ROE-08 | human-gate | merged `#1082` | accepted extract; no implementation |
| ROE-01 | closed | merged `#1081` | already owned; nits recorded in memo |
| ROE-02 | closed | merged `#1083` | already owned |
| ROE-09 | closed | merged `#1084` | already owned; no second notes engine |
| ROE-07 | researcher | `roe/memo-07-notification-policy` | researching |
| ROE-03 | researcher | `roe/memo-03-timeline-rows` | researching |
| ROE-06 | researcher | `roe/memo-06-room-sort` | assigned |

## Blockers

D10: ROE-08 extract waits on an explicit human implementation decision.
Shared-Core P4 engine-ready remains blocked; that does not block
docs-only memos.

## Stop / do not

- Do not invent S38 or start P5.
- Do not open ROE-04 as a Core AST design.
- Do not register leftover secret/byte commands on `Core::command`.
- Do not merge this branch to `main` from the loop.
- Do not delete TypeScript or Swift approval detectors.
