# Operating protocol

Human charter: 2026-09-01. Integration branch:
`feature/rust-ownership-residual-census`.

This is a **docs-only residual census**. Overnight success is reviewed
memos merged into the feature branch, not product behavior.

## Roles

| Role | Who | May | Must not |
| --- | --- | --- | --- |
| Orchestrator | This overnight session and its timer loop | Assign lanes, open/merge research PRs into the feature branch, update program docs, launch researchers and reviewers | Author a memo it will also ACCEPT; implement product code; merge to `main`; edit playbook or goal-graph status |
| Researcher | One agent per workstream or cluster memo | Census current source, write one memo under `docs/future-projects/**`, open a PR to the feature branch | Touch product/Core/UniFFI; open a second lane; treat [CENSUS.md](CENSUS.md) as live truth without re-reading source |
| Reviewer | A different agent from the author | Verdict the memo at an exact HEAD; demand source links, taxonomy, and a stay-put alternative | Rewrite the memo in place; expand scope; ACCEPT a proceed recommendation as authorization to implement |

One implementer per lane. Two agents must not edit the same memo or the
same program-status file at once. The orchestrator serializes
`program/STATE.md`, `TRACKING.md`, and `DECISIONS.md`.

## What overnight execution is

Allowed now:

1. Current-source census memos.
2. Close/stay-platform-side recommendations.
3. Shared-fixture *design* notes inside a memo (paths and corpus shape
   only). Actual fixture files under `docs/future-projects/**` may land
   if a memo recommends them and a reviewer ACCEPTs.
4. Recording extract/proceed recommendations and stopping that lane.

Forbidden until a numbered decision in [DECISIONS.md](DECISIONS.md)
explicitly opens an implementation gate:

- `crates/**`, `src-tauri/**`, `synara/src/**`, `synara-ios/**` product
  changes
- new Core commands, DTOs, UniFFI methods, or tests outside this
  portfolio
- PRs whose base is `main`
- inventing S38, starting P5, or rewriting shared-Core acceptance state
- a Core message AST or new byte/path envelope

ADR 0004 now lists shared notification eligibility and agent-approval
policy as Core-shaped *authority*. That does not authorize deleting the
TypeScript or Swift planners tonight. Sequence any removal behind a
memo, a reviewer ACCEPT, a human implementation decision, and the
current goal-graph stop gate.

## Lane policy

- **One deep cluster at a time:** Notifications and agent policy
  (ROE-08 first, then ROE-07 if the ROE-08 memo is closed or waiting on
  a human gate).
- **Parallel census-and-close** is allowed for already-owned priors:
  ROE-01, ROE-02, ROE-09. Later: ROE-03/06/10/11 as short close memos
  only when a researcher is free and files do not overlap.
- **Deferred deep work:** Message format and safety (ROE-04/12) after
  the agent-policy cluster, unless a researcher is idle and ROE-08 is
  blocked on review only. Fixtures before types.
- ROE-05 after the fixture cluster unless a memo proves it is the
  actual next residual.

## Branch and PR rules

```text
main
  └── feature/rust-ownership-residual-census     (integration; do not merge to main overnight)
        ├── roe/memo-08-agent-approvals
        ├── roe/memo-01-orchestration
        └── roe/memo-02-verification
```

- Worker branches: `roe/memo-XX-short-name` or `roe/review-fix-XX`.
- `gh pr create --base feature/rust-ownership-residual-census`.
- Paths: `docs/future-projects/**` only. `npm run check:docs` must pass.
- Use an isolated git worktree. Do not checkout another branch in the
  orchestrator worktree.
- After merge, delete the worker branch.

## Review verdict

Reviewers post a single top-level PR comment:

```markdown
## Verdict: ACCEPT | ACCEPT_WITH_NITS | REJECT
Exact HEAD: <full sha>

<why, including the strongest stay-put objection and whether source
paths were re-verified>
```

The orchestrator merges only `ACCEPT` or `ACCEPT_WITH_NITS` (nits fixed
or recorded in the memo). `REJECT` returns to the same researcher lane
or a replacement researcher; the reviewer does not take authorship.

ACCEPT a close/stay memo when:

1. The census is source-linked on desktop, iOS, and Core.
2. Each concern is classified as authority / observation / rendering.
3. Hard invariant vs platform boundary vs technology preference is
   marked.
4. At least one stay-put alternative is argued in good faith.
5. The recommendation matches the evidence, including “already owned.”

REJECT if the memo treats adapters as a second engine, proposes product
code, silently reinterprets an ADR, or skips iOS or desktop.

## Orchestrator tick

Each loop tick:

1. `git fetch origin` and work on
   `feature/rust-ownership-residual-census` only.
2. Inventory PRs targeting that branch and any in-flight worktrees.
3. Merge eligible ACCEPT memos; update [STATE.md](STATE.md) and
   [TRACKING.md](TRACKING.md); commit those updates on the feature
   branch.
4. Assign empty allowed lanes. Do not open a second deep cluster.
5. Launch a reviewer when a memo PR is up and has no verdict at HEAD.
6. If a memo recommends extract/proceed: record it in
   [DECISIONS.md](DECISIONS.md), mark the lane `human-gate`, and do not
   write a plan or product slice.
7. Stop the tick if disk, CI, or a conflicting agent would cause thrash.
   Record the blocker.

## Stop conditions

Stop assigning new work when:

- the human asks to stop;
- a worker proposes product code or a `main` PR;
- the only remaining items are implementation-gated;
- two agents are contending for the same files;
- `check:docs` cannot be made green without leaving the portfolio tree.

Census-and-close of every prior, plus a reviewed ROE-08 memo, is a
complete overnight outcome.
