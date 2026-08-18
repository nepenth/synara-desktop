# Synara Desktop Production Readiness Goal

> **Superseded readiness loop (2026-06/07).** This document is a frozen record
> of an earlier desktop hardening program. Its active rules, missing-source
> notices, version baseline, and pending tasks are not current instructions.
> Use [the codebase knowledge base](CODEBASE_KNOWLEDGE_BASE.md),
> [documentation guide](docs/README.md), and current validation-status files.

> Living single source of truth for the Codex-Orchestrator-v2 production-readiness loop.
> Updated after major decisions, accepted changes, validation runs, and release-state changes.

## Source Header

### Original Staff+/Principal Engineer Analysis Prompt

**Status:** Required source not yet located in the repository during Phase 0 initialization.

The user identified the prompt as beginning:

```text
You are a Staff+/Principal Software Engineer and expert Codebase Architect…
```

and ending:

```text
Begin the analysis now and produce the full Executive Report.
```

Phase 1 must replace this placeholder with the full verbatim prompt if the source is provided or discovered. Until then, decisions may rely only on repository state, `CODEBASE_KNOWLEDGE_BASE.md` v1.2.19, direct code inspection, and independently verifiable sources.

### Required External Reports

| Artifact | Required By User | Phase 0 Availability | Action |
|---|---:|---:|---|
| Grok Executive Codebase Report, 2026-06-29 | Yes | Not found under normal prompt/report filenames | Locate or request source before claiming reconciliation complete |
| Previous user Section 7 | Yes | Not found as separate artifact | Locate or request source before claiming reconciliation complete |
| `CODEBASE_KNOWLEDGE_BASE.md` v1.2.19 | Yes | Found | Read and incorporated initially |

## Mission

Drive `nepenth/synara-desktop` to production-release readiness through a persistent validation, prioritization, implementation, verification, commit, and release loop.

Loop termination requires all of the following:

- Three named pain points resolved:
  - Timeline Resurrection Epic.
  - System-browser link opening.
  - Desktop composer parity for spell check, file drag/drop, and clipboard image paste.
- Automated gates 100% green.
- Interactive smoke checklists in `docs/production-smoke-checklist.md` signed off by a human.
- Auto-updater enabled with signed metadata channel.
- This file contains `✅ PRODUCTION RELEASE READY — 2026-##-##`.
- Human confirmation.

## Operating Rules

| Rule | Status | Notes |
|---|---|---|
| Append `LLM_AGENT_LOG.md` entry before any other action each turn | Active | Initialized on 2026-06-29 |
| Maintain this file as single source of truth | Active | Created in Phase 0 |
| Update `CHANGELOG.md` on accepted changes | Active | Required before commits that accept behavior or harness changes |
| Use descriptive commits | Active | Phase 0 target: `chore: initialize Codex Orchestrator v2 harness + living goal files` |
| Do not overwrite unrelated user changes | Active | `devAssets/index.html` was already modified before Phase 0 |
| Stuck-loop detection | Active | If no measurable progress in 3 consecutive turns, flag and propose pivot or human intervention |

## Current Repository Baseline

| Item | Status | Evidence |
|---|---|---|
| Branch | `main` tracking `origin/main` | `git status --short --branch` |
| Pre-existing dirty files | `devAssets/index.html` | Generated runtime hash drift existed before harness initialization; root modernization build confirms current runtime hash |
| KB version | 1.2.20, last reviewed 2026-06-29 | `CODEBASE_KNOWLEDGE_BASE.md` |
| Desktop maturity per KB | Production-oriented, automated gates previously green | KB validation snapshot |
| iOS maturity per KB | Strong MVP, not release-ready | KB maturity table |

## Prioritized Backlog

### Priority Model

The user-declared pain points are **release P0 epics** because they are current client-facing blockers. The KB §7.5 **P0 interactive macOS/Linux smoke checklists** remain a mandatory release gate and run in parallel with implementation; they are not deprioritized by the pain-point epics.

Latest human smoke feedback, 2026-06-30:

- macOS desktop now builds, launches, and broadly works with the committed
  disabled-updater config after `f938246`.
- External link opening is still broken on both macOS and Linux desktop clients.
  This supersedes prior "first slice implemented" status as the immediate P0
  execution target.
- Timeline/session-history behavior appears tentatively improved, but requires
  continued daily-use confirmation and targeted smoke evidence before signoff.
- Follow-up Linux desktop smoke, 2026-06-30: a build from roughly 08:30 still
  restored `Project - Tech` backward in history after switching rooms, even
  after pressing Jump to Latest and waiting 5-10 seconds. Treat this as an
  active Timeline Resurrection regression until a newer build is smoked.
- Tauri/GitHub Release updater user-facing implementation is now in progress
  again. Updater signing key material is configured in GitHub, and the app now
  has Settings/About, macOS menu, background-check, macOS install/relaunch, and
  Linux package-manager guidance surfaces. Proof still requires a signed
  updater-capable release N followed by a newer release N+1.
- Updater test planning update: the manual macOS signed-build workflow is not an
  updater-channel proof because it disables updater artifacts. A real updater
  smoke now needs version N and N+1 builds from the release workflow so an
  installed updater-capable app can detect, install, and relaunch into the newer
  macOS version.
- Linux update policy update: do not support AppImage self-update for the
  current goal. Linux updates should be package-manager-owned through a fixed
  GitHub Release-backed pacman repo, with app-side notification/instructions
  only. Release CI owns `repo-add`, database generation, and asset replacement.
- Release branch and client-visible update CI planning is captured in
  `RELEASE_BRANCH_CI_PLAN.md`. Milestone 1 trigger coverage has been
  implemented after explicit approval; first release-branch push evidence is
  still pending.

Latest human smoke feedback, 2026-07-07:

- macOS and Linux desktop link opening still fails in the latest smoke build.
  Diagnosis found the release webview is served from Tauri localhost but
  `src-tauri/capabilities/main.json` did not grant capability access to that
  remote origin. The capability now allows `http://localhost:*/*`; macOS/Linux
  release re-smoke is required.
