# MIP1 Commit Evidence Map

**Branch:** `maturity_improvement_plan1`  
**Base:** `main`  
**Recorded:** 2026-06-10  
**Purpose:** Reconcile plan commit hygiene (47 commits: `mip1-00` + `mip1-01`…`mip1-46`) with actual branch history when work was bundled.

## Summary

| Metric | Plan | Actual (2026-06-10) |
|--------|------|---------------------|
| Total commits `main..HEAD` | 47 | **26** |
| Plan commits (`mip1-00`) | 1 | **4** (`ecb405e`, `fb92b2f`, `1f79559`, `9d89ae5`) |
| Implementation commits (`mip1-01`…`mip1-46`) | 46 | **22** subject lines; **46/46 items** evidenced |
| Dedicated 1:1 item commits | 46 | **22** |
| Bundled items (shared commit) | 0 | **24** across **6** mega-commits |

### Bundling clusters

| Commit | Subject | Items bundled in body |
|--------|---------|------------------------|
| `2ec455d` | mip1-05 | MIP1-05, MIP1-06, MIP1-07 |
| `e893b2b` | mip1-08 | MIP1-08, MIP1-11 |
| `0011eb7` | mip1-19 | MIP1-19, MIP1-20 |
| `d1aa30b` | mip1-21 | MIP1-21, MIP1-24, MIP1-25, MIP1-26, MIP1-27, MIP1-28, MIP1-29 |
| `dfbd2d3` | mip1-30 | MIP1-30, MIP1-31, MIP1-32, MIP1-33 |
| `f9da50f` | mip1-34 | MIP1-34, MIP1-35, MIP1-36, MIP1-37, MIP1-38, MIP1-39, MIP1-41 |
| `40ac646` | mip1-40 | MIP1-42, MIP1-43, MIP1-44, MIP1-45, MIP1-46 (subject also claims MIP1-40; doc diff in `f9da50f`/`9d89ae5`) |

**Note:** MIP1-41 (`linux.md`) landed in `f9da50f` (file diff); `40ac646` commit message also claims MIP1-41 but does not touch `docs/linux.md`.

### Quick reference table

| ID | Title (short) | Commit(s) | Kind |
|----|---------------|-----------|------|
| MIP1-01 | DevTools gate | `74414a7` | dedicated |
| MIP1-02 | Bridge capabilities | `20fd678` | dedicated |
| MIP1-03 | CSP tighten | `a7d871b` | dedicated |
| MIP1-04 | Windows honesty | `9edd959` | dedicated |
| MIP1-05 | Notification route | `2ec455d` | bundled |
| MIP1-06 | Tray DND | `2ec455d` | bundled |
| MIP1-07 | Agent-action listener | `2ec455d` | bundled |
| MIP1-08 | Keyutils fix | `e893b2b` | bundled |
| MIP1-09 | Native store UI | `bd8392a` | dedicated |
| MIP1-10 | Secret Service probe | `3bc458b` | dedicated |
| MIP1-11 | macOS Keychain probe | `e893b2b` | bundled |
| MIP1-12 | Secret-store errors | `08391aa` | dedicated |
| MIP1-13 | Selective logout | `da72fe1` | dedicated |
| MIP1-14 | SW session push | `faaac52` | dedicated |
| MIP1-15 | Unified logout | `a8b4798` | dedicated |
| MIP1-16 | Clear secret keys | `43ace8a` | dedicated |
| MIP1-17 | Account switch | `0c43203` | dedicated |
| MIP1-18 | Incremental timeline | `54d54d4` | dedicated |
| MIP1-19 | Stream file IPC | `0011eb7` | bundled |
| MIP1-20 | Drop allowlist | `0011eb7` | bundled |
| MIP1-21 | Throttle tray | `d1aa30b` | bundled |
| MIP1-22 | Notification LRU | `dfc76c1` | dedicated |
| MIP1-23 | Focus timeout | `226053c` | dedicated |
| MIP1-24 | Atomic shortcuts | `d1aa30b` | bundled |
| MIP1-25 | Single shortcut path | `d1aa30b` | bundled |
| MIP1-26 | Port fallback | `d1aa30b` | bundled |
| MIP1-27 | Badge clamp | `d1aa30b` | bundled |
| MIP1-28 | External URL policy | `d1aa30b` | bundled |
| MIP1-29 | Session expiry | `d1aa30b`, `0c43203` | bundled |
| MIP1-30 | Sync splash timeout | `dfbd2d3` | bundled |
| MIP1-31 | Pagination errors | `dfbd2d3` | bundled |
| MIP1-32 | Invoke strictness | `dfbd2d3` | bundled |
| MIP1-33 | Sync status copy | `dfbd2d3` | bundled |
| MIP1-34 | Shortcut help | `f9da50f` | bundled |
| MIP1-35 | Tray parity doc | `f9da50f` | bundled |
| MIP1-36 | Arch depends | `f9da50f` | bundled |
| MIP1-37 | Standalone .desktop | `f9da50f` | bundled |
| MIP1-38 | config.json sync | `f9da50f` | bundled |
| MIP1-39 | CI hardening | `f9da50f` | bundled |
| MIP1-40 | Validation docs | `f9da50f`, `9d89ae5` | bundled |
| MIP1-41 | linux.md fixes | `f9da50f` | bundled |
| MIP1-42 | pkgrel bump | `40ac646` | bundled |
| MIP1-43 | Repo URL normalize | `40ac646` | bundled |
| MIP1-44 | Refresh token | `40ac646` | bundled |
| MIP1-45 | macOS signing docs | `40ac646` | bundled |
| MIP1-46 | Spellcheck logging | `40ac646` | bundled |

