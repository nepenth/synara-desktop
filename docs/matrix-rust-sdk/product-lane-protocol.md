# Product lane protocol (Matrix Rust full-replacement)

| Field | Value |
| ----- | ----- |
| Status | **Operational protocol** — docs only |
| Audience | Implementers, reviewers, daytime watcher, orchestrator |
| Related | [full-vertical-policy.md](full-vertical-policy.md), finish-line SCOREBOARD |

## Problem

`src-tauri/src/matrix/auth/product.rs` is a large serial bottleneck (thousands of
lines, many Tauri commands). Parallel “product” agents that all edit this file
race, discard work, and thrash CI. dual_backend remains **forbidden**; full
verticals remain the product standard.

## PRODUCT_LANE (pre-extract)

1. **Exactly one lane owner** may edit `product.rs` and register new `matrix_*`
   commands (`lib.rs`, `build.rs`, capabilities/permissions) at a time.
2. **TS-first** work that reuses existing IPC (example: members drawer wiring) may
   proceed in parallel if it does **not** touch `product.rs` or new command
   registration.
3. **Docs / residual packets / C3–C5 honesty / V-BURN HOLD** may proceed in
   parallel; prefer **one SCOREBOARD writer** at a time.
4. **Tip thrash freeze:** while a product PR has ACCEPT@HEAD and is waiting for
   CI, do not mass-merge tip-SHA-only docs PRs that force product branch rebases.
5. Watchers must **not** spawn a second product.rs implementer while a lane
   owner is set.

### Model selection (single local model — current instruction)

The configured model is **locally hosted** and supports ~2–3 concurrent
sub-agents. No external model APIs are used at this time. Assign effort by role
and reasoning budget on the same model:

| Work | Execution |
| ---- | --------- |
| Lane owner (`product.rs` vertical) | Local model, high-effort role; serial — one lane owner at a time |
| **product.rs extract/split** | Local model, high reasoning budget (structural, behavior-preserving) |
| Product ACCEPT review | Independent sub-agent review on the same local model |
| Parallel docs / residuals / TS-first | Local model, ≤2–3 concurrent sub-agents; must not touch `product.rs` / command registration |

Product.rs stays serial (one lane owner); only docs/TS-first work that reuses
existing IPC may parallelize, within the 2–3 concurrency limit.

## Queue after this protocol

1. Finish the **current lane owner** vertical (powers-bulk if in flight).
2. **Extract/split `product.rs`** into domain command modules (behavior-preserving).
3. After extract merges: fan-out multiple product lanes on **different modules**
   with the same local model (≤2–3 concurrent); keep ACL/`lib.rs` conflicts small.

## Extract goals (next structural lane)

- Mechanical, **behavior-preserving** move of `matrix_*` command implementations
  into modules aligned with the existing `src-tauri/src/matrix/*` tree
  (`members`, `media`, `room_ops`, `widgets`, …).
- Thin registration surface (re-exports or a small command table).
- No IPC contract changes, no dual_backend, no main/#39.
- Single focused PR; prove `cargo check` / desktop shell tests and command ACL
  guardrails.

## Forbidden

- Two concurrent PRs that both edit `product.rs`.
- dual_backend / JS fallthrough on native desktop session for claimed verticals.
- Merging umbrella **#39** or targeting `main` without explicit operator approval.

## Daytime pipeline binding

Local ops may mirror protocol state under `/tmp/synara-daytime-pipeline/`
(gitignored — **not** public). Execution now runs through **this agent harness**
with its locally hosted model; `/tmp` mirrors are informational only. Keep
harness-specific secrets, skills, and orchestrator loops out of the public tree
([operating-instructions.md](operating-instructions.md)). If ops files and this
doc disagree, **operator/orchestrator resolves** and updates both.
