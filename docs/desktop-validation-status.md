# Desktop Validation Status

Reviewed: 2026-06-11  
Desktop version: **1.2.3** (`npm run check:versions`)  
Branch: `maturity_improvement_plan1` (rebased on `main` with iOS UX maturity)

## Automated gates (branch tip)

| Gate | Status | Notes |
|------|--------|-------|
| `npm run check:versions` | Pass | 1.2.3 desktop + iOS build metadata |
| `npm run test:modernization` | Pass | 255 tests |
| `npm run typecheck:modernization` | Pass | |
| `npm run check:mip1-evidence` | Pass | 46/46 items mapped |
| `npm run check:runtime-assets` | Pass | devAssets ↔ synara/dist |
| `cargo test --lib` | Pass | 78 tests (macOS workstation) |
| `cargo check --locked --release` | Pass | |

## MIP1 remediation (2026-06-11)

Post-review fixes landed on `maturity_improvement_plan1`:

| Item | Status |
|------|--------|
| Rust/TS URL policy parity (private IP, local hosts) | Done |
| `map_keyring_error` stderr sanitization (G-NFR-1) | Done |
| Timeline revision fingerprint (MIP1-18 correctness) | Done |
| File transfer session TTL (5 min) + opaque IDs | Done |
| Notification route → navigation contract test | Done |
| Shortcut rollback scope test | Done |
| Validation doc refresh | Done |
| localStorage fallback risk documented | Done |

## Pre-iOS Prep

Status: complete on `main` (merged `feature/ios-ux-maturity`).

## macOS

Status: **CI package-validated; interactive smoke pending (merge gate).**

Validated:

- `cargo check` / `cargo test --lib` passing on branch tip.
- Release hardening: DevTools denied in release builds (`build.rs` + `release-hardening` capability).
- GitHub `Desktop Package Smoke` previously passing (see workflow history).

### macOS interactive smoke checklist (required before merge)

Execute on a logged-in macOS session with the branch tip build:

- [ ] Launch app, login, confirm session persists across restart (Keychain).
- [ ] Logout clears session; re-login succeeds without crypto store mismatch.
- [ ] Global shortcuts: register, trigger navigation, permission-denied messaging.
- [ ] Notification: show + click navigates to sanitized internal route.
- [ ] Tray: Show Synara, Later, Notifications, Quit (macOS subset per parity matrix).
- [ ] Release build: DevTools shortcut denied.

Record pass/fail and build revision in this file when complete.

### macOS signing and notarization (release scaffolding)

Release signing is not yet wired in CI. See prior scaffolding in this document
(`bundle.macOS.signingIdentity`, entitlements, notarytool workflow comments).

## Linux

Status: **partial workstation smoke** (headless + packaging pass; interactive GUI checklist pending).

Validated on CachyOS / KDE Plasma Wayland (2026-06-10, refreshed 2026-06-11):

- `npm run check:versions`: passing at `1.2.3`.
- `npm run build:runtime` + `cargo build --release`: passing.
- `cargo test --lib`: passing (includes live Secret Service / keyutils probes on session D-Bus when available).
- `packaging/arch/PKGBUILD`: `synara-desktop-bin` / `1.2.3` / `pkgrel=1`.

### Linux interactive smoke checklist (required before merge)

- [ ] Tray icon, menu, DND toggle (`synara-tray-dnd-toggle`).
- [ ] In-session notifications + click routing to Later/Notifications/Home.
- [ ] Global shortcuts + KDE Plasma Wayland permission help text.
- [ ] Desktop Integration panel rows in Settings.
- [ ] Portal file/media flows (drag-drop allowlist + streamed read).
- [ ] `pacman -U` install from launcher.

See [linux.md](./linux.md) for platform notes.

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

## MIP1 validation waves

| Wave | Theme | Automated | Interactive |
|------|-------|-----------|-------------|
| A | Security & trust boundaries | Pass | Release DevTools manual |
| B | Broken native UX | Pass | Notifications/tray smoke pending |
| C | Secret store truthfulness | Pass | Keychain/SS session UI |
| D | Session lifecycle & logout | Pass | Login/logout smoke pending |
| E | Performance & memory | Pass | Large-file manual optional |
| F | Rust shell hardening | Pass | Shortcut smoke pending |
| G | Frontend resilience & UX | Pass | Sync splash timeout manual |
| H | Platform parity & packaging | Pass | Arch install smoke pending |
| I | Polish, docs, long-term | Pass | User review pending |

## Merge readiness

- [x] Code remediation for PR review findings (2026-06-11)
- [x] Automated gates on branch tip
- [ ] macOS interactive smoke completed
- [ ] Linux interactive GUI checklist completed
- [ ] User personal review completed

**Recommendation:** squash-merge to `main` after interactive gates pass (bundled
commit history documented in `docs/mip1-commit-evidence.md`).