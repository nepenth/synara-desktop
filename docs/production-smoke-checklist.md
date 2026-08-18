# Production Smoke Checklist

Reviewed: 2026-08-18

This checklist is the release handoff surface for human-run desktop and iOS
validation gates. Automated commands and release sequencing live in
[the build and release runbook](build-and-release.md). The dated feedback below
is preserved as regression history; it does not override newer evidence.

## Current Baseline

The latest consolidated automated, simulator, signed-live, and local macOS
proof is [the 2026-08-17 shared-core proof](shared-native-core/15-2026-08-17-local-proof.md).
The core macOS Matrix path now also has Xcode-driven interactive evidence dated
2026-08-18 in [desktop validation status](desktop-validation-status.md). Linux
package/install interaction, physical-device iOS checks, and the unexercised
macOS platform-integration cases remain release-candidate gates.

## Latest Smoke Feedback

2026-08-18 automated interactive macOS smoke:

- A local release app with an isolated bundle/Keychain identity completed fresh
  login, joined-room load, encrypted timeline decrypt, room-member display,
  formatted composer input, encrypted send, termination/relaunch, secure
  session restore, and exact message readback. Xcode reported one UI test and
  zero failures.
- The release-profile shared-core matrix passed 704 library tests with one
  intentional live gate ignored and all integration binaries passing. The
  focused desktop SDK session-rotation/logout test passed as well.
- The room read state is now one context-aware overflow action. Timeline
  pagination is edge-triggered with progress indicators, and jump-to-latest is
  a compact icon control; persistent read/unread and textual load buttons were
  removed.
- The pass fixed SDK refresh-token persistence, member-list fallback, timeline
  row field serialization, safe unsupported-event handling, structured agent
  card rendering, and full-URL homeserver login input.
- The run did not exercise system-browser links, native spellcheck, file
  drop/paste, OS notifications, tray, shortcuts, updater installation, or
  signed/notarized packaging. Their existing open statuses remain accurate.

2026-06-30 human smoke feedback:

- macOS desktop builds, launches, and broadly works after the disabled-updater
  launch fix; formal command/log evidence is still needed for signoff.
- macOS and Linux link opening currently fails: clicking external links does not
  open the system browser. `MAC-DESK-003` and `LINUX-DESK-006` are release
  blockers until fixed.
- Timeline/session-history behavior appears tentatively improved during use, but
  the Timeline Resurrection cases still need formal evidence before signoff.

2026-07-07 human smoke feedback:

- macOS and Linux link opening still fails. A shared release-runtime fix now
  allows the packaged `http://localhost:*/*` Tauri webview origin to use native
  desktop capabilities; `MAC-DESK-003` and `LINUX-DESK-006` remain blocked until
  re-smoked on packaged builds with this change.
- macOS and Linux clipboard image paste plus attachment drag/drop still fail.
  Treat `MAC-DESK-005`, `MAC-DESK-006`, and `LINUX-DESK-005` as likely affected
  by the same packaged-localhost native IPC capability gap until re-smoke proves
  otherwise.
- macOS and Linux spellcheck has a native remediation pending packaged smoke.
  macOS activates continuous checking on the focused AppKit responder; Linux
  enables the WebKitGTK context and packages an English dictionary. Re-smoke
  must record the locale, a misspelled test word, and visible correction menu.
- Timeline Resurrection is much better but still visibly repositions during
  initial room-history load. Collect anchor/read-marker/saved-bottom evidence
  before attempting another Timeline behavior change.
- macOS updater signoff remains open. Linux updates are package-manager-owned;
  the app may notify and instruct, but install should still happen through
  `paru -Syu` or `pacman -Syu` after repo setup.

## Evidence Rules

Every smoke pass must record:

- Commit SHA and branch.
- Build type: dev, local release, signed release candidate, or CI artifact.
- OS name/version, desktop environment where relevant, and hardware class.
- Test account/homeserver class, without secrets.
- Exact command used to build or launch.
- Per-case pass/fail, with reproduction notes for every failure.
- Links or paths for screenshots, screen recordings, logs, updater metadata, or
  crash reports when they exist.

Do not mark a section signed off from memory. The evidence must be attached to
this file, `MACOS_IOS_VALIDATION_QUEUE.md`, or a linked release issue/PR.