- Clipboard image paste and attachment drag/drop still fail on both desktop
  platforms. Treat these as likely sharing the same packaged-localhost native
  IPC capability root cause until re-smoke proves otherwise.
- Desktop composer spellcheck has a native remediation pending packaged smoke.
  The editor activates AppKit continuous spell checking on focus on macOS;
  Linux reapplies the WebKitGTK spell-check context and packaged Arch/Debian
  installs now include an English dictionary. Live verification remains required.
- Timeline Resurrection is far better, but room-open history still visibly
  repositions while initial history loads. Do not make a broad timeline change
  without focused scroll-anchor diagnostics because this area has already been
  fragile.
- macOS updater finalization remains open. Linux package-manager updating works;
  current policy remains app-side notification/instructions only, with actual
  Linux installation owned by `paru`/`pacman`.

### Epic 1: Timeline Resurrection

**Priority:** P0  
**Status:** In progress - stale unread/live-bottom re-entry fix implemented; human re-smoke pending
**Success criteria:** Fully-read rooms and rooms with one new message open near the expected live/read position without multi-second traversal through old history on desktop and iOS.

Initial task backlog:

1. Audit desktop timeline ownership in `RoomTimeline.tsx`, `timelineLifecycle.ts`, `timelinePagination.ts`, and `timelineVirtualization.ts`. **Status:** First pass complete for open-room viewport/read-marker policy.
2. Audit iOS timeline focus behavior in `RoomTimelineView.swift`, `TimelineService.swift`, and related read-marker services. **Status:** First pass complete for focus policy; local compile blocked by missing Xcode/Swift toolchain.
3. Reconcile Matrix semantics for `m.fully_read`, read receipts, live timelines, timeline gaps, `matrix-js-sdk` `getLatestTimeline`, and Matrix Rust SDK timeline focus. **Status:** Captured in `docs/timeline-open-focus-contract.md`.
4. Define a shared room-open focus contract covering:
   - Fully-read room.
   - Room with one or few unread events.
   - Saved historical viewport.
   - Explicit jump-to-event.
   - Timeline reset or gap.
   **Status:** Added `docs/timeline-open-focus-contract.md`.
5. Implement narrowly scoped desktop fix with tests and timeline performance harness. **Status:** First slice shipped.
6. Mirror contract and behavior in iOS with tests. **Status:** Added iOS policy tests for normal-room read-marker initial focus and jump-latest live-stream override; simulator/unit execution pending macOS/Xcode host.
7. Add smoke checklist evidence for large, encrypted, and unread room cases.
8. Verify Linux desktop re-entry case: open `Project - Tech`, press Jump to
   Latest, wait 5-10 seconds, switch rooms, return, and confirm the room stays
   at the live end unless a newer message arrived while away. **Status:** Code
   fix added on 2026-06-30; live re-smoke pending.

Mac/Xcode handoff: `MACOS_WORKSTATION_HANDOFF.md` contains the prompt, command
preflight, evidence requirements, and priority queue for the external macOS
workstation session.

Desktop first-slice finding, 2026-06-29:

- Root cause: desktop kept a process-local saved timeline viewport for non-bottom positions and restored it before unread/read-marker placement. A user who had previously viewed old history could later reopen a room and force `RoomTimeline` back through that old event window even if the room had one new unread event or had been fully read elsewhere. This mixed UI viewport memory with SDK read-marker/timeline focus semantics.
- Implemented mitigation: `RoomTimeline` now records viewport age, uses a tested `shouldRestoreRoomTimelineViewport(...)` policy, lets unread state and synchronous loaded-slice unread inference win over saved history, and expires non-bottom historical anchors after a 10 minute local-navigation window. Bottom snapshots still open at the live end.
- Remaining gap: iOS already has read-marker initial-load focus policy, but the
  shared contract and live smoke matrix still need to be formalized and verified
  across desktop/iOS. 2026-06-30 human feedback says desktop session-history
  behavior appears tentatively improved; keep collecting real-use evidence.
- Follow-up root cause, 2026-06-30: saved bottom snapshots were ignored whenever
  local unread/read-marker state still looked unread. After Jump to Latest,
  Matrix read receipt/account-data echo can lag long enough that returning to
  the room reopens at the old read marker instead of honoring the user's
  explicit live-end position. Desktop now persists the live-tail event id with
  bottom snapshots and restores that bottom snapshot through stale unread state
  only when the current live tail still matches; if a newer event arrived while
  away, unread placement still wins.
- Follow-up, 2026-07-07: human smoke says Timeline Resurrection is far better,
  but the room history view can still visibly move during initial load as more
  history arrives and the eventual read/live focus settles. Treat this as a
  diagnostic target, not an immediate broad rewrite: capture initial anchor,
  loaded-window changes, read-marker target, saved bottom snapshot, and
  jump-to-latest state before attempting another behavior change.

Shared contract update, 2026-06-29:

- Added `docs/timeline-open-focus-contract.md` with ownership rules, desktop/iOS behavior matrix, and a smoke checklist covering fully-read rooms, unread rooms, stale saved history, jump-latest, stale sync state, live appends, and timeline reset/gap cases.
- Updated `docs/matrix-sdk-lifecycle-semantics-audit.md` to point at the shared contract and record the desktop stale-viewport hardening.
- Expanded `TimelineServiceTests` policy coverage for iOS normal room updates after read-marker initial focus and jump-latest override back to live stream.

### Epic 2: Link Opening

**Priority:** P0  
**Status:** Packaged-localhost capability fix implemented after failed smoke - pending macOS/Linux re-smoke
**Success criteria:** Every user-visible external link from rich text, message content, Hermes cards, and related surfaces opens through the system browser via `desktop_open_external_url` on desktop, with safe fallback behavior outside Tauri.

Initial task backlog:

1. Inventory all anchor/link rendering paths. **Status:** First pass complete; no direct `window.open` production exception remains after the desktop SSO popup handoff was removed.
2. Centralize link activation through platform link helpers. **Status:** Desktop helper now routes normal external opens through `desktop_open_external_url`; non-desktop fallback remains browser-native.
3. Cover rich text, Matrix HTML, Hermes cards, room topics, and attachment/link previews. **Status:** Global anchor interceptor remains active for rendered anchors; Hermes cards, profile/server actions, OIDC account-management actions, registration terms, and feature-check anchors were wired explicitly.
4. Add tests for desktop bridge invocation and fallback. **Status:** Added regression tests for desktop bridge use, unsafe desktop no-fallback behavior, modified-click preservation, relative-link preservation, and agent-action no-fallback behavior.
5. Smoke check external URL opening on macOS/Linux. **Status:** Failed by human report on 2026-06-30; clicking links did not open the system browser on either desktop platform. Re-smoke required after latest bridge/interceptor hardening.
6. Perform an end-to-end bridge/runtime diagnosis:
   - Confirm `window.__SYNARA_DESKTOP__` is present in packaged/dev runtime.
   - Confirm click interception calls `desktop_open_external_url`.
   - Confirm the Tauri command reaches Rust.
   - Confirm `tauri_plugin_opener` succeeds or captures a useful platform error.
   - Add a runtime diagnostic or smokeable test surface if needed.
   **Status:** Partial code hardening complete on 2026-06-30: app-shell
   capture-phase link interception, explicit injected-bridge IPC failures, and
   desktop diagnostics for unavailable/rejected/false opener results.

Desktop first-slice finding, 2026-06-29:

- Root cause: most anchors were protected by the global desktop click interceptor, but several programmatic link opens and confirmation flows still called `window.open` directly. Hermes confirmed URL/artifact opens were the main P0 gap because they bypassed `desktop_open_external_url` entirely.
- Implemented mitigation: introduced a shared `openExternalUrl(...)` helper for normal external opens. Desktop calls use `openPlatformExternalUrl`/`desktop_open_external_url` with no browser fallback if the bridge rejects the URL; non-desktop keeps `window.open(..., 'noopener,noreferrer')`. Explicit link handlers now preserve modifier-click and app-relative native behavior.
- Current blocker: human smoke on 2026-06-30 reports that links do not open in
  the browser on macOS or Linux. Because both desktop platforms fail, prioritize
  the shared frontend-to-Tauri bridge and command path before platform-specific
  opener debugging.
- Latest hardening: the desktop anchor interceptor is mounted in `App` instead
  of `ClientRoot`, listens in capture phase, records native opener failures in
  diagnostics, and the injected desktop bridge throws a clear IPC-unavailable
  error instead of returning `undefined`. Frontend modernization tests and
  typecheck pass; live macOS/Linux smoke remains the release gate.
- Follow-up root cause, 2026-07-07: packaged release windows load the bundled UI
  from `http://localhost:{port}`, but the main capability applied only to local
  app URLs. That remote-origin gap can block every native IPC command and event
  listener used by link opening, clipboard image paste, native file drop,
  updater checks, and desktop diagnostics. `main.json` now grants
  `remote.urls: ["http://localhost:*/*"]`, and the release-updater readiness
  check fails if that packaged-localhost allowance disappears.

### Epic 3: Composer Desktop Parity

**Priority:** P0  
**Status:** In progress - packaged-localhost capability and spellcheck hints implemented
**Success criteria:** Slate composer supports native spell checking, drag-and-drop files, and clipboard image paste identically on desktop.

Initial task backlog:

1. Audit existing Slate editor, `RoomInput.tsx`, `RoomComposer.tsx`, `useFileDrop`, upload board, and paste handlers. **Status:** First pass complete; spellcheck defaults to enabled in `CustomEditor`, drag/drop already routes through `useFileDropZone(handleFiles)`, and desktop clipboard image reads route through `readPlatformClipboardImage`.
2. Enable native spellcheck without breaking Matrix markdown/rich text behaviors. **Status:** Existing `CustomEditor` default `spellCheck=true` covers the Slate composer; live desktop smoke still required.
3. Wire drag/drop files into existing upload state. **Status:** Browser and native desktop drop paths call `handleFiles`, which encrypts when needed and appends to the upload board. File-drop detection now tolerates desktop payloads that expose `files` or file items without a `Files` type marker; live desktop smoke still required.
4. Wire clipboard image paste into upload flow. **Status:** Improved desktop image-like clipboard handling so native image upload wins before rich-text insertion, with rich-text fallback if native image read yields no file.
5. Add unit/integration tests and desktop smoke checklist. **Status:** Added utility coverage for image-like clipboard probing; macOS smoke checklist queued in `MACOS_IOS_VALIDATION_QUEUE.md`.

Desktop first-slice finding, 2026-06-29:

- Root cause: RoomInput already supported pasted `DataTransfer` files and Tauri clipboard image reads, but focused editor paste tried rich-text insertion before native clipboard image probing. A clipboard advertising both `text/html` and `image/*` could insert HTML instead of uploading the native image.
- Implemented mitigation: exported a tested `shouldProbeNativeClipboardImage(...)` helper and made desktop focused composer paste try native image upload before Matrix rich-text insertion for image-like payloads. If the native image read returns no file, the handler falls back to the original rich/plain insertion path.
- Drag/drop hardening: `useFileDropZone` now uses a tested `dataTransferHasFiles(...)` helper instead of requiring the `Files` type marker, so desktop/WebView drops that expose `files` or file items still activate the upload flow while text-only drags remain ignored.
- Follow-up, 2026-07-07: both drag/drop and clipboard image paste still fail on
  macOS and Linux smoke builds, matching the shared packaged-localhost native
  IPC capability gap described under Epic 2. The shared Slate editor now also
  passes `lang`, `autoCorrect`, and `autoCapitalize` hints when spellcheck is
  enabled. Remaining gap: live desktop smoke is still required for native
  spellcheck, drag/drop files, clipboard screenshot paste, and mixed image+HTML
  clipboard sources; Linux spellcheck failures may still require dictionary or
  WebKitGTK environment evidence after the bridge fix.

