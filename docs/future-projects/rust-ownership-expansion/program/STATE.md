# Program state

Last updated: 2026-09-01 (ROE-05 and ROE-11 in review).
Integration branch: `feature/rust-ownership-residual-census`.
Base: `main` at `011cf39a`.

## Headline

All census-and-close priors plus the ROE-04/12 fixture cluster are
closed or stay-platform. ROE-08 remains a human implementation gate
(D10). ROE-05 and ROE-11 are in review. No product work. No merge to
`main`.

## Next actions

1. Merge #1090 and #1091 only after independent ACCEPT at HEAD.
2. Keep ROE-08 implementation stopped.
3. Optional later: land the ROE-04/12 fixture directory under
   `docs/future-projects/**` only. Do not implement a renderer or AST.

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
| ROE-10 | closed | merged `#1088` | already owned split; leftover UniFFI / Jotai are seams |
| ROE-04/12 | closed | merged `#1089` | stay platform; `formatted_body` comment still misleading; nits recorded |
| ROE-05 | in-review | `roe/memo-05-visibility` | [#1091](https://github.com/nepenth/synara-desktop/pull/1091) `25c1ee02` |
| ROE-11 | in-review | `roe/memo-11-media-metadata` | [#1090](https://github.com/nepenth/synara-desktop/pull/1090) `41c3d35b` |

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
- Do not put media paths or bytes on `Core::command`.
