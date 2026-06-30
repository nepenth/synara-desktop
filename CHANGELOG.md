# Changelog

## Unreleased

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
