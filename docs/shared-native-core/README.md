# Shared Native Core (synara-core) — Program

A plan to unify the **desktop** (Tauri) and **iOS** (SwiftUI) clients of Synara
Desktop onto **one transport-agnostic Rust application-logic core** —
`crates/synara-core` — consumed by both platforms.

| | |
|---|---|
| Owner | Synara engineering |
| Status | P0 complete; P1 extraction and bounded P2, P3, and P4 slices are merged at the evidence base below. P2–P4 remain in progress; P5 has not started. |
| Evidence base | `origin/feature/shared-native-core` at `4c4615dc` (`feat(core): migrate cross-signing status observation`, #702) |
| Decision | [ADR 0003](../adr/0003-shared-native-rust-core.md) |
| Program ledger | `docs/shared-native-core/` (this directory) |
| Related ADRs | [0001](../adr/0001-ios-repository-layout.md), [0002](../adr/0002-ios-architecture.md) |

## TL;DR

- At the P0 census, the desktop matrix layer contained ~285 Rust source files,
  a large Tauri command surface, and an existing `ipc/` protocol. That remains
  the migration inventory, not evidence that every module has moved.
- **P1 extraction slices are merged.** `synara-core` now owns the moved DTO,
  transport/IPC, pure task, sync, room-list, timeline, and UTD-recovery pieces;
  `src-tauri` retains compatibility re-exports. P1.6 also introduced the
  `Platform` trait and a desktop `AppHandle` adapter with no intended behavior
  change. The remaining desktop-owned matrix domains still make the full P1
  end-state incomplete.
- **P2 is a partial transport registry, not the complete command migration.**
  `Core::command` has typed envelopes and currently registers exactly seven
  desktop-census commands: `matrix_login_flows`, `matrix_register_flows`,
  `matrix_session_snapshot`, `matrix_sync_status`, `matrix_crypto_status`,
  `matrix_media_config`, and `matrix_cross_signing_status`. The status,
  media, and cross-signing paths preserve their bounded legacy DTO contracts;
  the rest of the census remains unregistered and fail-closed.
- **P3 has a bounded desktop seam.** Existing Tauri commands route the two
  stateless auth probes and the session lifecycle/snapshot, sync-status, and
  crypto-status observations through the managed Core while retaining their
  React-facing DTOs. The payload-free media-config and read-only
  cross-signing-status bridges are also routed through Core. This is not the
  planned whole-shell swap: the other desktop commands and live session
  ownership remain in `src-tauri`.
- **P4 has a project-owned UniFFI scaffold, credential-free typed login-flow
  discovery, iOS homeserver-discovery use, an XCTest that calls the generated
  Rust FFI scaffold, and the bounded `SessionProjectionCore` mirror.** The
  mirror is not auth/session truth and has not migrated the iOS session,
  room-list, timeline, crypto, push/NSE, or `MatrixRustSDK` service layer. P5
  is not complete and iOS is not migrated to the shared engine.

## Current milestone ledger

The following describes only merged source reachable from `4c4615dc`.

| Phase | Merged evidence | Current boundary |
|---|---|---|
| P0 | ADR, plan, and census | Complete planning baseline. |
| P1 | #669, #673–#677, #680–#681 | Extraction slices and the `Platform`/desktop adapter are merged; remaining matrix domains have not all moved. |
| P2 | #683–#689, #694, #698, #701–#702 | The seven-command registry above is live; it is not parity with the full desktop invoke census. |
| P3 | #690–#691, #694, #698, #701–#702 | The listed auth and read-only/session bridges use Core; `src-tauri` is not yet a fully thin shell. |
| P4 | #685, #692–#693, #696, #699 | UniFFI scaffold, bounded login discovery/link coverage, and a safe session projection mirror exist; service migration and Apple release proof do not. |
| P5 | None | Not started. The release gates below remain required. |

### Unmerged work is not current program state

[PR #703](https://github.com/nepenth/synara-desktop/pull/703),
`feat(ios): read back safe Core session identity`, remains **open and
unmerged** at this evidence base. Its proposed Settings display-only consumer
of the already-merged `SessionProjectionCore` mirror is not P4 service
migration, iOS migration, or P5 progress. It remains future work unless and
until it lands separately.

## Ownership, privacy, and release gates

- **Native ownership stays explicit.** Desktop remains the sole owner of its
  Matrix SDK client, credentials, stores, and live sync/crypto state. The
  currently routed Core boundaries use only a safe session projection and
  closed platform-status projections; they do not transfer a live client or raw
  diagnostic. iOS continues to own SwiftUI, Keychain, APNs, app lifecycle, and
  NSE behavior; the current UniFFI package is not a service replacement.
- **Privacy boundaries are intentional.** Core session open/close is an
  in-memory projection, not platform persistence. The sync and crypto seams
  reject dynamic shell diagnostics and SDK-bearing values; Core serializes the
  fixed public DTOs. UniFFI login-flow discovery is read-only and
  credential-free, with fixed privacy-safe errors—no passwords, tokens, client
  handles, keys, stores, or raw HTTP diagnostics cross that boundary.
- **P5 and Apple release are still gated.** Before claiming shared-engine iOS
  parity or shipping it, run the shared-core desktop matrix and compatibility
  gates; iOS simulator coverage; signed physical-device and profiling evidence;
  production APNs validation; TestFlight archive/upload; and production E2EE
  completion (recovery, verification/cross-signing, key-backup restore, and
  encrypted-media decryption). The [iOS device-readiness checklist](../../synara-ios/docs/device-readiness.md)
  and [iOS release checklist](../../synara-ios/docs/release-checklist.md) also
  retain their Apple enrollment, privacy, legal, and signing gates.
- **No status entry is an operator or release authorization.** Production
  publication remains the exact-tag, protected-environment process in the
  [build-and-release runbook](../build-and-release.md), including its required
  human review. This documentation-only update changes none of those controls.

## How to navigate this program

| Doc | Content |
|---|---|
| [`01-context-and-goals.md`](01-context-and-goals.md) | Problem statement, goals, non-goals, success criteria |
| [`02-module-boundary-census.md`](02-module-boundary-census.md) | Exact, file-referenced census of what exists today |
| [`03-target-architecture.md`](03-target-architecture.md) | Target crate layout, adapter model, data flow |
| [`04-platform-sinks.md`](04-platform-sinks.md) | The OS platform seams and how each platform implements them |
| [`05-transport-and-ffi.md`](05-transport-and-ffi.md) | The native transport API (ipc protocol), Tauri + uniffi adapters, NSE constraints |
| [`06-migration-phases.md`](06-migration-phases.md) | P1–P5 phases: PR-level mechanics, acceptance criteria, CI gates |
| [`07-risk-and-decisions.md`](07-risk-and-decisions.md) | Risk register, decision log, open questions |
| [`08-parity-matrix.md`](08-parity-matrix.md) | Capability parity desktop vs iOS, now and after unification |
| [`09-references.md`](09-references.md) | Glossary + concrete file/upstream references |

## Core idea in one sentence

> Put the app logic in a crate with zero Tauri types, let desktop call it
> in-process and iOS call it through generated bindings — so both platforms
> share one engine, one test suite, and one definition of "correct."
