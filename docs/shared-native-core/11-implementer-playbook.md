# 11 — Implementer playbook (hand this to the next model)

This file is the operating manual for finishing the shared native core.
Read it before editing product code. If this file and an older plan
paragraph disagree, this file plus `10-current-handoff.md` win for
*what to do next*; ADR-0003 plus `01-context-and-goals.md` win for
*what done means*.

Older “144 `Core::command` handlers” sentences in `03-target-architecture.md`
and `06-migration-phases.md` are stale. Do not register the 21 leftovers in
section 6 to satisfy them.

Evidence tip when this playbook was written: `feature/shared-native-core`
`ee896416` (#935 S3a SecretVault). Re-fetch before you start.
Do not treat this SHA as eternal.

---

## 1. Desired end state (do not paraphrase away)

One Rust core (`crates/synara-core`) that both the desktop Tauri app
(macOS and Linux) and the iOS app consume, so sync, room list, timeline,
and crypto are not implemented twice.

That end state has **not** been reached. The work lives only on
`feature/shared-native-core`. It is not on `main`. It is not a release.

Never claim the program is 100% complete. Never claim iOS is on the
shared engine. Never claim P5 or MAC-IOS-006 is done.

---

## 2. Current state (honest)

| Surface | Today |
|---|---|
| Desktop macOS/Linux | Live Matrix `Client` and native owners live in Core. React still invokes `matrix_*`. **111** of those names are registered on `Core::command`. Desktop is a thinner shell, not a thin shell. |
| iOS | UniFFI scaffold + `login_flows` + `register_flows` + session-projection mirror + Settings readback + two pure helpers (unread row, cold-start recovery) + `SharedCore` constructors + optional `IosSecretVault` + `restore_persisted_session` + dedicated `login_with_password` FFI + `attach_session_owners` + typed `room_list_snapshot` (not on-engine). Live room list, timeline, crypto, push, and `MatrixRustSDKService` are still Swift. iOS does **not** call the other 110 Core commands. XCTest construction of `SharedCore` is not iOS-on-engine. |
| `main` | Diverged. Recovery / MAC-IOS-006 docs live there. Not feature evidence. |
| Release | Forbidden until engineering finish is accepted and then merged to `main`. |

Census size: `REACT_MATRIX_COMMAND_CENSUS` in
`crates/synara-core/src/transport/census.rs` is the full React/Tauri
`matrix_*` list (lexically sorted). **111 registered. 21 unregistered
and fail-closed.** Those 21 are intended shell leftovers (section 6).

---

## 3. Absolute rules (break any one and stop)

1. Merge only to `feature/shared-native-core`.
2. Never merge `main`. Never touch #39, #705, #672, tags, or releases.
3. Never claim 100%, iOS shared-engine parity, or Apple readiness.
4. Core may use `matrix_sdk`. Core must not import Tauri. Core must not
   depend on the `keyring` crate.
5. Register a `Core::command` only when the capability already exists on
   an attached owner (or is a credential-free probe like login/register
   flows). Do not invent a route so a watchdog has a merge.
6. React command names stay `matrix_*`. Payloads stay camelCase with
   `deny_unknown_fields`.
7. Product events take a shell emit callback
   (`Arc<dyn Fn(Payload) + Send + Sync>`). **`Platform::emit` is the
   typed IPC envelope stream.** Do not wrap product events in
   `Platform::emit`. That would break React event names.
8. Passwords, recovery passphrases, `client_secret`, file paths, and
   media/attachment bytes do **not** cross the Core envelope
   (`MAX_ENVELOPE_PAYLOAD_JSON_BYTES` = 1 MiB). Attachment/media stay
   desktop because the product cap is 32 MiB.
9. Skip `cargo test` / `cargo check` / `cargo build` unless free disk
   is **≥ 20 Gi**. rustfmt from the worktree is always allowed:
   `cd <worktree> && rustfmt --edition 2021 <files>`.
   Never rustfmt `Cargo.toml`.
10. UniFFI / bindgen / xcframework work also needs disk ≥ 20 Gi.
    If disk is under 20 Gi: stop. Docs-only PRs are still allowed.
    Do not change UDL. Do not hand-edit generated Swift. Do not invent
    a no-bindgen path.
11. After each **product** PR: squash-merge to `feature/shared-native-core`
    only, then a **docs honesty** PR, then **re-run section 5**. If
    section 5 step 4, stop. Do not start a second product slice to keep
    a turn busy.
12. `gh pr create --base feature/shared-native-core`. Merge with
    `gh pr merge N --squash`. Do not pass `--delete-branch` if the
    feature branch is checked out locally (GitHub still merges).
13. Do not launch DeepSeek on scheduler fires.
14. MAC-IOS-006 is operator-gated. It is not the coding path.
15. Apple-only UI, Keychain, APNs, and NSE stay Swift. Logic moves; UI
    does not.

Worktree for this program: `synara-desktop-snc-image-packs`.
The owner surface is `origin/feature/shared-native-core`, not the folder
name. Orchestrator notes: `/Users/nepenthe/.grok/snc-orchestrator/STATE.md`.

---

## 4. Owner decisions in force (2026-08-13)

These are numbered owner calls. Do not re-litigate them in a slice PR.

| # | Decision |
|---|---|
| 1B | Core may use `matrix_sdk`. No Tauri. No `keyring` crate. |
| 2B | Live `Client::builder` + persist/restore live in Core (#800). Keyring vault stays desktop. |
| 3B | Core owns login orchestration (#801). Tauri product commands that take passwords stay desktop. |
| 4B | Native owners and the timeline registry live in Core (through #816). |
| 5B | P1 owner/adapter extraction for 2–4 landed. The P1 ledger is not closed. |
| 6 | Register Core commands only as capabilities land. No speculative routes. |
| 7B | Route remaining `matrix_*` through `Core::command` as owners land. Desktop leftovers in section 6 stay desktop. |
| 8B | Serial iOS after Core owns a live client. Apple-only stays Swift. |
| 9 | Feature branch only until engineering complete, then `main`. |
| 10 | Continue engineering. MAC-IOS-006 is not the coding path. |
| 11 | No release until shared core is complete and merged to `main`. |
| 12 | Apple proof stays operator-gated. |
| 13 | DeepSeek paused. Grok does the routes. |

---

## 5. How to pick the next slice

Run this checklist in order. Stop at the first yes.

1. **Mid-slice?** Finish the open product branch in the worktree. Do not
   start a second product branch.
2. **Is there an unregistered census name whose write already lives on
   an attached owner and does not need a secret, file path, or bytes in
   the envelope?** That is a 7B desktop route. Use section 8.
   **As of #928 there are none.** Presence subscribe/unsubscribe and
   device rename are already registered. Confirm with section 6 before
   inventing one.
3. **Is free disk ≥ 20 Gi?** Start the next **P4** slice in section 9.
   S1/#931, S2/#933, S3a/#935, S3b/#937, S3c/#938, S3d/#939,
   S4/#940, S5/#942, S6/#944, S7/#945, S8/#947, S9/#948,
   S9-2/#950, S9-3/#951, S9-4/#952, S9-5/#953, S9-6/#954,
   S9-7/#955, S9-8/#956, S9-9/#957, S9-10/#958, S9-11/#959, and
   S9-12/#960, S9-13/#961, S9-14/#962, S9-15/#963, S9-16/#964,
   S9-17/#965, and S9-18/#966 landed. S9-19 timeline read-state
   (section 9.5) is on this branch. Next after merge is timeline
   reactions.
   UDL/bindgen/cargo require this disk
   gate. There is no “land UDL as source without local cargo/bindgen”
   exception. If disk is under 20 Gi: stop. Docs-only PRs are still
   allowed. Do not change UDL. Do not hand-edit generated Swift. Do
   not invent a no-bindgen path.
4. **Otherwise stop.** Update `STATE.md`. Do not open a padding PR.
   Do not route logout, email-token, or media "just to have a merge."

---

## 6. The 21 unregistered census names (stay desktop)

Source of truth: `crates/synara-core/src/transport/census.rs` minus
`Core::registered_commands()` in `crates/synara-core/src/core.rs`.

| Census name | Why it stays desktop |
|---|---|
| `matrix_login_password` | Password must not cross the Core envelope. |
| `matrix_register` | Password + UIAA. |
| `matrix_register_request_email_token` | Carries `client_secret`. |
| `matrix_password_reset_request_email_token` | Carries `client_secret`. |
| `matrix_password_reset_complete` | New password + `client_secret`. |
| `matrix_restore_session` | Keyring vault I/O. |
| `matrix_logout` | Keyring + AppHandle + app-data cleanup. Remote logout is not a standalone 7B. |
| `matrix_device_delete_password` | Password UIAA. Start/cancel already go through Core. |
| `matrix_cross_signing_setup_password` | Password UIAA. Setup start already goes through Core. |
| `matrix_backup_setup` | Recovery passphrase. Status already through Core. |
| `matrix_backup_restore` | Recovery secret. |
| `matrix_backup_repair` | Recovery secret. |
| `matrix_secret_storage_bootstrap` | Passphrase. Status already through Core. |
| `matrix_secret_storage_unlock` | Recovery secret. |
| `matrix_secret_storage_reset` | Passphrase. |
| `matrix_room_key_export` | Passphrase + file path. Status already through Core. |
| `matrix_room_key_import` | Passphrase + file path. |
| `matrix_room_key_import_select` | Desktop file picker; paths must not cross the envelope. |
| `matrix_send_attachment` | Bytes can exceed 1 MiB envelope (32 MiB product cap). |
| `matrix_upload_media` | Same bytes rule. |
| `matrix_media_download` | Same bytes rule. Config already through Core. |

Do not register these unless a **new written owner decision** says the
secret or bytes may cross the envelope, or a new Platform ADR defines a
byte channel. A watchdog prompt is not that decision.

**S3b is not `matrix_restore_session`.** That census name stays desktop
because it does Keyring vault I/O. iOS restore is a new `SharedCore`
FFI over Core persist/restore plus the S3a vault. Do not register
`matrix_restore_session` on `Core::command` to “help” S3b.

---

## 7. Attached owners (where live I/O lives)

Desktop session install (`src-tauri/src/matrix/auth/product_commands.rs`)
builds owners, wraps them in `Arc`, stores them on
`ManagedMatrixSession`, and calls `Core::attach_*`. `Core::close` clears
them. Missing owner → `Forbidden` with `p2-*-no-session`.

| Owner | Core type | Typical commands |
|---|---|---|
| Typing | `NativeTypingOwner` | `matrix_typing_snapshot`, `matrix_typing_set` |
| Presence | `NativePresenceOwner` | snapshot, subscribe, unsubscribe |
| Verification | `NativeVerificationOwner` | list + SAS flow commands |
| Devices | `NativeDeviceOwner` | snapshot, rename, delete start/cancel, backup_status, room-key status, cross-signing setup start |
| Join rules / room profile | `NativeRoomJoinRuleOwner` | join-rule snapshot, name/topic/avatar, directory, leave/join, moderation, power levels, create, members, spaces, **all five invite commands** |
| Image packs / account data | `NativeImagePackOwner` | image packs, later, m.direct, room notes, own display-name/avatar |
| Timelines | `NativeTimelineOwner` | open/close/paginate/read/reactions/composer/send text/edit/sticker/poll |
| Sync | `SyncServiceOwner` | `matrix_room_list_snapshot` |

Invite avatar handles live on `NativeRoomJoinRuleOwner`. Desktop keeps
an `Arc` of the same map for URI resolve (bytes stay desktop).

Product event owners are constructed with a shell emit callback.
Desktop adapters in `src-tauri/src/bridge/` keep existing Tauri event
names.

---

## 8. Recipe: a 7B desktop `Core::command` route

Use this only when section 5 step 2 is yes.

### 8.1 Before you type

- `git fetch origin feature/shared-native-core`
- Worktree clean. Branch `agent/snc-<short-name>` from origin tip.
- Confirm the census name exists and is **not** already in
  `core.registered_commands()`.
- Confirm the owner method can run without a new Platform ADR.

### 8.2 Implementation order

1. **Owner method** on the existing owner (usually
   `crates/synara-core/src/app/<domain>/live.rs`).
   - Retired / missing session → existing `requires-session` diagnostic.
   - Invalid ids → `SdkInvariant` diagnostics.
   - Copy the current desktop behavior bit-for-bit.
2. **Request struct** in `crates/synara-core/src/core.rs`:
   `#[derive(Deserialize)] #[serde(rename_all = "camelCase", deny_unknown_fields)]`.
3. **Handler** `fn matrix_<name>(state, request) -> CommandFuture`.
   Missing owner → `Forbidden` + `p2-<name>-no-session`.
4. **Register** next to neighbors. Update the
   `default_registry_dispatches_matrix_session_snapshot` vec (lexical).
5. **Fail-closed test** `matrix_<name>_without_owner_fails_closed`.
6. **Desktop bridge** in `src-tauri/src/bridge/` (new file or extend a
   sibling). Envelope `session_generation: 0` for these writes unless
   the existing command already stamps a generation.
7. **Thin the Tauri command** to `crate::bridge::…`. React name and
   args unchanged.
8. **Product source test** in
   `src-tauri/src/matrix/auth/product_tests.rs` asserting the thin
   bridge and `!active.client`.
9. `rustfmt --edition 2021` from the worktree on touched `.rs` files.
10. Commit `feat(core): route <name> through Core::command`.
11. `gh pr create --base feature/shared-native-core`.
12. `gh pr merge N --squash` with subject ending `(#N)`.
13. Docs honesty PR (section 10).
14. Re-run section 5. If step 4, stop.

### 8.3 Do not

- Change React names or camelCase fields.
- Put Tauri types in `crates/synara-core`.
- rustfmt `Cargo.toml`.
- Run cargo when disk < 20 Gi.
- Route two unrelated commands in one PR unless they are a documented
  pair (example: invite accept + decline).

---

## 9. Recipe: serial iOS (P4) — the next real lane

P4 is **not** "add UniFFI and delete Swift in one PR." iOS may consume
a Core surface only after the dependency under it exists.

### 9.1 Dependency order (do not skip)

```text
P4-S0  already landed: UniFFI scaffold, login_flows, SessionProjectionCore,
       Settings readback, room_unread_presentation, room_activity_recovery_required

P4-S1  UniFFI register_flows          LANDED #931 (credential-free)
P4-S2  Swift Platform + Core::new     LANDED #933 (fail-closed; no command)
P4-S3  iOS live Client via Core       plan: 12-p4-s3-live-client.md
       S3a LANDED #935 → S3b LANDED #937 → S3c LANDED #938 → S3d LANDED #939
P4-S4  iOS room_list_snapshot         LANDED #940
P4-S5  iOS invites_snapshot           LANDED #942
P4-S6  iOS timeline open/close/paginate LANDED #944
P4-S7  iOS typing / presence          LANDED #945
P4-S8  iOS verification list          LANDED #947
P4-S9  iOS verification SAS           LANDED #948
P4-S9-2 iOS devices                   stacked #950
       matrix_device_snapshot / rename / delete_start / delete_cancel
       SyncService is not started. Backup/room-key/cross-signing stay off.
P4-S9-3 iOS join rules                stacked #951
       matrix_room_join_rule_snapshot only. No writer.
P4-S9-4 iOS image packs               stacked #952
       six registered get/set commands. Metadata/JSON only.
P4-S9-5 iOS later                     stacked #953
       six registered later commands.
P4-S9-6 iOS m.direct                  stacked #954
       three registered m.direct commands.
P4-S9-7 iOS room notes                stacked #955
       five registered room-notes commands.
P4-S9-8 iOS own display-name/avatar   stacked #956
       matrix_set_own_display_name / matrix_set_own_avatar.
       Avatar is mxc:// (or empty clear) only. Image bytes stay off.
P4-S9-9 iOS room name/topic/avatar    stacked #957
       matrix_set_room_name / matrix_set_room_topic / matrix_set_room_avatar.
       Room avatar is mxc:// (or empty clear) only. Image bytes stay off.
P4-S9-10 iOS directory visibility     stacked
       matrix_get_room_directory_visibility / matrix_set_room_directory_visibility.
P4-S9-11 iOS directory search         stacked
       matrix_room_directory_protocols / matrix_room_directory_search /
       matrix_room_directory_cancel. Results stay metadata. No avatar bytes.
P4-S9-12 iOS room leave/join          stacked
       matrix_room_leave / matrix_room_join. Write ack is status only.
P4-S9-13 iOS room invite/kick/ban     stacked
       matrix_room_invite / matrix_room_kick / matrix_room_ban /
       matrix_room_unban. Write ack is status only.
P4-S9-14 iOS room power levels        stacked
       matrix_room_set_power_level / matrix_room_set_power_levels /
       matrix_room_set_power_level_tags. Write ack is status only.
P4-S9-15 iOS room create              stacked
       matrix_room_create only. Typed name/topic/alias/visibility/preset
       plus Core scalar extras. Nested create-content and power-level
       overrides stay off.
P4-S9-16 iOS members snapshots        stacked
       matrix_room_members_snapshot / matrix_room_power_levels_snapshot /
       matrix_room_creators_snapshot / matrix_room_power_level_tags_snapshot.
       Reads only.
P4-S9-17 iOS spaces                   stacked
       matrix_space_parents_snapshot / matrix_space_hierarchy_snapshot /
       matrix_space_children_snapshot / matrix_space_child_set /
       matrix_space_child_remove / matrix_restricted_join_reparent.
       Child set/remove are metadata only. Invite accept/decline stay off.
P4-S9-18 iOS invite actions           stacked
       matrix_invites_accept / matrix_invites_decline /
       matrix_invites_report_spam / matrix_invites_block_sender.
       Returns the existing invite snapshot. Do not re-wrap S5 snapshot.
P4-S9-19 iOS timeline read-state      **this branch**
       matrix_timeline_event_readback / matrix_timeline_set_read_state /
       matrix_timeline_jump_latest.
       Jump returns the existing open readback. Do not re-wrap S6 open.
       Reactions stay off.
P4-S10 retire MatrixRustSDKService / RoomListService / TimelineService
       only when grep shows no remaining product callers
P4-S11 NSE read-only store API        (never boot sync in NSE)
```

Apple-only stays Swift the whole way: SwiftUI, Keychain UI, APNs
permission, file pickers, settings chrome.

### 9.2 Hard gate for P4-S1 and later that touch UDL

- Free disk **≥ 20 Gi** (`df -h /` and the volume that holds the
  worktree). If either is under 20 Gi: stop. Docs-only PRs are still
  allowed. Do not change UDL. Do not hand-edit generated Swift. Do
  not invent a no-bindgen path.
- Regenerating UniFFI without cargo/bindgen is forbidden. Do not
  hand-edit generated Swift to fake a UDL change.
- Generated artifacts live under `synara-ios/SynaraCore/`. Follow the
  existing `login_flows` pattern in:
  - `crates/synara-core/src/synara_core.udl`
  - `crates/synara-core/src/ffi.rs` (namespace probes: login/register flows)
  - `crates/synara-core/src/shared_core_ffi.rs` (**S3 `SharedCore` FFI**)
  - `crates/synara-core/src/session_projection_ffi.rs` (projection only)
- Errors that cross UniFFI must be **static** (category/code/description
  from source constants). Never echo URLs, tokens, or HTTP bodies.

### 9.3 P4-S1 — `register_flows` (historical; LANDED #931)

Do not start this slice. Copy of the landed recipe, kept so later
UniFFI probes can mirror it.

| Item | Value |
|---|---|
| Rust domain | `probe_register_flows` already in `crates/synara-core/src/app/auth/` |
| Desktop command | already `matrix_register_flows` through `Core::command` |
| New UniFFI | `register_flows(homeserver_url) -> RegisterFlowsDto` or reuse the desktop DTO shape |
| iOS call site | only if a registration discovery UI exists; otherwise expose + XCTest |
| Must not add | `matrix_register`, email token, password |
| Tests | invalid URL fails closed without echoing input (mirror `ffi.rs` login_flows tests) |
| Commit | `feat(core): expose register_flows on UniFFI` |

### 9.4 P4-S3 — live Client on iOS (four serial PRs; do not start before S2)

S3 is **not** one PR. Read the matching subsection only.

| Slice | Section | What it is |
|---|---|---|
| S3a | landed #935 | Vault callback only |
| **S3b** | **9.4b landed #937** | Restore via vault. No password. |
| **S3c** | **9.4c landed #938** | Dedicated `login_with_password` FFI |
| **S3d** | **9.4d landed #939** | Attach owners after a session exists |

Desktop reference (do not copy Tauri types):

- `src-tauri/src/matrix/auth/product_commands.rs` login / restore /
  `open_after_desktop_session_install` / `attach_*`
- Keyring vault stays a **shell** `SecretVault` impl. iOS equivalent is
  Keychain behind the same trait. The `keyring` crate stays out of Core.

### 9.4b P4-S3b — restore only (landed #937)

Plan: `12-p4-s3-live-client.md`. Use this subsection only.

**Lands:** restore an already-persisted session through Core
persist/restore using the S3a vault. No password.

**S3b is not `matrix_restore_session`.** Do not register that desktop
leftover. Do not put Keyring types in Core.

**Must not land:** password; `attach_*` (that is S3d); `Core.command` /
command families (S4 adds the first UniFFI command wrapper, for
`matrix_room_list_snapshot` only); desktop `matrix_restore_session` as
a Core envelope command; leftover registration; an invented 7B;
`Platform::emit` for product events; NSE / APNs / booting sync in NSE;
retiring `MatrixRustSDKService`; merge to `main`.

**UDL:** `SharedCore` is constructor-only until this slice. S3b **adds**
restore FFI on `SharedCore`. That is a UDL change. Section 9.2 disk
gate applies. Implement in `crates/synara-core/src/shared_core_ffi.rs`.
Do not extend `session_projection_ffi.rs`.

**Rust reference (do not copy Tauri/Keyring types):**
`restore_session_from_vault` / `restore_session_onto_client` in
`crates/synara-core/src/app/lifecycle/`.

**Swift:** wire the restore path through the S3a vault only as far as
restore requires. XCTest construction of `SharedCore` is not
iOS-on-engine. Do not install password login. Do not attach owners.

**If disk < 20 Gi:** stop. Docs-only updates to this playbook are
allowed. Do not change UDL.

### 9.4c P4-S3c — password login (Option A chosen)

S3b landed in #937 (`4edfc1f5`). S3c landed in #938 (`9b4ec54f`).
This slice proceeds from that tip on `feature/shared-native-core`.

**Chosen: Option A — dedicated `SharedCore.login_with_password` FFI.**

Why not keep Swift `MatrixRustSDK` login (Option B): Core already owns
`login_with_password` (decision 3B); desktop already calls it from the
shell. S3b restore only works if login persists into the same vault +
`StoreKeyId`. A Swift SDK login would write a different session store
and leave S3b unrestorable from product login.

**Password on UniFFI:** it may cross as the dedicated method argument
only. It is not stored, not copied into a DTO, and never echoed in
errors. Rust zeroizes the argument after the Core call. Password never
rides generic `Core.command`. Do not register `matrix_login_password`.

**Must not land:** `attach_*` (S3d); `Core.command` (S4); leftover
registration; retiring `MatrixRustSDKService`; register / email token /
recovery passphrase.

Dual-engine until S3d: login+restore on `SharedCore`; room list,
timeline, and crypto stay `MatrixRustSDK`. Live product login may stay
on the Swift auth service for this slice; helper + XCTest are enough.

### 9.4d P4-S3d — attach owners (landed #939)

S3c landed in #938 (`9b4ec54f`). S3d landed in #939 (`ad63d56d`).
After a session exists, attach the same
owner set desktop attaches from
`src-tauri/src/matrix/auth/product_commands.rs`:

`attach_typing`, `attach_presence`, `attach_verification`,
`attach_devices`, `attach_join_rules`, `attach_image_packs`,
`attach_timelines`, `attach_sync`.

`SharedCore.attach_session_owners` builds those owners on the retained
Client and calls the Core `attach_*` APIs. Emit sinks are no-op
(`Platform::emit` stays later). SyncService is attached but not started
so iOS does not run a second live sync while `MatrixRustSDKService`
still owns product room list / timeline. Second attach fail-closes.
Missing attach → later commands fail closed. That is correct.
Do not retire `MatrixRustSDKService`. Do not add `Core.command`.

### 9.5 P4-S4+ — consume an already-registered command

S9-18 invite actions is stacked at #966. **S9-19 (this branch)** adds typed
UniFFI wrappers for the three registered timeline read-state commands.

1. `SharedCore.timeline_event_readback` / `timeline_set_read_state` /
   `timeline_jump_latest` call `Core.command` with the same camelCase
   payloads desktop uses (`{ roomId, eventId }`, `{ streamId, action }`,
   `{ streamId }`). Jump returns the existing `TimelineOpenDto`. Do not
   re-wrap S6 `timeline_open`.
2. Do not reimplement timeline read-state in Swift. Do not wrap timeline
   reactions, leftover media, or leftover secret envelopes.
3. Do not start `SyncService`. Missing owner fail-closes with the
   registered `p2-timeline-event-readback-no-session` /
   `p2-timeline-set-read-state-no-session` /
   `p2-timeline-jump-latest-no-session` codes. Unstarted sync returns
   the registered handler's real outcome. Planted event-readback must
   fail on local room/event lookup (`v-crypto.6-event-room-not-found` /
   `d0.3-timeline-invalid-room-id` / `v-crypto.6-invalid-event-id`).
   Planted set-read-state / jump-latest must fail on local stream lookup
   (`v-timeline-view-not-open`) and must not require a live server.
   Failed errors must not echo event id, room id, or stream id.
4. Helper + XCTest are the iOS surface this slice. Do not swap
   `AppEnvironment.live()`. Do not retire `MatrixRustSDK`.
5. One command family per PR. Next is timeline reactions
   (`matrix_reaction_ensure` / `matrix_reaction_redact` /
   `matrix_timeline_reaction_toggle`).
   Do not wrap leftover password/export/import/bootstrap,
   `matrix_crypto_status`, or `matrix_cross_signing_status`. Do not hit
   production homeservers. Do not start S10.

### 9.6 When you may delete `MatrixRustSDK`

Only when:

```text
grep -rn 'MatrixRustSDK' synara-ios/Synara --include='*.swift'
```

returns zero product references (tests/docs may still mention it), and
RoomListService / TimelineService / MatrixRustSDKService have no
remaining callers. That is a late P4 PR of its own, not a side effect.

---

## 10. Recipe: docs honesty PR

After every product merge:

1. Branch `agent/snc-docs-after-<N>` from the **product merge commit**.
2. Update every tip SHA in:
   - `docs/shared-native-core/README.md`
   - `PLAN.md`
   - `02-module-boundary-census.md`
   - `06-migration-phases.md`
   - `08-parity-matrix.md`
   - `10-current-handoff.md`
   - this playbook's "Evidence tip" line if the next-slice list changed
   - `12-p4-s3-live-client.md` slice table if a P4-S3 product PR landed
3. Write numbers in words in running prose ("one hundred eleven") to
   match existing style. Lists may use digits.
4. State leftovers honestly. Do not mark P4/P5 accepted.
5. Commit `docs(core): refresh provenance after #<N>`.
6. Merge only to `feature/shared-native-core`.
7. Re-run section 5. If step 4, stop.

---

## 11. Files you will touch most often

| Path | Role |
|---|---|
| `crates/synara-core/src/core.rs` | Registry, handlers, fail-closed tests |
| `crates/synara-core/src/transport/census.rs` | Full `matrix_*` list (do not add names React does not invoke) |
| `crates/synara-core/src/app/*/live.rs` | Owner methods |
| `crates/synara-core/src/synara_core.udl` | UniFFI surface (P4 only) |
| `crates/synara-core/src/ffi.rs` | UniFFI translation for namespace probes (P4) |
| `crates/synara-core/src/shared_core_ffi.rs` | `SharedCore` FFI (P4-S3) |
| `crates/synara-core/src/session_projection_ffi.rs` | Session-projection mirror only |
| `src-tauri/src/bridge/` | Desktop adapters |
| `src-tauri/src/matrix/*/product_commands.rs` | Thin Tauri wrappers |
| `src-tauri/src/matrix/auth/product_commands.rs` | Session install / attach / logout leftovers |
| `src-tauri/src/matrix/auth/product_tests.rs` | `PRODUCT_SOURCE` thin-bridge tests |
| `synara-ios/Synara/Services/` | iOS adapters (P4) |
| `docs/shared-native-core/` | Plans and ledger |

---

## 12. Definition of program-done (not now)

All of the following, then merge to `main`, then P5 operator/Apple
gates. Checking any one box early is a lie.

- [ ] Desktop `src-tauri` is a thin registrar + Keychain + byte/secret
      leftovers only.
- [ ] Every React `matrix_*` name is either registered on Core or
      documented as a permanent shell leftover (the 21 in section 6,
      unless an owner decision moves one).
- [ ] iOS session, room list, timeline, and crypto product paths call
      `synara-core`, not a second Swift engine.
- [ ] `grep -rn 'MatrixRustSDK' synara-ios/Synara --include='*.swift'`
      is empty of product callers.
- [ ] One bugfix in `synara-core` is proven on desktop and iOS.
- [ ] P5 gates in `06-migration-phases.md` and
      `synara-ios/docs/device-readiness.md` have operator evidence.

Until that list is true: keep implementing on `feature/shared-native-core`.
