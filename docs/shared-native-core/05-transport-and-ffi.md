# 05 — Transport & FFI

## 5.1 One protocol, two carriers

The `src-tauri/src/matrix/ipc/` module is transport-agnostic already (envelope,
stream topics, wire counter, generation checks). It becomes the **public
transport API of `synara-core`**, carried differently on each platform:

| Carrier | Desktop | iOS |
|---|---|---|
| Command path | `#[tauri::command]` fns → `Core::command(envelope)` | Swift async methods over **uniffi** → same `Core::command(envelope)` |
| Product events | owner emit callback → desktop `app.emit(existing_name)` | future UniFFI callback on the same owner (not `Platform::emit`) |
| IPC envelope stream | `Platform::emit` (typed envelopes only) | uniffi **callback interface** `Platform` when that stream is migrated |
| Ordering | existing wire counter | same counter enforced by the iOS adapter |

## 5.2 Command surface

- Keep the **exact command names** the React layer already uses
  (`matrix_*`). **Frontend API compatibility is a hard invariant.**
- The census in `crates/synara-core/src/transport/census.rs` is the full
  React/Tauri list. As of #928, **111** names are registered on
  `Core::command`. The other census names fail closed. Twenty-one of
  those leftovers are **intended shell** (passwords, `client_secret`,
  Keyring logout/restore, passphrases, file paths, attachment/media
  bytes). Do not register them without a new owner decision.
- A command registry (`transport::Command` → handler) grows **only as
  capabilities land** (owner decision 6). It is not a race to 144.

P4 UniFFI still exposes only the scaffold, `login_flows`, session
projection, and two pure helpers. Serial expansion is
[11-implementer-playbook.md](11-implementer-playbook.md) §9. Disk must
be ≥ 20 Gi before bindgen.

## 5.3 uniffi bindings (iOS, P4)

- Add `crates/synara-core` uniffi scaffolding (`uniffi = "0.2x"`,
  `[lib] crate-type = ["staticlib", "cdylib", "lib"]`, `uniffi-bindgen` via a
  `xtask` or `build.rs`), generating Swift for `aarch64-apple-ios` +
  `aarch64-apple-ios-sim` (+ macOS for tests).
- The generated `SynaraCore` Swift module exposes:
  - `Core` (async `command`, `open`, `close`, `new(platform:)`)
  - `PlatformCallbackInterface` (Swift implements the sink)
  - `dto`/`transport` structs (envelopes, topics, counters)
- This mirrors how `matrix-org/matrix-rust-components-swift` wraps
  `matrix-sdk`; we ship **project-owned bindings** instead of the prebuilt
  package, so the matrix-sdk version comes from `synara-core`'s pin.
  The dependency on `matrix-rust-components-swift` can be dropped once P4
  lands (the NSE and UI both use `SynaraCore`).

### Async + streams notes
- matrix-sdk's own async client methods already cross uniffi in the upstream
  Swift wrapper; the app-level async handlers (session restore, send queue,
  timeline pagination) use the same pattern: `async fn` with
  `Send + 'static` futures, no blocking-in-FFI, and the core's tokio runtime
  driven from the iOS app (or a background task from the NSE for store reads).
- Large stream payloads (room list snapshots, timeline deltas) cross the FFI
  as owned `Vec<u8>`/structured arrays produced by `stream_body.rs` and are
  decoded Swift-side into Combine publishers; the same derive/encoding is
  already exercised by the desktop `contract_tests.rs`.

## 5.4 Notification Service Extension (NSE) constraints

- Never boot the full sync engine in `SynaraNotificationService`
  (short-lived process, memory budget).
- Give the NSE a **narrow read-only store API**: open the persisted
  `synara-core` store, decrypt event bodies (crypto state loaded read-only),
  and render the notification preview using `SynaraNotificationPreviewSupport.swift`.
- This is the one place where a small **second** binding target
  (`synara-core-nse`) is acceptable if the main bindings pull in the runtime;
  keep both built from the same crate so the store format is identical.

## 5.5 Version-pinning policy

- `matrix-sdk = "=0.18.0"`, `matrix-sdk-ui = "=0.18.0"`,
  `matrix-sdk-crypto = "=0.18.0"` move into `synara-core/Cargo.toml`.
- `src-tauri` re-exports them from the core (no separate pins).
- Feature set is centralized: `sqlite`, `markdown`, `qrcode`, e2e (via ui).
- Any future SDK bump is a single-crate change, tested once against the shared
  suite.
