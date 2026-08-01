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

### Model selection (selective)

| Work | Preferred model |
| ---- | ---------------- |
| Lane owner (`product.rs` vertical) | Stronger coding model (e.g. gpt-5.6-sol high) when available; else gpt-5.6-luna xhigh |
| **product.rs extract/split** | Stronger model + high reasoning / max thinking |
| Product ACCEPT review | luna xhigh or sol high |
| Parallel docs / residuals / TS-first | luna xhigh and/or DeepSeek-V4-Flash (cap concurrent DeepSeek) |

Do **not** burn the largest model on tip-SHA-only docs.

## Queue after this protocol

1. Finish the **current lane owner** vertical (powers-bulk if in flight).
2. **Extract/split `product.rs`** into domain command modules (behavior-preserving).
3. After extract merges: fan-out multiple product lanes on **different modules**
   with smaller models; keep ACL/`lib.rs` conflicts small.

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

Local daytime ops also keep a copy under `/tmp/synara-daytime-pipeline/`:

- `PRODUCT_LANE.md` — full protocol
- `LANE_OWNER` — current owner line
- `daytime-watcher.sh` — must not spawn product racers

If ops files and this doc disagree, **operator/orchestrator resolves** and updates both.
