# Operating instructions — Matrix Rust full replacement

| Field           | Value                                                                                       |
| --------------- | ------------------------------------------------------------------------------------------- |
| Status          | **Active operating instructions** — supersede external-model playbooks                      |
| Scope           | How this program is executed day-to-day and what every slice must respect                   |
| Repo visibility | **PUBLIC** — every doc, note, status, and commit in this tree is public                     |
| Related         | [README.md](README.md), [SCOREBOARD.md](SCOREBOARD.md), [full-vertical-policy.md](full-vertical-policy.md), [pause-handoff-2026-08-03.md](pause-handoff-2026-08-03.md) |

These are the standing operating instructions for this project. They apply to
every PR, doc, residual, progress note, and slice, and they are the go-forward
authority where older playbooks disagree.

## 1. Public repository — no secrets, ever

This repository is **public** and remains public for the life of the project.

- **Never commit**: credentials, API keys, tokens (access, refresh, session),
  recovery/backup material, ciphertext/private keys, private endpoints or
  hosts, personal data, or any account identifier not already public.
- Assume any document committed here is visible to everyone. There is no private
  "docs note" tier in the repo.
- Do not paste real values into fixtures, IPC examples, DTO diagrams, or
  diagnostic examples. Use obvious placeholders (`s3cret-must-never-parse`,
  `EXAMPLE-mxc://...`) like the existing IPC fixtures, and treat a real-looking
  value as a blocker.
- `.env`, signing credentials, and local agent/harness tooling stay in
  `.gitignore` / local-only paths. Keep `/tmp/synara-daytime-pipeline/` mirrors
  (if any) local and out of history.
- Before committing any new doc or log, scan for secret patterns and real
  identifiers. If something sensitive slipped into an already-committed file,
  flag it immediately (do not try to "unwrap" it quietly — work with the repo
  owner and rotate/remove the value).
- Diagnostics, schemas, and negative-test fixtures that prove secrets must
  never parse are fine; real values are not.

## 2. Execution model — prime-agent orchestrator + DeepSeek V4 Flash 0731 sub-agents

**The harness is prime-agent** (the runtime we run in). The orchestrator runs
here, drives scope, reviews, and merges. Sub-agents are spawned **inside
prime-agent** (via the agent runtime's sub-agent facility) — up to **2
concurrent sessions max**.

- **The only model configured in prime-agent is `deepseek-v4-flash-0731`**
  (selector `whyland-spark/deepseek-v4-flash-0731`), **locally hosted**. Every
  orchestrator turn and every sub-agent session uses it. There are no other
  model APIs in use.
- **Concurrency: up to 2 concurrent sub-agent sessions.** Do not spawn a third
  concurrent session; do not flood the queue.
- Children do bounded slices and reply with results; results fan back through
  files and messages; the orchestrator (prime-agent) reviews, approves, and
  merges PRs.
- Shared-file lanes stay serial: exactly one product-lane owner edits
  `product.rs`-family command files / registers new `matrix_*` commands at a
  time ([product-lane-protocol.md](product-lane-protocol.md)); docs and
  TS-first work that reuses existing IPC may run in the 2 parallel sessions.
- Keep at least one independent sub-agent review for product merges before
  ACCEPT.

### 2.1 PR and merge workflow (orchestrator)

- Each slice lands as a **task branch → PR onto
  `feature/matrix-rust-sdk-full-replacement`**, never directly to `main`
  (`main` and umbrella **#39** remain gated; no merge without explicit operator
  approval).
- The orchestrator opens the PR, labels it, and updates the project tracking
  docs **as work starts and as it finishes** (PROGRESS / SCOREBOARD /
  `v-*.md` / program status as applicable).
- An independent sub-agent review (same model) or the repo's own review gate
  approves the PR; the orchestrator merges the PR into the feature branch once
  approved and CI is green, then republishes the scoreboard/burn board.

## 3. UI/UX high-fidelity mandate

Replacing a capability must **not** change the app's look and feel. There is no
reason to change the UX or UI while we re-home features to the native path.

- A completed slice presents the **same visuals, layout, copy, spacing,
  behavior, and interactions** as before the rewiring — nothing more, nothing
  less.
- No visual redesign, no layout/UI/UX/copy changes, and no component swap that
  alters rendering are acceptable as a side effect of migration. Native owners
  must render the same UI from the same Synara-owned DTOs.
- **Whoever notices a visual difference treats it as a defect in the slice**,
  not a decoration: file a named residual and fix forward until fidelity is
  restored. This includes differences observed in a beta build after partial
  migration.
- Where feasible, keep a guardrail/source-guard that proves the UI assembly is
  unchanged by a slice (existing `*SourceGuard` tests are the house pattern),
  and record a "no visual change" statement in the slice evidence when a live
  visual check is not practical.
- A deliberately requested design change is a separate, explicit engineering
  task with its own approval — it is never smuggled in as part of a
  js-sdk→rust-sdk migration slice.

## 4. Go-forward workflow

1. Serial full-vertical slices on `feature/matrix-rust-sdk-full-replacement`
   only; physical deletion of the superseded JS owner happens **inside each
   vertical** ([full-vertical-policy.md](full-vertical-policy.md)).
2. Residual-empty importer burn with inventory honesty: regenerate
   `desktop-sdk-usage.{md,json}`, ratchet allowlist `pathCount` / `paths[]`,
   and update test + P1.6 guardrail floors together.
3. Never a dual backend; never main / umbrella **#39** without explicit
   operator approval.
4. Every land updates the ledgers it touches (SCOREBOARD / PROGRESS /
   program-status when relevant) in the same PR or an immediate ledger PR.
5. V-BURN stays **HOLD** until zero production importers and the drop-npm
   criteria are met.
6. Prefer the smallest focused evidence set; use required PR CI as the broad
   integration proof. Do not add scope, cleanup, or harden unrelated to the
   slice ([operating-path-contract.md](operating-path-contract.md)).

## 5. Supersession / reading old playbooks

Historical docs (e.g. the 2026-08-03 pause handoff, model-routing tables, Grok /
Codex / DeepSeek consumption accounting) remain on the branch as snapshots of
what was true when written. They are **not** current operating instructions.
Where any of them conflicts with this document, **this document wins** for
go-forward execution.