---

## Per-item evidence (machine-checked)

The blocks below are parsed by `scripts/check-mip1-commit-evidence.mjs`.

<!-- mip1-evidence:MIP1-01
commits: 74414a7
kind: dedicated
title: Gate DevTools to debug builds only
evidence:
  - file: src-tauri/Cargo.toml
    note: devtools feature limited to debug/dev profile
  - file: src-tauri/build.rs
    note: release build omits devtools capability generation
-->

<!-- mip1-evidence:MIP1-02
commits: 20fd678
kind: dedicated
title: Runtime-accurate desktop bridge capabilities
evidence:
  - file: src-tauri/src/desktop_bridge.js
    note: conservative default flags; no hardcoded secure store
  - file: synara/src/app/platform/capabilities.ts
    note: runtime status before advertising persistence
  - file: synara/src/app/platform/secrets.ts
    note: bridge flags aligned with desktop_secret_store_status
-->

<!-- mip1-evidence:MIP1-03
commits: a7d871b
kind: dedicated
title: Tighten Content-Security-Policy
evidence:
  - file: src-tauri/tauri.conf.json
    note: narrowed CSP directives
  - file: docs/desktop-matrix-sdk-boundaries.md
    note: documented federation/media/call exceptions
-->

<!-- mip1-evidence:MIP1-04
commits: 9edd959
kind: dedicated
title: Windows session storage honesty (Option A)
evidence:
  - file: README.md
    note: Windows native persistence documented as unsupported
  - file: src-tauri/src/desktop.rs
    note: windows cfg returns can_persist_session false
  - file: synara/src/app/platform/capabilities.ts
    note: UI reflects Windows limitation
-->

<!-- mip1-evidence:MIP1-05
commits: 2ec455d
kind: bundled
title: Notification click navigates to sanitized route
evidence:
  - file: src-tauri/src/desktop.rs
    note: notification click handler + sanitize_notification_route
  - file: synara/src/app/utils/desktop.ts
    note: route passed on desktop_notify call sites
  - file: synara/src/app/pages/client/ClientNonUIFeatures.tsx
    note: message/Later/agent notifications include route
-->

<!-- mip1-evidence:MIP1-06
commits: 2ec455d
kind: bundled
title: Tray Do Not Disturb toggle works
evidence:
  - file: src-tauri/src/desktop.rs
    note: MENU_DND_TOGGLE handler + tray_dnd_toggle_dispatch_script
  - file: synara/src/app/platform/tray.ts
    note: synara-tray-dnd-toggle event listener
  - file: synara/src/app/pages/client/ClientNonUIFeatures.tsx
    note: DND state wired to notification suppression
-->

<!-- mip1-evidence:MIP1-07
commits: 2ec455d
kind: bundled
title: Frontend listens for synara://agent-action
evidence:
  - file: synara/src/app/platform/agentActions.ts
    note: registerPlatformAgentActionListener on synara://agent-action
  - file: synara/src/app/platform/__tests__/agentActions.test.ts
    note: contract validation + listen registration tests
