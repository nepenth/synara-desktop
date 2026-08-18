# Desktop Validation Status

> Current production-release smoke evidence should be recorded against
> `docs/production-smoke-checklist.md`. This file preserves the older MIP1
> desktop validation snapshot and platform notes.

## 2026-08-18 Interactive macOS Matrix Smoke

Validation candidate: local release application based on `origin/main` at
`b1bbf89780c4e8831f0122f32bb006f543d981f8`, with the fixes described below.
Host: Apple silicon, macOS 26.5.2. Account class: disposable Matrix test
account in an encrypted test room. The smoke build used an isolated bundle
identity and isolated Keychain service so it could not read or modify the
installed app's session.

- An Xcode-driven UI test launched the release application, completed a fresh
  password login, loaded the joined-room list, opened an encrypted room, and
  displayed its decrypted timeline.
- The room header exposed member and overflow icon controls. The room read
  action appeared once in the overflow menu; persistent `Mark read`,
  `Mark unread`, `Load older messages`, and `Load newer messages` buttons were
  absent. Edge pagination used progress indicators, and jump-to-latest used a
  compact down-arrow control.
- The room member drawer showed the two joined members. Timeline dates no
  longer fell back to 1969, generic SDK timeline entries did not surface as
  unsupported-event noise, and agent payloads rendered as structured cards.
- The test entered formatted text through the real composer and sent it to the
  encrypted room. After terminating and relaunching the application, Keychain
  session restore succeeded and the exact sent marker reappeared in decrypted
  room state.
- Xcode reported one UI test executed with zero failures. Frontend typecheck,
  all 758 modernization/contract tests, runtime-asset consistency, Rust
  formatting, and `cargo check -p synara-core` also passed on the candidate.
  The release-profile shared-core matrix passed 704 library tests with one
  intentionally gated live test ignored, followed by every integration-test
  binary. The focused desktop session-rotation/logout test also passed.

This pass found and fixed missing Matrix SDK session-rotation callbacks. The
SDK can rotate access/refresh tokens; the desktop client now persists each
rotation to the account-scoped Matrix vault and desktop session envelope.
Remote logout is best-effort so an expired server token cannot prevent local
Keychain and in-memory cleanup.

Not covered by this UI run: OS notification delivery/click routing, tray and
global-shortcut interaction, clipboard/drop attachments, system-browser link
surfaces, signed installer/notarization, updater installation, or Linux
desktop integration. Those remain explicit release-candidate checks.

## 2026-08-17 Main Hardening Proof

Validation candidate: `release/runtime-assets-2.0.7`, based on
`origin/main` at `e3d8a45414d0a6438c216ffe4c638d4c108df928`.

- Full frontend quality, contract, delivery, Playwright timeline, ESLint,
  Prettier, high-severity npm audit, and production build gates passed. Delivery
  script coverage was 224 of 224. The final candidate reran typecheck, the
  production runtime build, and all 755 modernization/contract tests.
- Shared Rust workspace formatting, clippy with warnings denied, check, and all
  703 library tests plus integration/doc tests passed; one explicitly ignored
  live gate remained intentional.
- Desktop Rust formatting, clippy with warnings denied, check, and 454 tests
  plus doc tests passed.
- Root and desktop `cargo audit` passed with only the repository's documented
  allowed advisories.
- Synara `2.0.7` built as arm64 and universal macOS applications. The isolated
  local smoke copy was
  ad-hoc signed, passed strict code-signature verification, launched as a
  foreground process, served its internal runtime over loopback, returned HTTP
  200 for the generated JavaScript, stylesheet, and icon, and emitted no macOS
  error/fault log entries during the launch window.
- Interactive macOS clicks were unavailable during this dated pass. A later
  Xcode-driven application smoke is recorded above and supersedes that tooling
  limitation for the core login/room/timeline/composer/session path.

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

Status: **core interactive Matrix smoke passed; platform-integration smoke is
partial.**

Validated:

- `cargo check` / `cargo test --lib` passing on branch tip.
- Release hardening: DevTools denied in release builds (`build.rs` + `release-hardening` capability).
- GitHub `Desktop Package Smoke` expected to pass after Tauri alignment.
- Fresh login, encrypted-room load, formatted send, relaunch restore, and
  decrypted message readback passed in the isolated 2026-08-18 Xcode smoke.

### Remaining macOS interactive release checklist

Execute on a logged-in macOS session with the branch tip build:

- [x] Launch app, login, confirm session persists across restart (Keychain).
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

Automated gates must be green before merge. The core interactive macOS Matrix
path is now proven; the untested macOS platform-integration cases above and the
Linux interactive checklist remain release-candidate gates.
