# Synara Desktop Production Readiness Goal

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
- Interactive smoke checklists signed off by a human.
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
| KB version | 1.2.19, last reviewed 2026-06-29 | `CODEBASE_KNOWLEDGE_BASE.md` |
| Desktop maturity per KB | Production-oriented, automated gates previously green | KB validation snapshot |
| iOS maturity per KB | Strong MVP, not release-ready | KB maturity table |

## Prioritized Backlog

### Priority Model

The user-declared pain points are **release P0 epics** because they are current client-facing blockers. The KB §7.5 **P0 interactive macOS/Linux smoke checklists** remain a mandatory release gate and run in parallel with implementation; they are not deprioritized by the pain-point epics.

### Epic 1: Timeline Resurrection

**Priority:** P0  
**Status:** In progress - desktop first slice implemented
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

Desktop first-slice finding, 2026-06-29:

- Root cause: desktop kept a process-local saved timeline viewport for non-bottom positions and restored it before unread/read-marker placement. A user who had previously viewed old history could later reopen a room and force `RoomTimeline` back through that old event window even if the room had one new unread event or had been fully read elsewhere. This mixed UI viewport memory with SDK read-marker/timeline focus semantics.
- Implemented mitigation: `RoomTimeline` now records viewport age, uses a tested `shouldRestoreRoomTimelineViewport(...)` policy, lets unread state and synchronous loaded-slice unread inference win over saved history, and expires non-bottom historical anchors after a 10 minute local-navigation window. Bottom snapshots still open at the live end.
- Remaining gap: iOS already has read-marker initial-load focus policy, but the shared contract and live smoke matrix still need to be formalized and verified across desktop/iOS.

Shared contract update, 2026-06-29:

- Added `docs/timeline-open-focus-contract.md` with ownership rules, desktop/iOS behavior matrix, and a smoke checklist covering fully-read rooms, unread rooms, stale saved history, jump-latest, stale sync state, live appends, and timeline reset/gap cases.
- Updated `docs/matrix-sdk-lifecycle-semantics-audit.md` to point at the shared contract and record the desktop stale-viewport hardening.
- Expanded `TimelineServiceTests` policy coverage for iOS normal room updates after read-marker initial focus and jump-latest override back to live stream.

### Epic 2: Link Opening

**Priority:** P0  
**Status:** In progress - desktop bridge hardening first slice implemented
**Success criteria:** Every user-visible external link from rich text, message content, Hermes cards, and related surfaces opens through the system browser via `desktop_open_external_url` on desktop, with safe fallback behavior outside Tauri.

Initial task backlog:

1. Inventory all anchor/link rendering paths. **Status:** First pass complete; remaining direct `window.open` production path is SSO popup handoff, intentionally exempt because it relies on `postMessage`.
2. Centralize link activation through platform link helpers. **Status:** Desktop helper now routes normal external opens through `desktop_open_external_url`; non-desktop fallback remains browser-native.
3. Cover rich text, Matrix HTML, Hermes cards, room topics, and attachment/link previews. **Status:** Global anchor interceptor remains active for rendered anchors; Hermes cards, profile/server actions, OIDC account-management actions, registration terms, and feature-check anchors were wired explicitly.
4. Add tests for desktop bridge invocation and fallback. **Status:** Added regression tests for desktop bridge use, unsafe desktop no-fallback behavior, modified-click preservation, relative-link preservation, and agent-action no-fallback behavior.
5. Smoke check external URL opening on macOS/Linux. **Status:** Queued in `MACOS_IOS_VALIDATION_QUEUE.md`.

Desktop first-slice finding, 2026-06-29:

- Root cause: most anchors were protected by the global desktop click interceptor, but several programmatic link opens and confirmation flows still called `window.open` directly. Hermes confirmed URL/artifact opens were the main P0 gap because they bypassed `desktop_open_external_url` entirely.
- Implemented mitigation: introduced a shared `openExternalUrl(...)` helper for normal external opens. Desktop calls use `openPlatformExternalUrl`/`desktop_open_external_url` with no browser fallback if the bridge rejects the URL; non-desktop keeps `window.open(..., 'noopener,noreferrer')`. Explicit link handlers now preserve modifier-click and app-relative native behavior.
- Remaining gap: live desktop smoke on macOS/Linux is still required for rich text, message links, Hermes cards, settings/about links, profile/server links, OIDC account-management links, and location links.

### Epic 3: Composer Desktop Parity

**Priority:** P0  
**Status:** In progress - desktop clipboard image first slice implemented
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
- Remaining gap: live desktop smoke is still required for native spellcheck, drag/drop files, clipboard screenshot paste, and mixed image+HTML clipboard sources.

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

1. Decompose `RoomTimeline.tsx`.
2. Modularize `desktop.rs`.
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

#### 7.5 Prioritized Next Steps

