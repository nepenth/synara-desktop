# Shared Native Core (synara-core) — Program Plan

**Status at `feature/shared-native-core`
`b811319f0fcc7ecd6eee82e4255d57e8e5699360` (#714, after #713):** P0 is
complete; P1 extraction and bounded P2, P3, and P4 slices are merged. P2–P4
remain in progress. P5 has not started. Owner: Synara engineering. The current
provenance, gates, and successor steps are in `10-current-handoff.md`.

## Goal

One Rust app-logic core (`crates/synara-core`) consumed by both desktop
(Tauri) and iOS (SwiftUI via UniFFI). This ends the dual implementation of
sync, room list, timeline, and crypto logic only when the remaining migrations
and release gates have actually passed.

## Decision record

- ADR 0003: `docs/adr/0003-shared-native-rust-core.md`

Full docs: see this directory's README and 01–10 (architecture, census,
platform sinks, transport/FFI, phases, risk, parity matrix, references, and the
current handoff ledger).

## Merged progress and remaining work

This is a source-and-log status ledger, not a claim that a phase's final
acceptance criteria have all passed.

- **P0 — complete.** ADR, plan, and module-boundary census are established.
- **P1 — in progress; extraction slices are merged.** #669 created the
  workspace/core scaffold; #673–#675 moved DTO, transport/IPC, and the pure
  task subset; #676–#677 and #680 moved sync, room-list, timeline, and
  UTD-recovery pieces. #681 added the `Platform` sink and desktop `AppHandle`
  adapter without changing intended behavior. #713 then mechanically moved the
  pure notifications, polls, relations, threads, and unread projections; #714
  mechanically moved raw content, receipts, routes, and security. Both clusters
  retain thin desktop re-exports and add no P2 command, UDL, or iOS behavior.
  Compatibility re-exports remain, and many matrix domains are still
  desktop-owned, so the full extraction/end state is not yet complete.
- **P2 — in progress; transport registry is intentionally partial.** #683–#684
  added `Core::command`, typed envelopes, the registry, and the desktop command
  census. The merged registry currently has exactly:
  `matrix_login_flows`, `matrix_register_flows`, `matrix_session_snapshot`,
  `matrix_sync_status`, `matrix_crypto_status`, `matrix_media_config`,
  `matrix_cross_signing_status`, and `matrix_secret_storage_status`
  (#686–#689, #694, #698, #701–#702, #706). The latter six preserve bounded
  legacy status/media/cross-signing/secret-storage contracts through Core.
  Neither #708, #710, #713, nor #714 adds a Core command route. It is not the
  complete desktop command registry; unregistered census names fail closed.
- **P3 — in progress; a desktop seam is merged, not a whole adapter swap.**
  #690 routes the credential-free login and registration flow probes through
  Core. #691 mirrors only the installed safe session lifecycle, and #694/#698
  route the existing sync- and crypto-status commands through Core; #701/#702
  add bounded media-config and cross-signing-status bridges; #706 adds only the
  read-only secret-storage status bridge. The desktop still owns its live
  Matrix SDK client, credentials, persistence, and all remaining direct command
  paths.
- **P4 — in progress; bootstrap and discovery are merged.** #685 provides the
  project-owned UniFFI/Swift package scaffold. #692 exposes only typed,
  credential-free login-flow discovery; #693 has iOS homeserver discovery call
  it; #696 adds an XCTest that invokes `bindingScaffoldVersion()` through the
  generated Rust FFI; #699 adds only a safe transient session-projection mirror;
  #703 adds a display-only Settings readback that exact-matches Swift session
  state and falls back safely. #708 adds only the pure iOS room-row unread
  presentation from closed `Joined`/`Invited` membership, scalar counters, and
  a marked-unread flag to a `u64` unread count plus highlight boolean. #710
  adds only the pure cold-start recovery decision from a latest-state boolean
  and `{Missing, Known}` to a boolean; Swift maps `nil`/`.distantPast` to
  `Missing` and a real `Date` to `Known`. Neither slice adds a Core command
  route or Core SDK/service owner. Actual SDK `Room` and timeline
  listener/pagination/recovery execution, plus session, Keychain, store,
  crypto, sync, and lifecycle ownership, remain `MatrixRustSDKService`-owned.
  This does **not** migrate iOS session, room-list, timeline, crypto, push/NSE,
  or `MatrixRustSDK` services, and it does not remove the upstream Swift SDK
  dependency.
- **P5 — not started.** Do not claim iOS shared-engine parity, iOS migration,
  or Apple release readiness from the bounded work above.

## Phase acceptance targets

The original phase targets remain the definition of done; partial completion
above does not relax them.

1. **P1:** finish moving the intended app logic while preserving behavior,
   compatibility re-exports, tests, formatting, clippy, and matrix boundaries.
2. **P2:** register the complete desktop command census with parity/contract
   coverage while preserving the React-facing `matrix_*` command contract.
3. **P3:** reduce `src-tauri` to the planned thin Core adapter without changing
   renderer-visible command behavior; retain desktop package and full-matrix
   proofs.
4. **P4:** build project-owned bindings for the Apple targets and migrate iOS
   services in dependency-safe slices; only then retire direct
   `MatrixRustSDKService`, room-list, and timeline service use when no consumer
   remains.
5. **P5:** establish shared-engine parity and close the release gates below.

## Ownership, privacy, and operator/Apple release gates

- Shells retain platform-native ownership: desktop owns its SDK client,
  credentials, stores, and live observations; iOS owns SwiftUI, Keychain, APNs,
  app lifecycle, and NSE behavior. The currently routed Core paths accept safe
  projections and commands, never a live client or raw diagnostic.
- Preserve privacy boundaries in every slice. `Core::open`/`close` keeps only an
  in-memory safe session projection; sync and crypto use closed, string-free
  platform projections; and current UniFFI login discovery is read-only and
  credential-free. Never widen those surfaces with tokens, passwords, keys,
  client handles, store locations, raw diagnostics, or raw HTTP payloads.
- P5 requires shared-core desktop matrix/compatibility evidence, iOS simulator
  evidence, signed physical-device and profiling evidence, production APNs,
  TestFlight archive/upload, and production E2EE completion (recovery,
  verification/cross-signing, key-backup restore, encrypted-media decryption).
  Preserve the [device-readiness](../../synara-ios/docs/device-readiness.md)
  and [iOS release](../../synara-ios/docs/release-checklist.md) Apple signing,
  privacy, legal, and enrollment gates.
- A merged implementation slice is not release authorization. Production
  publication remains the exact-tag, protected `production-release` process
  with required human review documented in
  [build-and-release.md](../build-and-release.md). No PR may bypass it.

## Guardrails (from js→rust burn-down methodology)

- Small additive slices, each a PR with green desktop CI (Quality + Desktop
  package + full matrix at tip).
- Worktree isolation; branch base = feature; squashed PRs; provenance anchor.
- Domain modules carry their tests; preserve behavior while extracting them.
- The IPC/transport contract tests and command census are the north star for
  transport migration; unregistered commands fail closed rather than silently
  changing behavior.
- No phase or release checkbox closes without its stated evidence, including
  privacy review and the relevant operator/Apple gates.