-->

<!-- mip1-evidence:MIP1-08
commits: e893b2b
kind: bundled
title: Fix Linux keyutils detection false positive
evidence:
  - file: src-tauri/src/desktop.rs
    note: keyring round-trip probe; Secret Service preferred
-->

<!-- mip1-evidence:MIP1-09
commits: bd8392a
kind: dedicated
title: Surface nativeStoreError in Settings UI
evidence:
  - file: synara/src/app/features/settings/general/General.tsx
    note: native store warning section
  - file: synara/src/app/state/sessionBootstrap.ts
    note: nativeStoreError propagation
-->

<!-- mip1-evidence:MIP1-10
commits: 3bc458b
kind: dedicated
title: Live Secret Service probe
evidence:
  - file: src-tauri/src/desktop.rs
    note: dbus keyring read/write probe with timeout + mock trait
-->

<!-- mip1-evidence:MIP1-11
commits: e893b2b
kind: bundled
title: macOS Keychain probe
evidence:
  - file: src-tauri/src/desktop.rs
    note: macos_keychain_probe + distinct locked/denied/unavailable codes
-->

<!-- mip1-evidence:MIP1-12
commits: 08391aa
kind: dedicated
title: Structured secret-store error reporting
evidence:
  - file: src-tauri/src/desktop.rs
    note: stable public error codes; no token echo in IPC
  - file: synara/src/app/platform/secrets.ts
    note: TS types for new error codes
-->

<!-- mip1-evidence:MIP1-13
commits: da72fe1
kind: dedicated
title: Selective logout preserves user settings
evidence:
  - file: synara/src/app/state/sessions.ts
    note: clearLoginData removes session keys only
  - file: synara/src/client/initMatrix.ts
    note: removed localStorage.clear from logout paths
-->

<!-- mip1-evidence:MIP1-14
commits: faaac52
kind: dedicated
title: Push session to service worker after login
evidence:
  - file: synara/src/app/pages/auth/login/loginUtil.ts
    note: pushSessionToSW after persistAuthenticatedSession
  - file: synara/src/sw-session.ts
    note: SW session envelope handling
-->

<!-- mip1-evidence:MIP1-15
commits: a8b4798
kind: dedicated
title: Unify logout code paths
evidence:
  - file: synara/src/client/initMatrix.ts
    note: shared performLogout helper
  - file: synara/src/app/state/__tests__/performLogout.test.ts
    note: tray/dialog/SessionLoggedOut parity tests
-->

<!-- mip1-evidence:MIP1-16
commits: 43ace8a
kind: dedicated
title: Clear secret storage keys on logout
evidence:
  - file: synara/src/client/initMatrix.ts
    note: clearSecretStorageKeys in unified logout
  - file: synara/src/app/matrix/__tests__/secretStorageKeys.test.ts
    note: idempotent clear tests
-->

<!-- mip1-evidence:MIP1-17
commits: 0c43203
kind: dedicated
title: Account switch safety for fixed IndexedDB names
evidence:
  - file: synara/src/app/state/sessionPersistence.ts
    note: identity comparison + matrix local store clear on mismatch
-->

<!-- mip1-evidence:MIP1-18
commits: 54d54d4
kind: dedicated
title: Incremental timeline row building
evidence:
  - file: synara/src/app/utils/timelineVirtualization.ts
    note: incremental row build for append-only updates
  - file: synara/src/app/utils/__tests__/timelineVirtualization.test.ts
    note: virtualization regression suite
-->

<!-- mip1-evidence:MIP1-19
commits: 0011eb7
kind: bundled
title: Stream large file save/drop IPC
evidence:
  - file: src-tauri/src/desktop.rs
    note: chunked save/drop above 8 MiB threshold
  - file: synara/src/app/utils/desktop.ts
    note: stream API for large blobs
-->

<!-- mip1-evidence:MIP1-20
commits: 0011eb7
kind: bundled
title: Dropped-file allowlist lifecycle
evidence:
  - file: src-tauri/src/desktop.rs
    note: allowlist cap, TTL, clear on leave, consume after read
  - file: src-tauri/src/lib.rs
    note: DragDropEvent::Leave clears allowlist
-->

