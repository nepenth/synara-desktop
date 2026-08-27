# Changelog

## Unreleased

## 2.1.16 - 2026-08-27

- Refreshed the existing Synara wing-and-network app icon for stronger visual
  mass and small-size legibility across iOS, macOS, Linux, and Windows.
- Added platform-correct iOS, ICNS, ICO, desktop PNG, and Linux symbolic assets,
  including Debian and Arch symbolic-icon installation.
- Added deterministic icon generation, a reviewed asset manifest, and a
  fail-closed icon-only CI path that preserves packaging/signing release proof
  without repeating unrelated hour-plus runtime suites.
- See [`docs/releases/v2.1.16.md`](docs/releases/v2.1.16.md) for details.

## 2.1.15 - 2026-08-27

- Fixed the remaining iOS suspension crash by making foreground authority a
  prerequisite for constructing the active app shell or opening Matrix stores,
  including cold notification/background launches and rapid scene transitions.
- Deferred notification approval handling until the app is active and Matrix
  has resumed, while keeping Approve and Deny actions visibly foregrounded.
- Added reusable TestFlight crash diagnostics and lifecycle regressions for
  cold-background startup, duplicate suspension callbacks, and session relogin.
- Fixed the macOS message composer so its text follows the selected Appearance
  > Message text tone just like the timeline.
- See [`docs/releases/v2.1.15.md`](docs/releases/v2.1.15.md) for details.

## 2.1.14 - 2026-08-26

- Fixed the iOS background `RUNNINGBOARD 0xdead10cc` termination by stopping
  Matrix sync, draining in-flight store work, and closing every retained SQLite
  connection before suspension; foreground activation reopens the stores before
  restarting sync through the same serialized native lifecycle route.
- Started iOS store quiescence at `applicationWillResignActive` so the complete
  persistence boundary begins before UIKit can suspend the process.
- Fixed the desktop message-text appearance setting across both timeline
  renderers and made Bright use true white in dark mode (true black in light
  mode), with distinct AAA-readable Soft and Balanced choices.
- See [`docs/releases/v2.1.14.md`](docs/releases/v2.1.14.md) for details.

## 2.1.13 - 2026-08-26

- Fixed an iOS foreground room crash by serializing and coalescing diffable
  timeline snapshots and replacing run-loop-pumping HTML entity decoding with
  deterministic decoding.
- Improved typing responsiveness in busy rooms by coalescing native timeline
  invalidations, bounding streams to newest state, suppressing unchanged
  snapshots, removing synthetic refreshes, and avoiding redundant full-text
  composer layout work.
- Added privacy-safe performance signposts plus unit and UI regressions for
  snapshot reentrancy, formatted HTML, and exact long-paragraph input during
  rapid live updates.
- See [`docs/releases/v2.1.13.md`](docs/releases/v2.1.13.md) for details.

## 2.1.12 - 2026-08-25

- Fixed iOS background suspension crashes by stopping and awaiting the retained
  native Matrix SyncService under a bounded UIKit background assertion before
  suspension, then serializing foreground restart through the same session
  owner. Rapid foreground/background transitions cannot restart sync while the
  app is backgrounded.
- Added privacy-safe iOS lifecycle signposts for native sync pause, background
  assertion expiry, suppressed stale resume, and foreground restart.
- See [`docs/releases/v2.1.12.md`](docs/releases/v2.1.12.md) for details.

## 2.1.11 - 2026-08-25

- Rich Matrix messages now use semantic, sanitized renderers across iOS and
  desktop, preserving exact code whitespace, nested and ordered lists, tables,
  details, spoilers, headings, colors, links, media fallbacks, and readable
  malformed-content fallbacks without reparsing HTML as Markdown.
- Hermes approval prompts are classified in the shared native core, surfaced as
  time-sensitive notification actions on iOS and distinctive desktop alerts,
  and resolved through the native Matrix reaction path with expiry, existing-
  decision, authentication, and cold-launch safeguards.
- Device verification is proven through the complete Matrix SAS operating path:
  both clients exchange identical emoji/decimal values, confirm trust, and read
  the peer back as Verified after relaunch.
- Opt-in iOS notification previews more reliably retain decrypted message
  content when available while preserving privacy-safe fallback text.
- Added a Pop!_OS/Debian-family APT repository path that publishes the tagged
  `.deb` and signed flat repository metadata alongside every production
  release.
- Updated Linux update guidance to cover APT as well as pacman/paru, with
  focused repository generation and signature verification tests.
- See [`docs/releases/v2.1.11.md`](docs/releases/v2.1.11.md) for details.

## 2.1.10 - 2026-08-25

