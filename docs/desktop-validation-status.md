# Desktop Validation Status

Reviewed: 2026-05-26

## Pre-iOS Prep

Status: complete.

The local pre-iOS implementation gate is closed. The runtime now has platform
APIs, native-first session storage, scoped desktop credential commands, shared
settings/platform settings split, and machine-readable shared contracts for:

- agent actions
- agent cards
- Later account data
- media/external URL policy
- notification summaries
- room notes
- room/event/thread anchors
- route paths
- settings compatibility
- space/sidebar folders
- unread anchors

Validation:

- `npm run test:modernization`: 165 tests passing.
- `npm run typecheck:modernization`: passing.
- `npm run check:eslint`: passing.
- `npm run check:prettier`: passing.
- `npm run check:versions`: passing at `1.1.0`.

## macOS

Status: locally build-validated.

Validated from the current `main` branch:

- `cargo check`: passing.
- `npm run tauri build -- --bundles app`: passing.
- Built bundle:
  `src-tauri/target/release/bundle/macos/Synara.app`.
- Bundle version:
  `CFBundleShortVersionString = 1.1.0`,
  `CFBundleVersion = 1.1.0`.
- Bundle identifier:
  `app.synara.desktop`.
- Strict code-signing verification:
  `codesign --verify --deep --strict --verbose=2 ...`: passing.
- No local Synara crash reports were present in
  `~/Library/Logs/DiagnosticReports`.

The local macOS build is ad-hoc signed with hardened runtime and sealed
resources. Developer ID signing and notarization remain release-channel work
after Apple Developer credentials are available.

## Linux

Status: source/package metadata validated; target workstation smoke pending.

Validated locally from this repository:

- `npm run check:versions`: passing at `1.1.0`.
- `packaging/arch/PKGBUILD` resolves:
  `pkgname=synara-desktop-bin`, `pkgver=1.1.0`, `pkgrel=1`.

Remaining validation requires the Linux target machine because WebKitGTK,
GStreamer, Secret Service, portals, Wayland, tray, notification, and global
shortcut behavior depend on the desktop session. Use the CachyOS / KDE Plasma
Wayland smoke checklist in [linux.md](./linux.md).
