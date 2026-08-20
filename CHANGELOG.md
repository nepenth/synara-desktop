# Changelog

## Unreleased

## 2.1.3 - 2026-08-20

- Completed the native SAS device-verification path: incoming requests stay
  on the app chrome, Confirm waits for emoji or decimal codes, and trust
  refreshes without a restart. iOS cannot swipe away an in-progress SAS
  sheet, and Security settings hide Verify This Device once verified.
- Tightened desktop chrome so the chat pane matches the rest of the window,
  title-cased Rooms, and made Favorites and Rooms sort independently.
- Removed Twitter Emoji from Appearance, showed the real theme accent
  instead of unused mint, and held Connection Lost until Offline lasts.
- Jump to latest stays available until the loaded window is the live tail.
- See [`docs/releases/v2.1.3.md`](docs/releases/v2.1.3.md) for details.

## 2.1.2 - 2026-08-20

- Highlighted fenced chat code with Prism tokens, a matching line-number
  gutter, and a distinct panel on the desktop native timeline. iOS code
  blocks now draw a line-number gutter.
- Replaced the Recent (24h) room rail with Matrix favorites and a sortable
  rooms list.
- Restored iOS live sync after TestFlight updates, showed connection status,
  and kept composer attachments as drafts until Send.
- Added Discord-like dark/light stacked chrome with a user base color, and
  made Settings lead with native verification, the server avatar, and honest
  controls.
- Release reuses a proven Quality gate for the tagged SHA, skips iOS and
  Synapse on version-only bumps, and publishes the GitHub Release without
  waiting on TestFlight.
- See [`docs/releases/v2.1.2.md`](docs/releases/v2.1.2.md) for details.

## 2.1.1 - 2026-08-19

- Reused leftover local crypto devices on password login so a prior interrupted
  session no longer fails with Olm unavailable on macOS and Linux.
- Opened unread rooms on the live timeline at the newest receipt instead of a
  stale fully-read marker, and made Jump to latest follow the live tail.
- Tightened desktop room-list chips to two-letter initials and a muted palette,
  and rendered timeline avatars, display names, timestamps, grouped sends, and
  formatted message bodies.
- See [`docs/releases/v2.1.1.md`](docs/releases/v2.1.1.md) for details.

## 2.1.0 - 2026-08-18

- Hardened native credential custody, remote transport policy, bounded media
  and file operations, diagnostic redaction, and notification privacy across
  iOS, macOS, and Linux.
- Removed retired renderer session credentials and repaired startup recovery
  so an unrestorable native session can be cleared without an insecure fallback.
- Added versioned GitHub and TestFlight release notes for coordinated client
  releases. See [`docs/releases/v2.1.0.md`](docs/releases/v2.1.0.md) for details.

- Bumped coordinated macOS, Linux, and iOS release metadata to `1.2.59` after
  the `1.2.58` iOS build was promoted to internal TestFlight testing.
- Added opt-in, privacy-filtered desktop diagnostics for performance, session
  persistence, room state and positioning, recent-room organization, and an
  on-screen performance overlay, with bounded local retention and manual
  export controls.
- Stabilized iOS unread/read-marker positioning and variable-height timeline
  anchor restoration by deferring placement until layout settles and using
  view-space cell geometry consistently.
- Restored Tauri app-bundle notarization credentials to the production macOS
  build and made PR/exact-tag validation fail early when the updater,
  notarization, signing, metadata, or distributable-verification contract is
  incomplete.
- Bumped coordinated macOS, Linux, and iOS release metadata to `1.2.34`.
- Activated native desktop spell checking when the shared composer receives
  focus: macOS now enables AppKit continuous spell checking on the active
  webview text responder, while Arch and Debian packages declare the English
  dictionary support required by WebKitGTK.
- Allowed the packaged Tauri localhost webview origin to use the main desktop
  capability set, restoring the native IPC path used by system-browser link
  opening, clipboard image reads, native file drops, desktop events, and update
  checks in release builds.
