# MIP1 Commit Evidence Map

**Branch:** `maturity_improvement_plan1`  
**Base:** `main`  
**Recorded:** 2026-06-10  
**Purpose:** Reconcile plan commit hygiene (47 commits: `mip1-00` + `mip1-01`…`mip1-46`) with actual branch history when work was bundled.

## Summary

| Metric | Plan | Actual (2026-06-10) |
|--------|------|---------------------|
| Total commits `main..HEAD` | 47 | **26** |
| Plan commits (`mip1-00`) | 1 | **4** (`5ad722a`, `950a12f`, `1d59bdd`, `c0a8f9a`) |
| Implementation commits (`mip1-01`…`mip1-46`) | 46 | **22** subject lines; **46/46 items** evidenced |
| Dedicated 1:1 item commits | 46 | **22** |
| Bundled items (shared commit) | 0 | **24** across **6** mega-commits |

### Bundling clusters

| Commit | Subject | Items bundled in body |
|--------|---------|------------------------|
| `1a1cc78` | mip1-05 | MIP1-05, MIP1-06, MIP1-07 |
| `924dc70` | mip1-08 | MIP1-08, MIP1-11 |
| `98c24fc` | mip1-19 | MIP1-19, MIP1-20 |
| `e196a90` | mip1-21 | MIP1-21, MIP1-24, MIP1-25, MIP1-26, MIP1-27, MIP1-28, MIP1-29 |
| `b0b7967` | mip1-30 | MIP1-30, MIP1-31, MIP1-32, MIP1-33 |
| `bd519cb` | mip1-34 | MIP1-34, MIP1-35, MIP1-36, MIP1-37, MIP1-38, MIP1-39, MIP1-41 |
| `ea9ddc7` | mip1-40 | MIP1-42, MIP1-43, MIP1-44, MIP1-45, MIP1-46 (subject also claims MIP1-40; doc diff in `bd519cb`/`c0a8f9a`) |

**Note:** MIP1-41 (`linux.md`) landed in `bd519cb` (file diff); `ea9ddc7` commit message also claims MIP1-41 but does not touch `docs/linux.md`.

### Quick reference table

| ID | Title (short) | Commit(s) | Kind |
|----|---------------|-----------|------|
| MIP1-01 | DevTools gate | `3459e52` | dedicated |
| MIP1-02 | Bridge capabilities | `a7a17f3` | dedicated |
| MIP1-03 | CSP tighten | `d68a713` | dedicated |
| MIP1-04 | Windows honesty | `7093069` | dedicated |
| MIP1-05 | Notification route | `1a1cc78` | bundled |
| MIP1-06 | Tray DND | `1a1cc78` | bundled |
| MIP1-07 | Agent-action listener | `1a1cc78` | bundled |
| MIP1-08 | Keyutils fix | `924dc70` | bundled |
| MIP1-09 | Native store UI | `0628b2a` | dedicated |
| MIP1-10 | Secret Service probe | `85594c0` | dedicated |
| MIP1-11 | macOS Keychain probe | `924dc70` | bundled |
| MIP1-12 | Secret-store errors | `a3c5ef1` | dedicated |
| MIP1-13 | Selective logout | `2f4d96c` | dedicated |
| MIP1-14 | SW session push | `cea7fd8` | dedicated |
| MIP1-15 | Unified logout | `11a76ad` | dedicated |
| MIP1-16 | Clear secret keys | `f0c2b37` | dedicated |
| MIP1-17 | Account switch | `3e31110` | dedicated |
| MIP1-18 | Incremental timeline | `cd8b27c` | dedicated |
| MIP1-19 | Stream file IPC | `98c24fc` | bundled |
| MIP1-20 | Drop allowlist | `98c24fc` | bundled |
| MIP1-21 | Throttle tray | `e196a90` | bundled |
| MIP1-22 | Notification LRU | `e2a3e06` | dedicated |
| MIP1-23 | Focus timeout | `4d237db` | dedicated |
| MIP1-24 | Atomic shortcuts | `e196a90` | bundled |
| MIP1-25 | Single shortcut path | `e196a90` | bundled |
| MIP1-26 | Port fallback | `e196a90` | bundled |
| MIP1-27 | Badge clamp | `e196a90` | bundled |
| MIP1-28 | External URL policy | `e196a90` | bundled |
| MIP1-29 | Session expiry | `e196a90`, `3e31110` | bundled |
| MIP1-30 | Sync splash timeout | `b0b7967` | bundled |
| MIP1-31 | Pagination errors | `b0b7967` | bundled |
| MIP1-32 | Invoke strictness | `b0b7967` | bundled |
| MIP1-33 | Sync status copy | `b0b7967` | bundled |
| MIP1-34 | Shortcut help | `bd519cb` | bundled |
| MIP1-35 | Tray parity doc | `bd519cb` | bundled |
| MIP1-36 | Arch depends | `bd519cb` | bundled |
| MIP1-37 | Standalone .desktop | `bd519cb` | bundled |
| MIP1-38 | config.json sync | `bd519cb` | bundled |
| MIP1-39 | CI hardening | `bd519cb` | bundled |
| MIP1-40 | Validation docs | `bd519cb`, `c0a8f9a` | bundled |
| MIP1-41 | linux.md fixes | `bd519cb` | bundled |
| MIP1-42 | pkgrel bump | `ea9ddc7` | bundled |
| MIP1-43 | Repo URL normalize | `ea9ddc7` | bundled |
| MIP1-44 | Refresh token | `ea9ddc7` | bundled |
| MIP1-45 | macOS signing docs | `ea9ddc7` | bundled |
| MIP1-46 | Spellcheck logging | `ea9ddc7` | bundled |

