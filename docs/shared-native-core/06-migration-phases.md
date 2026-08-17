# 06 — Migration Phases (P1–P5)

Methodology mirrors the js→rust burn-down: small additive slices, each a
squashed PR onto `main` with a provenance anchor and the
path-scoped CI required by the workflow plus its aggregate Quality gate.
The former `feature/shared-native-core` mechanical-extract shortcut no
longer applies. Desktop package and full Synapse evidence run when
selected or required by the relevant protected, integration, or release
path. **Never a big-bang move.**

## P0 — ADR + plan + census (done)

- ADR-0003 (`docs/adr/0003-shared-native-rust-core.md`), this doc set,
  `PLAN.md`. Patch numbers cited in `02-module-boundary-census.md`.

## P1 — Crate extraction (no behavior change)

Goal: introduce `crates/synara-core` holding `matrix/`, `tasks/`, `dto/`,
`ipc/` **by `git mv` + path updates only**; every test must pass identically.

**Current bounded status at `76f67441`
(#1006):** #713 mechanically moved notifications, polls,
relations, threads, and unread; #714 moved raw content, receipts, routes, and
security; #716 moved search, legacy, and media_cache; #717 moved media_export
and crypto_store; #734 moved the room-directory session harness; #735 moved
the verification inbox harness; #737 moved the account-data index harness;
#738 moved the send-queue harness; #740 moved the room-keys transfer harness;
#741 moved the supervisor actor harness; #744 moved the diagnostics health
harness; #743 moved the store identity/paths harness; #748 moved the
client-builder error/features harness; #747 moved the lifecycle recovery-copy
harness; #751 moved the auth device-name harness; #753 moved the store
vault trait / key material and auth discovery/UIA/client_config (Keyring I/O
and live login stayed desktop); #755 moved the lifecycle error domain
(logout/session vault I/O/SDK restore stayed desktop); #757 moved lifecycle
recovery/remote-logout/wipe; #758 moved client-builder config (SDK open stayed
desktop); #760 moved the session-material vault trait / sealed envelope
(Keyring I/O stayed desktop); #762 moved the well-known HTTP discovery
transport (live login stayed desktop); #764 moved logout orchestration and
the task-supervisor bridge; #766 moved later/room-notes codecs (Client RMW
stayed desktop); #768 moved image-pack DTO, type filters, and write guards
(Client snapshot/set and Tauri subscribe stayed desktop); #770 moved m.direct
snapshot DTO and string-map helpers (Client load/store and DirectEventContent
write stayed desktop); #772 moved device presentation DTOs and sort helper
(Client snapshot, UIAA delete, and Tauri owner stayed desktop); #774 moved
secret-storage presentation DTOs and projector (Client recovery I/O stayed
desktop); #776 moved backup presentation DTOs and projector (Client
backup/recovery I/O stayed desktop); #778 moved presence DTOs and subscription
registry (Client stream and Tauri owner stayed desktop); #780 moved typing
presentation snapshot DTO (Client m.typing owner stayed desktop); #781 moved
verification presentation DTOs and phase rank (Client request/SAS owner stayed
desktop); #783 moved room-directory DTOs and search normalize (ruma request/Client
fetch stayed desktop); #784 moved space presentation DTOs and cycle guard (live
Client I/O later moved in #908); #786 moved cross-signing presentation DTOs
and projector (Client crypto I/O and UIAA stayed desktop); #788 moved room-key
transfer presentation DTOs and projector (Client/file I/O stayed desktop); #790 moved
members presentation snapshots and write result (Client member/power-level I/O stayed
desktop); #792 moved room join-rule presentation DTO (SDK mapping and Tauri owner stayed
desktop); #794 moved timeline presentation DTOs (NativeTimelineRegistry and Client/Tauri
streams stayed desktop); #796 moved send/profile-write/room-create/room-profile IPC
DTOs (Client I/O stayed desktop); #798 moved media upload/download/config IPC DTOs
(Client media I/O stayed desktop); #800 moved live `Client::builder` plus session
persist/restore (Keyring vault and `SdkClientHandle` stayed desktop); #801 moved
live password login / register / password-reset (Tauri product commands stayed
desktop); #803 moved live `NativeTypingOwner` / `set_typing_notice` (Tauri
typing commands stayed desktop); #805 moved live `NativeRoomJoinRuleOwner`
behind a shell emit sink (Tauri event adapter stayed desktop); #807 moved live
`NativeDeviceOwner` behind a shell emit sink (Tauri wakeup adapter stayed
desktop); #808 moved live `NativePresenceOwner` behind a shell emit sink
(Tauri event adapter stayed desktop); #810 moved live `NativeImagePackOwner`
plus snapshot/set behind a shell emit sink (Tauri adapter stayed desktop);
#812 extracted the timeline `ViewDeltaEmitter` behind a shell emit sink
; #814 moved live `NativeTimelineRegistry` into Core (desktop keeps the
AppHandle adapter); #816 moved live `NativeVerificationOwner` into Core
(desktop maps diagnostic ids onto Tauri errors). These retain thin desktop re-exports and any
leftover `live.rs` / `product_commands.rs`. They are P1 extraction
only: no P2 command registration, UDL, or iOS behavior changed.

