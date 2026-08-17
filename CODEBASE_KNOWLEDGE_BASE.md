# Synara Desktop — Codebase Knowledge Base

> **Purpose:** Shared starting point for onboarding agents and contributors to this
> repository. Review this document before initiating new tasks or expansion work.
>
> **Last reviewed:** 2026-06-29 · **Version:** 1.2.20 · **Branch:** `main`

---

## How to Use This Document

1. Read **Executive Summary** and **Architecture** for orientation.
2. Check **Core Features** for what exists and its implementation status.
3. Review **In-Progress Work** before touching related areas.
4. Consult **Key Insights** for file paths and patterns during implementation.
5. Follow **Recommendations** and the **Cross-Platform Expansion Protocol** when
   scoping new work.

**Validation snapshot (local, 2026-06-29):**

| Gate | Result |
|------|--------|
| `npm run check:versions` | 1.2.20 consistent (Tauri 2.11 aligned) |
| `npm run test:modernization` | 294/294 pass |
| `npm run check:mip1-evidence` | 46/46 mapped on `main` |

---

## 1. Executive Summary

**Synara Desktop** is an AGPL-3.0-licensed, native-first Matrix client monorepo
optimized for secure messaging, desktop polish, Linux support, and **AI agent
workflows** (Hermes agent cards, approval flows, structured action bridge).

| Product | Stack | Role |
|---------|-------|------|
| **Desktop app** | Tauri 2 (Rust) + React/Vite runtime | macOS + Linux native shell |
| **App runtime** (`synara/`) | React 19, matrix-js-sdk, Jotai, Vite 7 | Matrix UI, sync, features |
| **iOS app** (`synara-ios/`) | SwiftUI + Matrix Rust SDK | Native mobile client (parallel track) |

### Maturity Assessment

| Area | State |
|------|-------|
| **Desktop runtime** | Feature-rich, production-oriented. Core Matrix chat, E2EE, calls, agent workflows, Later inbox, room notes, polls, forwarding, and native integrations implemented. Automated gates green. |
| **Desktop shell** | Mature Tauri integration — tray, shortcuts, notifications, keyring session store, agent-action bridge, file I/O. Hardened release builds. |
| **iOS** | Strong MVP with 209+ tests and simulator validation. **Not release-ready** — production E2EE, push gateway, physical-device validation, and App Store gates remain open. |
| **Expansion readiness** | **High** for desktop feature work within the React runtime and contract layer. **Moderate risk** for cross-platform features (must update shared JSON schemas). **Low** for full native desktop rewrite or Windows session persistence. |

The 46-item Maturity Improvement Plan (MIP1) was absorbed into `main` (evidence
check: 46/46 mapped). A stale `maturity_improvement_plan1` branch exists (28
commits not on `main`; `main` is 92 commits ahead). Interactive macOS/Linux GUI
smoke remains the primary human validation gap before treating desktop as fully
release-grade.

---

## 2. Architecture & Codebase Layout

### 2.1 Monorepo Topology

```text
synara-desktop/                    # Root — Tauri project + orchestration
├── config.json                    # Canonical homeserver/config (synced → synara/)
├── package.json                   # Tauri CLI, build scripts, version gates
├── src-tauri/                     # Rust desktop shell (~1.1k LOC in desktop.rs)
│   ├── src/{main,lib,desktop,desktop_agent_actions,desktop_file_transfer,desktop_integration,desktop_notifications,desktop_sanitize,desktop_secret_store,desktop_session,desktop_session_store,desktop_shortcuts,desktop_tray,desktop_url,build_info,menu}.rs
│   ├── capabilities/{main,release-hardening}.json
│   └── tauri.conf.json
├── synara/                        # React/Vite Matrix runtime (~865 TS/JS files)
│   └── src/{index.tsx, client/, app/, types/, util/}
├── synara-ios/                    # Native SwiftUI iOS app
│   └── Synara/{App,Features,Services,Contracts,SharedUI}/
├── devAssets/                     # Built runtime output (Tauri frontendDist)
├── scripts/                       # Build, version, layout, MIP1 evidence checks
├── packaging/arch/                # Arch/CachyOS PKGBUILD
├── docs/                          # Desktop ADRs, MIP1, validation, Linux guide
└── .github/workflows/             # CI, package smoke, release, iOS skeleton
```