## Common Preflight

Run from the repository root before any interactive smoke pass:

```sh
npm run check:production-smoke
git status --short --branch
git rev-parse --short HEAD
npm run check:versions
npm run check:repo-layout
npm run check:docs
npm run check:matrix-boundaries
npm run check:quality-gates
npm run check:synapse-harness
npm run check:release-updater
```

For a release candidate with updater material configured, replace the last
command with:

```sh
npm run check:release-updater -- --require-enabled
```

## macOS Desktop Smoke

Required host: macOS workstation with prior Synara build capability.

Suggested local build command for unsigned smoke:

```sh
npm run tauri build -- --bundles app
```

Cases:

| ID            | Area                                | Pass Criteria                                                                                                                                                                                                                                                                                                                                                                                               | Evidence                                                  |
| ------------- | ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| MAC-DESK-000  | Launch with disabled updater config | App launches from the committed local config and reaches login/session UI; updater plugin may log that it is disabled, but startup must not fail.                                                                                                                                                                                                                                                           | Build SHA, macOS version, command/log excerpt, pass/fail. |
| MAC-DESK-001  | Launch/session                      | App launches, login succeeds, session persists after quit/reopen through Keychain.                                                                                                                                                                                                                                                                                                                          | Build SHA, macOS version, account class, pass/fail.       |
| MAC-DESK-002  | Logout/session                      | Logout clears session; relogin succeeds without crypto/session mismatch.                                                                                                                                                                                                                                                                                                                                    | Pass/fail plus any session-store status.                  |
| MAC-DESK-003  | Link opening                        | Rich text links, Matrix HTML links, normal message links, Hermes action/artifact links, settings/about links, profile/server links, OIDC account-management links, registration terms, feature-check help link, and location links open in the system browser, not an embedded webview.                                                                                                                     | Per-surface pass/fail; browser used.                      |
| MAC-DESK-004  | Composer spellcheck                 | Slate composer shows native spellcheck behavior while normal Markdown/rich text editing still works.                                                                                                                                                                                                                                                                                                        | Pass/fail with test words and locale.                     |
| MAC-DESK-005  | Composer file drop                  | Drag/drop one file and multiple files into a room; upload board shows correct attachments and encrypted-room upload path still succeeds.                                                                                                                                                                                                                                                                    | File types/sizes and pass/fail.                           |
| MAC-DESK-006  | Composer clipboard image            | Paste a screenshot/native clipboard image and an image copied from a browser that also advertises HTML/text; both upload as files instead of inserting unwanted rich HTML.                                                                                                                                                                                                                                  | Source app, upload board evidence, pass/fail.             |
| MAC-DESK-007  | Notifications                       | In-session notification appears; click routes to the sanitized internal room/inbox route.                                                                                                                                                                                                                                                                                                                   | Notification permission state and route result.           |
| MAC-DESK-007a | Agent approval native actions       | Approve once / Deny from a fresh agent-approval OS notification revalidates the event and sends the reaction; approve-always is not offered (or only opens the room); in-app approval cards show bounded full prompt context (reason, command, reply instructions) and require confirmation for approve-always; a stale/expired notification does not send a reaction; double-tapping does not double-send. | Action ids used, event age, pass/fail.                    |
| MAC-DESK-008  | Tray/status                         | Show Synara, Later, Notifications, Do Not Disturb, build label, and Quit behave as documented for macOS.                                                                                                                                                                                                                                                                                                    | Menu screenshot or per-item pass/fail.                    |
| MAC-DESK-009  | Shortcuts                           | Register global shortcuts, trigger navigation, and confirm permission-denied messaging if registration fails.                                                                                                                                                                                                                                                                                               | Shortcut config and pass/fail.                            |
| MAC-DESK-010  | Release hardening                   | Release build denies DevTools shortcut and still exposes build identity in the tray/About surfaces.                                                                                                                                                                                                                                                                                                         | Build type and pass/fail.                                 |

## Linux Desktop Smoke

Primary target: CachyOS or another Arch-family KDE Plasma Wayland session.
Secondary target: one Debian-family KDE Wayland session before public release.

Suggested package smoke build:

```sh
npm run tauri build -- --bundles deb --config '{"bundle":{"createUpdaterArtifacts":false}}'
```

Cases:

