# Shared Native Core (synara-core) — Program Plan

Status: P0 complete; P1-5 not started. Owner: Synara engineering.

## Goal

One Rust app-logic core (`crates/synara-core`) consumed by both desktop
(Tauri) and iOS (SwiftUI via uniffi). Ends the dual implementation of sync,
room list, timeline, and crypto logic.

## Decision record

- ADR 0003: `docs/adr/0003-shared-native-rust-core.md`

Full docs: see this directory's README and 01-09
(architecture, census, platform sinks, transport/FFI, phases, risk,
parity matrix, references).

## Phases

- [x] P0 — ADR + plan + module-boundary census. Evidence:
  - 285 .rs files under src-tauri/src/matrix/
  - 144 #[tauri::command] fns; 38 AppHandle refs in the matrix layer
  - existing ipc/ protocol (envelope, stream, wire counter, contract tests)
  - iOS currently re-implements over matrix-rust-components-swift 26.06.06
- [ ] P1 — Crate extraction (no behavior change). Move matrix/tasks/dto/ipc
      into crates/synara-core; add `platform` sink trait; desktop CI green end-state.
- [ ] P2 — Native transport API: commands + event-stream protocol from ipc/
      become the core public surface; observer/sink abstraction for both adapters.
- [ ] P3 — Desktop adapter swap: src-tauri thin shell over synara-core.
- [ ] P4 — uniffi bindings for iOS targets + Swift SynaraCore adapter;
      remove direct MatrixRustSDKService/RoomListService/TimelineService usage.
- [ ] P5 — iOS parity + release gates (shared full matrix, iOS sim, device,
      APNs, TestFlight, production E2EE completion).

## Guardrails (from js→rust burn-down methodology)

- Small additive slices, each a PR with green desktop CI (Quality + Desktop
  package + full matrix at tip).
- Worktree isolation; branch base = feature; squashed PRs; provenance anchor.
- Domain modules already carry their own tests; move them intact (no behavior
  change during P1).
- The ipc/ contract tests are the north star for the transport API.
