# Program state

Last updated: 2026-09-01 (ROE-10 in review; ROE-04/12 relaunched).
Integration branch: `feature/rust-ownership-residual-census`.
Base: `main` at `011cf39a`.

## Headline

ROE-01, ROE-02, ROE-03, ROE-06, and ROE-09 are closed as already owned
or stay-platform. ROE-07 is closed as already-owned settings /
stay-platform delivery (nits recorded). ROE-08 remains a human
implementation gate (D10). ROE-10 is in review. ROE-04/12 fixture-first
cluster is assigned (first spawn failed; relaunched). No product work.
No merge to `main`.

## Next actions

1. Merge #1088 only after independent ACCEPT at HEAD.
2. Review the ROE-04/12 memo when the PR exists. Fixtures before types;
   do not design a Core AST.
3. Keep ROE-08 implementation stopped.
4. Do not start ROE-05 or ROE-11 until the fixture cluster is idle.

## Active lanes

| Lane | Role | Branch | Status |
| --- | --- | --- | --- |
| Orchestrator | assign/merge | feature branch | active |
| ROE-08 | human-gate | merged `#1082` | accepted extract; no implementation |
| ROE-01 | closed | merged `#1081` | already owned; nits recorded in memo |
| ROE-02 | closed | merged `#1083` | already owned |
| ROE-03 | closed | merged `#1086` | already owned; shared `thread_root` omission is not an extract |
| ROE-09 | closed | merged `#1084` | already owned; no second notes engine |
| ROE-07 | closed | merged `#1085` | already owned / stay platform; nits recorded in memo |
| ROE-06 | closed | merged `#1087` | stay platform-side; unused Core helpers; nits recorded |
| ROE-10 | in-review | `roe/memo-10-drafts` | [#1088](https://github.com/nepenth/synara-desktop/pull/1088) `11aa9881` |
| ROE-04/12 | researcher | `roe/memo-04-message-format` | assigned; fixtures before types; no AST |

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