---

## Per-item evidence (machine-checked)

The blocks below are parsed by `scripts/check-mip1-commit-evidence.mjs`.

<!-- mip1-evidence:MIP1-01
commits: 3459e52
kind: dedicated
title: Gate DevTools to debug builds only
evidence:
  - file: src-tauri/Cargo.toml
    note: devtools feature limited to debug/dev profile
  - file: src-tauri/build.rs
    note: release build omits devtools capability generation
-->

<!-- mip1-evidence:MIP1-02
commits: a7a17f3
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
commits: d68a713
kind: dedicated
title: Tighten Content-Security-Policy
evidence:
  - file: src-tauri/tauri.conf.json
    note: narrowed CSP directives
  - file: docs/desktop-matrix-sdk-boundaries.md
    note: documented federation/media/call exceptions
-->

<!-- mip1-evidence:MIP1-04
commits: 7093069
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
commits: 1a1cc78
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
commits: 1a1cc78
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
commits: 1a1cc78
kind: bundled
title: Frontend listens for synara://agent-action
evidence:
  - file: synara/src/app/platform/agentActions.ts
    note: registerPlatformAgentActionListener on synara://agent-action
  - file: synara/src/app/platform/__tests__/agentActions.test.ts
    note: contract validation + listen registration tests
-->

<!-- mip1-evidence:MIP1-08
commits: 924dc70
kind: bundled
title: Fix Linux keyutils detection false positive
evidence:
  - file: src-tauri/src/desktop.rs
    note: keyring round-trip probe; Secret Service preferred
-->

<!-- mip1-evidence:MIP1-09
commits: 0628b2a
kind: dedicated
title: Surface nativeStoreError in Settings UI
evidence:
  - file: synara/src/app/features/settings/general/General.tsx
    note: native store warning section
  - file: synara/src/app/state/sessionBootstrap.ts
    note: nativeStoreError propagation
-->

<!-- mip1-evidence:MIP1-10
commits: 85594c0
kind: dedicated
title: Live Secret Service probe
evidence:
  - file: src-tauri/src/desktop.rs
    note: dbus keyring read/write probe with timeout + mock trait
-->

<!-- mip1-evidence:MIP1-11
commits: 924dc70
kind: bundled
title: macOS Keychain probe
evidence:
  - file: src-tauri/src/desktop.rs
    note: macos_keychain_probe + distinct locked/denied/unavailable codes
-->

<!-- mip1-evidence:MIP1-12
commits: a3c5ef1
kind: dedicated
title: Structured secret-store error reporting
evidence:
  - file: src-tauri/src/desktop.rs
    note: stable public error codes; no token echo in IPC
  - file: synara/src/app/platform/secrets.ts
    note: TS types for new error codes
-->

<!-- mip1-evidence:MIP1-13
commits: 2f4d96c
kind: dedicated
title: Selective logout preserves user settings
evidence:
  - file: synara/src/app/state/sessions.ts
    note: clearLoginData removes session keys only
  - file: synara/src/client/initMatrix.ts
    note: removed localStorage.clear from logout paths
-->

<!-- mip1-evidence:MIP1-14
commits: cea7fd8
kind: dedicated
title: Push session to service worker after login
evidence:
  - file: synara/src/app/pages/auth/login/loginUtil.ts
    note: pushSessionToSW after persistAuthenticatedSession
  - file: synara/src/sw-session.ts
    note: SW session envelope handling
-->

