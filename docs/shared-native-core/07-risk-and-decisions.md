# 07 — Risk Register & Decision Log

## Risk register

| # | Risk | L | I | Mitigation |
|---|---|---|---|---|
| R1 | Extraction churn breaks desktop CI / renderer contract | H | H | Pure `git mv` slices; renderer-facing names invariant; full matrix per slice |
| R2 | uniffi async/stream divergence from Tauri event semantics (ordering, gap/dup) | M | H | Reuse `ipc/` wire-counter + protocol helpers on both carriers; contract tests cover ordering |
| R3 | NSE memory/time budget blown by core bindings | M | M | Narrow read-only store API; separate `-nse` bindings target if needed; never boot sync engine in NSE |
| R4 | Version pin drift between desktop (0.18) and iOS (upstream components 26.06.06 pins its own sdk) | M | M | Central pin in `synara-core`; drop the upstream Swift package at P4 so one version governs both |
| R5 | Crypto behaviors (SAS, backup, UTD recovery) regress subtly on iOS during adapter swap | M | H | Crypto already lives in Rust (crypto_store/verification/backup/utd_recovery); iOS delegates replaced by core supervisors; Synapse proofs + iOS e2ee-validation runbook |
| R6 | Two-workspace confusion (src-tauri + crates) churns CI caching | L | M | Workspace single `Cargo.lock`; keep `--locked` CI as-is; cache keyed on workspace lock |
| R7 | Scope creep (UI unification, rewrite of realization) | H | M | Non-goals pinned in `01-context-and-goals.md`; parity matrix keeps UI separate |

L = likelihood, I = impact (H/M/L).

## Decision log

| Date | Decision | Rationale |
|---|---|---|
| 2026-08-10 | Adopt ADR-0003 shared native core | Ends dual logic implementation; reuse heavily-tested engine |
| 2026-08-10 | Keep ADR-0002's native-SwiftUI UI decision | UI stays platform-owned; only logic unifies |
| 2026-08-10 | Ship project-owned uniffi bindings (drop `matrix-rust-components-swift`) | One version pin, one code path, full control of exposed surface |

## Open questions

- Should `synara-core` publish a brand (v0.1.0) version gate in P1, or align to
  the app 2.0.x from day one? (Leaning: separate semver for the library crate.)
- Does iOS need the full `Platform` sink for dialogs/spellcheck in P4, or only
  the reactive subset (emit/status/badge/secret/notify)? (Leaning: reactive
  subset first; extend when the SwiftUI feature set demands.)
- Keep `tasks/registry` moving into the core, or leave heavy background jobs
  (media export) in the shells? (Leaning: move registry, keep heavyweight OS
  jobs in shells via the sink.)
- NSE binding target: single `synara-core` vs `synara-core-nse` split at P4 vs
  defer until APNs work begins?
