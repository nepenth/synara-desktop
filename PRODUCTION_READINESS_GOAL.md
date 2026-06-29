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

### Epic 1: Timeline Resurrection

**Priority:** P0  
**Status:** Not started  
**Success criteria:** Fully-read rooms and rooms with one new message open near the expected live/read position without multi-second traversal through old history on desktop and iOS.

Initial task backlog:

1. Audit desktop timeline ownership in `RoomTimeline.tsx`, `timelineLifecycle.ts`, `timelinePagination.ts`, and `timelineVirtualization.ts`.
2. Audit iOS timeline focus behavior in `RoomTimelineView.swift`, `TimelineService.swift`, and related read-marker services.
3. Reconcile Matrix semantics for `m.fully_read`, read receipts, live timelines, timeline gaps, `matrix-js-sdk` `getLatestTimeline`, and Matrix Rust SDK timeline focus.
4. Define a shared room-open focus contract covering:
   - Fully-read room.
   - Room with one or few unread events.
   - Saved historical viewport.
   - Explicit jump-to-event.
   - Timeline reset or gap.
5. Implement narrowly scoped desktop fix with tests and timeline performance harness.
6. Mirror contract and behavior in iOS with tests.
7. Add smoke checklist evidence for large, encrypted, and unread room cases.

### Epic 2: Link Opening

**Priority:** P0  
**Status:** Not started  
**Success criteria:** Every user-visible external link from rich text, message content, Hermes cards, and related surfaces opens through the system browser via `desktop_open_external_url` on desktop, with safe fallback behavior outside Tauri.

Initial task backlog:

1. Inventory all anchor/link rendering paths.
2. Centralize link activation through platform link helpers.
3. Cover rich text, Matrix HTML, Hermes cards, room topics, and attachment/link previews.
4. Add tests for desktop bridge invocation and fallback.
5. Smoke check external URL opening on macOS/Linux.

### Epic 3: Composer Desktop Parity

**Priority:** P0  
**Status:** Not started  
**Success criteria:** Slate composer supports native spell checking, drag-and-drop files, and clipboard image paste identically on desktop.

Initial task backlog:

1. Audit existing Slate editor, `RoomInput.tsx`, `RoomComposer.tsx`, `useFileDrop`, upload board, and paste handlers.
2. Enable native spellcheck without breaking Matrix markdown/rich text behaviors.
3. Wire drag/drop files into existing upload state.
4. Wire clipboard image paste into upload flow.
5. Add unit/integration tests and desktop smoke checklist.

### Epic 4: KB Section 7 Expansion Items

**Priority:** P1-P4  
**Status:** Not started

Backlog imported from `CODEBASE_KNOWLEDGE_BASE.md` sections 7.1-7.5:

1. Complete interactive macOS + Linux smoke checklists.
2. Implement join route (`_JOIN_PATH`).
3. Connect agent backend services to existing bridge.
4. Decompose `RoomTimeline.tsx`.
5. Modularize `desktop.rs`.
6. Enable auto-updater with signed metadata channel.
7. Deploy iOS push gateway.
8. Implement iOS production E2EE.
9. Archive stale `maturity_improvement_plan1` branch.
10. Add component/integration tests for auth and room flows.
11. Evaluate Windows session store only if Windows enters the release matrix.

### Epic 5: Production Release Operations

**Priority:** P1-P2  
**Status:** Not started

1. Verify version consistency and release metadata.
2. Validate signing/notarization path for macOS.
3. Validate Linux packaging and CachyOS/KDE Wayland smoke.
4. Complete updater signing and release-channel documentation.
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
| `npm run test:modernization` | Yes | Pass, 2026-06-29 | Runtime build completed; 273/273 tests passed |
| `npm --prefix synara run check:eslint` | Yes | Pass, 2026-06-29 | Frontend lint clean |
| `npm --prefix synara run check:prettier` | Yes | Pass after mechanical format fix, 2026-06-29 | Four pre-existing formatting drifts corrected |
| `cargo check` in `src-tauri` | Yes | Pass, 2026-06-29 | Rust compile gate clean |
| `cargo test` in `src-tauri` | Yes | Pass, 2026-06-29 | 90/90 Rust tests passed |
| `npm --prefix synara run test:timeline-performance` | Required for timeline changes | Pass, 2026-06-29 | 10k events: 5.03 ms; 50k events: 17.31 ms |
| iOS tests | Required for iOS release | Not run in Phase 0 | `xcodebuild` unavailable on this Linux host |

## Status Table

| Phase | Status | Evidence | Next Step |
|---|---|---|---|
| Phase 0: Initializer | Committed | Current `HEAD` includes harness, validation evidence, changelog, and mechanical Prettier drift fix | Push commit, then begin Phase 1 source reconciliation |
| Phase 1: Validation & Reconciliation | Blocked pending source artifacts | Grok report and prior Section 7 not found locally | Locate/report missing context, then reconcile |
| Phase 2: Timeline Resurrection | Not started | N/A | Begin after Phase 1 baseline |
| Phase 3: Link Opening | Not started | N/A | Pick scoped link inventory/fix |
| Phase 4: Composer Parity | Not started | N/A | Pick scoped composer audit/fix |
| Phase 5: Release Readiness | Not started | N/A | Updater, smoke, signing, docs |

## Decision Log

| Timestamp | Decision | Rationale |
|---|---|---|
| 2026-06-29T09:13:15-04:00 | Created initial goal file with missing-source placeholders. | Required artifacts are not all present locally; goal state must distinguish verified repo evidence from unavailable report context. |
| 2026-06-29T09:16:44-04:00 | Accepted Phase 0 validation baseline with one local limitation. | Desktop gates are green after mechanical Prettier correction; iOS simulator gate cannot run because `xcodebuild` is unavailable on this host. |
| 2026-06-29T09:17:59-04:00 | Created Phase 0 harness commit. | Initial commit `3f19dd2` used requested message `chore: initialize Codex Orchestrator v2 harness + living goal files`; evidence entry was amended into the same commit. |
| 2026-06-29T09:18:24-04:00 | Amended Phase 0 commit evidence. | `devAssets/index.html` remains an unstaged pre-existing generated hash drift. The exact final SHA is intentionally not embedded because amending the file changes the commit object. |
| 2026-06-29T09:18:55-04:00 | Stabilized commit evidence wording. | Removed self-referential final-SHA wording from committed artifacts; use `git rev-parse --short HEAD` for the current commit id. |
