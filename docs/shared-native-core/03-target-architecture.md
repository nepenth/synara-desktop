# 03 — Target Architecture

## 3.1 Target crate layout

Introduce a cargo workspace with a shared crate and keep the platform shells:

```text
Cargo.toml                     # workspace root (members: crates/synara-core, src-tauri)
crates/
  synara-core/                 # transport-agnostic application logic (NEW)
    Cargo.toml                 # pins matrix-sdk =0.18.0, matrix-sdk-ui, matrix-sdk-crypto
    src/
      lib.rs                   # public API: commands, events, Platform sink trait, entry points
      app/                     # (moved 1:1) sync, room_list, timeline, ... supervisors
      transport/               # (moved 1:1 from matrix/ipc/) envelope, stream, protocol, wire_counter
      dto/                     # (moved 1:1 from matrix/dto/)
      task/                    # (moved 1:1 from matrix/tasks/)
      platform/                # Platform trait (sink) + zero implementations
      error.rs
src-tauri/                     # desktop shell (slim after P3)
  src/
    main.rs, lib.rs            # builder: invoke_handler over synara-core, tray, window
    desktop_*.rs               # Platform sink impls (keychain, notifications, tray, ...)
    bridge/                    # thin adapters: tauri::command fns as a REGISTRAR over synara-core
synara-ios/                    # iOS shell (slim after P4)
  Synara/                      # SwiftUI UI + SynaraCoreAdapter (uniffi bindings)
  ...                          # push NSE reuses synara-core store-read API via bindings
```

## 3.1a Current vs target (do not confuse them)

**Target** is section 3.2–3.5: both shells are thin adapters over one Core.

**Current (tip `76f67441` on `main`):** desktop already calls `Core::command` for one
hundred eleven names and attaches live owners. iOS has typed SharedCore
wrappers through S9-31 (helper + XCTest), plus credential-free
`login_flows` / `register_flows`, `SharedCore` constructors, optional
`IosSecretVault`, restore, dedicated `login_with_password`, owner
attach, the S11 NSE read-only store helper (#984; never starts
sync; not a product NSE swap), S10 leftover UniFFI (#986), and
P4-S12–S37 product consume (#1001). Desktop JS encrypt/decrypt is
retired (#1006). Product `MatrixRustSDK` callers
are retired (comments may remain). This is not iOS-on-engine. Hosted
iOS CI is paused (#1003). Live homeserver proof is paused. Local
Apple generate has been run on Darwin for the S30–S35 fields
(`dd62d24d`). Generated sources remain gitignored.
Checked-in `SynaraCore.swift` remains the bootstrap stub. This is not
P4 acceptance and not dual-platform proof. Product events on desktop use **owner emit callbacks**, not
`Platform::emit`. `Platform::emit` is the typed IPC envelope stream.
See [11-implementer-playbook.md](11-implementer-playbook.md) §3 rule 7.

## 3.2 Adapter model (what each platform talks to)

```text
              +------------------+        +-------------------+
  React UI -> | Desktop adapter  |        | SwiftUI UI        | <- React UI is not shared
              | (tauri::command  |        | (SynaraCoreAdapter|
              |  registrar +     |        |  = uniffi Swift)  |
              |  events)         |        |                   |
              +--------+---------+        +---------+---------+
                       | ipc protocol               | ipc protocol (same)
                       v                            v
              +-------------------------------------------------+
              |               synara-core (Rust)                |
              |  sync service | room list | timeline | crypto    |
              |  send/media/receipts/typing/unread/polls/...     |
              |  transport (ipc protocol) | dto | task           |
              +--------+----------------------------------+------+
                       | Platform sink trait (keychain, notify,
                       | tray/badge, dialogs, updater, etc.)
              +--------+-------------------+--------------+
              v                            v              v
     desktop_*.rs (Tauri/OS)    iOS Keychain/APNs/      NSE store-read API
                                badge (Swift impls)     (read-only)
```

Key properties:

- `synara-core` **never imports Tauri**, `AppKit`, `UIKit`, or any shell type.
- The renderer/UI on both platforms speaks the **same `ipc/` protocol** — for
  desktop it rides Tauri channels today; on iOS it rides uniffi stream
  callbacks. The envelope/stream/wire-counter DTOs are shared verbatim.
- The `Platform` sink trait is generic over the few OS actions the engine
  needs; both shells implement it and register it once at startup.

## 3.3 Public API of synara-core (shape)

```rust
// crates/synara-core/src/lib.rs (illustrative)
pub mod transport;   // ipc protocol: envelope, stream, counter, version
pub mod dto;         // SDK-neutral DTOs (moved intact)
pub mod app;         // the current matrix/* domain modules
pub mod task;        // task registry/bridge

/// OS seams the engine needs. Implemented by each shell.
pub trait Platform: Send + Sync {
    fn emit(&self, topic: transport::Topic, body: dto::EventBody) 
        -> Result<(), transport::Error>;
    fn secret_store(&self) -> &dyn SecretVault;
    fn notify(&self, event: dto::Notification) -> Result<(), transport::Error>;
    fn badge_count(&self, n: u64) -> Result<(), transport::Error>;
    // dialogs, file dialogs, spellcheck, shortcuts, updater...
}

/// Wire the engine: build once, drive from any shell.
pub struct Core;                                   // session/supervisor actor
impl Core {
    pub fn new(platform: Arc<dyn Platform>) -> Result<Self, error::Error>;
    pub async fn command(&self, c: transport::CommandEnvelope)
        -> Result<transport::ResponseEnvelope, error::Error>;
    pub async fn open(&self, session: dto::SessionMaterial) -> Result<(), error::Error>;
    pub async fn close(&self) -> Result<(), error::Error>;
}
```

The `144` `#[tauri::command]` bodies become `transport::CommandEnvelope`
handlers inside `synara-core`; the desktop registrar keeps its names so the
React side never changes.

## 3.4 Event flow (both platforms)

1. Shell starts → builds `Core` with a `Platform` impl → `Core::open(session)`.
2. `synara-core` supervisors (sync, room list, timeline, crypto) start; each
   writes into the shared in-memory projections. **Product** updates
   (timeline view, presence, devices, join rules, image packs, verification)
   go through the owner’s shell emit callback (`Arc<dyn Fn(Payload)+Send+Sync>`).
   Desktop maps that callback to the existing Tauri event name. iOS will map
   it to a UniFFI callback. **Do not** send those payloads through
   `Platform::emit`.
3. `Platform::emit` remains the typed IPC **envelope** stream (not React
   product events). iOS will implement the same envelope sink later if a
   stream-envelope path is needed.
4. UI commands flow the reverse direction through `Core::command` (async over
   the transport protocol).

## 3.5 What stays behind in each shell

| Concern | Desktop (src-tauri) | iOS (synara-ios) |
|---|---|---|
| UI | React (`synara/`) | SwiftUI (`Synara/Features/*`) |
| Credential store | `desktop_secret_store.rs` (+ platform keychain) | `SecureSessionStore.swift` (Keychain) |
| Notifications | `desktop_notifications.rs` (tray/native) + read model | `PushService.swift` + `NotificationService.swift` (NSE via APNs) |
| Tray/badge/shortcuts | `desktop_tray.rs`, `desktop_shortcuts.rs`, badge via `desktop_integration.rs` | App badge / SF badge |
| Dialogs/files | `desktop_file_transfer.rs`, Tauri dialog plugin | SwiftUI file pickers |
| Logging/telemetry | `desktop_logging.rs`, `diagnostics/desktop_compat.rs` | `AppLogging.swift`, `PerformanceInstrumentation.swift` |
