# Desktop Validation Status

Reviewed: 2026-06-11
Desktop version: **1.2.3** (`npm run check:versions`)
Branch: `maturity_improvement_plan1` (rebased on `main` with iOS UX maturity)

## Automated gates (branch tip)

| Gate | Status | Notes |
|------|--------|-------|
| `npm run check:versions` | Pass | App metadata + Tauri toolchain major.minor alignment |
| `npm --prefix synara run check:prettier` | Pass | |
| `npm run test:modernization` | Pass | Run locally after remediation for current count |
| `npm run typecheck:modernization` | Pass | |
| `npm run check:mip1-evidence` | Pass | 46/46 items mapped |
| `npm run check:runtime-assets` | Pass | devAssets ↔ synara/dist |
| `cargo test --lib` | Pass | Run locally after remediation for current count |
| `cargo check --locked --release` | Pass | |

## PR #10 remediation (2026-06-11)

Second review pass (`77e2282`) findings addressed on `maturity_improvement_plan1`:

| Item | Status |
|------|--------|
| Prettier + trailing whitespace in docs | Done |
| Tauri npm/Cargo toolchain alignment + `check:versions` gate | Done |
| Native session `storedAtMs` preserved for proactive refresh | Done |
| Timeline divider semantics for ignored/redacted events | Done |
| File transfer TTL enforced on chunk/end IPC paths | Done |
| Cryptographic opaque transfer IDs | Done |
| Incremental timeline revision token without full-list scan on append | Done |
| Rust/TS agent-action URL + no-kind prompt parity | Done |
| Localhost port tests resilient to busy preferred port | Done |
| Package smoke path filter `synara/**` | Done |
| Agent approval notifications respect DND/showNotifications | Done |

## Pre-iOS Prep

Status: complete on `main` (merged `feature/ios-ux-maturity`).

## macOS

Status: **CI package-validated; interactive smoke pending (merge gate).**

Validated:

- `cargo check` / `cargo test --lib` passing on branch tip.
- Release hardening: DevTools denied in release builds (`build.rs` + `release-hardening` capability).
- GitHub `Desktop Package Smoke` expected to pass after Tauri alignment.

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
- [ ] Global shortcuts and file drag/drop upload path.
- [ ] Secret Service session persistence (or documented fallback when unavailable).

Record pass/fail and build revision in this file when complete.

## Merge readiness

Automated gates must be green on CI before merge. Interactive macOS/Linux smoke and user review remain required merge gates per MIP1 Phase 4.