**Key design decision (ADR / native-first spike):** Keep Tauri for macOS/Linux;
build iOS natively in SwiftUI. Extract **shared contracts** (JSON schemas) rather
than sharing UI code. Do **not** rewrite desktop to native before iOS ships.

See also: `docs/native-first-architecture-spike.md`, `docs/repository-layout.md`.

### 2.2 Three-Layer Desktop Architecture

```text
┌─────────────────────────────────────────────────────────┐
│  Tauri Shell (src-tauri/)                               │
│  Tray, shortcuts, notifications, keyring, file I/O,     │
│  agent-action sanitization, window lifecycle            │
└────────────────────┬────────────────────────────────────┘
                     │ window.__SYNARA_DESKTOP__ (26 IPC commands)
┌────────────────────▼────────────────────────────────────┐
│  Platform Abstraction (synara/src/app/platform/)        │
│  capabilities, badge, tray, shortcuts, sessions, files  │
└────────────────────┬────────────────────────────────────┘
                     │
┌────────────────────▼────────────────────────────────────┐
│  App Runtime (synara/src/app/)                          │
│  Matrix state (Jotai), UI features, hooks, timeline     │
│  IndexedDB sync/crypto stores, service worker media      │
└─────────────────────────────────────────────────────────┘
```

### 2.3 Entry Points

| Layer | Entry | Bootstrap |
|-------|-------|-----------|
| Desktop binary | `src-tauri/src/main.rs` → `lib.rs::run()` | Plugins, tray, `invoke_handler`, `desktop_bridge.js` injection |
| Frontend | `synara/src/index.tsx` | Session bootstrap → SW register → `<App />` |
| App shell | `synara/src/app/pages/App.tsx` | QueryClient + Jotai + Router |
| Matrix client | `synara/src/client/initMatrix.ts` | `createClient`, IndexedDB stores, rust crypto, token refresh |
| Post-auth | `synara/src/app/pages/client/ClientRoot.tsx` | `initClient` / `startClient`, `MatrixClientProvider` |
| Native wiring | `ClientNonUIFeatures.tsx` | Tray, badge, shortcuts, notifications, agent actions |
| iOS | `Synara/App/SynaraApp.swift` | `AppEnvironment`, `RootShellView`, `AppRouter` |

### 2.4 Routing

React Router v6 with optional hash routing (`config.json`). Authenticated routes:
`/home/`, `/direct/`, `/:spaceId/`, `/explore/`, `/inbox/`, `/settings/`.

Separate `synara/src/app/routes/synaraRoutes.ts` parses deep-link paths for
desktop notifications and agent actions.

### 2.5 State Management

| Layer | Technology | Notes |
|-------|------------|-------|
| Primary client state | **Jotai** | Atoms bound to Matrix events via `useBindAtoms.ts` |
| Fetch/cache | **React Query** | Devices, space hierarchy, message search (7 files) |
| Persistence | `atomWithLocalStorage`, native keyring, IndexedDB | Matrix sync/crypto in IndexedDB |
| Session | `sessionBootstrap.ts` / `sessionPersistence.ts` | Single active session with native-store first persistence and localStorage fallback |

### 2.6 Build Pipeline

1. Edit root `config.json` only.
2. `scripts/build-runtime.mjs` → Vite build in `synara/` → copies to `devAssets/`.
3. `npm run tauri build` → Rust release + platform bundles (.deb, AppImage, .app, DMG).
4. `npm run check:versions` enforces version alignment across Tauri, npm, Arch PKGBUILD, iOS.

**Local dev:** `npm run tauri dev` (starts Vite on `:8080`, Tauri loads dev URL).

---

## 3. Core Features & Functionality

Status labels: **Full** · **Partial** · **Stub** · **Planned**

### 3.1 Authentication & Session

