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

## 2. Execution model — this harness only

We execute this project through **this agent harness and its configured,
locally hosted model** — no external model APIs at this time.

- Keep the orchestrator + sub-agent operating method: the parent orchestrates,
  scopes, reviews, and merges; children do bounded slices and reply with
  results; results fan back through files and messages.
- The configured model is **locally hosted** and comfortably supports
  **2–3 concurrent** sub-agents. Stay at or below that concurrency; do not
  flood the queue.
- **Do not spawn or reference external model services** (no API-based model
  calls, no third-party agent loops) while this instruction is in force.
- Shared-file lanes stay serial: exactly one product-lane owner edits
  `product.rs` / registers new `matrix_*` commands at a time
  ([product-lane-protocol.md](product-lane-protocol.md)); docs / TS-first work
  that reuses existing IPC may parallelize within the concurrency limit.
- Keep at least one independent sub-agent review for product merges (same
  harness, same local model) before ACCEPT.

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
