# Desktop Validation Status

> Current production-release smoke evidence should be recorded against
> `docs/production-smoke-checklist.md`. This file preserves the older MIP1
> desktop validation snapshot and platform notes.

## 2026-08-17 Main Hardening Proof

Validation branch: `codex/main-proof-hardening-20260817`, based on
`origin/main` at `e2ef9bd252e9179978ca2acc6326368490a4f931`.

- Full frontend quality, contract, delivery, Playwright timeline, ESLint,
  Prettier, high-severity npm audit, and production build gates passed. Delivery
  script coverage was 224 of 224.
- Shared Rust workspace formatting, clippy with warnings denied, check, and all
  703 library tests plus integration/doc tests passed; one explicitly ignored
  live gate remained intentional.
- Desktop Rust formatting, clippy with warnings denied, check, and 454 tests
  plus doc tests passed.
- Root and desktop `cargo audit` passed with only the repository's documented
  allowed advisories.
- Synara `2.0.6` built as an arm64 macOS application. The local smoke copy was
  ad-hoc signed, passed strict code-signature verification, launched as a
  foreground process, served its internal runtime over loopback, returned HTTP
  200 for the generated JavaScript, stylesheet, and icon, and emitted no macOS
  error/fault log entries during the launch window.
- Interactive macOS clicks could not be automated because this Codex process
  lacks Accessibility and Screen Recording permission. Login/send/formatting
  behavior was instead covered through shared Rust tests, deterministic desktop
  frontend tests, and the signed iOS live Matrix suite. A human macOS GUI smoke
  is still required before merge/release.

Linux package and interactive desktop validation were not rerun on this macOS
host. Cross-platform Rust/frontend gates passed, but Linux packaging and desktop
integration remain platform-specific release checks.

Reviewed: 2026-06-11
Desktop version: **1.2.3** (`npm run check:versions`)
Branch: `maturity_improvement_plan1` (rebased on `main` with iOS UX maturity)

## Automated gates (branch tip)

| Gate                                     | Status | Notes                                                                       |
| ---------------------------------------- | ------ | --------------------------------------------------------------------------- |
| `npm run check:versions`                 | Pass   | App metadata + Tauri toolchain major.minor alignment                        |
| `npm --prefix synara run check:prettier` | Pass   |                                                                             |
| `npm run test:modernization`             | Pass   | 260/260 (after pass 3 remediation)                                          |
| `npm run typecheck:modernization`        | Pass   |                                                                             |
| `npm run check:mip1-evidence`            | Pass   | 46/46 items mapped; falls back to `origin/main` when local `main` is absent |
| `npm run check:runtime-assets`           | Pass   | devAssets ↔ synara/dist                                                     |
| `cargo test --lib`                       | Pass   | 82/82 (macOS workstation)                                                   |
| `cargo check --locked --release`         | Pass   | No warnings after pass 3 cleanup                                            |

## PR #10 remediation (2026-06-11)

Second review pass (`77e2282`) findings addressed on `maturity_improvement_plan1`:

| Item                                                                 | Status |
| -------------------------------------------------------------------- | ------ |
| Prettier + trailing whitespace in docs                               | Done   |
| Tauri npm/Cargo toolchain alignment + `check:versions` gate          | Done   |
| Native session `storedAtMs` preserved for proactive refresh          | Done   |
| Timeline divider semantics for ignored/redacted events               | Done   |
| File transfer TTL enforced on chunk/end IPC paths                    | Done   |
| Cryptographic opaque transfer IDs                                    | Done   |
| Incremental timeline revision token without full-list scan on append | Done   |
| Rust/TS agent-action URL + no-kind prompt parity                     | Done   |
| Localhost port tests resilient to busy preferred port                | Done   |
| Package smoke path filter `synara/**`                                | Done   |
| Agent approval notifications respect DND/showNotifications           | Done   |

## PR #10 remediation pass 3 (2026-06-11)

Third review (`f1a3d00`) findings addressed:

| Item                                                                    | Status |
| ----------------------------------------------------------------------- | ------ |
| Timeline test TS2367/TS2339 type error                                  | Done   |
| `config.json` Prettier drift after `build-runtime` copy                 | Done   |
| Live Matrix client credentials after proactive refresh                  | Done   |
| Recurring proactive refresh scheduling after rotation                   | Done   |
| `check:mip1-evidence` clone-safe base ref resolution                    | Done   |
| `desktop_save_file_begin` temp file leak on transfer cap                | Done   |
| Truncated streamed dropped-file reads rejected                          | Done   |
| Release `cargo check` warning cleanup (`AtomicU64`, `is_gnome_session`) | Done   |

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

### macOS signing and notarization

Local macOS smoke builds remain ad-hoc signed. Published macOS releases are
gated by `.github/workflows/release.yml`: CI imports the configured
Developer ID Application certificate, builds with that signing identity,
submits notarization through Tauri's Apple credentials, verifies the resulting
signature, and validates stapling on the app bundle and DMG. The release job
fails before build when any required Apple signing secret is missing.

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