| Feature | Status | Location |
|---------|--------|----------|
| Password login, registration, reset; desktop SSO unsupported | Full | `synara/src/app/pages/auth/` |
| Matrix UIA stages (email, reCAPTCHA, terms) | Full | `synara/src/app/components/uia-stages/` |
| Token refresh + proactive scheduling | Full | `initMatrix.ts`, `sessionPersistence.ts` |
| Native session store (macOS Keychain, Linux Secret Service) | Full | `platform/sessions.ts`, `desktop.rs`, `desktop_secret_store.rs`, `desktop_session.rs`, `desktop_session_store.rs` |
| Windows native session store | Stub | Explicitly unsupported; localStorage fallback |
| Logout + crypto store cleanup | Full | `performLogout`, `matrixLocalStores.ts` |
| Auto-discovery of homeserver | Full | `ClientRoot.tsx` |

### 3.2 Core Messaging

| Feature | Status | Location |
|---------|--------|----------|
| Room timeline (virtualized, ~2857 LOC) | Full | `features/room/RoomTimeline.tsx`; linked/opening helpers extracted to `utils/timelineLinks.ts` and `utils/timelineOpening.ts` |
| Rich composer (Slate editor) | Full | `components/editor/`, `RoomInput.tsx` |
| Reactions, edit, redact, reply | Full | `features/room/message/` |
| Threads (reply-backed UX) | Partial | UI distinct; true `m.thread` depth varies |
| Read receipts / typing indicators | Partial | Desktop has typing atoms; iOS missing |
| E2EE (matrix-js-sdk rust crypto) | Full | `initMatrix.ts`, verification flows |
| Media upload/download/thumbnails | Full | `matrix/media.ts`, upload board |
| PDF viewer | Full | `components/Pdf-viewer/` |
| Polls (create, vote, results) | Full | `utils/polls.ts`, composer `/poll` |
| GIF picker (config-gated provider) | Full | `utils/gifProvider.ts`, `GifPicker.tsx` |
| Message forwarding (multi-select, quotes) | Full | `utils/forward.ts` |
| Local composer drafts | Full | `utils/drafts.ts`, `roomInputDrafts.ts` |

### 3.3 Navigation & Organization

| Feature | Status | Location |
|---------|--------|----------|
| Home, DMs, spaces, explore | Full | `pages/client/` |
| Space lobby + hierarchy | Full | `features/lobby/` |
| Drag-and-drop space folders | Full | `in.synara.spaces` account data |
| Favorites / folder organization | Full | Sidebar hooks |
| Global search (`Cmd/Ctrl+K`, `>` action mode) | Full | `features/search/` |
| Message search (sender, date, type filters) | Full | `features/message-search/` |
| Join room by address prompt | Full | `/home/join/` route and prompt-backed sidebar entry |

### 3.4 Synara-Specific Features

| Feature | Status | Location |
|---------|--------|----------|
| **Later inbox** (snooze, due dates, completed) | Full | `utils/later.ts`, inbox views |
| **Room notes** (notes, todos, pinned messages) | Full | `utils/roomNotes.ts`, `RoomNotesPanel.tsx` |
| **Unread anchors** (private markers + divider) | Full | `utils/notifications.ts` |
| **Hermes agent cards** (code blocks, actions, copy) | Full | `HermesAgentCard.tsx`, `utils/hermes.ts` |
| **Agent approvals** (detect + notify) | Full | `utils/agentApprovals.ts` |
| **Agent action bridge** (copy/open/emit) | Full | `agents/agentActions.ts`, `desktop_agent_actions.rs` |
| Agent backend services (regenerate, export, etc.) | Planned | Bridge exists; no concrete backends |
| Theme accent customization | Full | `settings/general/General.tsx` |

### 3.5 Notifications

| Feature | Status | Location |
|---------|--------|----------|
| In-app notification center | Full | `/inbox/notifications/` |
| Desktop native notifications + click routing | Full | `ClientNonUIFeatures.tsx`, `desktop_notifications.rs` |
| Dock/taskbar badge counts | Full | `badgeSummary.ts`, `desktop_set_badge_count` |
| DND / tray state | Full | Linux tray DND toggle |
| Service worker push (browser dev) | Full | `sw.ts`, `sw-session.ts` |
| `agentApprovalCount` durable source | Partial | Notification-time detection only per contract |

