# 10 — Current Handoff Ledger (post-#718)

> **Purpose and scope.** This is the transfer record for the shared-native-core
> program. It records source, repository, and gate state; it does not authorize
> a merge, reconciliation, release, tag, Apple action, or claim that a phase is
> complete. The phase acceptance targets in `PLAN.md` and
> `06-migration-phases.md` remain unchanged.

## 1. Verified provenance and branch boundary

| Item | Verified state |
|---|---|
| Feature evidence tip | `feature/shared-native-core` is `aeb5b13e`, the merge commit for #866. |
| Immediately preceding merges | #865 docs after #864, #866 timeline open/jump-latest. |
| Main evidence tip | `main` is `608763799125a121572fc3b7ff613680159cbf2a`, after #712. |
| Verified common ancestry | `git merge-base` is `afe1e3148b83ee48d389d253734fdad5e8aeccd5` (#666). |

The two branch tips intentionally diverge after that common ancestor. The
feature branch is the shared-core migration lane; the later recovery work on
`main` has not thereby become feature evidence. Do not describe main recovery
documentation as reachable from this feature tip, and do not infer that either
lane has reconciled the other.

#713 and #714 are **P1-only mechanical extraction clusters**. Their exact
residency changes are:

| Merge | Domains now in `crates/synara-core/src/app/` | Desktop result |
|---|---|---|
| #713 / `3c179f03` | notifications, polls, relations, threads, unread | Thin `src-tauri/src/matrix/` re-exports preserve the desktop paths. |
| #714 / `b811319f` | raw_content, receipts, routes, security | The same thin desktop re-export pattern preserves the desktop paths. |
| #716 / `4b0775e8` | search, legacy, media_cache | Same thin desktop re-export pattern. |
| #717 / `abef91dc` | media_export, crypto_store | Same thin desktop re-export pattern. |
| #720 / `6cae3220` | members harness (error/index/tests/mod) | Desktop keeps `product_commands.rs` for `#[path]`. |
| #721 / `6666556a` | user_profile harness (error/index/tests/mod) | Desktop keeps `product_commands.rs` for `#[path]`. |
| #723 / `2df8d971` | typing index | Desktop keeps `live.rs` + `product_commands.rs`. |
| #724 / `48b767be` | presence index | Desktop keeps `live.rs` + `product_commands.rs` (clippy allows on facade). |
| #725 / `f6996ac5` | spaces hierarchy | Desktop keeps `live.rs` + `product_commands.rs`. |
| #726 / `d5cc78f8` | room_profile index | Desktop keeps `live.rs` + `product_commands.rs`. |
| #728 / `f1275b37` | media upload/download queues | Desktop keeps `product_commands.rs`. |
| #729 / `5cd9f5d1` | room_ops queue harness | Desktop keeps `product_commands.rs`. |
| #731 / `f75def05` | backup flow harness | Desktop keeps `live.rs` + `product_commands.rs`. |
| #732 / `fc08edf7` | cross-signing identity harness | Desktop keeps `live.rs` + `product_commands.rs`. |
| #734 / `9293c1f7` | room-directory session harness | Desktop keeps `live.rs` + `product_commands.rs`. |
| #735 / `a37bcbb1` | verification inbox harness | Desktop keeps `live.rs` + `product_commands.rs`. |
| #737 / `6e1609c4` | account-data index harness | Desktop keeps image_packs/later/live/room_notes + `product_commands.rs`. |
| #738 / `fac3ddce` | send-queue harness | Desktop keeps `product_commands.rs` + `live_synapse_proof`. |
| #740 / `b78fc531` | room-keys transfer harness | Desktop keeps `live.rs` + `product_commands.rs`. |
| #741 / `46f698e6` | supervisor actor harness | Thin desktop re-export; no leftover live file. |
| #744 / `94a95017` | diagnostics health harness | Thin desktop re-export; supervisor/task imports retargeted to core. |
| #743 / `ddacf34a` | store identity/paths harness | Desktop keeps keyring vault (`key_material` / `key_vault` / tests). |
| #748 / `d79dbd71` | client-builder error/features harness | Desktop keeps SDK open/config/`sdk_handle`/tests. |
| #747 / `a030aeb1` | lifecycle recovery-copy harness | Desktop keeps vault/SDK leftovers; remote-logout policy enums moved with copy. |
| #751 / `3c089693` | auth device-name harness | Desktop keeps live login/discovery/UIA/product commands. |
| #753 / `d1c5d6bb` | store vault trait + key material; auth discovery/UIA/client_config | Desktop keeps `KeyringStoreKeyVault`, `HttpDiscoveryTransport`, live login/product/register/reset. |
| #755 / `b26b2a4b` | lifecycle error domain | Desktop keeps logout, session vault I/O, wipe, persist/restore (`keyring` / `matrix_sdk`). |
| #757 / `ca7160de` | lifecycle recovery / remote-logout / wipe | Desktop keeps logout orchestration, session vault I/O, persist/restore. |
| #758 / `18b5c647` | client-builder config harness | Desktop keeps SDK `open.rs` / `sdk_handle.rs` / tests. |
| #760 / `fd8f9df3` | session-material vault trait + envelope | Desktop keeps `KeyringSessionMaterialVault`, persist/restore (`matrix_sdk`). |
| #762 / `615b501b` | well-known HTTP discovery transport | Desktop keeps product user-agent helper and live login/product. |
| #764 / `4dae1b16` | logout orchestration + task-supervisor bridge | Desktop keeps Keyring session I/O and SDK persist/restore. |
| #766 / `ffb22699` | later / room-notes codecs | Desktop keeps Client RMW live wrappers. |
| #768 / `187cb780` | image-pack DTO, type filters, write guards | Desktop keeps Client snapshot/set and `NativeImagePackOwner`. |
| #770 / `dfdfdaf4` | m.direct snapshot DTO and string-map helpers | Desktop keeps Client load/store and `DirectEventContent` write apply. |
| #772 / `12f07e8b` | device presentation DTOs and sort helper | Desktop keeps Client snapshot, UIAA delete, and `NativeDeviceOwner`. |
| #774 / `56fc4841` | secret-storage presentation DTOs and projector | Desktop keeps Client recovery I/O and host recovery-document write. |
| #776 / `26caf58d` | backup presentation DTOs and projector | Desktop keeps Client backup/recovery I/O and SDK state mapping. |
| #778 / `f31e8af2` | presence DTOs and subscription registry | Desktop keeps Client stream, event projection, and `NativePresenceOwner`. |
| #780 / `0ec8e902` | typing presentation snapshot DTO | Desktop keeps Client `m.typing` owner and typing-notice send. |
| #781 / `e23801ec` | verification presentation DTOs and phase rank | Desktop keeps Client request/SAS owner. |
| #783 / `7d277c23` | room-directory DTOs and search normalize | Desktop keeps ruma request mapping and Client protocol fetch. |
| #784 / `b2e1dacc` | space presentation DTOs and cycle guard | Desktop keeps Client hierarchy/child I/O and AllowRule reparent. |
| #786 / `c7b83d22` | cross-signing presentation DTOs and projector | Desktop keeps Client crypto I/O and UIAA. |
| #788 / `e53c4fbf` | room-key transfer presentation DTOs and projector | Desktop keeps Client/file I/O and `SelectedRoomKeyImport`. |
| #790 / `09e84e02` | members presentation snapshots and write result | Desktop keeps Client member/power-level I/O. |
| #792 / `7ee604e6` | room join-rule presentation DTO | Desktop keeps SDK JoinRule mapping and `NativeRoomJoinRuleOwner`. |
| #794 / `fe55ac06` | timeline presentation DTOs | Desktop keeps `NativeTimelineRegistry` and Client/Tauri streams. |
| #796 / `6dd69bc6` | send/profile-write/room-create/room-profile IPC DTOs | Desktop keeps Client I/O and re-exports. |
| #798 / `213b5cc5` | media upload/download/config IPC DTOs | Desktop keeps Client media I/O. |
| #800 / `571cffde` | live `Client::builder` + persist/restore | Desktop keeps Keyring vault and `SdkClientHandle`. |
| #801 / `588de584` | live login / register / password-reset | Desktop keeps Tauri product commands. |
| #803 / `93c3655b` | live `NativeTypingOwner` / `set_typing_notice` | Desktop keeps Tauri typing commands. |
| #805 / `c040632b` | live `NativeRoomJoinRuleOwner` behind emit sink | Desktop keeps Tauri event adapter. |
| #807 / `2eca137f` | live `NativeDeviceOwner` behind emit sink | Desktop keeps Tauri wakeup adapter. |
| #808 / `8e517c84` | live `NativePresenceOwner` behind emit sink | Desktop keeps Tauri event adapter. |
| #810 / `e197beea` | live `NativeImagePackOwner` plus snapshot/set | Desktop keeps Tauri event adapter. |
| #812 / `dcb0783e` | timeline `ViewDeltaEmitter` behind emit sink | Desktop keeps AppHandle adapter. |
| #814 / `986dc538` | live `NativeTimelineRegistry` | Desktop keeps `timeline_view_emit`. |
| #816 / `b9573e41` | live `NativeVerificationOwner` | Desktop maps diagnostic ids onto Tauri errors. |
| #818 / `0df2595e` | `matrix_typing_snapshot` via `Core::command` | Desktop attaches the typing owner after login. |
| #820 / `14828f73` | `matrix_presence_snapshot` via `Core::command` | Desktop attaches the presence owner after login. |
| #822 / `c45ad6ae` | `matrix_verification_list` via `Core::command` | Desktop attaches the verification owner after login. |
| #824 / `13c40365` | `matrix_device_snapshot` via `Core::command` | Desktop attaches the device owner after login. |
| #826 / `e99a61c3` | `matrix_room_join_rule_snapshot` via `Core::command` | Desktop attaches the join-rule owner after login. |
| #828 / `89b90bad` | `matrix_get_global_image_packs` via `Core::command` | Desktop attaches the image-pack owner after login. |
| #830 / `9f12bc38` | user and room image-pack snapshots via `Core::command` | Same attached image-pack owner. |
| #832 / `f003d8f7` | image-pack writes via `Core::command` | Same attached image-pack owner. |
| #834 / `72e9806e` | `matrix_typing_set` via `Core::command` | Typing owner now holds a Client clone. |
| #836 / `e681b2d8` | presence subscribe/unsubscribe via `Core::command` | Same attached presence owner. |
| #838 / `db12f31d` | `matrix_device_rename` via `Core::command` | Device owner already held a Client clone. |
| #840 / `faf5f28c` | `matrix_verification_accept` via `Core::command` | Same attached verification owner. |
| #842 / `1f8ba12b` | `matrix_verification_begin_sas` via `Core::command` | Same attached verification owner. |
| #844 / `3937c9f0` | `matrix_verification_confirm` via `Core::command` | Same attached verification owner. |
| #846 / `f00769fd` | `matrix_verification_mismatch` via `Core::command` | Same attached verification owner. |
| #848 / `4fb50ac3` | `matrix_verification_cancel` via `Core::command` | Same attached verification owner. |
| #850 / `11a430c1` | `matrix_verification_dismiss` via `Core::command` | Same attached verification owner. |
| #852 / `9bf4aa2c` | `matrix_verification_start` via `Core::command` | Verification owner now holds a Client clone. |
| #854 / `963f6719` | device delete start/cancel via `Core::command` | Owner now holds pending UIAA state. Password stays desktop. |
| #856 / `0935062a` | timeline attach + `matrix_timeline_close` | Shared `NativeTimelineOwner`; other timeline cmds still desktop. |
| #858 / `cf04eae0` | `matrix_timeline_event_readback` via `Core::command` | Timeline owner now holds a Client clone. |
| #860 / `b9b9d2db` | `matrix_timeline_paginate` via `Core::command` | Same attached timeline owner. |
| #862 / `eec805a1` | `matrix_timeline_set_read_state` via `Core::command` | Same attached timeline owner. |
| #864 / `b2a19d3f` | timeline reaction mutations via `Core::command` | Toggle, ensure, and redact. |
| #866 / `aeb5b13e` | timeline open/jump-latest via `Core::command` | Owner now holds the view-delta emit sink. |

They move pure projection code and path references only. They add **no** P2
command registration, no UDL expansion, and no iOS behavior or service-owner
change. The previous `fa6e6b63`/#710 evidence is still a useful bounded P4
anchor, but it is no longer the feature-tip provenance; use `aeb5b13e`/#866 for
current feature claims. #718 only narrows hosted iOS selection to the UniFFI /
Swift / iOS-shell surface; it does not change product behavior.

## 2. Live pull-request and release state

This state was checked at the evidence snapshot **before this ledger PR**:

- There was no open shared-core PR and no open release PR. This docs-only PR is
  the sole intended new scoped PR; it is not a source or release path.
- #705 is closed, stale, and approval-gated. Its remote head was deleted. It
  must not be reopened or merged.
- #672 is closed, stale, and unauthorized. Its remote head was deleted. It
  must not be reopened, rebased, merged, tagged, or published.
- The two open Dependabot PRs are external, stale, behind-`main`, dependency
  PRs. They target `main` and are not shared-core or release work; they are not
  part of this ledger.
- #39 is closed and draft. It authorizes no `main` action.

A **new** reconciliation PR may exist only after explicit recorded approval
from both the auth/store-security owner and the shared-Core owner. It must then
be a fresh, union-preserving reconciliation of current `main` into current
`feature/shared-native-core`, followed by fresh exact-head CI and independent
union review. These are replacement conditions, not permission to revive #705.

A **new** v2.0.4 preparation PR may exist only after separate explicit
authorization. It must start from then-current `main`, receive new review, CI,
and required physical/release evidence, and use only a new immutable v2.0.4 tag
at the exact resulting `main` SHA. v2.0.3 is untouched and is never reused.
These conditions do not authorize an action through #39.

## 3. Phase status at the feature evidence tip

### P0 — complete

ADR-0003, the plan, and the module census are established. P0 completion is a
planning baseline, not proof of an implementation, parity, release, or Apple
gate.

### P1 — in progress: extraction, not an end-state

Core-resident pieces now include DTOs, transport/IPC, the pure task registry,
and the P1 app domains `sync`, `room_list`, pure `timeline`,
`utd_recovery`, `notifications`, `polls`, `relations`, `threads`, `unread`,
`raw_content`, `receipts`, `routes`, `security`, `search`, `legacy`,
`media_cache`, `media_export`, `crypto_store`, `members`, `user_profile`,
`room_directory` session, `verification` inbox, `account_data` index,
`send` queue, `room_keys` transfer flow, `supervisor` actor,
`diagnostics` health, `store` identity/paths/key-material/vault-trait,
`client_builder` error/features/config, `lifecycle` recovery-copy /
remote-logout / wipe / error / session-material trait / logout, auth device-name, and auth
discovery/UIA/client_config, well-known HTTP transport, later/room-notes codecs,
image-pack DTO/type-filters/write-guards, m.direct snapshot helpers,
device presentation DTOs, secret-storage presentation DTOs, backup presentation DTOs, presence DTOs, typing snapshot DTO, verification presentation DTOs, room-directory DTOs, space presentation DTOs, cross-signing presentation DTOs, room-key transfer DTOs, members presentation snapshots, room join-rule presentation DTO, timeline presentation DTOs, send/profile-write/room-create/room-profile IPC DTOs, and media upload/download/config IPC DTOs.
Later mechanical splits left live `product_commands.rs` and `live.rs` on
desktop where those files exist. Compatibility re-exports remain deliberately
in the desktop shell.

Desktop still owns the unmoved or leftover matrix surfaces: live auth
login/product/register/reset_password,
client-builder SDK open/handle, devices live/UIAA, Keyring session I/O
and persist/restore, media commands, secret-storage live recovery I/O, store Keyring I/O, and the
live/command leftovers
for already-split domains (including account-data image_packs/later/notes/m.direct live Client RMW, send
product commands / synapse proofs, and room-keys live/commands). It
also retains the desktop-side commands, live/platform adapters, and applicable
tests/proofs for moved domains. Core-owned discovery/UIA types and the vault
trait do not make live login or Keychain I/O Core-owned. Therefore P1 is not
complete and `src-tauri` is not a thin shell.

### P2 — in progress: forty-one registered commands

The Core registry registers exactly these forty-one names:

1. `matrix_login_flows`
2. `matrix_register_flows`
3. `matrix_session_snapshot`
4. `matrix_sync_status`
5. `matrix_crypto_status`
6. `matrix_media_config`
7. `matrix_cross_signing_status`
8. `matrix_secret_storage_status`
9. `matrix_typing_snapshot` (#818)
10. `matrix_presence_snapshot` (#820)
11. `matrix_verification_list` (#822)
12. `matrix_device_snapshot` (#824)
13. `matrix_room_join_rule_snapshot` (#826)
14. `matrix_get_global_image_packs` (#828)
15. `matrix_get_user_image_pack` (#830)
16. `matrix_get_room_image_packs` (#830)
17. `matrix_set_user_image_pack` (#832)
18. `matrix_set_global_image_packs` (#832)
19. `matrix_set_room_image_pack` (#832)
20. `matrix_typing_set` (#834)
21. `matrix_presence_subscribe` (#836)
22. `matrix_presence_unsubscribe` (#836)
23. `matrix_device_rename` (#838)
24. `matrix_verification_accept` (#840)
25. `matrix_verification_begin_sas` (#842)
26. `matrix_verification_confirm` (#844)
27. `matrix_verification_mismatch` (#846)
28. `matrix_verification_cancel` (#848)
29. `matrix_verification_dismiss` (#850)
30. `matrix_verification_start` (#852)
31. `matrix_device_delete_start` (#854)
32. `matrix_device_delete_cancel` (#854)
33. `matrix_timeline_close` (#856)
34. `matrix_timeline_event_readback` (#858)
35. `matrix_timeline_paginate` (#860)
36. `matrix_timeline_set_read_state` (#862)
37. `matrix_timeline_reaction_toggle` (#864)
38. `matrix_reaction_ensure` (#864)
39. `matrix_reaction_redact` (#864)
40. `matrix_timeline_open` (#866)
41. `matrix_timeline_jump_latest` (#866)

All other census command names remain unregistered and fail closed. This is
neither complete desktop command parity nor a basis to add a speculative route.

### P3 — in progress: bounded desktop bridges only

The merged desktop seam routes credential-free login and registration probes,
the safe session lifecycle/snapshot, sync status, crypto status, payload-free
media configuration, read-only cross-signing status, and read-only
secret-storage status through Core while preserving the React-facing DTOs.
Desktop retains its live SDK client, credentials, persistence, and direct
command paths. It is not yet the planned whole-shell adapter swap.

P3 remains bounded. Future eligible bounded command work remains subject to
normal scope, privacy, and ownership review plus exact-head CI. Only a
`Platform`-boundary expansion or proposal to carry unsafe/dynamic material
requires a separately approved ADR/foundation.

### P4 — in progress: narrow iOS use, not a service migration

The exact bounded iOS evidence is:

- #685: project-owned UniFFI/Swift package scaffold.
- #692: typed, credential-free login-flow discovery only.
- #693: iOS homeserver discovery calls that bounded surface.
- #696: XCTest invokes the generated Rust FFI scaffold.
- #699: safe transient session-projection mirror only.
- #703: display-only Settings readback, exact-matched to Swift state with a
  safe fallback.
- #708: pure room-row unread presentation from closed `Joined`/`Invited`
  membership, scalar counters, and a marked-unread flag to a `u64` unread count
  and highlight boolean.
- #710: pure cold-start room-activity recovery decision from a latest-state
  boolean and `{Missing, Known}`; Swift maps `nil`/`.distantPast` to `Missing`
  and a real `Date` to `Known`.

#708 and #710 add no Core command route. None of these slices makes Core an SDK
or service owner. Actual SDK `Room` work, timeline listener/pagination/recovery
execution, and session, platform credential storage, store, crypto, sync, and
lifecycle ownership remain in `MatrixRustSDKService`. iOS has not migrated its
session, room-list, timeline, crypto, push/NSE, or `MatrixRustSDK` service
layer, and the direct Swift SDK dependency has not been retired.

### P5 — not started

Do not claim shared-engine iOS parity, an iOS migration, or Apple release
readiness. P5 remains bounded by the engineering work and operator/Apple gates
listed in the existing phase and release documents.

## 4. Main recovery and physical-Mac evidence boundary

The source recovery work from #695 and #697 remains on `main`; it is not a
feature-branch recovery claim. `main` commit
`608763799125a121572fc3b7ff613680159cbf2a` (#712) adds MAC-IOS-006 in the
main-only version of `MACOS_WORKSTATION_HANDOFF.md` and updates
`MACOS_IOS_VALIDATION_QUEUE.md`. This ledger intentionally does not treat that
main documentation change as a feature source link or a reconciliation.

MAC-IOS-006 is source-independent and operator-gated. It requires a physical
Mac rehearsal and a fixed, privacy-safe record only: commit SHA; platform/tool
category; fixed-case pass/fail; static diagnostic ID; and minimal redacted
failure class. Physical-Mac proof is still absent. The record is not source,
CI, release, signing, archive, or Apple proof, and it must not be expanded with
raw logs, diagnostics, identities, or private artifacts.

## 5. Gates and non-authorizations

1. **Reconciliation:** require both auth/store-security and shared-Core-owner
   approval before a *new* union-preserving current-main → current-feature PR;
   then require fresh exact-head CI and independent union review.
2. **Physical recovery:** require the approved physical-Mac operator evidence
   before treating MAC-IOS-006 as satisfied.
3. **P2/P3 boundary:** the current qualified-candidate audit found no safe
   next command under the closed, string-free platform seam. A future eligible
   bounded command may be proposed under normal review; a platform-boundary
   expansion or unsafe/dynamic field requires a separately approved
   ADR/foundation.
4. **v2.0.4:** require its separate explicit authorization, a new current-main
   PR, later review/CI/physical evidence, and a new immutable tag. Do not alter
   v2.0.3.
5. **P5:** require the remaining Core/iOS engineering migration plus the
   existing simulator, physical-device/profiling, push, distribution, and E2EE
   operator/Apple prerequisites.

No merged implementation, docs record, CI result, or #39 status substitutes
for any of those gates.

## 6. Operating model for a successor

- For each future PR, record clearly scoped ownership and fresh provenance,
  then obtain the review and exact-head CI required by repository policy. Merge
  only a reviewed, green exact head; do not merge a stale head or treat an
  apparent recovery/release path as authorization.
- Preserve shared ownership and privacy. Core accepts only bounded,
  privacy-safe projections; shells retain their live client, credentials,
  persistence, and platform behavior until an approved migration moves them.
- Batch only mechanical P1 domains with their path/re-export changes. Semantic
  Core, UDL, or Swift changes require hosted iOS coverage in addition to the
  appropriate desktop/contract review; do not disguise them as mechanical
  extraction.
- CI is path-scoped. A docs-only diff under this directory is expected to skip
  Rust, frontend, iOS, and Synapse heavy jobs while the CI scope and aggregate
  gate report. Hosted iOS simulator coverage is selected only when the UniFFI /
  UDL / bindgen / Swift / iOS-shell surface can change. On PRs targeting
  `feature/shared-native-core`, mechanical P1 `src/app/` extraction runs
  `validate-rust` fmt/clippy/check (clippy compiles tests) and skips
  `cargo test`, `cargo-audit`, and Synapse live proofs unless send/timeline
  product commands, `live_synapse_proof`, lockfiles, or this workflow change.
  Development merges of those mechanical extracts may proceed after review
  without waiting for the full quality-gate wall-clock; P1 completion still
  requires later formatting/lint/test evidence at the exact head. Push to
  `main` / `release/**` and `workflow_dispatch` still run the full suite.

## 7. Ordered resume playbook

### Immediately safe

1. Review and merge this docs-only ledger PR only if its exact head is green;
   it changes neither source nor a release path.
2. Retain the verified feature/main provenance above, then clean the worker's
   own branch and worktree after that PR is merged or closed.
3. For any future docs-only correction, fetch first, use the fresh feature tip,
   and state its exact provenance. Do not reuse a stale branch or PR.

### Requires new approval

1. Obtain both named owner approvals before opening a new current-main →
   current-feature reconciliation, then perform the required fresh union review
   and exact-head CI.
2. For P2/P3, use normal review for a future eligible bounded command. Obtain
   a separately approved ADR/foundation only for a platform-boundary expansion
   or unsafe/dynamic field.
3. Obtain separate release authorization before opening a new v2.0.4
   preparation PR from then-current `main`.

### Requires a physical operator

1. Run MAC-IOS-006 only in the approved physical-Mac operator environment and
   retain only its fixed privacy-safe evidence record.
2. For P5, obtain the required physical-device/profiling, push, distribution,
   and production E2EE evidence through the existing operator/Apple procedures.

### Prohibited without approval

- Reopen, rebase, merge, tag from, or otherwise revive #705 or #672.
- Reconcile `main` into feature, act on #39/`main`, create or move a release
  tag, publish v2.0.4, or change v2.0.3.
- Widen the platform boundary or introduce an unsafe/dynamic field without the
  separately approved ADR/foundation.
- Treat source, CI, docs, simulator, or this ledger as physical-Mac, release,
  signing, distribution, Apple, or shared-engine-parity evidence.

## 8. Evidence checklist before program-goal completion

This checklist aggregates existing acceptance criteria; it does not replace or
relax them.

- [ ] P1 has moved the intended app logic with compatibility behavior preserved,
  matrix-boundary proof, formatting/lint/test evidence, and review at the exact
  head.
- [ ] P2 covers the complete desktop command census with contract/parity
  coverage and preserves the React-facing `matrix_*` contract.
- [ ] P3 reduces `src-tauri` to the planned thin adapter with exact-head full
  matrix, package-smoke, link-smoke, and boundary evidence.
- [ ] P4 builds the project-owned Apple bindings, migrates services in safe
  dependency order, passes hosted iOS coverage, and retires direct
  `MatrixRustSDK` use only when no consumer remains.
- [ ] P5 has the required shared-core desktop matrix/compatibility evidence,
  iOS simulator evidence, signed physical-device and profiling evidence,
  production push validation, distribution evidence, and production E2EE
  recovery/verification/backup/media evidence.
- [ ] MAC-IOS-006 has its separate approved physical-Mac, fixed privacy-safe
  record; no source or CI substitute is accepted.
- [ ] Any reconciliation or release action has its separate approvals, fresh
  current-tip review/CI, and immutable-tag controls.