- Added a release-readiness regression check that fails if the packaged
  localhost webview origin loses native desktop capability access.
- Added native spellcheck language, autocorrect, and capitalization hints to the
  shared Slate editor while preserving the existing composer spellcheck toggle.
- Bumped shared Synara app version metadata to `1.2.22` for packaged
  macOS/Linux update validation.
- Added the user-facing desktop updater layer: Settings/About update checks,
  macOS background prompts with install/relaunch, a macOS app menu update
  command, and Linux package-manager guidance for GitHub-release version checks.
- Tightened release-updater readiness checks to require install-capable updater
  permissions and process relaunch support.
- Added production release automation for a GitHub Release-backed `synara`
  pacman repository so Arch-family Linux updates are package-manager-owned via
  `paru -Syu` / `pacman -Syu` after one-time repo setup.
- Fixed macOS updater metadata generation for GitHub Actions' downloaded
  `macos-updater-artifacts` directory layout.
- Bumped shared Synara app version metadata to `1.2.21`.
- Fixed production macOS release validation by notarizing and stapling the DMG
  after Tauri signs and bundles it.
- Hardened pacman repository release publication by using the GitHub release
  event tag explicitly, setting `GH_REPO` for no-checkout `gh` commands, and
  verifying downloaded repository assets before upload.
- Added a macOS release preflight that validates the Tauri updater private key
  password before the expensive signed/notarized package build.
- Fixed the macOS release workflow bundle set so updater-enabled `.app`
  artifacts remain available for signature verification and updater metadata.
- Revised production release automation so macOS remains Tauri-updater managed
  while Linux no longer publishes AppImage self-update metadata for the current
  release goal.
