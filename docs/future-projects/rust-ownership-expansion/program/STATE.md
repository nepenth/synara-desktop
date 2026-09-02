# Program state

Last updated: 2026-09-01 (orchestrator tick; no memo PRs yet).
Integration branch: `feature/rust-ownership-residual-census`.
Base: `main` at `011cf39a`.

## Headline

First researchers are still in flight. No PRs target the feature branch.
No reviewer or ROE-09 assignment this tick. No product work. No merge to
`main`.

## Next actions

1. Wait for researcher PRs for `memos/ROE-08-agent-approval-memo.md`,
   `memos/ROE-01-orchestration-memo.md`, and
   `memos/ROE-02-verification-memo.md`.
2. On the next tick that sees a memo PR, launch a different-agent
   reviewer at exact HEAD.
3. Do not start ROE-09 until ROE-01 or ROE-02 frees a census-and-close
   slot. Do not start ROE-07 until the ROE-08 memo exists.

## Active lanes

| Lane | Role | Agent/worktree | Branch | Status |
| --- | --- | --- | --- | --- |
| Orchestrator | assign/merge | this session + `loop-roe-residual-census` | feature branch | active |
| ROE-08 | researcher | isolated worktree | `roe/memo-08-agent-approvals` | researching |
| ROE-01 | researcher | isolated worktree | `roe/memo-01-orchestration` | researching |
| ROE-02 | researcher | isolated worktree | `roe/memo-02-verification` | researching |

## Blockers

None at start. Implementation remains gated (D3, D9). Shared-Core P4
engine-ready remains blocked on paused iOS CI and live homeserver proof;
that does not block docs-only memos.

## Stop / do not

- Do not invent S38 or start P5.
- Do not open ROE-04 as a Core AST design.
- Do not register leftover secret/byte commands on `Core::command`.
- Do not merge this branch to `main` from the loop.
