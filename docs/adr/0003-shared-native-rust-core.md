# ADR 0003: Shared Native Rust Core for Desktop and iOS

Originally accepted: 2026-08-10.

Last reviewed: 2026-09-01.

Status: accepted; architectural source shape implemented on `main`.

This ADR supersedes ADR 0002 only where ADR 0002 implied an independent Swift
Matrix/application service layer. ADR 0002's native SwiftUI and Apple-platform
decisions remain accepted.

## Context

The desktop and iOS clients need identical Matrix and Synara product semantics.
Independent Rust desktop and Swift iOS orchestration had duplicated lifecycle,
room, timeline, crypto, and policy decisions. That duplication made behavioral
parity dependent on two implementations converging after every change.

Decision-time source counts and migration phase numbers are preserved in
`docs/shared-native-core/`; they are historical evidence, not current inventory
or release acceptance.

## Decision

Maintain one workspace crate, `crates/synara-core`, as the shared
transport-independent Matrix/application authority for all supported clients.
It depends on `matrix-rust-sdk` and exposes typed operations, models, and event
streams through thin platform adapters:

1. **Desktop:** `src-tauri/` calls Core in-process and owns macOS/Linux shell,
   credential, notification, window, file, and byte-transfer integrations.
   `synara/` owns React presentation and composer/viewport behavior.
2. **iOS:** generated project-owned UniFFI bindings expose Core to SwiftUI.
   Swift owns Apple UI and services. The notification extension receives only
   its deliberately narrow store/preview surface and never starts full sync.

There must be no JavaScript or Swift Matrix engine competing with Core for
session, sync, crypto, room, timeline, account-data, or Matrix-write authority.

## Durable invariants

- One Core authority and one concurrency owner for shared Matrix behavior.
- Platform bridges stay thin and typed; presentation projections are not second
  domain owners.
- UI, OS integrations, credential stores, file dialogs, and lifecycle
  observations remain platform-owned.
- Shared behavior is tested in Rust and through cross-language contract/live
  proofs; platform behavior is also validated in its native environment.
- Release, CI, physical-device, APNs, and live-homeserver gates are tracked by
  current operational documents. The architectural source shape does not by
  itself claim every release gate complete.

## Current evidence

- `crates/synara-core/` contains shared lifecycle and product domains over
  matrix-sdk 0.18.
- `src-tauri/Cargo.toml` depends on the local Core crate.
- `crates/synara-core-bindgen/` generates the iOS binding package/XCFramework.
- Product Swift services import `SynaraCore`; direct `MatrixRustSDK` source
  imports are confined to the historical feasibility spike.
- The consolidated implementation proof is recorded in the
  [2026-08-17 local proof](../shared-native-core/15-2026-08-17-local-proof.md).

## Consequences

- Shared protocol and product behavior has one implementation and test owner.
- New iOS functionality must consume or extend Core instead of rebuilding
  Matrix state machines in Swift.
- New desktop functionality must not bypass Core with a JavaScript Matrix
  client.
- FFI/IPC schema design, cancellation, streaming, versioning, and platform
  lifecycle adapters remain real costs and must be justified per boundary.
- [ADR 0004](0004-rust-language-boundaries.md) decides what belongs in Core;
  [ADR 0005](0005-native-media-handle-channel.md) defines the media byte path.

## Historical implementation phases

The P0–P5 plan—crate extraction, transport API, desktop adapter cutover,
UniFFI adoption, and iOS release proof—explains how this decision was pursued.
It is not an evergreen queue. Current status and stop conditions live in the
[shared-Core program documentation](../shared-native-core/README.md), not in
this ADR.
