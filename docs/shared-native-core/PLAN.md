# Shared Native Core (synara-core) — Program Plan

**Status at `main`
`7ecbfdf9` (#1001 merge of P4-S12–S37):** P0 is
complete; P1 extraction and bounded P2, P3, and P4 slices are merged. P2–P4
remain in progress. P4 engine ready is not claimed. P5 has not started.
Hosted iOS CI is paused (#1003). Live homeserver proof is paused. Owner:
Synara engineering. The current provenance, gates, and successor steps are
in `10-current-handoff.md`.
**Hand a weaker model [11-implementer-playbook.md](11-implementer-playbook.md).**

## Goal

One Rust app-logic core (`crates/synara-core`) that both the desktop Tauri
app (macOS and Linux) and the iOS app consume, so sync, room list, timeline,
and crypto are not implemented twice.

That end state has **not** been reached. SNC engineering is on `main`
via #991, with P4-S12–S37 on `main` via #1001. It is not a release.

## Decision record

- ADR 0003: `docs/adr/0003-shared-native-rust-core.md`
- ADR 0004 (what may be written in Rust): `docs/adr/0004-rust-language-boundaries.md`

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
  mechanically moved raw content, receipts, routes, and security; #716 moved
  search, legacy, and media_cache; #717 moved media_export and crypto_store;
  #720/#721 moved the members and user_profile harnesses while leaving live
  `product_commands.rs` on desktop; #734/#735 moved the room-directory session
  and verification inbox harnesses the same way (`live.rs` + commands stayed
  desktop); #737/#738 moved the account-data index and send-queue harnesses
  (live leftovers stayed desktop); #740/#741 moved the room-keys transfer
  harness and the supervisor actor; #744 moved diagnostics health; #743 moved
  store identity/paths (keyring vault stayed desktop); #748 moved
  client-builder error/features (SDK open stayed desktop); #747 moved
  lifecycle recovery-copy (vault/SDK stayed desktop); #751 moved auth
  device-name (live login stayed desktop); #753 moved the store vault
  trait / key material and auth discovery/UIA/client_config (Keyring I/O
  and live login stayed desktop); #755 moved the lifecycle error
  domain (logout/session vault I/O/SDK restore stayed desktop); #757 moved
  lifecycle recovery/remote-logout/wipe; #758 moved client-builder config
  (SDK open stayed desktop); #760 moved the session-material vault trait
  (Keyring I/O stayed desktop); #762 moved the well-known HTTP
  discovery transport (live login stayed desktop); #764 moved logout
  orchestration and the task-supervisor bridge; #766 moved later/room-notes
  codecs (Client RMW stayed desktop); #768 moved image-pack DTO, type filters,
  and write guards (Client snapshot/set and Tauri subscribe stayed desktop);
  #770 moved m.direct snapshot DTO and string-map helpers (Client load/store
  and DirectEventContent write stayed desktop); #772 moved device presentation
  DTOs and sort helper (Client snapshot, UIAA delete, and Tauri owner stayed
  desktop); #774 moved secret-storage presentation DTOs and projector
  (Client recovery I/O stayed desktop); #776 moved backup presentation DTOs
  and projector (Client backup/recovery I/O stayed desktop); #778 moved presence DTOs
  and subscription registry (Client stream and Tauri owner stayed desktop);
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
  These retain thin desktop re-exports and
  add no P2 command, UDL, or iOS behavior. Compatibility re-exports remain, and many matrix domains are
  still desktop-owned, so the full extraction/end state is not yet complete.
- **P2 — in progress; transport registry is intentionally partial.** #683–#684
  added `Core::command`, typed envelopes, the registry, and the desktop command
  census. The merged registry currently has exactly those eight plus
  `matrix_typing_snapshot` (#818), `matrix_presence_snapshot` (#820),
  `matrix_verification_list` (#822), `matrix_device_snapshot` (#824), and
  `matrix_room_join_rule_snapshot` (#826), `matrix_get_global_image_packs` (#828),
  `matrix_get_user_image_pack` (#830), `matrix_get_room_image_packs` (#830),
  and the three image-pack writes (#832):
  `matrix_login_flows`, `matrix_register_flows`, `matrix_session_snapshot`,
  `matrix_sync_status`, `matrix_crypto_status`, `matrix_media_config`,
  `matrix_cross_signing_status`, `matrix_secret_storage_status`,
  `matrix_typing_snapshot`, `matrix_presence_snapshot`,
  `matrix_verification_list`, `matrix_device_snapshot`,
  `matrix_room_join_rule_snapshot`, `matrix_get_global_image_packs`,
  `matrix_get_user_image_pack`, `matrix_get_room_image_packs`,
  `matrix_set_user_image_pack`, `matrix_set_global_image_packs`, and
  `matrix_set_room_image_pack`
  (#686–#689, #694, #698, #701–#702, #706, #818, #820, #822, #824, #826, #828, #830, #832). The status/media
  routes preserve bounded legacy contracts through Core.
  Later 7B slices through #928 grow that registry to one hundred eleven names,
  including invite spam/block. Neither #708, #710, #713,
  #714, #716, nor #717 adds a Core command route. It is not the complete
  desktop command registry; unregistered census names fail closed.
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
  crypto, sync, and lifecycle ownership later moved off the leftover
  Swift SDK client in #986.
  #931 exposes credential-free `register_flows` on UniFFI (closed DTO, no
  `params` JSON, no register/password/email-token). #933 adds constructor-only
  UniFFI `SharedCore`. #935 installs a Swift `IosSecretVault` callback on that
  constructor. S3b–S3d (#937–#939) add restore, dedicated
  `login_with_password`, and owner attach (SyncService not started).
  S4–S9-31 (#940–#979) add typed SharedCore wrappers for the registered
  command families. #949 exposes the UniFFI 0.28 store constructor. #980
  refreshes desktop source-scan tests. #981 refreshes provenance. #982
  records local UniFFI generate. #983 recorded a temporary S10 leftover
  stop (playbook §9.6); operator authorization plus #986 superseded it.
  #984 lands the S11 NSE read-only store helper without starting sync;
  it is not a product NSE swap. #985 refreshes provenance. #986 lands
  S10 leftover UniFFI and retires product `MatrixRustSDK` callers.
  Leftover I/O that needs a live homeserver stays fail-closed planted.
  #987 refreshes provenance. #988 re-enables iOS CI (Quality on #988
  green including iOS, not skipped). #989 refreshes provenance. #990
  union-preserves `main` store recovery. #992 raises the rustc
  recursion limit. #991 merges SNC onto `main` (`05a0961c`; Quality +
  iOS + package smoke green). #1000 records ADR 0004. #1002 adds the
  Long-running recipe. #1003 pauses hosted iOS CI. #1001 lands
  P4-S12–S37 (`7ecbfdf9`). SyncService can be started through
  SharedCore; live homeserver proof is paused. This is not
  iOS-on-engine. Local Apple generate has been run for earlier fields;
  new S30–S35 UniFFI fields still need Apple generate. Generated
  sources remain gitignored. Checked-in `SynaraCore.swift` remains the
  bootstrap stub. iOS still has no registration product UI. P4 is not
  accepted.
- **P5 — not started.** Do not claim iOS shared-engine parity, iOS migration,
  or Apple release readiness from the bounded work above.

## What is left (ordered)

1. **No further 7B desktop route** unless a leftover in the playbook
   section 6 gains a new owner decision. Presence, device rename, and all
   five invite commands are already registered. Twenty-one census names
   stay desktop (secrets, file paths, bytes).
2. **P4 serial iOS** of surfaces Core already owns, in the playbook
   section 9 order. S1–S9-31, S10 leftover retirement (#986), S11,
   and S12–S37 (#1001) have landed. Product `MatrixRustSDK` callers
   are retired. Leftover I/O that needs a live homeserver stays
   fail-closed. Hosted iOS CI is paused (#1003). Live homeserver
   proof is paused. Regenerating Swift bindings needs disk ≥ 20 Gi
   and Apple generate for the new UniFFI fields. Dual-platform
   “one Core bugfix on desktop+iOS” is not claimed. Do not start P5.
   Do not claim P4 engine ready. The operator-approved feature-lane
   merge to `main` landed in #991; S12–S37 landed in #1001.
3. **P3 thinning** continues as iOS/desktop leftovers shrink. Desktop is
   not a thin shell while Keychain, byte commands, and password UIAA live
   there — and those leftovers are *intended*.
4. **P1 ledger** stays open until leftover Keychain/Tauri adapters are
   accepted as the permanent shell, not because more mechanical `git mv`
   remains.
5. **P5** after remaining engineering, then operator/Apple gates
   (signed device, APNs, TestFlight, production E2EE). MAC-IOS-006 is
   not the coding path. SNC engineering already lives on `main`.

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
- Worktree isolation; branch base = `main`; squashed PRs; provenance anchor.
- Domain modules carry their tests; preserve behavior while extracting them.
- The IPC/transport contract tests and command census are the north star for
  transport migration; unregistered commands fail closed rather than silently
  changing behavior.
- No phase or release checkbox closes without its stated evidence, including
  privacy review and the relevant operator/Apple gates.