1. Complete interactive macOS + Linux smoke checklists.
2. Connect agent backend services to existing bridge.
3. Decompose `RoomTimeline.tsx`.
4. Modularize `desktop.rs`.
5. Enable auto-updater with signed metadata channel.
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
4. Complete updater signing and release-channel documentation. **Status:** Release guard and check-only updater plugin scaffolding added; production updater enablement remains blocked on real endpoint/key material, signing secrets, artifact generation, and release workflow upload.
5. Refresh `README.md`, release checklist, and user-facing installation docs.
6. Confirm license, privacy, and security docs are current.

## Validation Gates

| Gate | Required Before Release | Latest Phase 0 Result | Notes |
|---|---:|---|---|
| `npm run check:versions` | Yes | Pass, 2026-06-29 | Version metadata consistent at 1.2.19; Tauri aligned at 2.11 |
| `npm run check:repo-layout` | Yes | Pass, 2026-06-29 | Repository layout canonical |
| `npm run check:matrix-boundaries` | Yes | Pass, 2026-06-29 | Passed with 2 active iOS and 2 active desktop exceptions |
| `npm run check:mip1-evidence` | Yes | Pass, 2026-06-29 | 46/46 mapped; warning about bundled history retained |
| `npm run check:runtime-assets` | Yes | Pass, 2026-06-29 | Runtime assets match `synara/dist` |
| `npm run typecheck:modernization` | Yes | Pass, 2026-06-29 | TypeScript modernization surface compiles |
| `npm run test:modernization` | Yes | Pass, 2026-06-29 | Latest root build/test passed 285/285 after join-route path helper coverage |
| `npm --prefix synara run check:eslint` | Yes | Pass, 2026-06-29 | Frontend lint clean |
| `npm --prefix synara run check:prettier` | Yes | Pass after mechanical format fix, 2026-06-29 | Four pre-existing formatting drifts corrected |
| `npm run check:release-updater` | Yes | Advisory pass with 8 warnings, 2026-06-29 | Plugin/package/check-only capability wiring is present; release metadata/channel prerequisites remain disabled |
| `npm run check:release-updater -- --require-enabled` | Required for published releases | Expected fail, 2026-06-29 | Release workflow now runs this gate; it fails until updater artifacts, signed metadata, plugin wiring, and release signing secrets are configured |
| `cargo check` in `src-tauri` | Yes | Pass, 2026-06-29 | Rust compile gate clean |
| `cargo test` in `src-tauri` | Yes | Pass, 2026-06-29 | 90/90 Rust tests passed |
| `npm --prefix synara run test:timeline-performance` | Required for timeline changes | Pass, 2026-06-29 | Latest rerun: 10k events 5.05 ms; 50k events 18.08 ms |
| iOS tests | Required for iOS release | Queued for macOS workstation | `xcodebuild` and `swift` unavailable on this Linux host; see `MACOS_IOS_VALIDATION_QUEUE.md` |

## Status Table

| Phase | Status | Evidence | Next Step |
|---|---|---|---|
| Phase 0: Initializer | Committed and pushed | Current `HEAD` includes harness, validation evidence, changelog, and mechanical Prettier drift fix | Continue Phase 1 source reconciliation |
| Phase 1: Validation & Reconciliation | In progress; externally blocked for full reconciliation | KB §7 read fully; Grok/composer-2.5-fast second opinion completed from pasted excerpts; original prompt, Grok Executive Report, and prior Section 7 not found locally | Commit KB Section 7 reconciliation edits, then continue repo/source validation |
| Phase 2: Timeline Resurrection | In progress | Desktop viewport restore policy first slice implemented; shared open-focus contract added; iOS policy tests expanded; macOS/iOS validation handoff is tracked in `MACOS_IOS_VALIDATION_QUEUE.md` | Run iOS tests on macOS/Xcode, then execute smoke checklist |
| Phase 3: Link Opening | In progress | Desktop bridge hardening first slice implemented; Grok reviewed the diff and its findings were incorporated; local gates pass at 281/281 | Run macOS/Linux interactive link-opening smoke checklist |
| Phase 4: Composer Parity | In progress | Clipboard image paste first slice and drag/drop detection hardening implemented; Grok reviewed both diffs and findings were incorporated/accepted; local gates pass at 283/283 | Run macOS/Linux interactive composer smoke checklist |
| Phase 5: Release Readiness | In progress | Added release-updater readiness checker; wired published desktop release workflow to fail until signed updater channel prerequisites are configured; added desktop updater plugin/package/check-only permission scaffolding; completed `_JOIN_PATH` route stub; removed legacy commented session atom | Complete real updater config/channel/signing workflow work, then run macOS signing/updater verification |

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
| 2026-06-29T10:31:32-04:00 | Accepted `_JOIN_PATH` route completion. | Replaced the `/home/join/` placeholder with the shared join-address prompt route, centralized room path generation with `viaServers`, routed the Home sidebar join action through the canonical route, and added modernization coverage for encoded room/event/via URL construction. |
| 2026-06-29T10:38:00-04:00 | Accepted session layer dead-code cleanup. | Removed the commented legacy Jotai `sessionsAtom` implementation from `sessions.ts`; active session ownership remains with `sessionBootstrap.ts`, `sessionPersistence.ts`, native secret storage, and the tested localStorage fallback path. |