### Epic 4: KB Section 7 Expansion Items

**Priority:** P1-P4  
**Status:** In progress - join route completed

Backlog reconciled from `CODEBASE_KNOWLEDGE_BASE.md` sections 7.1-7.5:

#### 7.1 Safe Extension Areas

1. Preserve and extend Synara-specific account-data features: Later, room notes, unread anchors.
2. Deepen agent workflow support through `desktop_agent_action` / `synara://agent-action` backend service wiring.
3. Improve search and inbox polish across message search, notifications, and Later.
4. Extend desktop native surfaces using established tray, shortcut, notification, and Rust sanitization patterns.
5. Require contract-first additions: human contract, JSON schema, fixture, desktop implementation/tests, and iOS mirror.

#### 7.2 Refactoring Before Major Expansion

1. Decompose `RoomTimeline.tsx`. **Status:** In progress; extracted linked-timeline navigation/count/index helpers to `utils/timelineLinks.ts` and opening/window/unread helpers to `utils/timelineOpening.ts` with direct modernization coverage. Component is now 2857 LOC.
2. Modularize `desktop.rs`. **Status:** In progress; slices extracted external-link, session-base, and agent URL policy helpers into `src-tauri/src/desktop_url.rs`, shared text/route sanitization helpers into `src-tauri/src/desktop_sanitize.rs`, file-transfer policy helpers plus save/drop IPC commands, transfer-session state, drag/drop allowlist lifecycle, and direct tests into `src-tauri/src/desktop_file_transfer.rs`, session-envelope validation/expiry policy into `src-tauri/src/desktop_session.rs`, secret-store status/backend/error classification plus platform probe/cache mechanics into `src-tauri/src/desktop_secret_store.rs`, keyring session persistence flow into `src-tauri/src/desktop_session_store.rs`, global shortcut policy/registration into `src-tauri/src/desktop_shortcuts.rs`, tray/menu state plus badge/DND handling into `src-tauri/src/desktop_tray.rs`, agent-action payload sanitization/local handling/event emission into `src-tauri/src/desktop_agent_actions.rs`, notification payload validation/permission/route-click dispatch into `src-tauri/src/desktop_notifications.rs`, and Linux/KDE/session/portal integration status probes into `src-tauri/src/desktop_integration.rs`, all with direct Rust tests. `desktop.rs` is now 236 LOC; `desktop_file_transfer.rs` is 1077 LOC.
3. Implement join route (`_JOIN_PATH`). **Status:** Done; `/home/join/` now opens the shared join-address prompt and navigates to room routes with `viaServers` preserved.
4. Remove or document commented `sessionsAtom` legacy session code. **Status:** Done; removed the commented legacy Jotai multi-session atom from `sessions.ts` now that `sessionBootstrap.ts` and `sessionPersistence.ts` own the active session flow.

#### 7.3 Cross-Platform Expansion Protocol

Any cross-platform feature must follow this checklist before acceptance:

1. Update human contract docs, JSON schema, and fixture.
2. Implement desktop owner utility and tests.
3. Update contract schema tests.
4. Mirror schema/fixture behavior in `synara-ios/Synara/Contracts/` and `SynaraContractsTests`.
5. Implement iOS service adapter when in scope.
6. Update iOS functionality matrix status.

#### 7.4 Risk Controls

| Risk | Control |
|---|---|
| Tauri IPC signature drift | Preserve backward-compatible command payloads unless an intentional migration is documented. |
| Cross-platform schema drift | Require fixture conformance tests on desktop and iOS before acceptance. |
| Timeline performance regression | Run `npm --prefix synara run test:timeline-performance` after timeline changes. |
| Linux DE variance | Smoke on CachyOS/KDE Wayland before release signoff. |
| iOS SDK API changes | Pin SDK versions and audit `matrix-rust-components-swift` changes before iOS release work. |
| Native shell plugin/config drift | After Tauri plugin or config changes, verify the committed disabled/local config launches and the release-time materialized config still passes strict checks. |
| Extracted frontend helper drift | After moving or reusing TypeScript helpers, run `npm run typecheck:modernization` and a runtime build before accepting the change. |

#### 7.5 Prioritized Next Steps

1. Complete interactive macOS + Linux smoke checklists.
2. Connect agent backend services to existing bridge.
3. Decompose `RoomTimeline.tsx`.
4. Modularize `desktop.rs`.
5. Enable auto-updater with signed metadata channel. **Status:** Key material configured; user-facing updater layer implemented locally with Settings/About, macOS menu, background checks, macOS install/relaunch, Linux package-manager guidance, install-capable permissions, and stricter readiness checks. macOS signed release workflow proof remains in `GITHUB_RELEASE_UPDATER_PLAN.md`, while Linux publication is package-manager-owned through a fixed GitHub Release-backed pacman repo.
6. Deploy iOS push gateway.
7. Implement iOS production E2EE.
10. Archive stale `maturity_improvement_plan1` branch.
11. Add component/integration tests for auth and room flows.
12. Evaluate Windows session store only if Windows enters the release matrix.

### Epic 5: Production Release Operations

**Priority:** P1-P2  
**Status:** Not started

1. Verify version consistency and release metadata.
2. Validate signing/notarization path for macOS.
3. Validate Linux packaging and CachyOS/KDE Wayland smoke.
4. Complete updater signing and release-channel documentation. **Status:** Release guard, install-capable updater plugin/process scaffolding, CI signing-secret exposure, release-time updater config materialization from repository variables, strict-inspector compatibility coverage for the materialized config, artifact-generation handoff, macOS signature upload patterns, generated macOS `latest.json` upload, Linux pacman repo automation, and user-facing updater UI are added. GitHub updater secrets/variables are configured as of 2026-06-30 but the `v1.2.20` release proved `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` does not match `TAURI_SIGNING_PRIVATE_KEY`; fixed `pacman-repo` assets are hosted and pacman-smoked locally; production updater proof still requires rotating the updater signing keypair/password if not already fixed, rerunning the release workflow, installing updater-capable version N manually, publishing N+1, and proving macOS install/relaunch from the app.
5. Implement release branch CI and controlled production publish flow. **Status:** Milestone 1 release-branch validation triggers implemented for `ci.yml`, `desktop-package-smoke.yml`, and `ios-skeleton.yml`; production workflow now includes an automated GitHub Release-backed `synara` pacman repo for Linux and macOS Tauri updater metadata. Remaining proof is a real release workflow run plus hosted artifact verification.
   - Release-branch package smoke now builds Linux `.deb`, Linux Arch/CachyOS
     `synara-desktop-bin-*.pkg.tar.zst`, and macOS `.app` artifacts.