### 3.6 Calls

| Feature | Status | Location |
|---------|--------|----------|
| Element Call embedded widget | Full | `plugins/call/`, `@element-hq/element-call-embedded` |
| Call status overlay | Full | `features/call-status/` |
| macOS camera/mic permission strings | Full | `tauri.conf.json` bundle metadata |

### 3.7 Settings & Admin

| Feature | Status | Location |
|---------|--------|----------|
| Account, devices, notifications, general | Full | `features/settings/` |
| Room/space settings (permissions, members, emojis) | Full | `features/room-settings/`, `space-settings/`, `common-settings/` |
| Developer tools | Full | `common-settings/developer-tools/` |
| Desktop shortcut configuration | Full | Settings + `desktop_set_shortcuts` |
| Key backup / secret storage | Full | `SecretStorage.tsx`, crypto hooks |

### 3.8 Desktop Native Shell

| Feature | Status | Location |
|---------|--------|----------|
| System tray (show/later/notifications/quit) | Full | `desktop_tray.rs` `create_tray()` |
| Close-to-tray | Full | `lib.rs` window event handler |
| Global shortcuts | Full | `tauri_plugin_global_shortcut` |
| Native file save to Downloads | Full | Streaming IPC (≤8 MiB inline, chunked above) |
| Drag-and-drop file upload | Full | Allowlist-gated native read |
| External link handling | Regressed / P0 fix needed | `desktop_open_external_url`; 2026-06-30 human smoke reports macOS/Linux clicks do not open the browser |
| Linux integration status probe | Full | `desktop_integration.rs` `desktop_get_integration_status` |
| Auto-updater | Disabled locally / release-time configured | `createUpdaterArtifacts: false` in committed config; updater plugin registration is conditional on active `plugins.updater` so local/macOS builds launch; release CI materializes signed updater config from `SYNARA_UPDATER_PUBKEY` and optional `SYNARA_UPDATER_ENDPOINT`, generates `latest.json`, and uploads signature artifacts |
| Windows packaging | Not supported | README excludes from release matrix |

### 3.9 iOS App (Parallel Product)

| Area | Status |
|------|--------|
| Auth, session, Keychain, logout/wipe | Complete |
| Room list, timeline, text send, agents, Later | Complete |
| Room management (create/join/DM) | Partial |
| Production E2EE (recovery, verification, encrypted media) | Partial (first slice only) |
| Push (APNs registration) | Partial (gateway staging blocked) |
| Composer extras (mentions, emoji, GIF, voice, polls) | Missing |
| iPad, share extension, App Intents | Deferred |

See `synara-ios/docs/ios-functionality-matrix.md` for the full capability matrix.

---

## 4. Partially Implemented, Ongoing, or In-Progress Work

### 4.1 Explicit Stubs & Incomplete Routes

**404 fallback** — still intentionally minimal. The previous `/home/join/`
placeholder has been replaced with the prompt-backed `HomeJoin` route.

### 4.2 Commented-Out / Abandoned Code

| Location | What |
|----------|------|
| `synara/src/app/plugins/react-prism/ReactPrism.tsx` | Many Prism language imports commented out (bundle size). |
| `synara/src/app/utils/ASCIILexicalTable.ts` | Debug `printLex` and perf harness commented out. |

### 4.3 Documented but Not Yet Implemented

From `synara/docs/synara-modernization-roadmap.md`:

1. **Agent backend implementations** — Connect `desktop_agent_action` to real
   regenerate/continue/export services.
2. **Native distribution hardening** — Updater metadata channel, store permission copy.
3. **Extended theme customization** — Curated themes beyond accent picker.

From `synara/docs/synara-notification-contract.md`:

- `agentApprovalCount` durable count source: not implemented yet.

From iOS docs:

