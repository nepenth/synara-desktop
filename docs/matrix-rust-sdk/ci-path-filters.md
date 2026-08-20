# CI path filters — Matrix Rust integration workflow

| Field | Value |
| --- | --- |
| Date | 2026-07-27 |
| Workflow | `.github/workflows/ci.yml` |
| Related | `.github/workflows/desktop-package-smoke.yml` |

## Goal

Avoid full-repo CI cost when a PR only changes unrelated trees (docs, pure
markdown, iOS-only vs desktop-only, etc.), while keeping **required** checks
honest and mergeable.

## Design

**Do not** use workflow-level `paths:` alone for required CI jobs. If a required
check never runs, branch protection can block the PR forever.

**Do** use a cheap `Detect CI scopes` job + job-level `if:` so each heavy job
either:

- runs and reports success/failure, or  
- is **skipped** (path filter) — quality gate treats `skipped` as OK.

Push to `main` / `release/**` and `workflow_dispatch` always run the **full**
desktop/runtime suite.

**TEMPORARY (2026-08-17):** hosted iOS simulator CI is paused until `main` is
stable. `ios-tests` is hard-skipped (`if: false`); the `ios` scope output is
forced `false` on pull requests, pushes, and `workflow_dispatch`. Quality gate
already treats a skipped iOS job as OK. `ios-skeleton.yml` is skipped the same
way. Release.yml TestFlight / exact-tag iOS jobs are **not** paused. Re-enable
by restoring `ios=true` on the full-suite paths and
`if: needs.changes.outputs.ios == 'true'` on `ios-tests`.

## PR scopes

| Output | True when the PR diff touches… | Heavy job |
| --- | --- | --- |
| `validate` | `src-tauri/`, `synara/`, `scripts/`, root package lock/config, `ci.yml` | Validate desktop and runtime |
| `ios` | *paused 2026-08-17 — always `false` until main is stable* | iOS simulator tests (skipped) |
| `synapse_native_*` | `src-tauri/src/matrix/**`, `synara/`, `scripts/synapse-integration.sh`, package lock, `ci.yml` | Native Matrix Rust synapse proofs (reactions/attachments/call/polls/rich-messages/threads/**receipts**) |

> **Retired 2026-08-09:** the legacy `synapse` two-client **js-sdk** integration
> (`run-synapse-two-client-integration.mjs`) was removed with the complete
> `matrix-js-sdk` removal (V-BURN). Native proofs cover the shipped client.

Examples:

| PR content | validate | ios | synapse_native_* |
| --- | --- | --- | --- |
| `docs/matrix-rust-sdk/PROGRESS.md` only | skip | skip | skip |
| `src-tauri/src/matrix/**` only | run | skip | run (rust path) |
| `synara-ios/**` only | skip | skip (paused) | skip |
| `synara/` frontend only | run | skip | run |
| Workflow `ci.yml` change | run | skip (paused) | run |

## Quality gate

`Quality gate` still aggregates results. Acceptable results per heavy job:
`success` or `skipped`. Fail on `failure` / `cancelled` / missing.

## Package smoke (2026-07-28 — D0 policy)

Required check name remains **Desktop package gate** (must always report).

| Event | Heavy package jobs (deb / Arch / macOS) |
| --- | --- |
| PR → `feature/matrix-rust-sdk-full-replacement` | **Skipped by default** (gate succeeds as “not required”) |
| Same PR with label **`needs-package`** | **Run** full smoke |
| PR → `main` / `release/**` | Run when package-sensitive paths change (unchanged) |
| `workflow_dispatch` | **Always run** full smoke (“when ready”) |
| Push `release/**` | Run (workflow path filters) |

Rationale: during partial-path rewire, packaging every `src-tauri` PR was ~15–20m of
wall clock with little signal. Validate + Quality gate remain the merge bar for
integration. Run a full package smoke manually before release-candidate builds / main.

## Residual cost

A `src-tauri`-only integration PR still runs **Validate** (~15–20m). Package
smoke no longer doubles that by default.

Push and pull_request for the same branch share one cancellable concurrency
lane. Version/changelog/release-note diffs are **metadata-only** and skip iOS,
Synapse proofs, Rust tests, and package smoke.

CI iOS simulator jobs build a fat simulator XCFramework slice only. TestFlight
builds the device slice. The four-architecture XCFramework remains the default
local/full generate.

## Validate Rust cache

The **Validate desktop and runtime** job restores/saves `src-tauri` Cargo
artifacts via `Swatinem/rust-cache` (`shared-key: validate-desktop-runtime`).
This does not change which checks run or what success means; it only shortens
cold `cargo fmt` / `clippy` / `check` / `test` wall time on successive matrix
integration PRs (including V-SEND.2 / V-TIMELINE candidates waiting on CI).
