# 11 — Implementer playbook (hand this to the next model)

This file is the operating manual for finishing the shared native core.
Read it before editing product code. If this file and an older plan
paragraph disagree, this file plus `10-current-handoff.md` win for
*what to do next*; ADR-0003 plus `01-context-and-goals.md` win for
*what done means*; ADR-0004 wins for *what may be written in Rust*.
The language-boundary loop operator is
[13-language-boundary-goal-graph.md](13-language-boundary-goal-graph.md).
To launch a Long-running Cloud Agent, use
[14-long-running-agent.md](14-long-running-agent.md).

Older “144 `Core::command` handlers” sentences in `03-target-architecture.md`
and `06-migration-phases.md` are stale. Do not register the 21 leftovers in
section 6 to satisfy them.

Evidence tip when this playbook was written: `main`
`76f67441` (#1006 merge of desktop JS media retire, after
#1000/#1001/#1002/#1003/#1004/#1005).
Re-fetch before you start. Do not treat this SHA as eternal.

---

## 1. Desired end state (do not paraphrase away)

One Rust core (`crates/synara-core`) that both the desktop Tauri app
(macOS and Linux) and the iOS app consume, so sync, room list, timeline,
and crypto are not implemented twice.

That end state has **not** been reached. SNC engineering is on `main`
via #991. It is not a release.

Never claim the program is 100% complete. Never claim iOS is on the
shared engine. Never claim P5 or MAC-IOS-006 is done.

---

## 2. Current state (honest)

| Surface | Today |
|---|---|
| Desktop macOS/Linux | Live Matrix `Client` and native owners live in Core. React still invokes `matrix_*`. **111** of those names are registered on `Core::command`. Desktop is a thinner shell, not a thin shell. Composer send is native-only. JS encrypt/decrypt and SW token injection are retired. Leftover avatar `<img src=mxc://>` display remains. |
| iOS | UniFFI scaffold through S9-31 + S11 NSE + S10 leftover UniFFI + **P4-S12–S37 on `main` via #1001**. Product session, sync start, room list, timeline, verification, typing, room details, read markers, crypto status, reactions, opaque media handles, last-message previews, Settings devices, presence, and sticker-pack UI call SharedCore. Leftover recover/raw-send/media-bytes/pusher/notification/avatar I/O still fail closed without a live homeserver (decision 15). This is not iOS-on-engine and not P4 acceptance: Apple generate is still required for the new UniFFI fields; hosted iOS CI is paused (#1003); live homeserver proof is paused. Checked-in `SynaraCore.swift` remains the bootstrap stub. Quality treats skipped `ios-tests` as OK. XCTest construction of `SharedCore` is not iOS-on-engine. Desktop JS media retire (#1006) does not change those iOS gates. |
| `main` | SNC engineering tip. #991 brought the feature lane onto `main`. #1000 recorded ADR 0004. #1002 added the Long-running recipe. #1003 paused hosted iOS simulator CI. #1001 landed P4-S12–S37. #1004 docs honesty. #1005 leftover handle download. #1006 desktop JS media retire (`76f67441`). |
| Release | Forbidden until program-done is accepted and P5 operator/Apple gates pass. #991 is not a release. |

Census size: `REACT_MATRIX_COMMAND_CENSUS` in
`crates/synara-core/src/transport/census.rs` is the full React/Tauri
`matrix_*` list (lexically sorted). **111 registered. 21 unregistered
and fail-closed.** Those 21 are intended shell leftovers (section 6).

---

## 3. Absolute rules (break any one and stop)

1. Merge product/docs/CI slices to `main`. The
   `feature/shared-native-core` lane ended at #991.
2. Never rewrite `main` history. Never force-push. Never touch #39,
   #705, #672, tags, or releases. Do not start P5. Do not squash the
   entire SNC program.
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
10. UniFFI / bindgen / xcframework work needs headroom well above
    20 Gi for a four-target release generate, not just at start.
    If disk is under 20 Gi: stop. Docs-only PRs are still allowed.
    Do not change UDL. Do not hand-edit generated Swift. Do not invent
    a no-bindgen path.
11. After each **product** PR: squash-merge to `main`, then a
    **docs honesty** PR, then **re-run section 5**. If section 5
    step 4, stop. Do not start a second product slice to keep a
    turn busy.
12. `gh pr create --base main`. Merge with
    `gh pr merge N --squash`. Do not pass `--delete-branch` if the
    working branch is checked out locally (GitHub still merges).
13. Do not launch DeepSeek on scheduler fires.
14. MAC-IOS-006 is operator-gated. It is not the coding path.
15. Apple-only UI, Keychain, APNs, and NSE stay Swift. Logic moves; UI
    does not.

The owner surface is `origin/main`. Leftover SNC worktrees on disk are
historical; do not treat a folder name as the tip. Orchestrator notes:
`/Users/nepenthe/.grok/snc-orchestrator/STATE.md`.

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
| 9 | Feature-branch-only period ended at #991. Future slices target `main`. |
| 10 | Continue engineering. MAC-IOS-006 is not the coding path. |
| 11 | No release until shared core is program-done and P5 gates pass. #991 is not a release. |
| 12 | Apple proof stays operator-gated. |
| 13 | DeepSeek paused. Grok does the routes. |
| 14 | [ADR 0004](../adr/0004-rust-language-boundaries.md): apply the can/should rubric before new Rust. No Slint/Dioxus/egui desktop rewrite. No Tauri iOS or Rust-on-iOS UI. No Node CI/guardrail rewrite. The twenty-one leftovers in section 6 stay desktop. Native media cutover deletes JS decrypt; it does not register byte commands on `Core::command`. iOS-on-engine follows section 5 and section 9; do not start P5 from a language-boundary PR. |
| 15 | Leftover recover, raw-send, notification-mode, media bytes, room-avatar bytes, and pusher I/O stay fail-closed without a live homeserver. Do not invent a Core recover command or a byte/secret envelope. Leftover **status** that already has a Core owner (backup, room-key transfer) is the live leftover path. Crypto/cross-signing status stay on `Platform` (`IosFailClosedPlatform` remains fail-closed). |

### Language boundaries (ADR 0004)

Use this filter, then continue at section 5. Do not open a second core
crate or a parallel ledger.

| Should be Rust (playbook path) | Stay put / must not |
|---|---|
| 7B: non-secret live I/O already on an attached owner (as of #928 there are none left to invent) | Presentation, Slate, pdf.js, timeline viewport math |
| P4 then P5: iOS-on-engine; start SyncService through Core; NSE stays read-only and never starts sync | SwiftUI, Keychain, APNs, NSE lifecycle |
| Native media delivery cutover: desktop JS encrypt/decrypt retired; leftover send is native-only; leftover encrypted `mxc://` fail-closes; `sw.ts` is a stub | `matrix_send_attachment` / `matrix_upload_media` / `matrix_media_download` on the Core envelope |
| Harness → live only when the domain is shared product behavior | Node `scripts/*.mjs`, WASM/IndexedDB leftovers (delete, do not rewrite) |
| Optional after iOS-on-engine: agent-action *policy* in Core if iOS must share it | Agent-card / composer UI; Element Call WebRTC stack |

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
   S9-17/#965, S9-18/#966, S9-19/#967, S9-20/#968, S9-21/#969,
   S9-22/#970, S9-23/#971, S9-24/#972, S9-25/#973, S9-26/#974,
   S9-27/#975, S9-28/#976, S9-29/#977, S9-30/#978, and
   S9-31/#979 landed. #980 is CI hygiene after the S9 rematch.
   #981 refreshes provenance. #982 records local UniFFI generate.
   #983 recorded a temporary S10 leftover stop (playbook §9.6).
   Operator authorization plus #986 superseded that stop. #984
   lands S11 NSE read-only store (helper + XCTest; never starts
   sync; not a product NSE swap). #985 refreshes provenance. #986
   lands S10 leftover UniFFI and retires product `MatrixRustSDK`
   callers (operator authorized leftovers to cross UniFFI; leftover
   I/O that needs a live homeserver stays fail-closed planted).
   #987 refreshes provenance. #988 re-enables iOS CI (removes
   `if: false`; Quality on #988 green including iOS, not skipped)
   and restores Later sort/complete helpers. #989 refreshes
   provenance. #990 union-preserves `main` store recovery. #992
   raises the rustc recursion limit.    #991 merges SNC onto `main`
   (`05a0961c`; Quality + iOS + package smoke green). #1000 records
   ADR 0004. #1002 adds the Long-running recipe. #1003 pauses hosted
   iOS simulator CI until `main` is stable. #1001 lands P4-S12–S37
   (`7ecbfdf9`): start_sync, restore bootstrap, emit sinks, leftover
   status, product timeline/room-list/crypto/read-marker/device/
   last-message/media-handle/presence/sticker paths. #1006
   (`76f67441`) retires desktop JS encrypt/decrypt (composer send
   native-only; leftover encrypted `mxc://` fail-closes; `sw.ts` is a
   stub). Leftover avatar `<img src=mxc://>` display remains. Live
   homeserver proof is paused. Do not claim iOS-on-engine or P4
   engine ready. Do not invent S38.
   Local Apple UniFFI generate has been run for earlier fields;
   new S30–S35 UniFFI fields still need Apple generate. Generated
   sources remain gitignored. Checked-in `SynaraCore.swift` remains
   the bootstrap stub. Do not start P5. Dual-platform Core bugfix
   proof is not claimed. Four-target release bindgen needs headroom
   well above 20 Gi for the whole run, not just at start. If disk
   is under 20 Gi: stop. Docs-only PRs are still allowed.
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

- `git fetch origin main`
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
11. `gh pr create --base main`.
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
P4-S9-19 iOS timeline read-state      stacked
       matrix_timeline_event_readback / matrix_timeline_set_read_state /
       matrix_timeline_jump_latest.
       Jump returns the existing open readback. Do not re-wrap S6 open.
P4-S9-20 iOS timeline reactions       stacked
       matrix_reaction_ensure / matrix_reaction_redact /
       matrix_timeline_reaction_toggle.
       Write ack is the existing mutation result.
P4-S9-21 iOS composer reply draft     stacked
       matrix_composer_set_reply_draft / matrix_composer_get_reply_draft /
       matrix_composer_clear_reply_draft.
       Returns the existing reply-draft readback.
P4-S9-22 iOS send text                stacked
       matrix_send_text only. Write ack is the existing send result.
       No media bytes. Sticker, poll, edit, and respond stay off.
P4-S9-23 iOS send sticker             stacked
       matrix_send_sticker only. Write ack is the existing send result.
       Metadata / mxc only. No image bytes or file path. Core does not
       take a path or raw bytes, so this family is in scope.
       `matrix_send_attachment` stays a desktop leftover.
       Poll, edit, and respond stay off.
P4-S9-24 iOS send poll                stacked
       matrix_send_poll only. Write ack is the existing send result.
       No media bytes. Edit and respond stay off.
P4-S9-25 iOS edit message             stacked
       matrix_edit_message only. Write ack is the existing send result.
       No media bytes. Poll respond stays off.
P4-S9-26 iOS poll respond             stacked
       matrix_poll_respond only. Write ack is the existing send result.
       No media bytes. Timeline edit/redact/report stay off.
P4-S9-27 iOS timeline edit/redact/report stacked
       matrix_timeline_edit_text / matrix_timeline_redact /
       matrix_timeline_report. Write ack is the existing action readback.
       No media bytes. Pin/unpin stay off.
P4-S9-28 iOS timeline pin/unpin       stacked
       matrix_timeline_pin / matrix_timeline_unpin. Write ack is the
       existing action readback. No media bytes. Poll vote / call
       decline stay off.
P4-S9-29 iOS timeline poll vote / call decline stacked
       matrix_timeline_poll_vote / matrix_timeline_call_decline.
       Write ack is the existing action readback. No media bytes.
       Timeline forward stays off.
P4-S9-30 iOS timeline forward       stacked
       matrix_timeline_forward_text / matrix_timeline_forward_media.
       Write ack is the existing action readback. No media bytes.
       Session/status reads stay off.
P4-S9-31 iOS session/status reads LANDED #979
       matrix_session_snapshot / matrix_sync_status /
       matrix_media_config / matrix_secret_storage_status.
       Reads only. No leftover secret/bytes envelopes.
       Backup/crypto/cross-signing/room-key status stay off.
P4-S10 retire MatrixRustSDKService / RoomListService / TimelineService
       LANDED #986 (operator-authorized leftover UniFFI; product
       callers retired; leftover I/O fail-closed without a live
       homeserver; SyncService not started)
P4-S11 NSE read-only store API        LANDED #984
       (never boot sync in NSE; helper + XCTest; not a product NSE swap)
P4-S12 start attached SyncService     LANDED #1001
       SharedCore.start_sync only. NSE still cannot start sync.
       Not P4 acceptance. Do not start P5.
P4-S13 restore bootstrap              LANDED #1001
       Cold-start restore → attach → start on a fresh SharedCore.
       Product `SharedCoreMatrixClientService.start` is the one path.
       NSE still cannot start sync. Not P4 acceptance.
P4-S14 emit sinks                     LANDED #1001
       Timeline view-delta poll queue only. Summaries, not row
       bodies. NSE still cannot poll. Not Platform::emit.
P4-S15 leftover I/O live              LANDED #1001
       Owner leftover status after attach. Homeserver leftover
       I/O stays fail-closed (decision 15). No byte/secret
       envelopes.
P4-S16 product timeline rows          LANDED #1001
       Snapshot DTO keeps privacy-safe row bodies. Product
       SharedCoreTimelineService maps them. No media bytes.
P4-S17 owner emit poll                LANDED #1001
       Presence/devices/join_rules/image_packs poll queue.
       Summaries only. No presence user id. NSE cannot poll.
P4-S18 product timeline live poll     LANDED #1001
       SharedCoreTimelineService.timelineUpdates stays open and
       re-fetches on S14 summaries. One host poller. No room-list
       emit. No media bytes.
P4-S19 room-list live poll            LANDED #1001
       After start_sync, a joined-room entries stream queues
       session-generation wake-ups. Product roomUpdates re-fetches
       the existing snapshot. No room ids on the DTO.
P4-S20 product verification           LANDED #1001
       SharedCoreCryptoStatusService calls list/SAS. verification
       family on the S17 owner queue. No tokens or SAS secrets.
P4-S21 product typing live            LANDED #1001
       typing family on the S17 owner queue (room id only).
       Product typingUsers re-fetches the existing snapshot.
P4-S22 product room details           LANDED #1001
       Product roomDetails maps list / members / power /
       join-rule / invite snapshots. No media bytes. No UDL.
P4-S23 product foreground resume      LANDED #1001
       Product resumeFromForeground uses the S13 bootstrap.
       Second start is a restart. No pause command. No NSE.
P4-S24 product read markers           LANDED #1001
       Product mark-as-read uses timeline set_read_state.
       No HTTP access token. No media bytes. No UDL.
P4-S25 product room-list spaces/invites LANDED #1001
       Product loadRooms maps space parents and invite
       previews. Joined last-message later filled by S35. No UDL.
P4-S26 product room-list unread lookup LANDED #1001
       Product hasUnreadMessages uses the cached snapshot.
       Agent rooms stay false without Core agent cards.
P4-S27 product session crypto status  LANDED #1001
       Product sessionStatus maps leftover backup / crypto and
       secret-storage status. No recovery keys. No UDL.
P4-S28 product room crypto status     LANDED #1001
       Product roomStatus reuses the S27 mapper plus invite
       encryption. Joined-room encryption later filled by S30.
P4-S29 product timeline non-message rows LANDED #1001
       Poll / membership / state / call / other bodies already
       on the row DTO map to text. No media bytes. No UDL.
P4-S30 room-list encryption + notify mode LANDED #1001
       UniFFI room-list DTO keeps is_encrypted and notification_mode
       already on the Core snapshot. Product roomStatus / details
       consume them. No last-message invention.
P4-S31/S32/S33 timeline reactions + media handle LANDED #1001
       Row DTO keeps reaction counts and opaque media handles.
       SharedCore.timeline_media_bytes is a dedicated UniFFI byte
       channel (ADR 0005). Not Core.command. NSE cannot download.
P4-S34 product device list              LANDED #1001
       Settings lists leftover-safe device snapshot rows. No keys.
P4-S35 last-message preview             LANDED #1001
       Core projects a privacy-safe last_message_preview. UniFFI
       room-list DTO and both UIs consume it. No mxc/token.
P4-S36 desktop media handle cutover     LANDED #1001
       Leftover matrix_media_download resolves timeline-media-*
       through the native owner. Not Core.command.
P4-S37 presence + sticker pack UI       LANDED #1001
       Settings/room details consume presence. Composer lists
       image-pack names and sends via SharedCoreSendSticker.
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
This slice proceeded from that tip on `feature/shared-native-core`
and is now on `main` via #991.

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

S9-30 timeline forward landed in #978. S9-31 landed in #979 and
adds typed UniFFI wrappers for the registered session/status read
commands. #980 is CI hygiene after the S9 rematch.
#981 refreshes provenance. #982 records local UniFFI generate.
#983 recorded a temporary S10 leftover stop (superseded by
operator authorization + #986). #984 lands S11 NSE read-only
store. #985 refreshes provenance. #986 lands S10 leftover UniFFI
and retires product `MatrixRustSDK` callers. #987 refreshes
provenance. #988 re-enables iOS CI. #989–#992 and #991 bring
that tip onto `main`. Local Apple UniFFI generate has been run;
generated sources remain gitignored.

1. `SharedCore.session_snapshot` / `sync_status` / `media_config` /
   `secret_storage_status` call `Core.command` with the same null
   payloads desktop uses. They return the existing read DTOs. Do not
   re-wrap S9-30 timeline forward or leftover status commands.
2. Do not reimplement these reads in Swift. Do not wrap leftover
   password/export/import/bootstrap, `matrix_crypto_status`,
   `matrix_cross_signing_status`, `matrix_backup_status`, or
   `matrix_room_key_transfer_status`.
3. Do not start `SyncService`. `session_snapshot` without a session
   returns the registered `logged_out` readback. The other three fail
   on the registered iOS platform diagnostics
   (`p2-sync-status-platform-unavailable`,
   `p2-media-config-no-session`,
   `v-crypto.4-secret-storage-requires-session`). Planted
   `session_snapshot` returns the registered `logged_in` readback.
   Planted status reads return those same platform diagnostics
   because `IosFailClosedPlatform` remains the Platform. Must not
   require a live server. Failed errors must not echo user id,
   homeserver, device id, or tokens. Oversize fail-closes without
   truncating or echoing those values.
4. Helper + XCTest are the iOS surface this slice. Do not swap
   `AppEnvironment.live()`. Do not retire `MatrixRustSDK`.
5. One command family per PR. S9-31 has landed. S11 NSE read-only
   store landed in #984. S10 leftover retirement landed in #986.
   iOS CI re-enable landed in #988. The merge to `main` landed in
   #991. Do not start P5. Dual-platform Core bugfix proof is not
   claimed. Do not hit production homeservers.

### 9.6 When you may delete `MatrixRustSDK`

Only when:

```text
grep -rn 'MatrixRustSDK' synara-ios/Synara --include='*.swift'
```

returns zero product references (tests/docs may still mention it), and
RoomListService / TimelineService / MatrixRustSDKService have no
remaining callers. That is a late P4 PR of its own, not a side effect.

**S10 live map recorded in #983 at `1207aece` (after #982):** one
hundred nine hits in nine product Swift files. That #983 record was
a temporary leftover **stop**. Operator authorization plus #986
superseded it and **retired the product callers.** After #986,
`grep -rn 'MatrixRustSDK' synara-ios/Synara --include='*.swift'` is
comments only (`MatrixSessionProjectionMirror.swift`). The leftover
client file is deleted. Helpers already in pbxproj wrap the
registered SharedCore families plus the leftover UniFFI family.
UniFFI 0.28 constructors stay `SharedCore()` and
`SharedCore.newWithSecretStore(store:)`. No `SharedCore(store:)`.

`AppEnvironment.live()` now constructs one caller-owned `SharedCore`
and SharedCore product services. Leftover wipe/logout are local-only.
Recover, raw send, media bytes, pusher I/O, notification-mode write,
room-avatar bytes, and leftover status reads fail closed without a
live homeserver (`p4-s10-leftover-no-session` /
`p4-s10-leftover-unavailable` / oversize `p4-s10-leftover-oversize`).
Failed errors stay static. SyncService is still not started. This is
not iOS-on-engine and not P4 acceptance. The spike under
`synara-ios/spikes/` may still import `MatrixRustSDK`.

**Leftover UniFFI landed in #986 (operator-authorized):**
`backup_status`, `crypto_status`, `cross_signing_status`,
`room_key_transfer_status`, `wipe_persisted_stores`, `logout`,
`recover`, `send_raw_room_event`, `set_notification_mode`,
`media_download`, `media_thumbnail`, `media_upload`,
`room_avatar_bytes`, `pusher_set`, `pusher_delete`.

**Remaining honesty:** `SessionLoginDto` is identity-only (no access
token, by design). Product `AuthenticatedSession` still stores an
empty access token after SharedCore login. SharedCore attach still
does not start SyncService; P4-S12 adds a separate `start_sync`.
P4-S13 product `start(session:)` restores, attaches, then starts.
Product event emit sinks: timeline view-delta is a poll queue (S14);
presence, devices, join_rules, and image_packs remain no-op. Desktop
twenty-one leftovers stay unregistered on `Core::command`. iOS CI is
re-enabled (#988). Do not start P5. This is not iOS-on-engine.

### 9.7 P4-S12 — start attached SyncService

S10 retired product `MatrixRustSDK` callers. The S3d reason for leaving
SyncService unstarted (no dual live sync) is gone. This slice starts
the already-attached owner only.

**Lands:** `SharedCore.start_sync` FFI. Helper + XCTest. NSE still
fail-closes. Product login no longer starts sync; S13 bootstrap does.

**Must not land:** leftover registration; byte/secret envelopes; starting
sync in NSE; P5; claiming iOS-on-engine or P4 acceptance; a generic
`Core.command` FFI; emit-sink product events.

**UDL:** adds `start_sync` on `SharedCore`. Section 9.2 disk gate
applies. Implement in `crates/synara-core/src/shared_core_ffi.rs`.
Do not change NSE methods to start sync.

**Tests:** no-attach fail-closed; NSE forbids start; planted attach then
start returns a privacy-safe readiness DTO with no tokens, user id, or
URL. Failed errors stay static.

### 9.8 P4-S13 — restore bootstrap

S12 starts an already-attached owner. Cold launch still only restored
the Keychain `AuthenticatedSession` and never called SharedCore
restore, so `start_sync` fail-closed.

**Lands:** `SharedCoreSessionBootstrap.prepareLiveSession` runs restore
→ attach → start, each fail-closed. Product `start(session:)` uses that
one path. Login no longer attaches or starts on its own. Rust proof:
planted persist on core A, restore+attach+start on core B with the same
vault.

**Must not land:** leftover registration; byte/secret envelopes; starting
sync in NSE; P5; claiming iOS-on-engine; emit-sink product events (S14).

### 9.9 P4-S14 — timeline view-delta emit sink

S13 starts sync on cold launch. Timeline still cannot tell iOS that
rows changed because attach used a no-op `TimelineViewUpdateEmit`.

**Lands:** attach installs a bounded queue (cap 32, drop oldest).
`SharedCore.poll_timeline_view_updates` drains privacy-safe summaries
only (`schema_version`, `session_generation`, `stream_id`, `room_id`,
`revision`, `op_count`). Empty queue is success. Helper + XCTest. NSE
still fail-closes. iOS re-fetches snapshot via existing timeline
open/paginate.

**Must not land:** leftover registration; byte/secret envelopes;
`Platform::emit` for product events; row bodies or tokens on the DTO;
polling in NSE; room-list live updates; presence/devices/join_rules/
image_packs sinks; P5; claiming iOS-on-engine or P4 acceptance; a
generic `Core.command` FFI.

**UDL:** adds `poll_timeline_view_updates` on `SharedCore`. Section 9.2
disk gate applies. Implement in `crates/synara-core/src/shared_core_ffi.rs`.
Do not change NSE methods to poll.

**Tests:** UDL surface; poll empty without attach; NSE forbids poll;
enqueue then poll is privacy-safe with no token, user id, or URL echo.
Failed errors stay static.

### 9.10 P4-S15 — leftover I/O that already has a Core owner

S10 authorized leftover UniFFI. Status wrappers already call Core
commands. After S13 attach, leftover **status** that lives on
`NativeDeviceOwner` can return a privacy-safe DTO without a homeserver.
Leftover recover/raw-send/media/pusher still need live I/O.

**Lands:** planted attach then `room_key_transfer_status` is live and
privacy-safe. Owner no-session leftover status stays static. Recover,
raw send, and media after attach stay `p4-s10-leftover-unavailable`.
Owner decision 15. Helper already exists (`SharedCoreLeftovers`).
XCTest fail-closed leftover status without attach.

**Must not land:** leftover registration on `Core::command`; byte/secret
envelopes; implementing recover/media against a live homeserver;
starting leftover I/O in NSE; P5; claiming iOS-on-engine or P4
acceptance.

**Tests:** leftover backup/room-key status without attach is
`p2-*-no-session` with no token/user/URL echo. After planted
attach+start, room-key status is a DTO; recover/raw-send/media stay
unavailable and never echo secrets.

### 9.11 P4-S16 — product timeline rows

S6 open/paginate returned a Core snapshot but the UniFFI DTO kept only
`row_count`. Product `SharedCoreTimelineService` always returned
`.empty`, and it opened with kind `live` which the FFI rejected.

**Lands:** `TimelineSnapshotDto.rows` as privacy-safe
`TimelineViewRowDto` (kind, ids, sender, body, timestamp). `live` is
an alias for `live_bottom`. Product maps rows to `TimelineItem` and
keeps the stream for paginate. No media bytes.

**Must not land:** leftover registration; byte/secret envelopes;
`Platform::emit`; P5; claiming iOS-on-engine or P4 acceptance.

**Tests:** UDL has `rows`; `live` open without session is still
`p2-timeline-open-no-session`; mapper unit test has no token echo.

### 9.12 P4-S17 — remaining owner emit sinks

S14 queued timeline view-deltas. Presence, devices, join_rules, and
image_packs still used `Arc::new(|_| {})` at attach.

**Lands:** attach installs a bounded owner-update queue (cap 32).
`SharedCore.poll_owner_updates` drains `{ family, session_generation,
room_id? }`. Presence user ids never appear. Empty queue is success.
NSE fail-closes. Helper + XCTest.

**Must not land:** leftover registration; byte/secret envelopes;
`Platform::emit`; P5; claiming iOS-on-engine or P4 acceptance.

**Tests:** UDL surface; poll empty without attach; NSE forbids poll;
enqueue then poll is privacy-safe.

### 9.13 P4-S18 — product timeline live poll

S14 queued view-delta summaries. Product `timelineUpdates` still used
the one-shot protocol default, so the UI stream finished after the
first open.

**Lands:** `SharedCoreTimelineService.timelineUpdates` yields the
initial open, then stays open. One `SharedCoreLivePoller` per host
drains `poll_timeline_view_updates` so two rooms cannot steal each
other's summaries. Matching `room_id` (and stream id when known)
re-fetches via `timeline_jump_latest` on live, or re-open when
focused. Helper + XCTest.

**Must not land:** leftover registration; byte/secret envelopes;
`Platform::emit`; room-list emit; P5; claiming iOS-on-engine or P4
acceptance.

**Tests:** refresh matcher is room/stream-safe; without a session the
product stream yields `.empty` with no token echo and can be cancelled.

### 9.14 P4-S19 — room-list live emit

S14 deferred room-list live because attach is before SyncService
start. S12 starts that owner. Product `roomUpdates` still finished
immediately.

**Lands:** `start_sync` starts a joined-room entries listener in the
background (start_sync does not wait on `all_rooms`).
`SharedCore.poll_room_list_updates` drains `{ session_generation }`
only. Empty queue is success. NSE fail-closes. Product
`SharedCoreRoomListService.roomUpdates` stays open and re-fetches
via the existing snapshot. Helper + XCTest.

**Must not land:** leftover registration; byte/secret envelopes;
`Platform::emit`; room ids/names on the DTO; P5; claiming
iOS-on-engine or P4 acceptance.

**Tests:** UDL surface; poll empty without start; NSE forbids poll;
enqueue then poll is privacy-safe with no room id, user id, or URL
echo.

### 9.15 P4-S20 — product verification consume

S8/S9 typed list/SAS wrappers exist. Product crypto still finished
`verificationUpdates` immediately and returned unavailable for every
SAS action. Incoming requests had no owner emit.

**Lands:** attach queues `verification` on the S17 owner poll (no user
id). Incoming register and SAS mutations signal. Product maps inbox
rows to `CryptoVerificationState` and calls start/accept/begin_sas/
confirm/mismatch/cancel. Helper + XCTest. Recover stays leftover.

**Must not land:** leftover registration; byte/secret envelopes;
`Platform::emit`; P5; claiming iOS-on-engine or P4 acceptance.

**Tests:** phase mapper has no token echo; start without session is
fail-closed; accept without a flow is unavailable.

### 9.16 P4-S21 — product typing live

S7 typed typing snapshot/set exist. Product `typingUsers` still
yielded one empty list and finished. The typing owner had no emit.

**Lands:** attach queues `typing` on the S17 owner poll with `room_id`
only (no user ids). Product `SharedCoreTimelineService.typingUsers`
stays open and re-fetches via the existing snapshot. Helper + XCTest.

**Must not land:** leftover registration; byte/secret envelopes;
`Platform::emit`; user ids on the owner DTO; P5; claiming
iOS-on-engine or P4 acceptance.

**Tests:** room matcher is privacy-safe; without a session the product
stream yields `[]` with no token echo and can be cancelled.

### 9.17 P4-S22 — product room details

S9-3/S9-16 typed join-rule and members snapshots exist. Product
`roomDetails` still returned a placeholder (name = room id, no
permissions). Room details UI already loads that one-shot.

**Lands:** product `SharedCoreRoomManagementService.roomDetails`
maps the existing list / members / power-level / join-rule / invite
snapshots. Helper + XCTest. Topic and encryption come from the
invite snapshot when the room is an invite. Notification mode stays
the leftover default.

**Must not land:** leftover registration; byte/secret envelopes;
UDL changes; room-avatar bytes; P5; claiming iOS-on-engine or P4
acceptance.

**Tests:** mapper fills name/members/permissions without token echo;
without a session the product path falls back to the room id and
disables edits.

### 9.18 P4-S23 — product foreground resume

S13 made `start(session:)` the one restore → attach → start path.
`SynaraApp` already calls `resumeFromForeground` on scene active, but
the SharedCore implementation was a no-op. S12 already treats a
second `start_sync` as a restart.

**Lands:** product `resumeFromForeground` uses the same S13 bootstrap
as `start`. Helper comment + XCTest. Pause stays a no-op (no Core
pause). NSE background sync stays false.

**Must not land:** leftover registration; byte/secret envelopes;
starting sync in NSE; a pause/stop SyncService command; P5; claiming
iOS-on-engine or P4 acceptance.

**Tests:** without a session, resume stays `.stopped` with no token
echo.

### 9.19 P4-S24 — product read markers

S9-19 typed set-read-state exists. Product mark-as-read still used
homeserver HTTP with the empty SharedCore access token, so the live
path could not acknowledge. `clearMarkedUnread` was a no-op.

**Lands:** product `SharedCoreRoomReadMarkerService` opens a live
view, calls `mark_read`, and maps `own_read_event_id` (else the last
ackable row). Helper + XCTest. Temporary stream is closed. HTTP
Bearer leftovers stay unused on the live path.

**Must not land:** leftover registration; byte/secret envelopes;
UDL changes; starting sync in NSE; P5; claiming iOS-on-engine or
P4 acceptance.

**Tests:** mapper prefers own-read and skips `$local` / `$pending`;
without a session mark/read stay nil with no token echo.

### 9.20 P4-S25 — product room-list spaces and invite previews

S4/S5/S9-17 typed room-list, invite, and space-parent snapshots
exist. Product `loadRooms` still left `lastMessagePreview` empty and
`parentSpaces` empty, so space chips and invite rows had no Core
text.

**Lands:** product `SharedCoreRoomListService.loadRooms` maps invite
sender/topic/reason into the preview for invited rooms and space
parent ids/names from the parents snapshot. Helper + XCTest.
Joined-room last-message text stays empty (not on the list DTO).

**Must not land:** leftover registration; byte/secret envelopes;
UDL changes; last-message invention; P5; claiming iOS-on-engine or
P4 acceptance.

**Tests:** mapper fills invite preview and parent space without
token echo; without a session `loadRooms` is empty.

### 9.21 P4-S26 — product room-list unread lookup

S25 mapped unread counts onto `RoomSummary`, but product
`hasUnreadMessages` still used the protocol default `false`. Timeline
focus already calls that method to decide whether to load a fully-read
marker.

**Lands:** `SharedCoreRoomListService` caches the last snapshot and
answers unread from `unreadCount` / `hasHighlight`. Agent rooms stay
false unless a mapped row already has agent activity. Helper + XCTest.

**Must not land:** leftover registration; byte/secret envelopes;
UDL changes; inventing agent cards; P5; claiming iOS-on-engine or
P4 acceptance.

**Tests:** unread helper is true for count or highlight; without a
cached snapshot the product lookup is false with no token echo.

### 9.22 P4-S27 — product session crypto status

Settings already shows Device Verification, Key Recovery, and Key
Backup from `sessionStatus()`, but the SharedCore impl hardcoded
recovery as unknown and mapped backup from `encryptionEnabled`.
Decision 15 leftover **status** already has Core owners for backup
and secret-storage.

**Lands:** product `SharedCoreCryptoStatusService.sessionStatus`
composes leftover crypto/backup plus `secret_storage_status` through
`SharedCoreSessionCrypto`. Recovery keys and missing-secret lists
never appear on the product status. `roomStatus` is S28.
`retryDecryption` stays fail-closed. Helper + XCTest.

**Must not land:** leftover registration; byte/secret envelopes;
UDL changes; mapping recover I/O as live; P5; claiming iOS-on-engine
or P4 acceptance.

**Tests:** mapper covers ready / incomplete / missing / secret-storage
fallback without token or recovery-key echo; without a session
`sessionStatus` is unknown.

### 9.23 P4-S28 — product room crypto status

Timeline already calls `roomStatus` for the crypto banner, but the
SharedCore impl returned `.unknown`. S27 mapped leftover session
status; invite snapshots already expose `is_encrypted`.

**Lands:** product `SharedCoreCryptoStatusService.roomStatus` reuses
the S27 session mapper and invite encryption when the room is an
invite. Joined-room encryption stays unknown (not on the list DTO).
UTD count stays 0. `retryDecryption` stays fail-closed. Helper +
XCTest.

**Must not land:** leftover registration; byte/secret envelopes;
UDL changes; inventing joined-room encryption or UTD counts; P5;
claiming iOS-on-engine or P4 acceptance.

**Tests:** mapper fills invite encryption and session recovery
without token echo; without a session `roomStatus` is unknown.

### 9.24 P4-S29 — product timeline non-message row bodies

S16 mapped message / redacted / encrypted rows. Core already puts
poll questions and membership / state / call summaries in `body`,
but the product mapper showed `Unsupported event: poll`.

**Lands:** `SharedCoreTimelineRows.displayKind` maps poll /
membership / state / call / other / sticker to `.text` when body
is non-empty. Empty sticker stays unknown. Virtual kinds still
skip. No media bytes. No mxc invention. Helper + XCTest.

**Must not land:** leftover registration; byte/secret envelopes;
UDL changes; media placeholders without a URL; P5; claiming
iOS-on-engine or P4 acceptance.

**Tests:** poll / membership / state / call map to text; empty
sticker stays unknown; date separators stay skipped; no token or
mxc echo.

### 9.25 P4-S30–S34 — room encryption, reactions, media handle, devices

Core already projected joined-room encryption, notification mode,
timeline reactions, and opaque media handles. UniFFI dropped them.
Settings had no device list despite `device_snapshot`.

**Lands:** room-list DTO `is_encrypted` / `notification_mode`;
timeline row reactions + media handle metadata; `timeline_media_bytes`
dedicated UniFFI channel (ADR 0005); Settings device rows from the
existing snapshot. No `mxc://` on the row DTO. No leftover
registration. NSE cannot download.

**Must not land:** `matrix_send_attachment` on Core; byte/secret
envelopes; last-message invention; P5; claiming iOS-on-engine or
P4 acceptance.

**Tests:** UDL has the new fields and `timeline_media_bytes`; without
a session media fail-closes with no handle/mxc/token echo; Swift
mappers cover notify mode, reactions, handle URL, and device name.

### 9.26 P4-S35–S37 — last-message preview, desktop media cutover, presence/stickers

Joined-room last-message text was empty even though the SDK latest
event is available. Desktop leftover `mxc://` download was still the
documented live path. iOS had presence and image-pack wrappers with
no product UI.

**Lands:** privacy-safe `last_message_preview` on Core / UniFFI /
product room lists; desktop `matrix_media_download` resolves
`timeline-media-*` handles through the native owner (ADR 0005);
Settings/room-details presence and a composer sticker-pack list that
sends via `SharedCoreSendSticker`. No leftover registration. No
pack image bytes.

**Must not land:** `matrix_send_attachment` on Core; byte/secret
envelopes; inventing preview text without a latest event; P5;
claiming iOS-on-engine or P4 acceptance.

**Tests:** UDL has `last_message_preview`; preview sanitizer rejects
mxc/token; handle download prefers `timeline-media-*`; Swift mappers
cover joined preview, presence display name, and pack rows.

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
6. Merge only to `main`.
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
| `docs/adr/0004-rust-language-boundaries.md` | Can/should rubric; stay-put list |

---

## 12. Definition of program-done (not now)

All of the following, then P5 operator/Apple gates. SNC engineering
already lives on `main` (#991). Checking any one box early is a lie.

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

Until that list is true: keep implementing on `main`. Do not start P5.
