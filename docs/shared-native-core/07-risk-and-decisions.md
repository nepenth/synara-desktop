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
| 2026-08-13 | Owner 1B–5B, 7B–13, judgement 6 (see playbook §4) | `matrix_sdk` in Core; no Tauri/`keyring`; live Client + owners in Core; register only as capabilities land; serial iOS; feature-branch-only; no release; Apple proof operator-gated |
| 2026-08-13 | Product events use owner emit callbacks, not `Platform::emit` | `Platform::emit` is the IPC envelope stream; wrapping product events would break React names |
| 2026-08-13 | 21 census names stay desktop | Passwords, `client_secret`, Keyring logout/restore, passphrases, file paths, attachment/media bytes must not cross the 1 MiB Core envelope |
| 2026-08-13 | Next lane is serial iOS (P4), not speculative 7B | Attached-owner writes that can land without secrets/bytes are exhausted as of #928 |

## Closed for implementers (do not re-open in a slice)

- Full `Platform` for dialogs/spellcheck on iOS: **no** until a SwiftUI
  feature demands it. Reactive subset first (playbook P4-S2).
- Heavy OS jobs (file pickers, 32 MiB attachments): **stay in shells**.
- NSE split crate: **defer** until APNs work (P4-S11).
- Routing leftover secrets/bytes: **forbidden** without a new owner
  decision.

## Open questions

- Should `synara-core` publish a crate semver separate from app 2.0.x?
  (Leaning: yes, later. Not a P4 blocker.)
- NSE binding target: single `synara-core` vs `synara-core-nse` when
  P4-S11 starts.