Slicing (each slice = one PR):
1. Workspace scaffolding: root `Cargo.toml` workspace; `crates/synara-core`
   with empty `lib.rs`; `src-tauri` stays as-is (nothing moved yet). Green.
2. Move `matrix/dto/` → `crates/synara-core/src/dto/`; re-export from
   `src-tauri` for local `use` paths; run cargo test/clippy/fmt. Green.
3. Move `matrix/ipc/` → `crates/synara-core/src/transport/` (same content;
   `ipc/` module name preserved via re-export if needed for the React-facing
   stream names). Green.
4. Move `matrix/tasks/` → `crates/synara-core/src/task/`. Green.
5. Move `matrix/*` domain modules → `crates/synara-core/src/app/` in **domain
   chunks** (sync+room_list+timeline first — the biggest consumers of the
   `Platform` seam; then crypto group: verification/backup/cross_signing/
   secret_storage/room_keys/utd_recovery/crypto_store; then the rest).
   Each chunk: `git mv`, fix intra-crate paths, keep `src-tauri` re-exporting
   what the 144 commands reference. Green per chunk.
6. Introduce `platform/` mod + `Platform` trait with stubs wired to the
   existing desktop impls (no behavior change: desktop continues using
   `AppHandle` behind an adapter).

Acceptance: `scripts/check-matrix-boundaries.mjs` green (no new
`/_matrix/`-literal or boundary violations), `cargo test` count identical
(832+/0), clippy `-D warnings` clean, frontend untouched.

## P2 — Native transport API

Goal: make `Core::command(envelope)` the command entry, owner emit
callbacks the product-event entry, and `Platform::emit` the IPC
envelope stream. Do not collapse those three.

- Add `synara-core::Core` with `command`, `open`, `close`, `new(platform)`.
- Register handlers by **command name** only as the owner capability
  lands (decision 6). Unregistered census names fail closed.
- Twenty-one names stay desktop on purpose (playbook §6).
- Commit per landed capability. Green per slice. As of #928 the
  registry is one hundred eleven names.

Acceptance: React still calls the same `matrix_*` commands (desktop adapter
unchanged in behavior); `ipc/contract_tests` extended to cover every registered
command name (a coverage test asserts parity between the registry and the
desktop invoke list).

## P3 — Desktop adapter swap

Goal: `src-tauri` becomes a thin shell.

- `src-tauri/src/bridge/` builds `Arc<dyn Platform>` from the `AppHandle` and
  registers the 144 `#[tauri::command]` fns that just call
  `Core::command(envelope)`.
- Delete the old `matrix/` path (now fully re-exported from the core) bit by
  bit; final state: `src-tauri` imports only `synara_core` + `desktop_*`.
- Keep the React-facing invocation contract byte-identical.

Acceptance: full matrix green; package-smoke macOS/Linux green at tip;
`matrix_sdk_link_smoke` remains; boundary check green.

## P4 — iOS uniffi bindings + Swift adapter

Goal: iOS consumes the same engine; Swift re-implementations retired.

1. `xtask` (or `scripts/`) to build `synara-core` for the three Apple targets
   and run `uniffi-bindgen` → `synara-core/Sources/SynaraCore/*.swift`.
2. Swift `SynaraCore` package in `synara-ios` (or a workspace package) with
   `Platform` callback impl (`SynaraPlatform`).
