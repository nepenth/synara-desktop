# Shared Native Core (synara-core) — Program

A plan to unify the **desktop** (Tauri) and **iOS** (SwiftUI) clients of Synara
Desktop onto **one transport-agnostic Rust application-logic core** —
`crates/synara-core` — consumed by both platforms.

| | |
|---|---|
| Owner | Synara engineering |
| Status | P0 — ADR written and this plan authored. P1–P5 not started |
| Decision | [ADR 0003](../adr/0003-shared-native-rust-core.md) |
| Program ledger | `docs/shared-native-core/` (this directory) |
| Related ADRs | [0001](../adr/0001-ios-repository-layout.md), [0002](../adr/0002-ios-architecture.md) |

## TL;DR

- The desktop app already contains ~285 Rust source files of pure
  application logic (`src-tauri/src/matrix/`), with only a thin Tauri seam:
  **144** `#[tauri::command]` fns and **38** `AppHandle`/emit references.
  An internal transport protocol already exists (`src-tauri/src/matrix/ipc/`).
- The iOS app is a **separate native SwiftUI implementation** that
  re-implements that same logic over the official Swift wrapper
  (`MatrixRustSDK` from `matrix-org/matrix-rust-components-swift`).
  The two implementations drift; the desktop engine is far more mature and
  heavily tested (800+ Rust tests, six Synapse integration proofs).
- **This program** extracts the desktop logic into a shared `synara-core`
  crate with **no behavior change** (P1), formalizes its native transport API
  (P2), swaps the desktop shell onto it (P3), and generates **uniffi Swift
  bindings** so iOS consumes the *same* crate, deleting its Swift
  re-implementations (P4), then closes iOS release gaps against the shared
  engine (P5).

## How to navigate this program

| Doc | Content |
|---|---|
| [`01-context-and-goals.md`](01-context-and-goals.md) | Problem statement, goals, non-goals, success criteria |
| [`02-module-boundary-census.md`](02-module-boundary-census.md) | Exact, file-referenced census of what exists today |
| [`03-target-architecture.md`](03-target-architecture.md) | Target crate layout, adapter model, data flow |
| [`04-platform-sinks.md`](04-platform-sinks.md) | The OS platform seams and how each platform implements them |
| [`05-transport-and-ffi.md`](05-transport-and-ffi.md) | The native transport API (ipc protocol), Tauri + uniffi adapters, NSE constraints |
| [`06-migration-phases.md`](06-migration-phases.md) | P1–P5 phases: PR-level mechanics, acceptance criteria, CI gates |
| [`07-risk-and-decisions.md`](07-risk-and-decisions.md) | Risk register, decision log, open questions |
| [`08-parity-matrix.md`](08-parity-matrix.md) | Capability parity desktop vs iOS, now and after unification |
| [`09-references.md`](09-references.md) | Glossary + concrete file/upstream references |

## Core idea in one sentence

> Put the app logic in a crate with zero Tauri types, let desktop call it
> in-process and iOS call it through generated bindings — so both platforms
> share one engine, one test suite, and one definition of "correct."