<!-- mip1-evidence:MIP1-15
commits: 11a76ad
kind: dedicated
title: Unify logout code paths
evidence:
  - file: synara/src/client/initMatrix.ts
    note: shared performLogout helper
  - file: synara/src/app/state/__tests__/performLogout.test.ts
    note: tray/dialog/SessionLoggedOut parity tests
-->

<!-- mip1-evidence:MIP1-16
commits: f0c2b37
kind: dedicated
title: Clear secret storage keys on logout
evidence:
  - file: synara/src/client/initMatrix.ts
    note: clearSecretStorageKeys in unified logout
  - file: synara/src/app/matrix/__tests__/secretStorageKeys.test.ts
    note: idempotent clear tests
-->

<!-- mip1-evidence:MIP1-17
commits: 3e31110
kind: dedicated
title: Account switch safety for fixed IndexedDB names
evidence:
  - file: synara/src/app/state/sessionPersistence.ts
    note: identity comparison + matrix local store clear on mismatch
-->

<!-- mip1-evidence:MIP1-18
commits: cd8b27c
kind: dedicated
title: Incremental timeline row building
evidence:
  - file: synara/src/app/utils/timelineVirtualization.ts
    note: incremental row build for append-only updates
  - file: synara/src/app/utils/__tests__/timelineVirtualization.test.ts
    note: virtualization regression suite
-->

<!-- mip1-evidence:MIP1-19
commits: 98c24fc
kind: bundled
title: Stream large file save/drop IPC
evidence:
  - file: src-tauri/src/desktop.rs
    note: chunked save/drop above 8 MiB threshold
  - file: synara/src/app/utils/desktop.ts
    note: stream API for large blobs
-->

<!-- mip1-evidence:MIP1-20
commits: 98c24fc
kind: bundled
title: Dropped-file allowlist lifecycle
evidence:
  - file: src-tauri/src/desktop.rs
    note: allowlist cap, TTL, clear on leave, consume after read
  - file: src-tauri/src/lib.rs
    note: DragDropEvent::Leave clears allowlist
-->

<!-- mip1-evidence:MIP1-21
commits: e196a90
kind: bundled
title: Throttle tray menu rebuilds
evidence:
  - file: synara/src/app/utils/desktop.ts
    note: debounced setDesktopTrayState
  - file: src-tauri/src/desktop.rs
    note: in-place tray label updates vs full rebuild
-->

<!-- mip1-evidence:MIP1-22
commits: e2a3e06
kind: dedicated
title: Bound in-memory notification caches
evidence:
  - file: synara/src/app/utils/boundedLru.ts
    note: LRU helpers (500 approvals / 200 rooms)
  - file: synara/src/app/pages/client/ClientNonUIFeatures.tsx
    note: bounded dedupe caches wired
-->

<!-- mip1-evidence:MIP1-23
commits: 4d237db
kind: dedicated
title: Cleanup focus highlight timeout on unmount
evidence:
  - file: synara/src/app/features/room/RoomTimeline.tsx
    note: useEffect cleanup clears highlight timer
-->

<!-- mip1-evidence:MIP1-24
commits: e196a90
kind: bundled
title: Atomic global shortcut updates
evidence:
  - file: src-tauri/src/desktop.rs
    note: register-before-unregister + rollback on failure
-->

<!-- mip1-evidence:MIP1-25
commits: e196a90
kind: bundled
title: Single global shortcut registration path
evidence:
  - file: src-tauri/src/lib.rs
    note: removed duplicate default shortcut handlers
-->

<!-- mip1-evidence:MIP1-26
commits: e196a90
kind: bundled
title: Resilient localhost port binding
evidence:
  - file: src-tauri/src/lib.rs
    note: select_localhost_port range 44548-44557
-->

<!-- mip1-evidence:MIP1-27
commits: e196a90
kind: bundled
title: Clamp dock badge count on macOS
evidence:
  - file: src-tauri/src/desktop.rs
    note: desktop_set_badge_count uses clamp_count parity
-->

<!-- mip1-evidence:MIP1-28
commits: e196a90
kind: bundled
title: Align external URL policy with agent URLs
evidence:
  - file: src-tauri/src/desktop.rs
    note: is_safe_external_url HTTPS + loopback HTTP exception
-->

<!-- mip1-evidence:MIP1-29
commits: e196a90, 3e31110
kind: bundled
title: Session expiry metadata enforcement
evidence:
  - file: src-tauri/src/desktop.rs
    note: SESSION_EXPIRY_CLOCK_SKEW_TOLERANCE_MS on desktop_get_session (e196a90)
  - file: synara/src/app/state/sessionPersistence.ts
    note: isSessionEnvelopeExpired helper with clock-skew tolerance (3e31110)