6. Refresh `README.md`, release checklist, and user-facing installation docs. **Status:** Added `docs/build-and-release.md` as the repo entry point and linked it from `README.md`.
7. Confirm license, privacy, and security docs are current.

## Validation Gates

| Gate | Required Before Release | Latest Phase 0 Result | Notes |
|---|---:|---|---|
| `npm run check:versions` | Yes | Pass, 2026-07-07 | Version metadata consistent at 1.2.22 with Arch pkgrel 1; Tauri aligned at 2.11; iOS App Store build number remains 1.2.20 while marketing version matches 1.2.22 |
| `npm run check:repo-layout` | Yes | Pass, 2026-07-07 | Repository layout canonical |
| `npm run check:matrix-boundaries` | Yes | Pass, 2026-07-07 | Passed with 2 active iOS and 2 active desktop exceptions |
| `npm run check:mip1-evidence` | Yes | Pass, 2026-06-29 | 46/46 mapped; warning about bundled history retained |
| `npm run check:production-smoke` | Yes | Pass, 2026-07-07 | Verifies the smoke checklist keeps required evidence sections, case IDs, signoff rows, preflight commands, and macOS/iOS queue linkage |
| `npm run check:runtime-assets` | Yes | Pass, 2026-07-07 | Runtime assets match `synara/dist` after the 1.2.22 modernization rebuild |
| `node --test scripts/__tests__/*.test.mjs` | Yes | Pass, 2026-07-07 | Root release/readiness script tests passed 23/23, including packaged-localhost capability regression coverage |
| `npm run typecheck:modernization` | Yes | Pass, 2026-07-07 | TypeScript modernization surface compiles with editor spellcheck hints |
| `npm run test:modernization` | Yes | Pass, 2026-07-07 | Latest root build/test passed 306/306 after packaged-localhost capability, editor hint, updater, and 1.2.22 version changes |
| `npm --prefix synara run check:eslint` | Yes | Pass, 2026-07-07 | Frontend lint clean |
| `npm --prefix synara run check:prettier` | Yes | Pass, 2026-07-07 | Frontend formatting clean |
| `npm run check:release-updater` | Yes | Advisory pass with committed config disabled, 2026-07-07 | Plugin/package install-capable updater wiring, process relaunch wiring, packaged-localhost capability access, release CI config materialization, strict-inspector compatibility coverage for materialized config, signing/artifact handoff, and generated `latest.json` upload are present for 1.2.22; committed updater config remains disabled until release-time variables are applied |
| `npm run check:release-updater -- --require-enabled` | Required for published releases | Release-time dry-run passed with GitHub variables, 2026-06-30 | GitHub updater secrets/variables are configured; release workflow materializes updater config from `SYNARA_UPDATER_PUBKEY`/`SYNARA_UPDATER_ENDPOINT` before this strict gate; local static config remains disabled |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | Yes | Pass, 2026-06-29 | Rust formatting gate clean after normalizing `build.rs` |
| `cargo check` in `src-tauri` | Yes | Pass, 2026-07-07 | Rust compile gate clean at 1.2.22 with packaged-localhost capability config and updater/process plugin wiring |
| `cargo test` in `src-tauri` | Yes | Pass, 2026-06-29 | 93/93 Rust tests passed serially after file-transfer command/session extraction |
| `npm --prefix synara run test:timeline-performance` | Required for timeline changes | Pass, 2026-06-29 | Latest rerun: 10k events 8.51 ms; 50k events 20.69 ms |
| iOS tests | Required for iOS release | Queued for macOS workstation | `xcodebuild` and `swift` unavailable on this Linux host; see `MACOS_IOS_VALIDATION_QUEUE.md` |

## Status Table

