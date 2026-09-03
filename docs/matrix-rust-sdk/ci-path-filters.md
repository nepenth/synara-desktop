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

iOS is split into two jobs so the PR merge bar is not the hour-long UI +
device-Release proof:

| Output | When it is true | Job |
| --- | --- | --- |
| `ios` | UniFFI / `synara-ios` / Apple generate paths change | **iOS simulator tests** — arm64 sim slice + `SynaraTests` only |
| `ios_ui` | `ios` is true **and** the event is a `main`/`release/**` push, nightly schedule, `workflow_dispatch`, or a PR labeled **`needs-ios-ui`** | **iOS simulator UI tests** — `SynaraUITests`, two simulators |

Device Release / NSE archive proof stays on `release.yml` TestFlight upload.
`ios-skeleton.yml` remains a manual unsigned simulator diagnostic.

## PR scopes

| Output | True when the PR diff touches… | Heavy job |
| --- | --- | --- |
| `validate` | `src-tauri/`, `synara/`, `scripts/`, root package lock/config, `ci.yml` | Validate desktop and runtime |
| `ios` | UniFFI / `synara-ios` / Apple generate paths | iOS simulator unit tests (`SynaraTests`, arm64) |
| `ios_ui` | `ios` plus main/release push, nightly, dispatch, or label `needs-ios-ui` | iOS simulator UI tests (`SynaraUITests`) |
| `synapse_native_*` | `src-tauri/src/matrix/**`, `synara/`, `scripts/synapse-integration.sh`, package lock, `ci.yml` | Native Matrix Rust synapse proofs (reactions/attachments/call/polls/rich-messages/threads/**receipts**) |

> **Retired 2026-08-09:** the legacy `synapse` two-client **js-sdk** integration
> (`run-synapse-two-client-integration.mjs`) was removed with the complete
> `matrix-js-sdk` removal (V-BURN). Native proofs cover the shipped client.

Examples:

| PR content | validate | ios | ios_ui | synapse_native_* |
| --- | --- | --- | --- | --- |
| `docs/matrix-rust-sdk/PROGRESS.md` only | skip | skip | skip | skip |
| `src-tauri/src/matrix/**` only | run | skip | skip | run (rust path) |
| `synara-ios/**` only | skip | run | skip unless `needs-ios-ui` | skip |
| `synara/` frontend only | run | skip | skip | run |
| Workflow `ci.yml` change | run | run | skip unless `needs-ios-ui` | run |

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

CI iOS simulator jobs build the **arm64** simulator XCFramework slice only.
TestFlight builds the device slice. Local `SYNARA_CORE_APPLE_SLICES=simulator`
still produces the fat Intel+ARM slice; `all` remains the default full generate.

## Validate Rust cache

The **Validate desktop and runtime** job restores/saves `src-tauri` Cargo
artifacts via `Swatinem/rust-cache` (`shared-key: validate-desktop-runtime`).
This does not change which checks run or what success means; it only shortens
cold `cargo fmt` / `clippy` / `check` / `test` wall time on successive matrix
integration PRs (including V-SEND.2 / V-TIMELINE candidates waiting on CI).