-->

<!-- mip1-evidence:MIP1-30
commits: b0b7967
kind: bundled
title: Sync splash timeout and recovery UI
evidence:
  - file: synara/src/app/utils/syncSplashRecovery.ts
    note: 90s PREPARED timeout + recovery actions
  - file: synara/src/app/pages/client/ClientRoot.tsx
    note: recovery UI wired
-->

<!-- mip1-evidence:MIP1-31
commits: b0b7967
kind: bundled
title: Timeline pagination error surfacing
evidence:
  - file: synara/src/app/utils/timelinePagination.ts
    note: error state + retry helper
  - file: synara/src/app/features/room/RoomTimeline.tsx
    note: pagination error chip UI
-->

<!-- mip1-evidence:MIP1-32
commits: b0b7967
kind: bundled
title: Strict desktop invoke error handling
evidence:
  - file: synara/src/app/utils/desktopDiagnostics.ts
    note: bounded diagnostics for IPC failures
  - file: synara/src/app/utils/desktop.ts
    note: invokeDesktop distinguishes missing bridge vs false
-->

<!-- mip1-evidence:MIP1-33
commits: b0b7967
kind: bundled
title: Distinct sync status for Catchup vs Prepared
evidence:
  - file: synara/src/app/pages/client/syncStatusCopy.ts
    note: state-specific banner copy
  - file: synara/src/app/pages/client/__tests__/syncStatusCopy.test.ts
    note: copy regression tests
-->

<!-- mip1-evidence:MIP1-34
commits: bd519cb
kind: bundled
title: Platform-specific shortcut permission help
evidence:
  - file: synara/src/app/platform/shortcutHelp.ts
    note: KDE Wayland / macOS / generic help selection
  - file: src-tauri/src/desktop.rs
    note: KDE Wayland unknown default shortcut state
-->

<!-- mip1-evidence:MIP1-35
commits: bd519cb
kind: bundled
title: Document macOS/Linux tray parity (Option A)
evidence:
  - file: docs/desktop-validation-status.md
    note: tray feature matrix macOS vs Linux
-->

<!-- mip1-evidence:MIP1-36
commits: bd519cb
kind: bundled
title: Arch PKGBUILD runtime dependencies
evidence:
  - file: packaging/arch/PKGBUILD
    note: dbus, libsecret, xdg-desktop-portal depends
-->

<!-- mip1-evidence:MIP1-37
commits: bd519cb
kind: bundled
title: Standalone .desktop file for Arch packaging
evidence:
  - file: packaging/arch/synara.desktop
    note: repository-owned desktop entry
-->

<!-- mip1-evidence:MIP1-38
commits: bd519cb
kind: bundled
title: Unified config.json sync in build pipeline
evidence:
  - file: scripts/build-runtime.mjs
    note: copies root config.json into synara before Vite build
-->

<!-- mip1-evidence:MIP1-39
commits: bd519cb
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
commits: bd519cb, c0a8f9a
kind: bundled
title: Refresh desktop validation docs to 1.1.1+
evidence:
  - file: docs/desktop-validation-status.md
    note: wave matrix + 1.1.1 references (bd519cb); Phase 4 gate refresh (c0a8f9a)
-->

<!-- mip1-evidence:MIP1-41
commits: bd519cb
kind: bundled
title: Fix docs/linux.md consistency
evidence:
  - file: docs/linux.md
    note: build order, Wayland smoke labels, .desktop reference
-->

<!-- mip1-evidence:MIP1-42
commits: ea9ddc7
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
commits: ea9ddc7
kind: bundled
title: Normalize GitHub repository URLs
evidence:
  - file: src-tauri/Cargo.toml
    note: canonical repository URL metadata
-->

<!-- mip1-evidence:MIP1-44
commits: ea9ddc7
kind: bundled
title: Refresh token support
evidence:
  - file: synara/src/app/pages/auth/login/loginUtil.ts
    note: persist refresh metadata on login
  - file: synara/src/app/state/__tests__/tokenRefresh.test.ts
    note: refresh rotation unit tests
-->

<!-- mip1-evidence:MIP1-45
commits: ea9ddc7
kind: bundled
title: macOS signing configuration scaffolding
evidence:
  - file: .github/workflows/release-desktop.yml
    note: notarization gate comments
  - file: src-tauri/tauri.conf.json
    note: minimumSystemVersion placeholder
-->

<!-- mip1-evidence:MIP1-46
commits: ea9ddc7
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