- Added release-branch Arch/CachyOS pacman package artifact generation for `synara-desktop-bin` package smoke.
- Fixed room re-entry after Jump to Latest by persisting live-tail bottom snapshots and allowing them to override stale unread/read-marker state only when no newer live-tail event has arrived.
- Hardened desktop external-link opening by mounting the interceptor at the app shell, using capture-phase link interception, surfacing native opener failures in desktop diagnostics, and making the injected Tauri bridge fail explicitly when IPC is unavailable.
- Added release-branch CI triggers for core CI, desktop package smoke, and iOS skeleton validation.
- Added a build-and-release runbook and linked it from the README documentation index.
- Added a release-branch CI and controlled client-update publication plan.
- Configured GitHub Actions updater signing secrets and updater public endpoint variables for future signed release workflow validation.
- Updated production-readiness plans with 2026-06-30 smoke feedback: desktop launch works, link opening fails on macOS/Linux, Timeline behavior is tentatively improved, and updater work remains deferred.
- Documented the 2026-06-29 macOS desktop non-launch postmortem and updated validation guardrails for updater config and Timeline helper changes.
- Bumped shared Synara app version metadata to `1.2.20`.
- Tightened the release-updater gate so signed metadata evidence must come from the generated updater metadata workflow.
- Added root-level macOS workstation handoff and deferred GitHub Release updater project plan documents.
- Added regression coverage proving release-time updater config materialization satisfies the strict release-updater readiness inspector.
- Added release workflow updater-channel configuration from GitHub repository variables before strict updater validation and packaging.
- Added release workflow generation and upload of static signed updater metadata from Linux and macOS updater artifacts.
- Tightened the release-updater gate so updater signature sidecars and signed updater metadata uploads are validated independently.
- Added a `check:production-smoke` gate that keeps production smoke checklist cases, signoff rows, preflight commands, and macOS/iOS queue linkage intact.
- Added a consolidated production smoke checklist covering evidence rules, macOS/Linux desktop smoke, Timeline Resurrection cases, iOS Xcode/simulator validation, and updater release smoke.
- Extracted desktop save/drop file-transfer commands, transfer-session state, drag/drop allowlist lifecycle, and tests into `desktop_file_transfer.rs`.
- Extracted desktop integration status DTOs, Linux/KDE/session/portal probes, and tests into `desktop_integration.rs`.
- Extracted desktop notification payload validation, permission commands, route-click dispatch, and tests into `desktop_notifications.rs`.
- Extracted desktop agent-action payload sanitization, local copy/open handling, event emission, and tests into `desktop_agent_actions.rs`.
- Extracted desktop tray/menu state, badge clamping, DND dispatch, and tray tests into `desktop_tray.rs`.
- Extracted desktop global shortcut config, registration lifecycle, integration status, plugin factory, and tests into `desktop_shortcuts.rs`.
- Extracted desktop keyring session persistence flow and error-sanitization tests into `desktop_session_store.rs`, leaving Tauri session commands in `desktop.rs`.
- Moved desktop secret-store platform probes, status caches, credential identity constants, and live probe tests into `desktop_secret_store.rs`, leaving `desktop.rs` focused on command/session storage flow.
- Extracted desktop secret-store status, backend classification, and stable reason/error-code contracts from `desktop.rs` into a focused Rust module with direct tests.
- Extracted desktop session-envelope validation and expiry policy from `desktop.rs` into a focused Rust module with direct tests.
- Aligned the published desktop release workflow with the signed updater gate by exposing Tauri updater signing secrets, removing release-time updater artifact suppression, and uploading generated updater signature artifacts.
- Extracted desktop file-transfer policy helpers from `desktop.rs` into a focused Rust module with direct tests.
- Extracted desktop text and route sanitization helpers from `desktop.rs` into a focused Rust module with direct tests.
- Stabilized the localhost-port Rust test so the validation gate still passes when the preferred dev port is already occupied.
- Split desktop URL safety helpers out of `desktop.rs` into a focused Rust module with direct policy tests.
- Extracted room timeline opening/window/unread helpers from `RoomTimeline.tsx` into a tested timeline utility.
- Extracted Matrix linked-timeline helpers from `RoomTimeline.tsx` into a tested shared timeline utility.
- Removed the commented legacy Jotai `sessionsAtom` implementation now that session bootstrap and persistence own the active session flow.
- Replaced the `/home/join/` route stub with the existing join-address prompt flow and shared room-link URL construction.
- Added desktop Tauri updater plugin scaffolding with check-only frontend permission while production updater metadata remains release-gated.
- Added a release-updater readiness checker and wired published desktop releases to fail until signed updater artifacts, metadata, plugin wiring, and release signing secrets are configured.
- Hardened desktop composer drag/drop detection so file payloads that expose `files` or file items without a `Files` type marker still activate the upload drop zone.
- Improved desktop composer clipboard image paste so image-like native clipboard payloads are uploaded before rich-text insertion, with a rich-text fallback when the native image read yields no file.
- Routed desktop external link opens through the native `desktop_open_external_url` bridge across Hermes cards, profile/server actions, account-management links, auth/info anchors, and agent actions without unsafe desktop `window.open` fallback.
- Added a cross-platform timeline open-focus contract and expanded iOS focus-policy coverage for read-marker and jump-latest behavior.
- Added a desktop timeline viewport restore policy so unread rooms and stale historical anchors no longer override live/read-marker opening.
- Added Codex-Orchestrator-v2 persistent harness artifacts for production-readiness tracking.
- Expanded the living production-readiness backlog to cover the full KB Section 7 recommendation set and reconciliation constraints.
- Fixed existing Prettier drift in timeline, notification, app-link, and timeline lifecycle files so the formatting gate passes.

## 1.0.4 - 2026-05-18

- Fixed room timeline viewport restoration when leaving and returning to a channel after scrolling into history.
- Added explicit saved-anchor restore handling so historical restores load around the saved event before normal pagination resumes.
- Prevented initial bottom pinning and generic pagination from overwriting an in-progress historical viewport restore.
- Updated displayed client version to match the packaged app version.
