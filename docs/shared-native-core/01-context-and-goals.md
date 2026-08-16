# 01 — Context & Goals

## Problem statement

Synara ships two independent native clients with two independent copies of the
same application logic:

- **Desktop** (`src-tauri/`): a Rust application-logic layer
  (`src-tauri/src/matrix/`, ~285 files) behind a Tauri command surface, with a
  React/Vite renderer (`synara/`).
- **iOS** (`synara-ios/`): a native SwiftUI app that re-implements client
  orchestration over the official Swift bindings
  (`MatrixRustSDK` from `matrix-org/matrix-rust-components-swift`, pinned
  `26.06.06`), e.g. `Synara/Services/MatrixRustSDKService.swift`,
  `Synara/Services/RoomListService.swift`, `Synara/Services/TimelineService.swift`.

Consequences:

1. **Logic drift** — fixes and features implemented once in the Rust engine
   (e.g. sync resilience, timeline projection, receipt/typing/unread handling,
   crypto workflows, UTD recovery) are not automatically present on iOS.
2. **Divided quality** — the desktop engine is locked by 800+ Rust tests and 6
   Synapse integration proofs; the Swift re-implementations are not.
3. **Divided maintenance** — crypto, session restore, and sync semantics are
   subtle; maintaining two implementations multiplies the risk surface.

## Why now (2026-08)

- The desktop js-sdk → rust-sdk replacement is **complete** (0 importers) and
  the engine is the de-facto reference implementation.
- Pre-release for both platforms — the cheapest moment to unify before iOS
  accrues more bespoke logic.
- The FFI technique is proven: `matrix-rust-components-swift` already generates
  Swift bindings from the identical `matrix-sdk` crate tree, including async
  client methods and event streams.

## Goals

1. **One application-logic core** (`crates/synara-core`) owned by the Rust
   domain modules below `src-tauri/src/matrix/`, `src-tauri/src/tasks/`,
   `src-tauri/src/matrix/dto/`, and `src-tauri/src/matrix/ipc/`.
2. **Zero Tauri/OS types in `synara-core`** — platform concerns enter only
   through a small `Platform` sink trait (`04-platform-sinks.md`).
3. **Both platforms consume the core**:
   - Desktop: in-process crate call (the current behavior after the adapter
     swap, P3).
   - iOS: **uniffi-generated Swift bindings** to the same crate (P4).
4. **One test suite gates both**: existing Rust unit/integration tests +
   Synapse native proofs run against the shared core once, and the iOS
   simulator job runs against the same code path.
5. **No behavior change during extraction** — desktop CI stays green at every
   slice (same methodology as the js→rust burn-down).

## Non-goals (keep platform-side)

- UI/UX (React for desktop, SwiftUI for iOS).
- OS integrations: notifications (native tray/desktop vs APNs+push), credential
  stores (desktop secret store vs iOS Keychain), tray/badge, dialogs,
  spellcheck, global shortcuts, updater metadata.
- Application lifecycle and settings/config UI.
- Re-writing either frontend to match the other.

## End state vs today

**End state:** one Rust core (`crates/synara-core`) that both the desktop
Tauri app (macOS and Linux) and the iOS app consume, so sync, room list,
timeline, and crypto are not implemented twice.

**Today:** that end state has not been reached. Desktop already drives
most product Matrix I/O through Core (`Core::command`, 111 registered
names). iOS product `MatrixRustSDK` callers are retired via leftover
UniFFI (#986); leftover I/O fail-closes without a live homeserver and
does not start SyncService. This is not iOS-on-engine. SNC engineering
is on `main` via #991. It is not a release.

How to implement the remaining gap without confusion:
[11-implementer-playbook.md](11-implementer-playbook.md).

## Success criteria

These are **program-done** checks. None of them is currently true as a
set. Do not tick them from a single PR.

- `cargo test` for `synara-core` passes with the full existing matrix suite
  (when disk policy allows cargo).
- Desktop build/test/package CI green at tip throughout P1–P3.
- iOS simulator build+tests (ci.yml `ios-tests`, `ios-skeleton.yml`) green
  against the shared core in P4+.
- `grep -rn MatrixRustSDKService/RoomListService/TimelineService` in `synara-ios`
  returns zero hits after P4 (supporting service names may live on in a thin
  adapter layer).
- A single feature toggle/bugfix authored once in `synara-core` lands on both
  platforms (demonstrated by at least one post-P4 change). **Not claimed.**
- SNC engineering merged to `main` in #991 (`05a0961c`). That merge is not
  program-done, P4 acceptance, or a release. Release remains a later,
  separate gate.
