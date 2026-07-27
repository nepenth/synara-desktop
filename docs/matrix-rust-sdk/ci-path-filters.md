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

Push to `main` / `release/**` and `workflow_dispatch` always run the **full** suite.

## PR scopes

| Output | True when the PR diff touches… | Heavy job |
| --- | --- | --- |
| `validate` | `src-tauri/`, `synara/`, `scripts/`, root package lock/config, `ci.yml` | Validate desktop and runtime |
| `ios` | `synara-ios/`, `scripts/ci-build.sh`, `ci.yml` | iOS simulator tests |
| `synapse` | `synara/`, `scripts/synapse-integration.sh`, selected scripts tests, package lock, `ci.yml` | Synapse two-client integration |

Examples:

| PR content | validate | ios | synapse |
| --- | --- | --- | --- |
| `docs/matrix-rust-sdk/PROGRESS.md` only | skip | skip | skip |
| `src-tauri/src/matrix/**` only | run | skip | skip |
| `synara-ios/**` only | skip | run | skip |
| `synara/` frontend only | run | skip | run |
| Workflow `ci.yml` change | run | run | run |

## Quality gate

`Quality gate` still aggregates results. Acceptable results per heavy job:
`success` or `skipped`. Fail on `failure` / `cancelled` / missing.

## Package smoke

`desktop-package-smoke.yml` already uses a diff-based gate: heavy package jobs
run only when package-sensitive paths change. The lightweight detection job still
runs on every PR so required “Desktop package gate” checks always report
(success with “not required”). We intentionally do **not** workflow-level-skip
this file for docs PRs, to avoid hanging required checks.

## Residual cost

A `src-tauri`-only PR still runs the full **Validate** job (Rust + frontend +
script tests) because that job is one monolithic suite today. Further splitting
Validate (Rust vs frontend vs scripts) is a follow-up optimization.