- Favorite swipe / starred rooms: deferred until starred rooms API.
- ADR 0002 module split (`SynaraCore`, `SynaraMatrix`, etc.): not yet split.

### 4.4 Branch & Plan State

| Item | State |
|------|-------|
| `maturity_improvement_plan1` branch | **Stale** — 28 commits not on `main`; `main` is 92 commits ahead |
| MIP1 46/46 items | **Absorbed into `main`** via bundled commits |
| Interactive macOS/Linux smoke | **Pending** — `docs/desktop-validation-status.md` |
| Auto-updater | **Intentionally disabled in committed config** until stable signed release channel; published release CI materializes updater config from repository variables before the strict gate, then uploads signed artifacts and `latest.json` |
| Windows session persistence | **Explicitly unsupported** |

2026-06-30 smoke update:

- macOS desktop build/launch is reported working after the disabled-updater fix.
- External link opening is reported broken on both macOS and Linux desktop
  clients. Treat `desktop_open_external_url`, frontend click interception, and
  Tauri opener wiring as the next P0 investigation surface despite the feature
  table's implementation status.
- Timeline/session-history behavior is tentatively improved, but formal smoke
  evidence is still pending.

### 4.4.1 Postmortem: macOS Desktop Non-Launch, 2026-06-29

Two late-night fixes corrected validation gaps from the production-readiness
automation pass:

- `f938246` made Tauri updater plugin registration conditional on
  `plugins.updater`. The committed config intentionally disables updater
  artifacts/config, but the app must still launch locally and on macOS before
  release-time updater variables are materialized.
- `3e0bd8e` restored a missing `getFirstLinkedTimeline` import after Timeline
  helper extraction. Frontend typecheck/build must be part of any future
  Timeline/helper or accumulated versioning change.

Takeaway: compile/readiness scripts are not a substitute for launch smoke when
native plugins/config or the bundled frontend are touched.

### 4.5 iOS Code TODOs

`synara-ios/Synara/Services/MatrixRustSDKService.swift`:

- Map additional non-`ClientError` login failures when SDK exposes stable types.
- Map `M_USER_DEACTIVATED` and credential vs connectivity failures.

### 4.6 TODO/FIXME Culture

No `TODO`/`FIXME` markers in `synara/src` or `src-tauri/`. Incomplete work is
tracked in docs, contracts, and functionality matrices instead.

---

## 5. Technical Observations

### 5.1 Tech Stack

| Layer | Technologies |
|-------|-------------|
| Desktop shell | Rust 2021, Tauri 2.11, keyring 3.6, notify-rust, mac-notification-sys |
| Frontend | React 19.2, TypeScript 5.7, Vite 7.3, vanilla-extract, folds UI kit |
| Matrix | matrix-js-sdk 38.2 (desktop), Matrix Rust SDK (iOS) |
| State | Jotai 2.6, TanStack Query 5.24, TanStack Virtual 3.2 |
| Editor | Slate 0.123 |
| Calls | matrix-widget-api, Element Call embedded 0.16.3 |
| iOS | SwiftUI, XcodeGen, XCTest (209+ tests) |
| CI | GitHub Actions (ubuntu-22.04 + macos-latest) |

### 5.2 Strengths

1. **Contract-first cross-platform design** — 12 JSON schemas + fixtures in
   `synara/docs/contracts/` with conformance tests.
2. **Defense-in-depth security** — Rust sanitization for sessions, URLs, agent
   actions, notifications, routes; release DevTools denial.
3. **Strong automated testing on critical paths** — 273 modernization tests,
   session lifecycle, desktop bridge, contracts, timeline virtualization.
4. **Platform abstraction layer** — `app/platform/` cleanly separates Tauri IPC
   from feature code (~10 direct bridge consumers).
5. **Mature desktop native integration** — Tray coalescing, KDE Wayland shortcut
   fallback, Linux Secret Service probes, streaming file IPC.
6. **Documentation density** — ADRs, validation status, functionality matrices,
   MIP1 plan, iOS project spec.

### 5.3 Technical Debt & Risks

