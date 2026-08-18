# 09 — References

## In-repo source of truth

- **Implementer playbook (start here):**
  [`11-implementer-playbook.md`](11-implementer-playbook.md)
- Engine: `crates/synara-core/` plus thin `src-tauri/src/matrix/` re-exports
  (census in [`02-module-boundary-census.md`](02-module-boundary-census.md))
- Transport protocol: `crates/synara-core/src/transport/` (desktop still
  re-exports as `crate::matrix::ipc`)
- DTOs: `crates/synara-core/src/dto/`
- Platform/OS desktop modules: `src-tauri/src/desktop_*.rs`, `main.rs`, `lib.rs`
- Dependency pins: `src-tauri/Cargo.toml`
- iOS SwiftUI app: `synara-ios/Synara/**`, `synara-ios/project.yml`
- iOS NSE: `synara-ios/SynaraNotificationService`, `synara-ios/SynaraShared`
- iOS decisions: `docs/adr/0001-ios-repository-layout.md`,
  `docs/adr/0002-ios-architecture.md`, `docs/adr/0003-shared-native-rust-core.md`
- Rust language boundaries: `docs/adr/0004-rust-language-boundaries.md`
- iOS feasibility spikes: `synara-ios/docs/tauri-ios-feasibility-spike.md`,
  `synara-ios/docs/matrix-sdk-feasibility-spike.md`
- iOS release gaps: `synara-ios/docs/device-readiness.md`,
  `synara-ios/docs/e2ee-validation.md`
- CI: `.github/workflows/ci.yml`, `ios-skeleton.yml`, `desktop-package-smoke.yml`
- Boundary guard: `scripts/check-matrix-boundaries.mjs`
- Renderer facade (desktop event/command vocabulary):
  `synara/src/app/features/native-client/nativeClientFacade.ts`
- App runtime docs: `synara/docs/`, `synara/README.md`
- js→rust program history (kept): `docs/matrix-rust-sdk/`

## Upstream references

- matrix-sdk Rust crate: https://github.com/matrix-org/matrix-rust-sdk
- Swift bindings we currently consume and will replace:
  https://github.com/matrix-org/matrix-rust-components-swift
- uniffi (Mozilla) FFI layer used by the above:
  https://mozilla.github.io/uniffi-rs/
- Element X (matrix-org's own shared-core product): a working reference for a
  shared matrix-rust-sdk core consumed by Swift (iOS) + other platforms.
- SDK capability review + migration tracking (our engine's authoritative
  design record): `docs/matrix-rust-sdk/0.18.0-capability-dossier.json`,
  `docs/matrix-rust-sdk/SCOREBOARD.md`

## Glossary

- **synara-core**: the shared Rust application-logic crate
  (`crates/synara-core`). Desktop consumes it in-process. iOS consumes a
  narrow UniFFI surface today; P4 must widen that until Swift no longer
  re-implements sync/room-list/timeline/crypto.
- **End state:** one core, two shells. Not reached. Feature branch only.
  Not `main`. Not a release.
- **Shell / platform shell**: the thin platform wrapper (src-tauri / synara-ios).
- **Platform sink**: the trait via which the engine talks to the OS/UI.
- **Envelope / stream / wire counter**: `ipc/` protocol concepts (see
  `src-tauri/src/matrix/ipc/` and `05-transport-and-ffi.md`).
- **uniffi**: Mozilla's tool that generates a Swift binding from a Rust crate.
- **NSE**: Notification Service Extension (iOS, `SynaraNotificationService`).
- **UTD**: unable-to-decrypt (crypto recovery term) — `utd_recovery/`.