<!-- mip1-evidence:MIP1-21
commits: d1aa30b
kind: bundled
title: Throttle tray menu rebuilds
evidence:
  - file: synara/src/app/utils/desktop.ts
    note: debounced setDesktopTrayState
  - file: src-tauri/src/desktop.rs
    note: in-place tray label updates vs full rebuild
-->

<!-- mip1-evidence:MIP1-22
commits: dfc76c1
kind: dedicated
title: Bound in-memory notification caches
evidence:
  - file: synara/src/app/utils/boundedLru.ts
    note: LRU helpers (500 approvals / 200 rooms)
  - file: synara/src/app/pages/client/ClientNonUIFeatures.tsx
    note: bounded dedupe caches wired
-->

<!-- mip1-evidence:MIP1-23
commits: 226053c
kind: dedicated
title: Cleanup focus highlight timeout on unmount
evidence:
  - file: synara/src/app/features/room/RoomTimeline.tsx
    note: useEffect cleanup clears highlight timer
-->

<!-- mip1-evidence:MIP1-24
commits: d1aa30b
kind: bundled
title: Atomic global shortcut updates
evidence:
  - file: src-tauri/src/desktop.rs
    note: register-before-unregister + rollback on failure
-->

<!-- mip1-evidence:MIP1-25
commits: d1aa30b
kind: bundled
title: Single global shortcut registration path
evidence:
  - file: src-tauri/src/lib.rs
    note: removed duplicate default shortcut handlers
-->

<!-- mip1-evidence:MIP1-26
commits: d1aa30b
kind: bundled
title: Resilient localhost port binding
evidence:
  - file: src-tauri/src/lib.rs
    note: select_localhost_port range 44548-44557
-->

<!-- mip1-evidence:MIP1-27
commits: d1aa30b
kind: bundled
title: Clamp dock badge count on macOS
evidence:
  - file: src-tauri/src/desktop.rs
    note: desktop_set_badge_count uses clamp_count parity
-->

<!-- mip1-evidence:MIP1-28
commits: d1aa30b
kind: bundled
title: Align external URL policy with agent URLs
evidence:
  - file: src-tauri/src/desktop.rs
    note: is_safe_external_url HTTPS + loopback HTTP exception
-->

<!-- mip1-evidence:MIP1-29
commits: d1aa30b, 0c43203
kind: bundled
title: Session expiry metadata enforcement
evidence:
  - file: src-tauri/src/desktop.rs
    note: SESSION_EXPIRY_CLOCK_SKEW_TOLERANCE_MS on desktop_get_session (d1aa30b)
  - file: synara/src/app/state/sessionPersistence.ts
    note: isSessionEnvelopeExpired helper with clock-skew tolerance (0c43203)
-->

<!-- mip1-evidence:MIP1-30
commits: dfbd2d3
kind: bundled
title: Sync splash timeout and recovery UI
evidence:
  - file: synara/src/app/utils/syncSplashRecovery.ts
    note: 90s PREPARED timeout + recovery actions
  - file: synara/src/app/pages/client/ClientRoot.tsx
    note: recovery UI wired
-->

<!-- mip1-evidence:MIP1-31
commits: dfbd2d3
kind: bundled
title: Timeline pagination error surfacing
evidence:
  - file: synara/src/app/utils/timelinePagination.ts
    note: error state + retry helper
  - file: synara/src/app/features/room/RoomTimeline.tsx
    note: pagination error chip UI
-->

<!-- mip1-evidence:MIP1-32
commits: dfbd2d3
kind: bundled
title: Strict desktop invoke error handling
evidence:
  - file: synara/src/app/utils/desktopDiagnostics.ts
    note: bounded diagnostics for IPC failures
  - file: synara/src/app/utils/desktop.ts
    note: invokeDesktop distinguishes missing bridge vs false
-->

<!-- mip1-evidence:MIP1-33
commits: dfbd2d3
kind: bundled
title: Distinct sync status for Catchup vs Prepared
evidence:
  - file: synara/src/app/pages/client/syncStatusCopy.ts
    note: state-specific banner copy
  - file: synara/src/app/pages/client/__tests__/syncStatusCopy.test.ts
    note: copy regression tests
-->

<!-- mip1-evidence:MIP1-34
commits: f9da50f
kind: bundled
title: Platform-specific shortcut permission help
evidence:
  - file: synara/src/app/platform/shortcutHelp.ts
    note: KDE Wayland / macOS / generic help selection
  - file: src-tauri/src/desktop.rs
    note: KDE Wayland unknown default shortcut state
