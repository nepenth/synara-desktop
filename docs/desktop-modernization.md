# Desktop Platform Integration

Reviewed: 2026-08-18

The macOS and Linux clients combine the shared Rust Matrix/application core, a
Tauri 2 platform shell, and the embedded React presenter. This document covers
the native desktop integration boundary.

## Ownership

The Tauri shell owns:

- windows, application lifecycle, tray and native menus;
- global shortcuts and desktop-environment diagnostics;
- native notifications, click routing, and dock/taskbar badges;
- Keychain or Secret Service session persistence;
- external URLs, clipboard/file access, downloads, and native file drops;
- platform-safe agent actions;
- updater checks, install/relaunch on macOS, and Linux package guidance.

The shared core owns Matrix state and behavior. React owns presentation,
routing, composer state, and timeline virtualization. The desktop shell must not
keep a second copy of room, notification, Later, or agent workflow state.

## Bridge

Tauri exposes a narrow bridge to the embedded runtime. Product components use
the facades under `synara/src/app/platform/` and `synara/src/app/matrix/`
instead of calling Tauri directly. Rust adapters under `src-tauri/src/bridge/`
delegate Matrix work to `synara-core`; `desktop_*` modules implement
operating-system behavior.

All external URLs, notification routes, filenames, session envelopes, and agent
payloads are validated again at the native boundary. Release builds deny
developer tools and expose build identity through supported diagnostic or About
surfaces.

## User-Facing Integration

- Closing the main window hides it while the tray remains available.
- Tray and app-menu actions open the main window or supported Synara routes.
- Notification clicks activate the app and navigate only to sanitized internal
  destinations.
- Badge counts mirror current application summaries supplied through the typed
  bridge.
- Global-shortcut registration reports unsupported or permission-denied states
  instead of silently failing.
- macOS can install signed updates and relaunch through the Tauri updater;
  Linux informs users of package updates but leaves installation to the package
  manager.

Exact shortcut defaults and available tray items are source- and
platform-dependent. Treat settings and the running app as authoritative rather
than duplicating a key list in documentation.

## Validation

Automated boundary and shell checks:

```sh
npm run check:matrix-boundaries
npm run check:quality-gates
npm --prefix synara run typecheck
npm --prefix synara run test:modernization
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

Native behavior still requires installed-package smoke on each target operating
system. Use [the production smoke checklist](production-smoke-checklist.md) and
[build and release runbook](build-and-release.md).