3. Migrate iOS services onto it, in the **serial order in
   [11-implementer-playbook.md](11-implementer-playbook.md) §9**. Do not
   call `matrix_room_list_snapshot` (or any attached-owner command) from
   iOS before P4-S3 has attached owners. Do not delete
   `MatrixRustSDKService` until grep is clean.
   - P4-S1 `register_flows` UniFFI — landed in #931 (credential-free)
   - P4-S2 Swift `Platform` + `Core::new` — landed in #933
   - P4-S3a SecretVault callback — landed in #935 (no live Client)
   - P4-S3b restore — landed in #937
   - P4-S3c `login_with_password` — landed in #938
   - P4-S3d attach owners — landed in #939 (SyncService not started)
   - P4-S4 through S9-31 typed SharedCore wrappers — landed in #940–#979
   - #980 desktop source-scan hygiene after the S9 rematch
   - #981 docs honesty after #980
   - #982 records local UniFFI generate
   - #983 recorded a temporary S10 leftover stop (superseded by operator authorization + #986)
   - P4-S11 NSE read-only store API — landed in #984 (never starts sync; helper + XCTest; not a product NSE swap)
   - #985 refreshes provenance
   - P4-S10 leftover UniFFI + product caller retirement — landed in #986 (operator-authorized)
   - #987 refreshes provenance
   - #988 re-enables iOS CI (Quality green including iOS, not skipped; not `if: false`)
   - #989 refreshes provenance
   - #990 union-preserves `main` store recovery into SNC
   - #992 raises rustc recursion limit for Linux release builds
   - #991 merges SNC engineering onto `main` (`05a0961c`; Quality + iOS + package smoke green)
   - #1000 records ADR 0004
   - #1002 adds the Long-running Cloud Agent recipe
   - #1003 pauses hosted iOS simulator CI until `main` is stable
   - #1001 lands P4-S12–S37 (`7ecbfdf9`)
   - #1004 docs honesty after #1001
   - #1005 leftover `matrix_media_download` handle resolve
   - #1006 desktop JS media retire (`76f67441`)
   - Local Apple UniFFI generate has been run on Darwin for the S30–S35 fields (`dd62d24d`); generated sources remain gitignored; not iOS-on-engine; not P4 acceptance
   - Product `MatrixRustSDK` grep is comments only; leftover I/O fail-closed; SyncService can be started through SharedCore; live homeserver proof paused; hosted iOS CI paused
4. Remove `matrix-rust-components-swift` from `project.yml` when nothing
   references `MatrixRustSDK` anymore.

Acceptance: `ci.yml` `ios-tests` + `ios-skeleton.yml` green against the shared
core; `grep -rn 'MatrixRustSDK' synara-ios/Synara --include='*.swift'` returns
zero; a sample feature command implemented once in `synara-core` and exercised
by a SwiftUI unit test and a React hook test.

> **Bounded evidence note — not P4 acceptance:** At the current `main` tip
> `76f67441` (#1006), UniFFI exposes credential-free `login_flows` /
> `register_flows`, SharedCore constructors, S3 vault/restore/login/attach,
> typed wrappers through S9-31, the S11 NSE read-only store helper
> (never starts sync; not a product NSE swap), S10 leftover UniFFI, and
> P4-S12–S37 product consume (start_sync through last-message, desktop
> media handle cutover, presence/stickers). #1006 retires desktop JS
> encrypt/decrypt. Product `MatrixRustSDK`
> callers are retired. Leftover I/O that needs a live homeserver stays
> fail-closed planted. SyncService can be started through SharedCore;
> live homeserver proof is paused. This is not iOS-on-engine.
> Local Apple generate has been run on Darwin for the S30–S35
> fields (`dd62d24d`). Generated sources remain
> gitignored. Checked-in `SynaraCore.swift` remains the bootstrap stub.
> This is not P4 acceptance and not dual-platform proof.
> Hosted iOS CI is paused (#1003).
> The prior #708 work is only the pure iOS room-row unread presentation from closed
> `Joined`/`Invited` membership, scalar counters, and a marked-unread flag to a
> `u64` count plus highlight boolean. The prior #710 work is only the pure
> cold-start decision from a latest-state boolean and `{Missing, Known}` to a
> boolean; Swift maps `nil`/`.distantPast` to `Missing` and a real `Date` to
> `Known`. #713/#714/#716/#717 add no Core command route, UDL, or iOS behavior.

## P5 — Parity + release gates

Goal: ship iOS on the shared engine.

- Re-run the **full desktop matrix** (all 6 Synapse proofs + rust/audit) against
  the shared core — this gates iOS correctness implicitly.
- Close iOS release gates from `synara-ios/docs/device-readiness.md` against
  the shared engine: physical-device run + profiling, APNs push validation,
  TestFlight archive/upload, production E2EE completion (recovery,
  verification/cross-signing, key backup restore, encrypted-media decryption).
- Update parity matrix (`08-parity-matrix.md`) to "single engine".

## Rollback

Each slice is additive and reversible (git revert of a squashed PR returns to a
green tip). No slice may ship if it removes a passing test or changes a
renderer-facing command signature.
