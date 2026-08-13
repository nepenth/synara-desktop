# Shared Native Core (synara-core) — Program

A plan to unify the **desktop** (Tauri) and **iOS** (SwiftUI) clients of Synara
Desktop onto **one transport-agnostic Rust application-logic core** —
`crates/synara-core` — consumed by both platforms.

| | |
|---|---|
| Owner | Synara engineering |
| Status | P0 complete; P1 extraction and bounded P2, P3, and P4 slices are merged at the evidence base below. P2–P4 remain in progress; P5 has not started. |
| Evidence base | `feature/shared-native-core` at `76b3a80d` (#924, after #923; prior bounded P4 evidence remains #708/#710) |
| Decision | [ADR 0003](../adr/0003-shared-native-rust-core.md) |
| Program ledger | `docs/shared-native-core/` (this directory) |
| Related ADRs | [0001](../adr/0001-ios-repository-layout.md), [0002](../adr/0002-ios-architecture.md) |

## TL;DR

- At the P0 census, the desktop matrix layer contained ~285 Rust source files,
  a large Tauri command surface, and an existing `ipc/` protocol. That remains
  the migration inventory, not evidence that every module has moved.
- **P1 extraction slices are merged.** `synara-core` now owns the moved DTO,
  transport/IPC, pure task, sync, room-list, timeline, and UTD-recovery pieces;
  #713 also mechanically moved notifications, polls, relations, threads, and
  unread; #714 moved raw_content, receipts, routes, and security; #716 moved
  search, legacy, and media_cache; #717 moved media_export and crypto_store;
  #720 moved the members harness (live `product_commands.rs` stayed desktop);
  #721 moved the user_profile harness the same way;
  #734 moved the room-directory session harness (`live.rs` + commands stayed desktop);
  #735 moved the verification inbox harness (`live.rs` + commands stayed desktop);
  #737 moved the account-data index harness (image_packs/later/live/room_notes + commands stayed desktop);
  #738 moved the send-queue harness (`product_commands.rs` + `live_synapse_proof` stayed desktop);
  #740 moved the room-keys transfer harness (`live.rs` + commands stayed desktop);
  #741 moved the supervisor actor harness (whole module);
  #744 moved the diagnostics health harness (whole module);
  #743 moved the store identity/paths harness (keyring vault stayed desktop);
  #748 moved the client-builder error/features harness (SDK open/config stayed desktop);
  #747 moved the lifecycle recovery-copy + remote-logout policy enums (vault/SDK stayed desktop);
  #751 moved the auth device-name harness (live login/discovery stayed desktop);
  #753 moved the store vault trait / key material and auth discovery/UIA/client_config
  (Keyring I/O and live login stayed desktop);
  #755 moved the lifecycle error domain (logout/session vault I/O/SDK restore stayed desktop);
  #757 moved lifecycle recovery/remote-logout/wipe;
  #758 moved client-builder config (SDK open stayed desktop);
  #760 moved the session-material vault trait (Keyring I/O stayed desktop);
  #762 moved the well-known HTTP discovery transport (live login stayed desktop);
  #764 moved logout orchestration and the task-supervisor bridge;
  #766 moved later/room-notes codecs (Client RMW stayed desktop);
  #768 moved image-pack DTO, type filters, and write guards (Client snapshot/set and Tauri subscribe stayed desktop);
  #770 moved m.direct snapshot DTO and string-map helpers (Client load/store and DirectEventContent write stayed desktop);
  #772 moved device presentation DTOs and sort helper (Client snapshot, UIAA delete, and Tauri owner stayed desktop);
  #774 moved secret-storage presentation DTOs and projector (Client recovery I/O stayed desktop);
  #776 moved backup presentation DTOs and projector (Client backup/recovery I/O stayed desktop);
  #778 moved presence DTOs and subscription registry (Client stream and Tauri owner stayed desktop);
  #780 moved typing presentation snapshot DTO (Client m.typing owner stayed desktop);
  #781 moved verification presentation DTOs and phase rank (Client request/SAS owner stayed desktop);
  #783 moved room-directory DTOs and search normalize (ruma request/Client fetch stayed desktop);
  #784 moved space presentation DTOs and cycle guard (live Client I/O later moved in #908);
  #786 moved cross-signing presentation DTOs and projector (Client crypto I/O and UIAA stayed desktop);
  #788 moved room-key transfer presentation DTOs and projector (Client/file I/O stayed desktop);
  #790 moved members presentation snapshots and write result (Client member/power-level I/O stayed desktop);
  #792 moved room join-rule presentation DTO (SDK mapping and Tauri owner stayed desktop);
  #794 moved timeline presentation DTOs (NativeTimelineRegistry and Client/Tauri streams stayed desktop);
  #796 moved send/profile-write/room-create/room-profile IPC DTOs (Client I/O stayed desktop);
  #798 moved media upload/download/config IPC DTOs (Client media I/O stayed desktop);
  #800 moved live `Client::builder` plus session persist/restore (Keyring vault and `SdkClientHandle` stayed desktop);
  #801 moved live password login / register / password-reset (Tauri product commands stayed desktop);
  #803 moved live `NativeTypingOwner` / `set_typing_notice` (Tauri typing commands stayed desktop);
  #805 moved live `NativeRoomJoinRuleOwner` behind a shell emit sink (Tauri event adapter stayed desktop);
  #807 moved live `NativeDeviceOwner` behind a shell emit sink (Tauri wakeup adapter stayed desktop);
  #808 moved live `NativePresenceOwner` behind a shell emit sink (Tauri event adapter stayed desktop);
  #810 moved live `NativeImagePackOwner` plus snapshot/set behind a shell emit sink (Tauri adapter stayed desktop);
  #812 extracted the timeline `ViewDeltaEmitter` behind a shell emit sink;
  #814 moved live `NativeTimelineRegistry` into Core (desktop keeps the AppHandle adapter);
  #816 moved live `NativeVerificationOwner` into Core (desktop maps diagnostic ids onto Tauri errors).
  `src-tauri` retains thin compatibility re-exports. P1.6 also
  introduced the `Platform` trait and a desktop `AppHandle` adapter with no
  intended behavior change. #713/#714/#716/#717 add no P2 command, UDL, or iOS behavior.
  The remaining desktop-owned matrix domains still make the full P1 end-state
  incomplete.
- **P2 is a partial transport registry, not the complete command migration.**
  `Core::command` has typed envelopes and currently registers one hundred
  seven desktop-census commands: the prior one hundred six plus
  `matrix_invites_snapshot` (#924). Invite accept/decline/spam/block stay
  desktop because they still mutate membership plus the shared handle map.
  Password continuation stays desktop so the credential never crosses the
  envelope. Export/import stay desktop so passphrases and file paths never
  cross the envelope. Setup/restore/repair stay desktop so secrets never
  cross the envelope. Attachment send stays desktop because bytes can
  exceed the 1 MiB Core envelope. The rest of the census remains
  unregistered and fail-closed.
- **P3 has a bounded desktop seam.** Existing Tauri commands route the two
  stateless auth probes and the session lifecycle/snapshot, sync-status, and
  crypto-status observations through the managed Core while retaining their
  React-facing DTOs. The payload-free media-config, read-only
  cross-signing-status, and read-only secret-storage-status bridges are also
  routed through Core. This is not the planned whole-shell swap: the other
  desktop commands and live session ownership remain in `src-tauri`.
- **P4 has a project-owned UniFFI scaffold, credential-free typed login-flow
  discovery, iOS homeserver-discovery use, an XCTest that calls the generated
  Rust FFI scaffold, the bounded `SessionProjectionCore` mirror, and a
  Settings display-only readback that exact-matches the Swift session state and
  otherwise falls back safely.** #708 adds only a pure iOS room-row unread
  presentation: closed `Joined`/`Invited` membership plus scalar counters and a
  marked-unread flag produce a `u64` unread count and highlight boolean. #710
  adds only a pure cold-start recovery decision: a latest-state boolean plus
  `{Missing, Known}` produces a boolean; Swift maps `nil` and `.distantPast` to
  `Missing` and a real `Date` to `Known`. Neither the mirror, readback, nor
  these two pure functions is auth/session truth or a Core SDK/service owner,
  and none migrates the iOS session, room-list, timeline, crypto, push/NSE, or
  `MatrixRustSDK` service layer. P5 is not complete and iOS is not migrated to
  the shared engine.

## Current milestone ledger

The following describes merged source reachable from
`76b3a80d` (#924, after #923). #708 and #710
remain the prior bounded P4 evidence.

| Phase | Merged evidence | Current boundary |
|---|---|---|
| P0 | ADR, plan, and census | Complete planning baseline. |
| P1 | #669, #673–#677, #680–#681, #713–#714, #716–#717, #720–#721, #723–#726, #728–#729, #731–#732, #734–#735, #737–#738, #740–#741, #743–#744, #747–#748, #751, #753, #755, #757–#758, #760, #762, #764, #766, #768, #770, #772, #774, #776, #778, #780–#781, #783–#784, #786, #788, #790, #792, #794, #796, #798, #800–#801, #803, #805, #807–#808, #810, #812, #814, #816 | Extraction slices plus live client/login adapters, emit owners, NativeTimelineRegistry, and NativeVerificationOwner; leftover Keychain/Tauri commands not all moved. |
| P2 | #683–#689, #694, #698, #701–#702, #706, #818, #820, #822, #824, #826, #828, #830, #832, #834, #836, #838, #840, #842, #844, #846, #848, #850, #852, #854, #856, #858, #860, #862, #864, #866, #868, #870, #872, #874, #876, #878, #880, #882, #884, #886, #888, #890, #892, #894, #896, #898, #900, #902, #904, #906, #908, #910, #912, #914, #916, #918, #920, #922, #924 | One-hundred-seven-command registry (invites snapshot added); full desktop-invoke parity is not reached. |
| P3 | #690–#691, #694, #698, #701–#702, #706 | The listed auth and read-only/session bridges use Core; `src-tauri` is not yet a fully thin shell. |
| P4 | #685, #692–#693, #696, #699, #703, #708, #710 | UniFFI scaffold, bounded login discovery/link coverage, a safe session projection mirror, display-only readback, and only the two pure row/recovery policies exist; service migration and Apple release proof do not. |
| P5 | None | Not started. The release gates below remain required. |

Read [10-current-handoff.md](10-current-handoff.md) for the exact feature/main
provenance, live PR state, recovery separation, non-authorizations, and ordered
successor playbook. It does not change the acceptance criteria below.

## Ownership, privacy, and release gates

- **Native ownership stays explicit.** Desktop remains the sole owner of its
  Matrix SDK client, credentials, stores, and live sync/crypto state. The
  currently routed Core boundaries use only a safe session projection and
  closed platform-status projections; they do not transfer a live client or raw
  diagnostic. iOS continues to own SwiftUI, Keychain, APNs, app lifecycle, and
  NSE behavior; the current UniFFI package is not a service replacement. The
  actual SDK `Room` and timeline listener/pagination/recovery execution, and
  session, Keychain, store, crypto, sync, and lifecycle ownership, remain
  `MatrixRustSDKService`-owned.
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
| [`10-current-handoff.md`](10-current-handoff.md) | Current provenance, gates, nonclaims, and successor playbook |

## Core idea in one sentence

> Put the app logic in a crate with zero Tauri types, let desktop call it
> in-process and iOS call it through generated bindings — so both platforms
> share one engine, one test suite, and one definition of "correct."