| ID             | Area                  | Pass Criteria                                                                                                                          | Evidence                                 |
| -------------- | --------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------- |
| LINUX-DESK-001 | Environment detection | Desktop Integration reports KDE Plasma Wayland or the correct fallback session/distribution labels.                                    | Screenshot or copied integration status. |
| LINUX-DESK-002 | Tray/status           | Tray icon appears; Show Synara, unread summary, Later, Notifications, Desktop Integration, Do Not Disturb, build label, and Quit work. | Per-item pass/fail.                      |
| LINUX-DESK-003 | Notifications         | In-session notifications appear and clicks route to sanitized internal destinations.                                                   | Permission state and route result.       |
| LINUX-DESK-004 | Shortcuts             | Shortcut save/trigger works or KDE Wayland failure path gives actionable manual-binding guidance.                                      | Shortcut config and pass/fail.           |
| LINUX-DESK-005 | Composer drop/paste   | File drag/drop and clipboard image paste both attach files through the upload board.                                                   | File types/sizes and pass/fail.          |
| LINUX-DESK-006 | Link opening          | Same external-link surfaces as `MAC-DESK-003` open in the system browser.                                                              | Browser used and per-surface pass/fail.  |
| LINUX-DESK-007 | Secret Service        | Session persists through Secret Service when available; documented fallback appears when unavailable.                                  | Backend status and restart result.       |
| LINUX-DESK-008 | Portals/media         | File/media portal readiness rows are present and accurate; file download/open handoff works.                                           | Portal status and pass/fail.             |

## Timeline Resurrection Smoke

Run on desktop and iOS. These cases mirror
`docs/timeline-room-state-reliability-contract.md`. Evidence uses fixture labels
and relative positions only; do not record room, event, or user identifiers.

| ID     | Scenario                                | Pass Criteria                                                                                                               | Evidence                                           |
| ------ | --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| TL-001 | Fully read room, no saved viewport      | Opens at live end without history traversal.                                                                                | Room size class and pass/fail.                     |
| TL-002 | Fully read room after old-history visit | Reopens at live end once saved historical anchor is stale.                                                                  | Anchor age and pass/fail.                          |
| TL-003 | One new message after old-history visit | Opens near unread/new context, not old history.                                                                             | Test-client label and pass/fail.                   |
| TL-004 | Read-marker focused open                | Shows jump-to-latest when latest is outside focused window.                                                                 | Read marker event age and pass/fail.               |
| TL-005 | Jump latest                             | Reaches true latest event after external sender posts.                                                                      | Relative newest position, latency, and pass/fail.  |
| TL-006 | Stale notification/read-marker state    | Does not restore unrelated old history; jump-latest path is clear.                                                          | Sync state notes and pass/fail.                    |
| TL-007 | Live append while pinned                | Follows bottom and marks read after visible delay.                                                                          | Pass/fail.                                         |
| TL-008 | Live append while scrolled up           | Preserves visible anchor.                                                                                                   | Fixture row/offset delta and pass/fail.            |
| TL-009 | Timeline reset/gap while pinned         | Reattaches to live tail without blank viewport.                                                                             | Reset trigger notes and pass/fail.                 |
| TL-010 | Timeline reset/gap while scrolled up    | Preserves anchor or keeps clear jump-latest affordance.                                                                     | Reset trigger notes and pass/fail.                 |
| TL-011 | Unread outside initial live window      | Desktop opens bounded unread context with the first event after `m.fully_read` at top, without walking intervening history. | Room size class, marker position notes, pass/fail. |
| TL-012 | iOS shared unread placement             | iOS opens the same bounded unread context, remains stable after placement, and still gives explicit event links priority.   | Marker/event positions and pass/fail.              |

## iOS Tool-Bound Smoke

Required host: macOS workstation with Xcode, Swift, XcodeGen, and an installed
iOS simulator.

From `synara-ios`:

```sh
xcodegen generate
xcodebuild -list -project Synara.xcodeproj
RUN_IOS_TESTS=1 IOS_TEST_DESTINATION='platform=iOS Simulator,name=iPhone 16' scripts/ci-build.sh
```

Required cases:

| ID      | Area                                | Pass Criteria                                                                                                                                                                                                                                                                                                                                                                                                                | Evidence                                                   |
| ------- | ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| IOS-001 | TimelineServiceTests                | Unit tests including `TimelineServiceTests` pass on the selected simulator.                                                                                                                                                                                                                                                                                                                                                  | Xcode version, simulator name/iOS version, command output. |
| IOS-002 | Timeline focus smoke                | Timeline reliability cases `TL-001` through `TL-012` pass where supported by current iOS functionality.                                                                                                                                                                                                                                                                                                                      | Per-case pass/fail and unsupported-case rationale.         |
| IOS-003 | Session/keychain                    | Login/session persistence behaves correctly on simulator or physical device.                                                                                                                                                                                                                                                                                                                                                 | Device target and pass/fail.                               |
| IOS-004 | Push/E2EE release gaps              | Push gateway and production E2EE remain explicitly marked pending until implemented and tested.                                                                                                                                                                                                                                                                                                                              | Current status and linked blocker.                         |
| IOS-005 | Agent approval notification actions | Approve once / Deny from a valid agent-approval notification revalidate the focused Matrix event (timeline load + approval detector) before reacting; approve-always opens the room (or is absent) and does not send ♾️; in-app approve-always requires an explicit confirmation step; expired/malformed/unresolved payloads do not approve. Production APNs/TestFlight remains external until proxy + APNs evidence exists. | Payload used, action plan/result, pass/fail.               |

## External Dependencies Still Open

Do not mark the following complete from in-repo client changes alone:

| Area                                          | Why external                                                                                                     | Tracking                                                                                      |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| Notification proxy trusted approval metadata  | Lives outside this repo; requires gateway + metadata cache + APNs category attach.                               | `docs/agent-approval-notification-proxy-spec.md`                                              |
| Production APNs / TestFlight approval actions | Needs physical device, production APNs certs/keys, and deployed proxy.                                           | `synara-ios/docs/push-gateway-staging.md`, TestFlight checklist                               |
| Installed-app updater smoke                   | Requires signed release artifacts and update channel config.                                                     | `docs/production-smoke-checklist.md` updater section, `MACOS_IOS_VALIDATION_QUEUE.md`         |
| Large-history timeline perf                   | Bounded rendering and directional range movement are implemented; daily-use geometry evidence is still required. | `docs/timeline-open-focus-contract.md` remaining-risk note and `docs/timeline-diagnostics.md` |

## Updater Release Smoke

Run only after real updater public key, endpoint, signing private key, and
release metadata location are configured.

```sh
npm run check:release-updater -- --require-enabled
```

Cases:

| ID      | Area             | Pass Criteria                                                                                   | Evidence                       |
| ------- | ---------------- | ----------------------------------------------------------------------------------------------- | ------------------------------ |
| UPD-001 | Config gate      | Strict updater gate passes with no placeholder key or endpoint.                                 | Command output.                |
| UPD-002 | Signed artifacts | Release build creates updater artifacts and `.sig` sidecars or signed `latest.json` metadata.   | Artifact paths and signatures. |
| UPD-003 | Hosted metadata  | Production HTTPS endpoint serves valid signed metadata for the built version.                   | URL and validation output.     |
| UPD-004 | App check        | Installed app can check for updates without crashing or contacting placeholder/local endpoints. | App logs and pass/fail.        |

## Signoff Table

| Section                     |                         Required Before Release | Status                                                                                                 | Evidence Link           |
| --------------------------- | ----------------------------------------------: | ------------------------------------------------------------------------------------------------------ | ----------------------- |
| Common preflight            |                                             Yes | Pending                                                                                                |                         |
| macOS desktop smoke         |                                             Yes | Core Matrix interaction passed 2026-08-18; link/open, paste/drop, spellcheck, notification, tray, shortcut, updater, and signed-package cases remain | `docs/desktop-validation-status.md` |
| Linux desktop smoke         |                                             Yes | Failed link/open, paste/drop, and spellcheck smoke; packaged-localhost capability fix pending re-smoke | 2026-07-07 human report |
| Timeline Resurrection smoke |                                             Yes | Much improved, but visible initial-load repositioning remains; diagnostics/formal evidence pending     | 2026-07-07 human report |
| iOS tool-bound smoke        | Yes for iOS release and shared Timeline signoff | Pending                                                                                                |                         |
| Updater release smoke       |                                             Yes | Pending                                                                                                |                         |
