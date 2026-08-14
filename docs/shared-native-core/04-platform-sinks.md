# 04 — Platform Sinks

The engine needs a small, stereotyped set of OS services. This document turns
the 38 `AppHandle`/`emit` references (census §2.2) and the desktop `desktop_*`
modules into one trait + two implementations.

## 4.1 Trait (in `synara-core`)

```rust
pub trait Platform: Send + Sync + 'static {
    /// Push a protocol envelope onto the UI stream (sic the ipc protocol).
    fn emit(&self, envelope: transport::Envelope) -> Result<(), transport::Error>;
    /// Key-value secret vault (session material, keys).
    fn secret_store(&self) -> Arc<dyn SecretVault + Send + Sync>;
    /// Deliver a native notification (tray/toast on desktop, APNs/badge on iOS).
    fn notify(&self, n: dto::Notification) -> Result<(), transport::Error>;
    /// App icon badge count (dock/taskbar/today on iOS).
    fn set_badge(&self, count: u64) -> Result<(), transport::Error>;
    /// Broadcast engine status (health/readiness) to the OS layer.
    fn status(&self, s: dto::PlatformStatus) -> Result<(), transport::Error>;
    // Optional-but-likely needed: open URL, request file open/save, request
    // permission (notifications), spellcheck, window/tray actions.
}
```

No default implementations in the crate — every shell provides all needed
methods (fail-closed defaults where acceptable, e.g. `set_badge` no-op on
unsupported OS).

## 4.2 Mapping today's desktop `AppHandle` uses → sink methods

Measured seams and their current homes:

| Current use in `src-tauri/src/matrix/` | Sink method | Files with the pattern |
|---|---|---|
| `app.emit(<topic>, <payload>)` — push stream updates to renderer | `emit` | `timeline/live.rs`, `room_profile/live.rs`, `presence/live.rs`, `devices/live.rs`, `account_data/image_packs.rs`, `timeline/product_commands.rs`, `auth/product.rs`, ...
| `app.emit` for login/readiness status | `status` | `auth/product_commands.rs` (12 refs — mostly auth/session events) |
| window/tray/badge orchestration | `set_badge`, `status`, dedicated sink | `room_list/live.rs`, `room_ops/product_commands.rs` (badge counts) |

**Correction (landed practice through #816 / #928):** product event
owners do **not** call `platform.emit`. They take a shell emit callback
at construct time. Desktop adapters keep the existing Tauri event names
(`timeline/live.rs`, `presence/live.rs`, `devices/live.rs`,
`room_profile/live.rs`, image packs, verification). `Platform::emit` is
reserved for typed IPC envelopes. Expanding `Platform` to carry product
events requires a new ADR. See the implementer playbook §3 rule 7.

## 4.3 Desktop implementation (stays in src-tauri)

- `desktop_integration.rs` / `main.rs` construct `Platform` impl and hand an
  `Arc<dyn Platform>` plus the Tauri `AppHandle` into `Core::new(...)`.
- `emit` → wraps the `AppHandle` and calls the existing
  `app.emit(topic, body)` (renderer-facing names unchanged).
- `secret_store` → wraps `desktop_secret_store.rs`
  (platform keychain: macOS Keychain, Linux Secret Service; Windows fallback).
- `notify` → wraps `desktop_notifications.rs` (+ the existing notification
  read model in `matrix/notifications/`).
- `set_badge` → `desktop_tray.rs` / `desktop_integration.rs` dock/taskbar badge.
- file/dialog/shortcut/spellcheck remain `desktop_file_transfer.rs`,
  `desktop_shortcuts.rs`, `desktop_spellcheck.rs` behind the (optional) sink
  slots or invoked directly by the shell (not via the engine).

## 4.4 iOS implementation (new, P4)

- A `SynaraPlatform` Swift class conforms to the uniffi callback interface
  `Platform` and forwards:
  - `emit` → converts the `transport::Envelope` into the existing SwiftUI
    publish/bus mechanism (reuse `SynaraContracts.success/Spective`
    event vocabulary where they exist).
  - `secret_store` → `SecureSessionStore.swift` (Keychain).
  - `notify` → `NotificationPermissionCoordinator.swift` +
    `PushService.swift` (local notification / APNs).
  - `set_badge` → `UIApplication.shared.applicationIconBadgeNumber`.
  - `status` → `SignedInSessionReadiness.swift` / `AppEnvironment`.
- The NSE (`SynaraNotificationService/NotificationService.swift`) does NOT
  install a full `Platform`; it uses the narrow read-only store API (§5.4).

## 4.5 Sink contracts worth codifying before P2

- Payload types must be **SDK-neutral DTOs** (`matrix/dto/*`), never raw
  ruma/`matrix-sdk` types across the sink.
- Emission must be **ordered and coalesced per stream topic** (the existing
  `wire_counter` in `ipc/wire_counter.rs` already guarantees this on desktop —
  the iOS adapter must preserve the same ordering guarantees).