| Risk | Severity | Detail |
|------|----------|--------|
| Browser-shaped desktop runtime | Medium | IndexedDB sync/crypto, localStorage drafts, service worker media |
| matrix-js-sdk coupling | High | ~209 runtime files; Rust SDK migration = multi-month rewrite |
| Monolithic files | Medium | `RoomTimeline.tsx` (2857 LOC after helper extractions), `desktop.rs` (1142 LOC after URL, sanitization, file-transfer, session-envelope, secret-store classification/probe/cache, keyring session-store, global shortcut, tray/menu, agent-action, notification, and integration-status helper extractions) |
| Limited UI test coverage | Medium | 44 unit test files, zero `*.test.tsx` component tests |
| Windows unsupported | Low (by design) | No Keychain equivalent; excluded from release matrix |
| Stale MIP1 branch | Low | Could confuse contributors |
| iOS API instability | Medium | Matrix Rust components for Swift warn of unstable API |
| Push gateway dependency | High (iOS) | `IOS-0404` blocked on infrastructure |
| AGPL + App Store | High (iOS) | Legal review gate documented but not resolved |

### 5.4 Testing

| Suite | Count | Runner |
|-------|-------|--------|
| Modernization (TS) | 273 pass | `node:test` via esbuild bundle |
| Rust desktop | 100 pass | `cargo test` |
| Contract schemas | Included above | `contractSchemas.test.ts` |
| Timeline perf harness | Separate | `test:timeline-performance` |
| iOS unit + UI | 209+ | XCTest (local; CI compile-only) |
| E2E / Playwright | None | — |
| Interactive smoke | Manual | Documented checklists |

### 5.5 Notable Gotchas

1. **Config sync:** Edit root `config.json` only; `npm run tauri` copies to `synara/config.json`.
2. **macOS app replace:** Copying into existing `.app` nests bundles — `mv` old aside first.
3. **Linux keyutils fallback:** Session may not persist if only keyutils available.
4. **Hash router:** Enabled by default — deep links use `#/` paths.
5. **Semantic release:** Configured on `dev` branch in `synara/package.json`; desktop uses `bump-version.mjs`.
6. **Former submodule:** `synara/` is inline-tracked; no `.gitmodules`.
7. **Provenance:** AGPL-derived codebase; preserve the repository license and provenance notices.

---

## 6. Key Insights & Knowledge Base Highlights

### Architecture Patterns

- **Shell owns native; runtime owns Matrix** — Tauri never duplicates room state,
  Later, or notifications data.
- **Contracts are the cross-platform API** — Update schema + fixture before
  implementing on either platform.
- **Jotai binds Matrix events; React Query fetches** — Do not add Query for
  sync-stream data.
- **`ClientNonUIFeatures` is the desktop integration hub** — Badge, tray,
  shortcuts, notifications, agent listener.

### Critical Files (Quick Reference)

| Concern | File |
|---------|------|
| Matrix init | `synara/src/client/initMatrix.ts` |
| Session persistence | `synara/src/app/state/sessionPersistence.ts` |
| Desktop bridge (TS) | `synara/src/app/utils/desktop.ts` |
| Desktop bridge (Rust) | `src-tauri/src/desktop.rs` |
| Desktop agent actions (Rust) | `src-tauri/src/desktop_agent_actions.rs` |
| Desktop file-transfer helpers (Rust) | `src-tauri/src/desktop_file_transfer.rs` |
| Desktop integration status (Rust) | `src-tauri/src/desktop_integration.rs` |
| Desktop notifications (Rust) | `src-tauri/src/desktop_notifications.rs` |
| Desktop sanitization helpers (Rust) | `src-tauri/src/desktop_sanitize.rs` |
| Desktop secret-store status/classification contracts (Rust) | `src-tauri/src/desktop_secret_store.rs` |
| Desktop session-envelope policy (Rust) | `src-tauri/src/desktop_session.rs` |
| Desktop session persistence store (Rust) | `src-tauri/src/desktop_session_store.rs` |
| Desktop global shortcuts (Rust) | `src-tauri/src/desktop_shortcuts.rs` |
| Desktop tray/menu state (Rust) | `src-tauri/src/desktop_tray.rs` |
| Desktop URL policy (Rust) | `src-tauri/src/desktop_url.rs` |
| Platform API | `synara/src/app/platform/index.ts` |
| Route parser | `synara/src/app/routes/synaraRoutes.ts` |
| Agent cards | `synara/src/app/components/hermes/HermesAgentCard.tsx` |
| Later | `synara/src/app/utils/later.ts` |
| Room notes | `synara/src/app/utils/roomNotes.ts` |
| Settings | `synara/src/app/state/settings.ts` |
| Build | `scripts/build-runtime.mjs` |
| Version gate | `scripts/check-version-consistency.mjs` |