- Room lists across iOS, macOS, and Linux now use compact channel markers and
  Slack-like `# Room Name` labels instead of decorative room avatars.
- Cross-client semantic colors, spacing, and typography soften dark-mode
  contrast while preserving readable hierarchy in timelines, tables, replies,
  settings, and composers.
- iOS room notes can be reordered by dragging, and message composition avoids
  unnecessary synchronous layout work for more responsive typing.
- iOS surfaces, Dynamic Type composer sizing, notification disclosure rows,
  and final-list-item clearance are hardened across light and dark appearance.
- Desktop room-note ordering and conflict handling now match the mobile
  behavior, with richer formatted-message and code presentation.
- See [`docs/releases/v2.1.10.md`](docs/releases/v2.1.10.md) for details.

## 2.1.9 - 2026-08-24

- iOS room timelines now include Notes backed by shared Matrix account data,
  bringing synced room notes to the mobile client.
- Opt-in iOS notification previews can show encrypted message content through
  a bounded, cancellation-safe notification service extension, with safe
  fallback text when content cannot be resolved.
- The notification extension uses a purpose-built native core instead of
  linking the full application core, with arm64, linkage, and size guards in
  pull-request and release CI.
- Device-verification session restore/reset preserves durable SAS trust
  readback, and Settings retains clearance above the floating tab bar.
- See [`docs/releases/v2.1.9.md`](docs/releases/v2.1.9.md) for details.

## 2.1.8 - 2026-08-23

- Device verification now targets the exact newly signed-in session, refreshes
  the device list before selection, and retains verified trust after relaunch.
- Direct SAS trust is projected with the Matrix SDK's complete device trust
  result instead of the narrower cross-signing-only state.
- The paired iOS diagnostic proves both devices exchange the same SAS values,
  confirm, and read each other back as Verified after both apps relaunch.
- iOS Rooms and Settings use continuous opaque light-mode surfaces through the
  safe areas, removing the gray veil and mismatched top and bottom colors while
  retaining floating-tab-bar clearance.
- See [`docs/releases/v2.1.8.md`](docs/releases/v2.1.8.md) for details.

## 2.1.7 - 2026-08-23

- Device verification now completes through the real Matrix SAS operating path:
  requests reach both clients, both sides exchange matching emoji or decimal
  codes, confirmation completes, and the verified trust state refreshes.
- iOS verification stays explicitly user-driven, keeps the comparison visible
  while state changes arrive, and identifies the exact session being verified.
- iOS Settings content now clears the floating tab bar, including the final rows
  in Dark Mode, and uses more readable message typography and table treatment.
- macOS message content has more comfortable measure, spacing, hierarchy, and
  alternating table rows for easier scanning in long technical messages.
- Matrix room and event routes are encoded consistently across desktop hosts.
- See [`docs/releases/v2.1.7.md`](docs/releases/v2.1.7.md) for details.

## 2.1.6 - 2026-08-22

- Native Core now owns homeserver push rules, 3PID/email, ignored users,
  per-room notification mode (including Default), user-directory search,
  message search, join-rule write, and presence SET. Linux uses the same
  Tauri commands as macOS.
- iOS can upload an avatar, send attachments, download plain `mxc://`
  media, register HTTP pushers, restore backup from a recovery key, and
  set Online/Away/Offline presence through product FFI rather than
  leftover stubs.
- Desktop Account can set presence; invite search uses the user
  directory; notifications and ignored-user/email settings write through
  Core. Leftover SharedCore FFI stays fail-closed.
- See [`docs/releases/v2.1.6.md`](docs/releases/v2.1.6.md) for details.

## 2.1.5 - 2026-08-22

- Settings → Devices no longer hangs on a spinner while identity lookup waits
  on `/keys/query`. Linux uses the same desktop path; iOS Security uses the
  same bounded Core lookup.
- iOS logout keeps the per-account crypto store so the next password sign-in
  reuses the same Matrix device instead of minting a new session.
- iOS Account → Sessions lists devices and can force-sign-out others with the
  account password. Desktop Settings → Devices → Others already had this.
- See [`docs/releases/v2.1.5.md`](docs/releases/v2.1.5.md) for details.

## 2.1.4 - 2026-08-21

- iOS failed sends retry from a chip, offline text queues until connected,
  and failed local echoes can be edited on the same pending id.
- Long-press Copy copies the plain message body; text selection is a
  best-effort substring path.
- Connection Lost holds through short SDK blips instead of bouncing on
  1–2s Offline gaps.
- Desktop unread badges and Mark as Read use native receipts, typing
  shifts the last message up, and timeline rows use a full-width surface.
- See [`docs/releases/v2.1.4.md`](docs/releases/v2.1.4.md) for details.

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