-->

<!-- mip1-evidence:MIP1-35
commits: f9da50f
kind: bundled
title: Document macOS/Linux tray parity (Option A)
evidence:
  - file: docs/desktop-validation-status.md
    note: tray feature matrix macOS vs Linux
-->

<!-- mip1-evidence:MIP1-36
commits: f9da50f
kind: bundled
title: Arch PKGBUILD runtime dependencies
evidence:
  - file: packaging/arch/PKGBUILD
    note: dbus, libsecret, xdg-desktop-portal depends
-->

<!-- mip1-evidence:MIP1-37
commits: f9da50f
kind: bundled
title: Standalone .desktop file for Arch packaging
evidence:
  - file: packaging/arch/synara.desktop
    note: repository-owned desktop entry
-->

<!-- mip1-evidence:MIP1-38
commits: f9da50f
kind: bundled
title: Unified config.json sync in build pipeline
evidence:
  - file: scripts/build-runtime.mjs
    note: copies root config.json into synara before Vite build
-->

<!-- mip1-evidence:MIP1-39
commits: f9da50f
kind: bundled
title: CI and smoke workflow hardening
evidence:
  - file: .github/workflows/ci.yml
    note: cargo test in CI
  - file: .github/workflows/desktop-package-smoke.yml
    note: check:versions + arch packaging path filters
  - file: MODERNIZATION.md
    note: ubuntu-22.04 vs Arch CI divergence notes
-->

<!-- mip1-evidence:MIP1-40
commits: f9da50f, 9d89ae5
kind: bundled
title: Refresh desktop validation docs to 1.1.1+
evidence:
  - file: docs/desktop-validation-status.md
    note: wave matrix + 1.1.1 references (f9da50f); Phase 4 gate refresh (9d89ae5)
-->

<!-- mip1-evidence:MIP1-41
commits: f9da50f
kind: bundled
title: Fix docs/linux.md consistency
evidence:
  - file: docs/linux.md
    note: build order, Wayland smoke labels, .desktop reference
-->

<!-- mip1-evidence:MIP1-42
commits: 40ac646
kind: bundled
title: Arch pkgrel bump support in bump-version.mjs
evidence:
  - file: scripts/bump-version.mjs
    note: --pkgrel flag and auto-increment
  - file: scripts/check-version-consistency.mjs
    note: pkgrel validation
  - file: scripts/__tests__/bump-version-pkgrel.test.mjs
    note: pkgrel path tests
-->

<!-- mip1-evidence:MIP1-43
commits: 40ac646
kind: bundled
title: Normalize GitHub repository URLs
evidence:
  - file: src-tauri/Cargo.toml
    note: canonical repository URL metadata
-->

<!-- mip1-evidence:MIP1-44
commits: 40ac646
kind: bundled
title: Refresh token support
evidence:
  - file: synara/src/app/pages/auth/login/loginUtil.ts
    note: persist refresh metadata on login
  - file: synara/src/app/state/__tests__/tokenRefresh.test.ts
    note: refresh rotation unit tests
-->

<!-- mip1-evidence:MIP1-45
commits: 40ac646
kind: bundled
title: macOS signing configuration scaffolding
evidence:
  - file: .github/workflows/release-desktop.yml
    note: notarization gate comments
  - file: src-tauri/tauri.conf.json
    note: minimumSystemVersion placeholder
-->

<!-- mip1-evidence:MIP1-46
commits: 40ac646
kind: bundled
title: Linux spellcheck failure logging
evidence:
  - file: src-tauri/src/lib.rs
    note: warn when WebKit spellcheck WebContext unavailable
-->

---

## Validation

```bash
node scripts/check-mip1-commit-evidence.mjs
node scripts/check-mip1-commit-evidence.mjs --base main
```

Expected: 46/46 items mapped; all commits reachable from `HEAD`; all evidence files present in cited commit diffs.

## Remediation decision (2026-06-10)

**Chosen approach:** evidence map + automated checker (no history rewrite).

Rewriting 26 commits into 47 risks merge conflicts, invalidates review SHAs, and offers limited functional benefit because bundled commit bodies already document co-located items. Use this document for merge review; optionally squash-merge to `main` as one commit if linear history on `main` is preferred.