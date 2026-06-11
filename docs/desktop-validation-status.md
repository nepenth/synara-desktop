# Desktop Validation Status

Reviewed: 2026-06-10  
Desktop version: **1.1.1** (`npm run check:versions`)

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

- `npm run test:modernization`: passing (see branch tip for current count).
- `npm run typecheck:modernization`: passing.
- `npm run check:eslint`: passing.
- `npm run check:prettier`: passing.
- `npm run check:versions`: passing at `1.1.1`.

## macOS

Status: locally and CI package-validated.

Validated from the current `maturity_improvement_plan1` branch:

- `cargo check`: passing.
- `npm run tauri build -- --bundles app`: passing.
- Built bundle:
  `src-tauri/target/release/bundle/macos/Synara.app`.
- Bundle version:
  `CFBundleShortVersionString = 1.1.1`,
  `CFBundleVersion = 1.1.1`.
- Configured bundle identifier:
  `com.whylandcreative.synara.desktop`.
- Strict code-signing verification:
  `codesign --verify --deep --strict --verbose=2 ...`: passing (ad-hoc).
- No local Synara crash reports were present in
  `~/Library/Logs/DiagnosticReports`.
- GitHub `Desktop Package Smoke` run `26464395682`: passing on
  `31c6ce6`, including macOS app build, signature verification, and artifact
  upload.

The local macOS build is ad-hoc signed with hardened runtime and sealed
resources. Developer ID signing and notarization remain release-channel work
after Apple Developer credentials are available.

### macOS signing and notarization (release scaffolding)

Release signing is not yet wired in CI. Before shipping a signed macOS channel:

1. Obtain an Apple Developer ID Application certificate and install it in the
   macOS keychain used by release builds.
2. Set `bundle.macOS.signingIdentity` in `src-tauri/tauri.conf.json` to the
   Developer ID identity (replace the current `"-"` ad-hoc placeholder).
3. Add an entitlements plist (for example `src-tauri/entitlements.plist`) and
   reference it from `bundle.macOS.entitlements`.
4. Set `bundle.macOS.minimumSystemVersion` to the supported macOS floor (for
   example `13.0`).
5. After `cargo tauri build`, run `codesign` verification, then `notarytool
   submit` and `stapler staple` on the `.app` / `.dmg` artifacts.
6. Gate the `Release Desktop` workflow macOS job on successful notarization once
   signing secrets (`APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
   `APPLE_API_KEY`, etc.) are available in GitHub Actions — see workflow
   comments in `.github/workflows/release-desktop.yml`.

Local development builds continue to use ad-hoc signing (`signingIdentity: "-"`).

## Linux

Status: CI package-validated; target workstation smoke pending.

Validated locally from this repository:

- `npm run check:versions`: passing at `1.1.1`.
- `packaging/arch/PKGBUILD` resolves:
  `pkgname=synara-desktop-bin`, `pkgver=1.1.1`, `pkgrel=1`.
- GitHub `Desktop Package Smoke` run `26464395682`: passing on
  `31c6ce6`, including Linux `.deb` package build and artifact upload.

Remaining validation requires the Linux target machine because WebKitGTK,
GStreamer, Secret Service, portals, Wayland, tray, notification, and global
shortcut behavior depend on the desktop session. Use the CachyOS / KDE Plasma
Wayland smoke checklist in [linux.md](./linux.md).

## Tray Parity Matrix (MIP1-35 — Option A: documented)

Decision: **Option A** — document platform differences; macOS tray remains the
minimal subset by design until product requests parity work.

| Tray item | macOS | Linux | Notes |
| --------- | ----- | ----- | ----- |
| Show Synara | Supported | Supported | Focuses main window |
| Unread summary | Not shown | Supported (label only) | Linux label reflects unread/highlights/later counts; navigates to Home |
| Later | Supported | Supported | Navigates to Later route |
| Notifications | Supported | Supported | Navigates to Notifications route |
| Desktop Integration | Not shown | Supported | Opens Settings for integration diagnostics |
| Do Not Disturb | Not shown | Supported | Toggles DND via `synara-tray-dnd-toggle` event (MIP1-06) |
| Build label | Supported (disabled) | Supported (disabled) | Read-only build identity |
| Quit | Supported | Supported | Exits application |

macOS intentionally omits unread summary, Desktop Integration, and DND tray
entries. None of the macOS tray items are no-op placeholders. Linux-only items
are documented here rather than mirrored on macOS in this wave.

## MIP1 validation (Wave I — final review placeholders)

Record orchestrator pass/fail for each wave during Phase 3 close-out. Update
status and evidence links before merge to `main`.

| Wave | Theme | Status | Notes |
|------|-------|--------|-------|
| A | Security & trust boundaries | _pending final review_ | DevTools gate, bridge caps, CSP, Windows honesty |
| B | Broken native UX | _pending final review_ | Notifications, tray DND, agent-action listener |
| C | Secret store truthfulness | _pending final review_ | Keyutils, Secret Service probe, Keychain probe, error codes |
| D | Session lifecycle & logout | _pending final review_ | Selective logout, SW session, unified logout |
| E | Performance & memory | _pending final review_ | Timeline, file IPC, tray throttle, caches |
| F | Rust shell hardening | _pending final review_ | Shortcuts, port fallback, badge, URLs, expiry |
| G | Frontend resilience & UX | _pending final review_ | Sync timeout, pagination errors, invoke strictness |
| H | Platform parity & packaging | _pending final review_ | Shortcut help, tray matrix, Arch packaging, CI |
| I | Polish, docs, long-term | _pending final review_ | This doc, linux.md, pkgrel, repo URLs, refresh token, signing scaffold, spellcheck |

### MIP1 global gate checklist (branch tip)

- [ ] `npm run check:versions`
- [ ] `npm run test:modernization`
- [ ] `cargo test` + `cargo check --locked --release`
- [ ] macOS manual smoke (tray, notifications, shortcuts, login/logout)
- [ ] Linux manual smoke (Arch package, Secret Service, Wayland WebKit)
- [ ] Zero open Critical/High orchestrator issues