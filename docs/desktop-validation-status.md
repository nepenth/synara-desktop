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

Status: **partial workstation smoke** (headless + packaging pass; interactive GUI checklist pending).

Validated on CachyOS / KDE Plasma Wayland (2026-06-10, branch `maturity_improvement_plan1`):

- `npm run check:versions`: passing at `1.1.1`.
- `npm run build:runtime` + `cargo build --release`: passing; release binary launches and serves bundled UI on localhost.
- `cargo test --lib` (85/85), including live Secret Service and keyutils probe tests on session D-Bus.
- `packaging/arch/PKGBUILD`: `synara-desktop-bin` / `1.1.1` / `pkgrel=1`; `makepkg -f` produced `synara-desktop-bin-1.1.1-1-x86_64.pkg.tar.zst`.
- `packaging/arch/synara.desktop` and Wayland/WebKit wrapper script present.
- GitHub `Desktop Package Smoke` run `26464395682`: passing on `31c6ce6` (`.deb` build).

**Headless pass (automated proxy):** build pipeline, Arch packaging, secret-store probes, process launch.

**Interactive pending (requires logged-in GUI session):** tray icon/menu/DND, in-session notifications + click routing, global shortcuts + KDE permission help, Desktop Integration panel rows, portal file/media flows, full `pacman -U` install from launcher. Use the checklist in [linux.md](./linux.md).

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
| A | Security & trust boundaries | Pass (automated) | DevTools release hardening, bridge caps, CSP, Windows honesty |
| B | Broken native UX | Pass (automated) | Notification routes, tray DND, `synara://agent-action` listener |
| C | Secret store truthfulness | Pass (automated) | Keyutils probe, Secret Service live probe, Keychain probe, error codes |
| D | Session lifecycle & logout | Pass (automated) | `performLogout`, selective `clearSessionLocalStorage`, SW session |
| E | Performance & memory | Pass (automated) | Incremental timeline, streaming file IPC, tray throttle, bounded caches |
| F | Rust shell hardening | Pass (automated) | Shortcuts, port fallback, badge clamp, URL policy, session expiry |
| G | Frontend resilience & UX | Pass (automated) | Sync splash recovery, pagination errors, invoke diagnostics |
| H | Platform parity & packaging | Pass (automated) | Shortcut help, tray matrix (Option A), Arch PKGBUILD, CI smoke |
| I | Polish, docs, long-term | Pass (automated) | Validation docs, linux.md, pkgrel, repo URLs, refresh token, spellcheck log |

### MIP1 global gate checklist (branch tip)

- [x] `npm run check:versions`
- [x] `npm run test:modernization` (254/254, root script delegates to `synara/`)
- [x] `npm run check:mip1-evidence` (46/46 items mapped; compensates for bundled commits)
- [x] `npm run check:runtime-assets` (devAssets ↔ synara/dist sync)
- [x] `cargo test` + `cargo check --locked --release` (85/85)
- [ ] macOS manual smoke (tray, notifications, shortcuts, login/logout) — CI package smoke only; interactive pending
- [x] Linux manual smoke — **partial**: headless + packaging + live Secret Service probes pass; interactive GUI checklist pending
- [x] Zero open Critical/High orchestrator issues