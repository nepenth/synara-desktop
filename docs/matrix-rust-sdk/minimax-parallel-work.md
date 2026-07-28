# MiniMax-M3 parallel work protocol

| Field | Value |
| --- | --- |
| Date | 2026-07-28 |
| Model | Local **MiniMax-M3** W4A16 GPTQ on DGX Spark |
| Endpoint | `http://spark-1.whyland.com:8000/v1` |
| Grok config ids | `minimax-m3-local`, `minimax-m3-local-eagle` |
| Helper PR | [#109](https://github.com/nepenth/synara-desktop/pull/109) (parked until product stack drains; optional) |

## Role split

| Agent | Role |
| --- | --- |
| **Grok** (this orchestrator / implementer) | Tool use, code edits, PR merge, CI babysit, final review of MiniMax drafts |
| **MiniMax-M3** | Free-token **parallel text** worker: inventories, drafts, gap lists, doc expansion, review notes |

MiniMax is **not** the sole implementer for product code. Grok (or a human) integrates its output.

## Why parallel is valuable here

Product CI for `src-tauri` is often **15–20+ minutes** (Validate + package smoke). During those waits:

1. Do **not** tip-merge docs mid-flight if it forces full re-CI on green product PRs.
2. **Do** run MiniMax jobs that only touch draft notes or offline analysis (or stage docs for a later batch PR).

Time-to-first-token can be slow; throughput is still useful for multi-minute background jobs at **~2–4 concurrent** requests.

## Good MiniMax jobs (preferred)

| Job | Inputs | Output |
| --- | --- | --- |
| Expand feature-graph JS surfaces | `desktop-sdk-usage.json`, node id | Proposed `js_sdk_surface` + file hits |
| Map FR rows → graph nodes | `feature-parity-traceability.json` | FR id → node id table |
| Hard-problem re-review draft | node + hard_problem_index | Checklist with SDK alignment notes |
| Gap vs 0.18 dossier | `0.18.0-feature-and-gap-analysis.json` | Gaps touching a node |
| PROGRESS work-log draft | merged PR titles | Bullet lines for human/Grok edit |
| Read-only code explain | single module path | Summary of ownership boundaries |

## Avoid with MiniMax alone

- Silent updates to **`program-status.json`** without Grok verification (ledger accuracy is mandatory).
- Merge decisions, CI re-runs, force-pushes.
- Claims of strict phase-gate acceptance.
- Dual-backend / token-migration designs (policy forbidden).

## Orchestrator hooks (when waiting on CI)

While babysitting product PRs:

```text
if product CI in_progress and no mergeable PR:
  spawn MiniMax job from client-feature-graph.json minimax_jobs
  or expand one hard_problem re-review draft
  stage results under docs/matrix-rust-sdk/drafts/ OR return in agent notes
  do NOT merge docs that thrash product CI until batch window
```

## Config reminder

`~/.grok/config.toml`:

- `[model.minimax-m3-local]` → `Sebesky/MiniMax-M3-W4A16-GPTQ`
- no API key; OpenAI-compatible chat completions
- large context (~189k); keep completions bounded (`max_tokens` sane — model may preamble if too low)

## Acceptance for using MiniMax output

- Grok (or human) spot-checks against repo facts.
- Feature-graph / inventory changes land in a **docs or docs+ledger** PR with green Quality gate.
- Product code still goes through normal CI.
