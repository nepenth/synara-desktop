# 11 — Implementer playbook (hand this to the next model)

This file is the operating manual for finishing the shared native core.
Read it before editing product code. If this file and an older plan
paragraph disagree, this file plus `10-current-handoff.md` win for
*what to do next*; ADR-0003 plus `01-context-and-goals.md` win for
*what done means*.

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
| iOS | UniFFI scaffold + `login_flows` + session-projection mirror + Settings readback + two pure helpers (unread row, cold-start recovery). Live session, room list, timeline, crypto, push, and `MatrixRustSDKService` are still Swift. iOS does **not** call the 111 Core commands. |
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
11. After each **product** PR: squash-merge to `feature/shared-native-core`
    only, then a **docs honesty** PR, then start the next slice in the
    same turn. Do not stop after one merge.
12. `gh pr create --base feature/shared-native-core`. Merge with
    `gh pr merge N --squash`. Do not pass `--delete-branch` if the
    feature branch is checked out locally (GitHub still merges).
13. Do not launch DeepSeek on scheduler fires.
14. MAC-IOS-006 is operator-gated. It is not the coding path.
15. Apple-only UI, Keychain, APNs, and NSE stay Swift. Logic moves; UI
    does not.

Worktree for this program: `synara-desktop-snc-image-packs`.
Orchestrator notes: `/Users/nepenthe/.grok/snc-orchestrator/STATE.md`.

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
3. **Is free disk ≥ 20 Gi, or can the next P4 slice land as source
   without local cargo/bindgen?** Start the next **P4** slice in
   section 9. S3a/#935 landed. Next is S3b restore (plan §12).
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
14. Start the next slice.

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
       S3a vault callback (this lane) → S3b restore → S3c login → S3d attach
P4-S4  iOS room_list_snapshot         (needs SyncServiceOwner)
P4-S5  iOS invites_snapshot           (needs join-rule owner)
P4-S6  iOS timeline open/close/paginate (needs NativeTimelineOwner)
P4-S7  iOS typing / presence          (needs those owners)
P4-S8  iOS verification list          (needs verification owner)
P4-S9  remaining already-registered Core commands, one owner family per PR
P4-S10 retire MatrixRustSDKService / RoomListService / TimelineService
       only when grep shows no remaining product callers
P4-S11 NSE read-only store API        (never boot sync in NSE)
```

Apple-only stays Swift the whole way: SwiftUI, Keychain UI, APNs
permission, file pickers, settings chrome.

### 9.2 Hard gate for P4-S1 and later that touch UDL

- Free disk **≥ 20 Gi** (`df -h /`).
- Regenerating UniFFI without cargo/bindgen is forbidden. Do not
  hand-edit generated Swift to fake a UDL change.
- Generated artifacts live under `synara-ios/SynaraCore/`. Follow the
  existing `login_flows` pattern in:
  - `crates/synara-core/src/synara_core.udl`
  - `crates/synara-core/src/ffi.rs`
  - `crates/synara-core/src/session_projection_ffi.rs` (projection only)
- Errors that cross UniFFI must be **static** (category/code/description
  from source constants). Never echo URLs, tokens, or HTTP bodies.

### 9.3 P4-S1 — `register_flows` (first slice when disk lifts)

Copy `login_flows` exactly.

| Item | Value |
|---|---|
| Rust domain | `probe_register_flows` already in `crates/synara-core/src/app/auth/` |
| Desktop command | already `matrix_register_flows` through `Core::command` |
| New UniFFI | `register_flows(homeserver_url) -> RegisterFlowsDto` or reuse the desktop DTO shape |
| iOS call site | only if a registration discovery UI exists; otherwise expose + XCTest |
| Must not add | `matrix_register`, email token, password |
| Tests | invalid URL fails closed without echoing input (mirror `ffi.rs` login_flows tests) |
| Commit | `feat(core): expose register_flows on UniFFI` |

### 9.4 P4-S3 — live Client on iOS (do not start before S2)

Desktop reference (do not copy Tauri types):

- `src-tauri/src/matrix/auth/product_commands.rs` login / restore /
  `open_after_desktop_session_install` / `attach_*`
- Keyring vault stays a **shell** `SecretVault` impl. iOS equivalent is
  Keychain behind the same trait. The `keyring` crate stays out of Core.

Password login on iOS stays in the Swift shell the same way desktop
keeps `matrix_login_password` desktop: the password never enters a
UniFFI string if you can avoid it. Prefer: Swift collects the password,
calls a **narrow** Rust login entry that already exists in Core's auth
module (`login_with_password`) **from the Swift shell via a dedicated
authenticated FFI that you design in that slice**, or keep password
login on `MatrixRustSDK` until S3 is designed as its own ADR-sized
write-up in the PR body. If unsure, **stop and write the PR plan; do
not guess.**

After a session exists, attach the same owner set desktop attaches.
Missing attach → later commands fail closed. That is correct.

### 9.5 P4-S4+ — consume an already-registered command

Only after S3.

1. iOS calls `Core.command` (or a typed UniFFI wrapper around one
   registered name) with the **same camelCase payload** desktop uses.
2. Do not reimplement snapshot logic in Swift.
3. Keep SwiftUI views. Replace the service body only.
4. One command family per PR (room list, or invites, or timeline).
5. XCTest: missing owner fails closed; happy path only if you have a
   test double / recorded fixture. Do not hit production homeservers.

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
3. Write numbers in words in running prose ("one hundred eleven") to
   match existing style. Lists may use digits.
4. State leftovers honestly. Do not mark P4/P5 accepted.
5. Commit `docs(core): refresh provenance after #<N>`.
6. Merge only to `feature/shared-native-core`.

---

## 11. Files you will touch most often

| Path | Role |
|---|---|
| `crates/synara-core/src/core.rs` | Registry, handlers, fail-closed tests |
| `crates/synara-core/src/transport/census.rs` | Full `matrix_*` list (do not add names React does not invoke) |
| `crates/synara-core/src/app/*/live.rs` | Owner methods |
| `crates/synara-core/src/synara_core.udl` | UniFFI surface (P4 only) |
| `crates/synara-core/src/ffi.rs` | UniFFI translation (P4 only) |
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