| Phase | Status | Evidence | Next Step |
|---|---|---|---|
| Phase 0: Initializer | Committed and pushed | Current `HEAD` includes harness, validation evidence, changelog, and mechanical Prettier drift fix | Continue Phase 1 source reconciliation |
| Phase 1: Validation & Reconciliation | In progress; externally blocked for full reconciliation | KB §7 read fully; Grok/composer-2.5-fast second opinion completed from pasted excerpts; original prompt, Grok Executive Report, and prior Section 7 not found locally | Commit KB Section 7 reconciliation edits, then continue repo/source validation |
| Phase 2: Timeline Resurrection | In progress | Desktop viewport restore policy first slice implemented; shared open-focus contract added; iOS policy tests expanded; 2026-06-30 human daily-use feedback is tentatively positive; macOS/iOS validation handoff is tracked in `MACOS_IOS_VALIDATION_QUEUE.md` and `MACOS_WORKSTATION_HANDOFF.md` | Continue daily-use confirmation and run targeted iOS/macOS smoke checklist |
| Phase 3: Link Opening | P0 failed smoke | Desktop bridge hardening first slice implemented, but 2026-06-30 human smoke reports link clicks do not open the system browser on macOS or Linux | Diagnose and fix the shared desktop link-opening bridge/runtime path before other implementation work |
| Phase 4: Composer Parity | In progress | Clipboard image paste first slice and drag/drop detection hardening implemented; Grok reviewed both diffs and findings were incorporated/accepted; local gates pass at 283/283 | Run macOS/Linux interactive composer smoke checklist |
| Phase 5: Release Readiness | In progress, updater implementation complete pending release proof | Added release-updater readiness checker; wired published desktop release workflow to fail until signed updater channel prerequisites are configured; added desktop updater plugin/package/process dependencies, install-capable updater permission, process restart permission, Settings/About updates tile, macOS menu update command, background update checks, macOS install/relaunch prompt, Linux package-manager guidance, and updater service tests; aligned release CI with updater signing secrets, release-time updater config materialization, strict-inspector compatibility coverage for the materialized config, macOS artifact generation, signature uploads, generated macOS `latest.json` upload, and automated Linux `synara` pacman repo publication; configured GitHub updater key material on 2026-06-30; `v1.2.20` release published Linux `.deb` and pacman package assets and refreshed `pacman-repo`; macOS build produced signed/notarized/stapled app, DMG, and updater archive internally but failed before upload because the updater private key password secret is wrong; added a release preflight and secret-rotation runbook; fixed `pacman-repo` release hosts `synara.db`, `synara.files`, and `synara-desktop-bin-1.2.20-1-x86_64.pkg.tar.zst`; local non-root smoke synced the repo and downloaded/integrity-checked the package; privileged CachyOS install succeeded from `synara/synara-desktop-bin 1.2.20-1`; latest main CI after workflow fix passed; documented release branch/client-update CI strategy in `RELEASE_BRANCH_CI_PLAN.md`; completed `_JOIN_PATH` route stub; removed legacy commented session atom; continued `RoomTimeline.tsx` decomposition; continued `desktop.rs` modularization with URL policy, sanitization, file-transfer command/session, session-envelope, secret-store classification/probe/cache, keyring session-store, global shortcut, tray/menu, agent-action, notification, and integration-status extractions; consolidated production smoke evidence requirements in `docs/production-smoke-checklist.md` | Run release workflow for updater-capable version N, install manually, publish N+1, and prove macOS in-app install/relaunch plus Linux package-manager guidance/update path |

## Postmortem: macOS Desktop Non-Launch, 2026-06-29

Late-night commits `f938246` and `3e0bd8e` corrected two release-readiness
regressions introduced during the prior automation pass:

1. `f938246 fix: tolerate disabled desktop updater config`
   - Failure mode: the desktop shell registered `tauri_plugin_updater` even when
     the committed `tauri.conf.json` intentionally omitted `plugins.updater`.
     The production plan expected updater config to be materialized only inside
     release jobs, but the local/macOS app still loaded the plugin against the
     disabled config and became non-functional.
   - Root process gap: `cargo check` and `npm run check:release-updater` proved
     compile/readiness logic, not actual native-shell startup with committed
     local config. The decision log overtrusted "commands fail closed" without a
     macOS launch smoke.
   - Current invariant: the updater plugin is registered only when
     `plugins.updater` exists; disabled local config must continue to launch.

2. `3e0bd8e fix: import timeline link helper`
   - Failure mode: `RoomTimeline.tsx` used the extracted
     `getFirstLinkedTimeline(...)` helper without importing it, breaking the
     frontend bundle used by the desktop client.
   - Root process gap: after accumulated docs/version/updater changes, the final
     local pass did not rerun the frontend typecheck/build that would catch
     missing TypeScript imports.
   - Current invariant: any Timeline/helper extraction or version commit that
     touches frontend source must include `npm run typecheck:modernization`
     before acceptance, and a runtime build/smoke before release signoff.

Plan impact:

- Updater implementation can continue only with release-proof work now; local
  disabled-updater launch remains part of the Mac handoff, and the next decisive
  updater evidence is a signed N to N+1 installed-app smoke.
- Treat macOS desktop launch as the next release-readiness gate, not an optional
  final polish check.
- Prefer evidence collection over further broad refactors. Future refactors must
  include the full gate for the affected surface, not only local utility tests.

## Decision Log