### Matrix Account Data Namespaces

Documented in `synara/docs/synara-namespaces.md`:

| Namespace | Purpose |
|-----------|---------|
| `in.synara.later` | Later inbox items |
| `in.synara.spaces` | Space folder ordering |
| `in.synara.room_notes` | Per-room notes/todos/pins |
| `in.synara.unread_anchor` | Private unread markers |
| Hermes agent card event types | Configured structured content keys |

### IPC Command Surface

26 commands, all prefixed `desktop_*`. Registered in `src-tauri/src/lib.rs`
`invoke_handler`. Frontend accesses via `window.__SYNARA_DESKTOP__.invoke()`.

Categories: window/navigation, tray, notifications, shortcuts, session/keyring,
external URLs, file save, drag-and-drop read, agent actions, performance metadata.

### CI Gates (must pass before merge)

```text
check:repo-layout → check:versions → check:matrix-boundaries
  → cargo check/test → typecheck:modernization → test:modernization (294)
  → eslint → prettier
```

### Related Documentation Index

| Document | Path |
|----------|------|
| Contract inventory | `synara/docs/synara-contracts.md` |
| Modernization roadmap | `synara/docs/synara-modernization-roadmap.md` |
| Desktop integration contract | `docs/desktop-modernization.md` |
| Native-first architecture | `docs/native-first-architecture-spike.md` |
| Production smoke checklist | `docs/production-smoke-checklist.md` |
| Desktop validation status | `docs/desktop-validation-status.md` |
| MIP1 plan | `docs/maturity_improvement_plan1.md` |
| iOS functionality matrix | `synara-ios/docs/ios-functionality-matrix.md` |
| iOS project spec | `synara/docs/synara-ios-project-spec.md` |
| Linux build guide | `docs/linux.md` |
| ADR: iOS layout | `docs/adr/0001-ios-repository-layout.md` |
| ADR: iOS architecture | `docs/adr/0002-ios-architecture.md` |
| ADR: shared native Rust core | `docs/adr/0003-shared-native-rust-core.md` |
| ADR: Rust language boundaries | `docs/adr/0004-rust-language-boundaries.md` |

---

## 7. Recommendations for Expansion

### 7.1 Safest Areas to Extend (Low Risk, High Value)

1. **Synara-specific account-data features** — Room notes, Later, unread anchors
   already have contracts, tests, and UI.
