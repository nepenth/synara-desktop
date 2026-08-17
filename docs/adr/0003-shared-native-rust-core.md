# ADR 0003: Shared native core (synara-core) for desktop + iOS

Status: accepted and in implementation (2026-08-10; progress recorded 2026-08-16; SNC engineering on `main` via #991).
Supersedes the separate Swift service-layer direction implied by ADR-0002 for
app-logic ownership (ADR-0002's native-SwiftUI UI decision is retained).

**End state:** one Rust core (`synara-core`) that both the desktop Tauri
app (macOS and Linux) and the iOS app consume, so sync, room list,
timeline, and crypto are not implemented twice. **That end state has not
been reached.** SNC engineering is on `main` via #991
(`05a0961c`). It is not a release. How to finish it:
`docs/shared-native-core/11-implementer-playbook.md`.
What may be written in Rust, and what must stay put:
[ADR 0004](0004-rust-language-boundaries.md).

## Context

- Desktop: `src-tauri/src/matrix/` holds ~285 Rust files implementing the entire
  app-logic layer (sync, room list, timeline, crypto, send, media, notifications,
  receipts, unread, typing, polls, threads, spaces, search, auth, lifecycle,
  DTOs, and an `ipc/` transport protocol). Only 21 `#[tauri::command]` fns and
  27 `tauri/AppHandle` references couple it to Tauri.
- iOS: `synara-ios` is a native SwiftUI app that currently re-implements client
  orchestration over the official `MatrixRustSDK` Swift package
  (matrix-rust-components-swift, pinned 26.06.06). Crypto/sync/timeline logic is
  duplicated in Swift (MatrixRustSDKService, RoomListService, TimelineService).
- Consequence: two independent implementations of the same app logic drift; the
  heavily-tested desktop engine (800+ Rust tests + Synapse proofs) is not reused
  by iOS.

## Decision

Introduce a workspace crate **`synara-core`** that owns the entire
transport-agnostic app-logic layer currently in `src-tauri/src/matrix/` plus
`tasks/`, `dto/`, and `ipc/`. It depends only on `matrix-sdk` (pinned 0.18) and
generic Rust.

Two thin adapters consume it:

1. **Desktop (src-tauri)**: the existing `#[tauri::command]` surface calls
   `synara-core` in-process; `desktop_*` modules implement the `platform`
   sink trait (keychain/secret store, native notifications, tray/badge,
   dialogs, spellcheck, shortcuts, updater metadata).
2. **iOS (synara-ios)**: `synara-core` is exported via **uniffi** to generate
   Swift bindings (same technique as matrix-rust-components-swift); the SwiftUI
   app becomes a thin UI + adapter over the shared core. Ship project-owned
   bindings instead of the prebuilt matrix-rust-components-swift package; the
   notification service extension uses a narrow read-only store-access surface
   (never boots the full sync engine).

Non-goals (kept platform-side): UI/UX (React vs SwiftUI), OS integrations
(APNs vs native tray/desktop notifications), credential stores, file dialogs,
app lifecycle, settings/config UI.

## Consequences

- One logic source for both platforms; parity by construction.
- One test suite (Rust unit + integration + Synapse proofs) gates both.
- iOS feature delivery reuses the proven desktop engine; TestFlight gates
  (crypto completion, physical-device, APNs) target one engine.
- Costs: FFI design around async/streams (uniffi async proven by matrix-org);
  version pinning unified inside synara-core; NSE store-access constraints;
  a phased migration that must never break desktop CI.

## Phase plan (tracked in docs/shared-native-core/PLAN.md)

- P0 — this ADR + plan doc + module-boundary census (DONE).
- P1 — crate extraction: move matrix/tasks/dto/ipc into `crates/synara-core`
  (no behavior change); introduce `platform` trait; desktop CI stays green.
- P2 — native transport API: formalize commands + event streams (envelope,
  wire counter) as the core public surface; observer/sink abstraction.
- P3 — desktop adapter swap: src-tauri becomes a thin shell over synara-core.
- P4 — uniffi bindings for iOS targets + Swift `SynaraCore` adapter; replace
  MatrixRustSDKService/RoomListService/TimelineService.
- P5 — iOS parity + release gates: shared-core full matrix, iOS simulator,
  physical-device, APNs, TestFlight, production E2EE completion.