| Timestamp | Decision | Rationale |
|---|---|---|
| 2026-06-29T09:13:15-04:00 | Created initial goal file with missing-source placeholders. | Required artifacts are not all present locally; goal state must distinguish verified repo evidence from unavailable report context. |
| 2026-06-29T09:16:44-04:00 | Accepted Phase 0 validation baseline with one local limitation. | Desktop gates are green after mechanical Prettier correction; iOS simulator gate cannot run because `xcodebuild` is unavailable on this host. |
| 2026-06-29T09:17:59-04:00 | Created Phase 0 harness commit. | Initial commit `3f19dd2` used requested message `chore: initialize Codex Orchestrator v2 harness + living goal files`; evidence entry was amended into the same commit. |
| 2026-06-29T09:18:24-04:00 | Amended Phase 0 commit evidence. | `devAssets/index.html` remains an unstaged pre-existing generated hash drift. The exact final SHA is intentionally not embedded because amending the file changes the commit object. |
| 2026-06-29T09:18:55-04:00 | Stabilized commit evidence wording. | Removed self-referential final-SHA wording from committed artifacts; use `git rev-parse --short HEAD` for the current commit id. |
| 2026-06-29T09:22:51-04:00 | Accepted Grok/composer-2.5-fast Phase 1 reconciliation finding. | Reviewer found §7.5 coverage strong but §7.1, §7.3, §7.4, and session-layer cleanup underrepresented; goal backlog expanded accordingly. Missing external artifacts still prevent full reconciliation completion. |
| 2026-06-29T09:34:44-04:00 | Accepted desktop Timeline Resurrection first slice. | Implemented bounded viewport restore policy: unread wins over saved history, non-bottom historical anchors expire after 10 minutes, and bottom snapshots still restore to live end. Grok/composer-2.5-fast approved with reservations; synchronous `roomHaveUnread` inference was added to reduce first-paint unread races. |
| 2026-06-29T09:38:11-04:00 | Added shared Timeline Resurrection open-focus contract. | `docs/timeline-open-focus-contract.md` now defines cross-platform ownership rules, behavior matrix, and smoke checklist; iOS focus policy tests were expanded. Local validation passed for TS/runtime gates, but iOS execution remains pending because the host lacks Xcode/Swift. |
| 2026-06-29T09:45:54-04:00 | Split macOS/iOS-only validation into a dedicated queue. | The current Linux host cannot execute `xcodebuild`, `swift`, or simulator checks, but the user has a macOS workstation available. `MACOS_IOS_VALIDATION_QUEUE.md` now tracks required commands, evidence, and handoff status separately from Linux-local gates. |
| 2026-06-29T09:53:07-04:00 | Accepted desktop external-link bridge hardening first slice. | Programmatic external opens now route through the platform helper, agent actions no longer fall back to `window.open` after desktop bridge rejection, and explicit handlers preserve modified-click/native relative-link behavior. Grok reviewed the diff; composer CLI was unavailable locally. |
| 2026-06-29T10:00:18-04:00 | Accepted desktop composer clipboard image first slice. | Desktop focused composer paste now prioritizes native image upload for image-like clipboard payloads before rich text insertion, with fallback to rich/plain insertion if the native image read yields no file. Grok reviewed the diff and the fallback finding was incorporated. |
| 2026-06-29T10:05:21-04:00 | Accepted desktop composer drag/drop detection hardening. | File-drop detection now accepts desktop/WebView payloads that expose `files` or file items without a `Files` type marker while text-only drags remain ignored. Grok/composer-2.5-fast reviewed the diff and found no blocking issue; malformed file-like payload suppression is an accepted safety tradeoff. |
| 2026-06-29T10:11:36-04:00 | Added release-only updater readiness enforcement instead of enabling placeholder updater config. | Real production updater enablement requires signed metadata endpoint/key material and runtime plugin wiring. The release workflow now blocks artifact builds until those prerequisites are present, while local advisory checks keep the current disabled state visible. |
| 2026-06-29T10:20:36-04:00 | Accepted check-only desktop updater plugin scaffolding. | Added the Tauri updater Rust plugin, npm updater binding, generated ACL schemas, and `updater:allow-check` only. Advisory release checker now reports only production channel/signing/artifact blockers; Grok found registration safe without config because commands fail closed when endpoints are empty. |
| 2026-07-02T00:00:00-04:00 | Implemented user-facing desktop updater layer. | Added frontend updater service/provider, Settings/About update tile, macOS app menu update command, background checks, macOS install/relaunch through updater/process plugins, Linux package-manager guidance, install-capable updater/process permissions, stricter release-readiness checks, and updater service tests. Real self-update proof still requires a signed updater-capable release N and newer N+1. |
| 2026-06-29T10:31:32-04:00 | Accepted `_JOIN_PATH` route completion. | Replaced the `/home/join/` placeholder with the shared join-address prompt route, centralized room path generation with `viaServers`, routed the Home sidebar join action through the canonical route, and added modernization coverage for encoded room/event/via URL construction. |
| 2026-06-29T10:38:00-04:00 | Accepted session layer dead-code cleanup. | Removed the commented legacy Jotai `sessionsAtom` implementation from `sessions.ts`; active session ownership remains with `sessionBootstrap.ts`, `sessionPersistence.ts`, native secret storage, and the tested localStorage fallback path. |
| 2026-06-29T10:45:53-04:00 | Accepted first `RoomTimeline.tsx` decomposition slice. | Extracted linked-timeline navigation/count/index helpers into `utils/timelineLinks.ts`, added direct tests for linked chains, isolated timelines, index boundaries, wrapper behavior, and kept the component wired to the same helper behavior. Grok review found no blocking or high-impact issues. |
| 2026-06-29T10:56:30-04:00 | Accepted second `RoomTimeline.tsx` decomposition slice. | Extracted opening/window/unread helpers into `utils/timelineOpening.ts`, added direct tests for bounded live-end windows, empty timeline state, unread read-receipt/anchor resolution, detached timeline handling, and initial unread detection. Grok review was attempted twice but hit its max-turn cap without findings. |
| 2026-06-29T11:04:38-04:00 | Accepted first `desktop.rs` modularization slice. | Extracted URL safety/session/agent URL helpers into `src-tauri/src/desktop_url.rs`, preserved the public `desktop::is_safe_external_url` wrapper, added direct Rust policy tests, and kept local Rust/repo guardrails green. Grok found no blocking issues; composer CLI remains unavailable locally. |
| 2026-06-29T11:12:25-04:00 | Accepted Rust validation hardening slice. | The localhost-port test now treats an already-occupied preferred dev port as the busy-port condition under test instead of failing before exercising `select_localhost_port`; `build.rs` was normalized by rustfmt and a duplicate `cfg` attribute was removed. |
| 2026-06-29T11:21:07-04:00 | Accepted second `desktop.rs` modularization slice. | Extracted shared text and route sanitization helpers into `src-tauri/src/desktop_sanitize.rs`, preserved notification/agent/route call sites, and added direct tests for trimming, truncation, and internal-route policy. Grok review hit its max-turn cap without findings. |
| 2026-06-29T11:30:17-04:00 | Accepted third `desktop.rs` modularization slice. | Extracted file-transfer policy helpers into `src-tauri/src/desktop_file_transfer.rs`, preserved save/drop command state in `desktop.rs`, and added direct tests for filename sanitization, unique paths, streaming thresholds, and transfer id shape. Grok review hit its max-turn cap without findings. |
| 2026-06-29T11:38:07-04:00 | Accepted release workflow updater-signing alignment. | The published desktop release jobs no longer force `createUpdaterArtifacts` off after the strict updater gate; they now expose and validate Tauri updater signing secrets and upload generated signature sidecars. Advisory updater warnings dropped from 8 to 4, leaving only committed updater public key/endpoint config blockers. |
| 2026-06-29T11:46:40-04:00 | Accepted fourth `desktop.rs` modularization slice. | Extracted session-envelope validation, timestamping, and expiry policy into `src-tauri/src/desktop_session.rs`, kept keyring/probe/storage commands in `desktop.rs`, and moved direct policy tests with the module. `desktop.rs` is now 4673 LOC. |
| 2026-06-29T11:53:49-04:00 | Accepted fifth `desktop.rs` modularization slice. | Extracted `DesktopSecretStoreStatus`, stable backend/reason/error-code constants, unavailable status construction, macOS Keychain status classification, Linux backend classification, operation error-code mapping, and bridge persistence classification into `src-tauri/src/desktop_secret_store.rs` with direct tests. Live platform probes and keyring commands remain in `desktop.rs`; `desktop.rs` is now 4328 LOC. |
| 2026-06-29T12:11:12-04:00 | Accepted sixth `desktop.rs` modularization slice. | Moved macOS Keychain and Linux Secret Service/keyutils live probe mechanics, status caches, credential identity constants, and direct platform probe tests into `src-tauri/src/desktop_secret_store.rs`. Keyring session command/storage flow remains in `desktop.rs`; `desktop.rs` is now 3717 LOC. |
| 2026-06-29T12:18:11-04:00 | Accepted seventh `desktop.rs` modularization slice. | Extracted keyring-backed session persistence, legacy credential migration, stable keyring error sanitization, session get/set/remove store helpers, and direct policy tests into `src-tauri/src/desktop_session_store.rs`. Tauri command exports remain in `desktop.rs`; `desktop.rs` is now 3281 LOC. |
| 2026-06-29T12:29:34-04:00 | Accepted eighth `desktop.rs` modularization slice. | Extracted global shortcut config/result types, validation, registration/rebind/retirement lifecycle, permission messaging, integration status summary, plugin factory, and direct policy tests into `src-tauri/src/desktop_shortcuts.rs`. Tauri command export remains in `desktop.rs`; `desktop.rs` is now 2560 LOC. |
| 2026-06-29T12:40:28-04:00 | Accepted ninth `desktop.rs` modularization slice. | Extracted tray/menu state coalescing, menu labels, badge clamping, DND dispatch, tray creation/event handling, and direct tray tests into `src-tauri/src/desktop_tray.rs`. `desktop.rs` now exports only command wrappers/shared window navigation for this surface and is 2025 LOC. Local Rust/repo gates passed, including release-mode `cargo check`; updater check remains advisory with the known four production config warnings. |
| 2026-06-29T14:28:14-04:00 | Accepted tenth `desktop.rs` modularization slice. | Extracted agent-action payload sanitization, supported-kind policy, local copy/open handling, `synara://agent-action` event emission, and direct tests into `src-tauri/src/desktop_agent_actions.rs`. IPC command registration now points at the focused module while preserving the `desktop_agent_action` command name and payload shape. `desktop.rs` is now 1717 LOC. Local Rust/repo gates passed, including release-mode `cargo check`; updater check remains advisory with the known four production config warnings. |
| 2026-06-29T14:34:22-04:00 | Accepted eleventh `desktop.rs` modularization slice. | Extracted notification payload validation, permission commands, route-click dispatch for Linux/macOS/fallback platforms, and direct tests into `src-tauri/src/desktop_notifications.rs`. IPC command registration now points at the focused module while preserving `desktop_get_notification_permission`, `desktop_request_notification_permission`, and `desktop_notify`. `desktop.rs` is now 1456 LOC. Local Rust/repo gates passed, including release-mode `cargo check`; updater check remains advisory with the known four production config warnings. |
| 2026-06-29T14:41:08-04:00 | Accepted twelfth `desktop.rs` modularization slice. | Extracted integration status DTOs, KDE/Wayland/session detection, `/etc/os-release` parsing, portal backend probes, notification/tray/shortcut readiness aggregation, and direct tests into `src-tauri/src/desktop_integration.rs`. IPC command registration now points at the focused module while preserving `desktop_get_integration_status`. `desktop.rs` is now 1142 LOC. Local Rust/repo gates passed, including release-mode `cargo check`; updater check remains advisory with the known four production config warnings. |
| 2026-06-29T14:48:22-04:00 | Accepted thirteenth `desktop.rs` modularization slice. | Extracted save/drop file-transfer commands, transfer-session state, drag/drop allowlist lifecycle, and direct tests into `src-tauri/src/desktop_file_transfer.rs`. IPC command registration and native drag/drop window events now point at the focused module while preserving command names and payload shapes. `desktop.rs` is now 236 LOC; `desktop_file_transfer.rs` is now 1077 LOC. Local Rust/repo gates passed, including release-mode `cargo check`; updater check remains advisory with the known four production config warnings. |
| 2026-06-29T14:50:00-04:00 | Created the production smoke handoff checklist. | `docs/production-smoke-checklist.md` now consolidates evidence rules, common preflight commands, macOS/Linux desktop smoke, Timeline Resurrection smoke, iOS Xcode/simulator commands, and updater release smoke so human-run validation can produce release-grade evidence. |
| 2026-06-29T14:56:24-04:00 | Added an enforceable production smoke checklist gate. | `npm run check:production-smoke` now verifies required smoke sections, case IDs, signoff rows, common preflight commands, and `MACOS_IOS_VALIDATION_QUEUE.md` linkage; root script tests pass 9/9. |
| 2026-06-29T14:59:10-04:00 | Tightened the release-updater metadata gate. | `npm run check:release-updater` now validates updater signature sidecars and signed updater metadata upload independently; root script tests pass 11/11. Advisory mode now reports 5 expected production blockers, adding missing `latest.json`/metadata upload to the prior public key/endpoint/config blockers. |
| 2026-06-29T15:02:59-04:00 | Added release workflow signed updater metadata generation. | Published release jobs now collect Linux/macOS updater archives, generate static Tauri updater `latest.json` with Linux and macOS platform entries, and upload it to the GitHub release. Advisory updater warnings are back to the four real committed config/key/endpoint blockers; root script tests pass 14/14. |
| 2026-06-29T15:07:25-04:00 | Added release-time updater config materialization. | Published release jobs now run `scripts/configure-release-updater.mjs` before strict updater validation and packaging, using `SYNARA_UPDATER_PUBKEY` plus optional `SYNARA_UPDATER_ENDPOINT` or the GitHub latest-release `latest.json` URL. `npm run check:release-updater` verifies the workflow configurator is present; root script tests pass 19/19. |