2. **Agent workflow depth** — Wire `desktop_agent_action` / `synara://agent-action`
   to backend services (roadmap item #1).
3. **Search & inbox polish** — Message search filters, notification center, Later views.
4. **Desktop native surfaces** — New tray items, shortcuts, or notification types
   follow established `desktop.rs` sanitization patterns.
5. **Contract additions** — Start with JSON schema + fixture in `synara/docs/contracts/`,
   then TS utils with tests, then iOS Swift mirror.

### 7.2 Refactoring Before Major Expansion

1. **`RoomTimeline.tsx` decomposition** — First slices extracted linked-timeline
   helpers to `utils/timelineLinks.ts` and opening/window/unread helpers to
   `utils/timelineOpening.ts`; continue extracting sub-modules before adding
   timeline features.
2. **`desktop.rs` modularization** — First slices extracted external-link,
   session-base, and agent URL policy helpers to `src-tauri/src/desktop_url.rs`,
   shared text/route sanitization helpers to `src-tauri/src/desktop_sanitize.rs`,
   file-transfer policy helpers, save/drop IPC commands, transfer-session
   state, and drag/drop allowlist lifecycle to
   `src-tauri/src/desktop_file_transfer.rs`,
   and session-envelope validation/expiry helpers to
   `src-tauri/src/desktop_session.rs`, secret-store status/backend/error
   classification plus platform probe/cache mechanics to
   `src-tauri/src/desktop_secret_store.rs`, keyring session persistence flow to
   `src-tauri/src/desktop_session_store.rs`, global shortcut
   policy/registration to `src-tauri/src/desktop_shortcuts.rs`, and tray/menu
   state, badge, DND, and tray creation handling to
   `src-tauri/src/desktop_tray.rs`, agent-action payload sanitization,
   local handling, and event emission to
   `src-tauri/src/desktop_agent_actions.rs`, notification payload
   validation, permission commands, and route-click dispatch to
   `src-tauri/src/desktop_notifications.rs`, and Linux/KDE/session/portal
   integration status probes to `src-tauri/src/desktop_integration.rs`, all
   with direct Rust tests. `desktop.rs` is now a 236 LOC command/window shell;
   continue splitting only if future desktop surfaces grow beyond focused
   modules.

### 7.3 Cross-Platform Expansion Protocol

```text
1. Update human contract (.md) + JSON schema + fixture
2. Implement desktop owner (synara/src/app/utils/*.ts) + test
3. Update contractSchemas.test.ts
4. Mirror in synara-ios/Synara/Contracts/ + SynaraContractsTests
5. Implement iOS service adapter
6. Update ios-functionality-matrix.md status
```

### 7.4 Risks to Watch

| Risk | Mitigation |
|------|------------|
| Breaking Tauri IPC signatures | Backward-compatible unless explicitly documented |
| Schema drift between platforms | Fixture conformance tests on both sides |
| Timeline performance regression | Run `test:timeline-performance` after timeline changes |
| Linux DE variance | Test on CachyOS/KDE Wayland, not just Ubuntu CI |
| iOS SDK API changes | Pin SDK versions; monitor matrix-rust-components-swift |

### 7.5 Prioritized Next Steps

| Priority | Action | Rationale |
|----------|--------|-----------|
| **P0** | Complete interactive macOS + Linux smoke checklists | Last human gate before release-grade desktop |
| **P1** | Agent backend service connections | Highest product differentiation; bridge ready |
| **P1** | Decompose `RoomTimeline.tsx` | Prerequisite for safe timeline expansion |
| **P2** | Enable auto-updater with signed metadata channel | Required for distribution at scale |
| **P2** | iOS push gateway deployment (`IOS-0404`) | Unblocks real-device notification validation |
| **P2** | iOS production E2EE | App Store blocker |
| **P3** | Archive stale `maturity_improvement_plan1` branch | Reduce contributor confusion |
| **P3** | Add component/integration tests for auth + room flows | Largest coverage gap |
| **P4** | Windows session store evaluation | Only if Windows enters release matrix |

---

## Appendix: Feature Module Map (`synara/src/app/features/`)

| Module | Purpose |
|--------|---------|
| `add-existing/` | Add existing rooms to a space |
| `call/` | In-call UI (Element Call widget) |
| `call-status/` | Floating call status chip/controls |
| `common-settings/` | Shared room/space settings panels |
| `create-chat/` | Start a DM |
| `create-room/` | Create room modal + form |
| `create-space/` | Create space modal + form |
| `join-before-navigate/` | Join room before navigation |
| `lobby/` | Space lobby/hierarchy |
| `message-search/` | In-room/global message search |
| `room/` | Core chat room experience |
| `room-nav/` | Room navigation items |
| `room-settings/` | Room settings modal |
| `search/` | Global search modal |
| `settings/` | User settings modal |
| `space-settings/` | Space settings modal |

---

*This document should be updated when major architectural decisions land, version
milestones ship, or significant feature areas change status. When updating, revise
the header metadata (date, version) and validation snapshot.*
