# Windows support spike

Status: fact-finding only (2026-10-27). Nothing here has been ported. The
findings come from one run of `.github/workflows/windows-spike.yml` on
`windows-latest` against the v2.4.0 code (`eade6d9f`), plus a read of the
platform-specific code:
[run 40557791572](https://github.com/nepenth/synara-desktop/actions/runs/40557791572)
(logs are kept as the `windows-spike-logs` artifact for 14 days).

## Current state of this branch

The spike findings below are from the first run. Since then this branch has
made Windows a working, CI-checked test target. `windows-spike.yml` is green
on Windows: the workspace compiles with no warnings, all Rust and renderer
unit tests pass, and the `windows-installer` job uploads an unsigned MSI and
NSIS `.exe` (`synara-windows-test-installers`). The probe fails on any test
failure or new Windows-only warning.

| Finding | Change |
| --- | --- |
| §1a session locator fsync | Flushes through a write handle; directory fsync is a no-op on Windows |
| §1b CRLF checkout | `.gitattributes` checks out LF everywhere (the index was already LF) |
| §1c `/tmp` test paths | Fixture roots use `std::env::temp_dir()` |
| §1d dropped-file test | Uses a real, never-dropped file. One SQLite-backed test cleans up best effort, as the others do. |
| §1e Windows warnings | Gated with `cfg`, or allowed module-wide for the Linux/macOS notification response code until Windows toasts exist |
| §2 taskbar badge | Red-dot overlay icon via `set_overlay_icon` (Windows has no badge count) |
| §3 secret store | keyring `windows-native` (Credential Manager) for both vaults, with the roughly 1280-character UTF-16 limit |
| §4 origin and CSP | Navigation pins `http://tauri.localhost`; the CSP allows `http://synara-media.localhost` |
| §6 build entry points | `build-runtime.mjs` uses `fileURLToPath`. `npm run tauri` is `scripts/tauri.mjs`, which calls bash only on macOS and Linux. The renderer test runner uses the esbuild API (the CLI hit `spawn EINVAL` and cmd's 8191-character limit). |

Still open:

- Windows toasts with actions and routes.
- Code signing, the updater, and SmartScreen reputation.
- Single instance and deep links.
- A manual pass on real hardware.

Also checked and needing no Windows code: the native title bar, Ctrl
shortcuts, the "Synara Windows" device name, and the renderer's fallback
sound. The `push` trigger on this branch is temporary and must be removed
before merging.

## Summary

The Rust code compiles for Windows. `cargo check --locked --workspace
--all-targets --target x86_64-pc-windows-msvc` finished with no errors in
6m43s, and matrix-sdk with bundled SQLite, Tantivy, aws-lc-rs, Tauri 2.12 and
`desktop_x_embed.rs` (reqwest with rustls) all built with the runner's stock
MSVC, CMake and NASM. What remains is runtime behaviour, not compilation:

| Area | Result on Windows |
| --- | --- |
| `cargo check --workspace --all-targets` | Passes, with 17 dead-code or unused warnings (would fail CI's `clippy -D warnings`) |
| `cargo test -p synara --lib` | 472 passed, **29 failed** |
| `cargo test -p synara-core` (default features) | 1449 passed, **16 failed** (integration test binaries all pass) |
| `cargo test -p synara-core --features search-index,x509-identity` | 1451 passed, **the same 16 failed** |
| `npm run tauri` from `cmd` | Fails: `'scripts' is not recognized` |
| `node scripts/build-runtime.mjs` (Tauri `beforeBuildCommand`) | Fails: `ENOENT ... lstat 'D:\D:\a\...\config.json'` |

All 45 failures come from four causes. Only one is a product bug, but it
blocks sign-in and logout outright:

1. **Session locator fsync** (13 failures): a product bug. Persisting the
   session fails on Windows.
2. **CRLF checkout** (16 failures): source-text tests that `include_str!` a
   file and match `"\n"`. Test-only.
3. **Unix-style absolute test paths** (9 failures): `/tmp/...` isn't absolute
   on Windows. Test-only.
4. **Dropped-file test path** (7 failures: 1 real, 6 from a poisoned test
   lock). Test-only.

## 1. Test failures and compile warnings, by module

### 1a. Session locator fsync: product bug, 13 failures

`crates/synara-core/src/app/lifecycle/session/locator_file.rs:103` runs
`fs::File::open(&path).and_then(|f| f.sync_all())`. On Windows `sync_all` is
`FlushFileBuffers`, which needs a handle with write access, so a read-only
handle fails with access denied and the code returns
`d0.1-session-locator-sync-failed`. The directory loop after it (line 84,
`fs::File::open(directory)?.sync_all()`) would also fail: on Windows a
directory can only be opened with `FILE_FLAG_BACKUP_SEMANTICS`, and you can't
flush a directory handle.

On Windows this means `ensure_logout_retry_locator` always fails. Every login,
restore and logout path that writes the locator reports "Native Matrix
session storage is unavailable."

Failures:

- synara-core `app::lifecycle::session::locator_file::tests::`
  `locator_round_trips_and_missing_is_forbidden`,
  `directory_sync_failure_is_a_static_fault`.
- synara `matrix::auth::product::tests::` `install_rollback_preserves_restored_refresh_but_cleans_new_authentication`,
  `logout_partial_vault_failure_retains_retry_identity_until_all_credentials_are_deleted`,
  `logout_directory_sync_failure_preserves_live_and_retry_syncs_created_parent_links`,
  `logout_preflight_failure_preserves_live_identity_and_skips_all_teardown`,
  `new_auth_preparation_without_credential_write_preserves_previous_login`,
  `persistence_lease_waits_for_inflight_callback_before_teardown_cleanup`
  (a `RecvError` once the callback fails),
  `preparation_failure_after_callback_save_uses_origin_rollback_and_stops_owners`,
  `revoked_old_callback_cannot_target_later_same_account_install`,
  `session_persistence_requires_durable_locator_and_retains_failed_cleanup_target`,
  `session_wiring_failures_retire_tentative_live_and_core_before_returning_error`,
  `password_login_activation_failure_revokes_token_before_metadata_without_clearing_old_vault`.

Fix: open the locator with `OpenOptions::new().write(true)` before
`sync_all`. Make the directory-sync step a no-op on Windows, where NTFS
journals directory entries and there's no directory fsync. The test seam
`ensure_logout_retry_locator_with_directory_sync` and the two product tests
that inject a directory fault (`product_tests.rs:3410`, `:3478`) need a
Windows story too. The other `sync_all` calls already use write handles
(`desktop_logging.rs:526`, `matrix/room_keys/live.rs:246`).
**Effort: 0.5–1 day.**

### 1b. CRLF checkout: test-only, 16 failures

The repo has no `.gitattributes`, and Windows Git (including the runner)
checks out with `core.autocrlf=true`. About 53 files contain source-text
tests that `include_str!` a `.rs`, `Cargo.lock` or JSON file and then split or
match on `"\n"`, so these tests fail there:

- synara: `desktop_integration::tests::notification_check_reads_the_permission_the_os_reports`,
  `desktop_notifications::tests::notification_failures_reach_the_app_log_and_clicks_always_show_the_window`,
  `desktop_tray::tests::every_tray_menu_offers_do_not_disturb`,
  `matrix::room_list::tests::every_session_install_path_owns_the_room_list_live_owner`,
  `matrix::auth::product::auth_commands::tests::{every_remote_logout_is_bounded,
  session_install_and_every_logout_path_revoke_store_recovery,
  store_recovery_ipc_is_explicit_registered_and_privacy_limited}`,
  `matrix::auth::product::tests::product_client_persists_rotated_sdk_tokens_in_native_vault_and_logout_is_local_first`.
- synara-core: `app::devices::live::tests::trust_signals_are_sampled_from_one_crypto_device_handle`,
  `app::devices::tests::authority_query_is_recoverable_metadata_not_a_snapshot_failure`,
  `app::notifications::nse_error::tests::every_shipping_nse_code_has_a_shared_ios_stage`,
  `app::secret_storage::live::tests::backup_setup_and_bootstrap_share_one_recovery_enrolment`,
  `app::send::room_queue::tests::discard_aborts_before_waking_the_room_queue`,
  `app::timeline::live::approval_history_tests::approval_history_stream_publication_has_no_cancellation_point_before_readback`,
  `app::timeline::live::tests::exact_read_receipts_target_one_event_for_server_counts_and_private_receipt`
  (matches `Cargo.lock`),
  `shared_core_ffi::tests::ios_logout_and_attach_use_the_shared_core_policies`.

Fix: add a `.gitattributes` with `* text=auto eol=lf` (plus `binary` for
images, sounds and `.bmp`/`.ico`). That's one file and leaves the tests as
they are. **Effort: 1–2 hours**, including a renormalise check.

### 1c. Unix-style absolute paths in tests: test-only, 9 failures

`Path::new("/tmp/...")` has no drive letter, so on Windows it isn't absolute,
and the store layer rightly rejects it with `RelativeAppDataRoot`:

- synara-core `app::client_builder::config::tests::` (all six go through the
  helper at `config.rs:457`): `indexed_message_search_defaults_on_and_plan_has_no_secrets`,
  `plan_x509_flag_defaults_false_and_omits_pem`,
  `product_user_agent_names_the_shell_and_its_app_version`,
  `store_footprint_sizes_every_sqlite_pool_and_cache`,
  `store_lock_holder_defaults_to_main_and_accepts_a_distinct_process`,
  `store_lock_holder_rejects_empty_and_oversized_values`.
- synara `matrix::store::tests::{same_identity_derives_stable_paths_and_key_ids,
  layout_serializes_without_secrets_or_absolute_paths}`,
  `matrix::client_builder::tests::desktop_user_agent_names_the_app_version_and_sdk_pin`.

Fix: build the fixture roots from `std::env::temp_dir()` or a `tempfile`
directory. **Effort: 1–2 hours.**

### 1d. Dropped-file allowlist tests: test-only, 7 failures

`desktop_file_transfer::command_tests::desktop_read_dropped_files_rejects_unauthorized_path`
passes `"/etc/passwd"`. On Windows `fs::canonicalize` fails first, so the
error says "Unable to read dropped path" instead of "not available". The
production behaviour, rejecting the path, is still correct. The panic poisons
`DROPPED_FILE_ALLOWLIST_TEST_LOCK`, which fails the six other tests in the
module (`dropped_file_allowlist_caps_at_max_entries`, `..._clears_on_drag_leave_without_drop`,
`..._expires_stale_entries`, `..._preserves_paths_after_drop_on_drag_leave`,
`dropped_files_over_the_upload_limit_are_rejected_before_any_read`,
`streaming_dropped_file_read_returns_chunks_without_loading_entire_file`).

Fix: use an existing, unregistered temp file, and recover the test lock with
`unwrap_or_else(PoisonError::into_inner)`. **Effort: 1 hour.**

### 1e. Compile warnings on Windows

These compile clean today only because nothing builds Windows with `-D
warnings`. They're Linux- or macOS-only items that Windows doesn't use:

- `desktop_notifications.rs:24, 47, 256, 263, 397, 431, 436`
- `desktop_notification_responses.rs:20, 56, 66, 146`
- `desktop_secret_store.rs:3, 51, 52`
- `desktop.rs:10`
- `desktop_logging.rs:4` (`Permissions`)
- `verification/tests.rs:857` in synara-core (unused `path`)

Fix: `cfg`-gate them. **Effort: 2–4 hours.**

## 2. `src-tauri/src/desktop_*.rs` modules with no Windows branch

"No Windows branch" means Windows reaches the generic
`not(linux/macos)` fallback, or the code compiles to nothing.

| Module | What Windows gets today | Work to do it properly | Effort |
| --- | --- | --- | --- |
| `desktop_secret_store.rs` | Stub that reports `windows-native-session-store-unsupported` (see §3) | Credential store backend | see §3 |
| `desktop_notifications.rs` | Only the `not(linux, macos)` fallback: Tauri's notification plugin with title and body only. Routes, action buttons, dismiss keys and click-to-open are dropped. Most of the response and dismiss machinery is dead code (§1e). | WinRT toast notifications (`windows` crate or `tauri-winrt-notification`) with activation args for routes and actions. AUMID registration needs the installed shortcut, so unpackaged dev builds behave differently. | 4–6 days |
| `desktop_notification_responses.rs` | Compiles, but nothing feeds it on Windows | Comes with the toast work | (above) |
| `desktop_notification_sound.rs` | `Ok(false)`: no sound | `PlaySoundW` with `SND_MEMORY` on the bundled WAVs, or toast audio | 0.5 day |
| `desktop_tray.rs` | Shares the macOS-shaped (`not(linux)`) menu; no Windows-specific code | Mostly works through Tauri. Verify left-click behaviour, icon size (16/32 px `.ico`) and the tray-only lifecycle. | 1 day |
| `desktop_unread_badge.rs` | `emit_unity_launcher_badge` is a no-op; there's no taskbar badge | `WebviewWindow::set_overlay_icon` (Tauri 2 supports it on Windows) with a rendered count or dot | 0.5–1 day |
| `desktop_shortcuts.rs` | Generic advice string only; registration goes through `tauri-plugin-global-shortcut`, which supports Windows | Windows-specific conflict hints and a manual check | 0.5 day |
| `desktop_spellcheck.rs` | No-op `Ok(())` | WebView2 spellcheck is on by default; confirm and report the status honestly | 0.5 day |
| `desktop_webview_performance.rs` | Reports WebView2, "system-managed" | Already acceptable | 0 |
| `desktop_window_drag.rs` | No-op (`not(linux)`); the GTK hook is Linux-only | Tauri's `data-tauri-drag-region` works on Windows. Check the custom titlebar and caption buttons, and decide on native decorations versus Snap Layouts. | 1–2 days |
| `desktop_logging.rs` | `#[cfg(unix)]` 0600 modes skipped; the files inherit the profile ACL | Usually fine under `%APPDATA%` (see §5) | 0 |
| `desktop_file_transfer.rs` | Has a Windows branch for `USERPROFILE`; the 0600 temp-file mode is skipped | Same ACL note as §5 | 0 |
| `desktop_navigation.rs` | Hard-coded `tauri://localhost` (see §4) | Origin per platform | see §4 |
| `desktop_x_embed.rs` (new in 2.4.0) | Builds and needs no platform code. reqwest with rustls verifies through the platform verifier (the Windows cert store). | Runtime smoke test only | 0 |
| `desktop.rs`, `desktop_agent_actions.rs`, `desktop_integration.rs`, `desktop_platform.rs`, `desktop_sanitize.rs`, `desktop_url.rs` | Platform-neutral | `desktop_platform` and the bridge's `os` value need a `windows` case in the renderer | 0.5 day |
| `desktop_notifications_macos.rs` | macOS-only by design | none | 0 |

The renderer also needs a Windows pass: titlebar and window controls, keyboard
shortcuts shown as Ctrl instead of ⌘, font stack (Segoe UI), and scrollbar
styling. That's outside this spike. **Estimate: 2–3 days.**

## 3. Secret store: `windows-native-session-store-unsupported`

The current state is deliberate and fails closed:

- `desktop_secret_store.rs:301` reports
  `DESKTOP_SECRET_STORE_WINDOWS_UNSUPPORTED` (available: false, cannot persist
  the session).
- `KeyringSessionMaterialVault::platform_supported()`
  (`matrix/lifecycle/session_material.rs`) and
  `KeyringStoreKeyVault::platform_supported()` (`matrix/store/key_vault.rs`)
  both return false off macOS and Linux, so both vaults refuse.
- `keyring` is built with `apple-native` and `linux-native-sync-persistent`
  only.

Without a vault, Windows can't keep a session or the per-account store
encryption key across launches, so every launch is a fresh login, with a new
device and lost E2EE history.

Two secrets need a home:

| Secret | Size |
| --- | --- |
| Store key (`key_vault.rs`) | 32 bytes, stored as 64 hex characters |
| Session envelope (`SessionEnvelopeV1`, JSON) | `user_id`, `device_id`, `homeserver_url`, `access_token`, optional `refresh_token`: typically 300–600 bytes with Synapse or MAS opaque tokens, but unbounded in principle (OIDC homeservers that issue JWT access and refresh tokens can reach 2–4 KB) |

### Option A: keyring's `windows-native` feature (Credential Manager)

This is the smallest code change: add `windows-native` to the `keyring`
features, add Windows to `platform_supported()`, and add a
`windows-credential-manager` status backend.

The catch is the blob limit. `CRED_MAX_CREDENTIAL_BLOB_SIZE` is 2560 bytes,
and keyring 3.6.3's `set_password` stores the string as UTF-16
(`keyring/src/windows.rs:222-227`). That caps an envelope at about **1280
characters** before it fails with `TooLong`. Switching both vaults to
`set_secret` and `get_secret` (raw bytes) raises the cap to 2560 bytes,
which is still a hard ceiling an OIDC deployment can exceed.

Two smaller points:

- keyring writes with `CRED_PERSIST_ENTERPRISE`, so the credential roams with
  a roaming domain profile. That's acceptable but worth knowing.
- `TooLong` currently maps to the generic `p3.5-keyring-encoding` diagnostic.
  It needs its own diagnostic so a user can understand why login didn't stick.

**Effort: 1 day for the backend**, plus 1–2 days if the envelope is split
across several credentials or compressed to stay under the limit.

### Option B: DPAPI-encrypted file

`CryptProtectData` (user scope, with app-specific optional entropy) encrypts
the envelope, and the result is written to
`%APPDATA%\com.whylandcreative.synara.desktop\...` with the same atomic
write-then-rename the store already uses.

- No size limit, no keyring dependency, and the same per-user protection
  Credential Manager uses internally (Credential Manager is DPAPI underneath).
- Downsides: new unsafe FFI in `windows-sys` (small), a vault adapter
  alongside the keyring adapters, and file-lifecycle work (delete on logout,
  orphan cleanup).
- Any process running as the user can decrypt it, which is the same as
  Credential Manager.

**Effort: 2–3 days** including tests.

### Recommendation

Use Option A for the 32-byte store key, where the size can't be a problem.
For the session envelope, use Option B, or Option A with `set_secret` plus a
fallback to B above 2560 bytes. Either way, the status, diagnostics and
renderer copy for "secure storage unavailable" need a Windows variant.
**Total: 2–4 days.**

## 4. App origin: `tauri://localhost` versus `http://tauri.localhost`

On Windows, WebView2 can't register a custom scheme as a secure origin, so
Tauri 2 serves packaged assets from `http://tauri.localhost` (or
`https://tauri.localhost` with `app.windows[].useHttpsScheme`). Custom
protocols are served as `http://<scheme>.localhost/`.

- **Navigation lock:** `desktop_navigation.rs:10` hard-codes
  `PACKAGED_ASSET_ORIGIN = "tauri://localhost"`, and `lib.rs:979` pins the
  main webview's `on_navigation` to it in every non-dev build.
  - On Windows the app's own URL is `http://tauri.localhost/`, which
    `is_app_navigation` rejects (its tests assert that
    `http://tauri.localhost/` is rejected), so reloads and in-app navigation
    would be blocked.
  - Dev builds are unaffected because they use `devUrl`.
  - Fix: pick the origin per platform. Use `http://tauri.localhost`, or set
    `useHttpsScheme: true` and use `https://tauri.localhost`, which gives the
    page a secure context and is the better choice for WebCrypto and
    `navigator.clipboard`. Then compare by tuple origin for the http case,
    and add Windows cases to the tests.
- **CSP** (`tauri.conf.json`, mirrored by `csp_tests.rs`):
  - `'self'` follows the real origin, so script and style are fine.
  - `connect-src` already allows `http://ipc.localhost`, the IPC endpoint on
    Windows (and Linux).
  - **Media is blocked.** `img-src`, `media-src` and `connect-src` allow
    `synara-media:`, but on Windows `convertFileSrc(handle, 'synara-media')`
    (`synara/src/app/utils/desktop.ts:603`) yields
    `http://synara-media.localhost/<handle>`, which none of those directives
    allow. Avatars, inline images, video and custom emoji would all be
    blocked. Fix: add `http://synara-media.localhost` (or the `https`
    variant, to match `useHttpsScheme`) to the three directives and to
    `csp_tests.rs`.
  - The protocol handler in `lib.rs:483` only looks at the path, so it needs
    no change.
- Changing the origin also changes where WebView2 keeps localStorage and
  IndexedDB. That doesn't matter for a first Windows release, but don't flip
  `useHttpsScheme` after shipping.

**Effort: 1 day** including the tests and a manual WebView2 check.

## 5. `#[cfg(unix)]` file permissions in `crates/synara-core/src/app/store/`

- `paths.rs:245` sets `0o700` on every store directory after `create_dir_all`.
- `revision.rs:308` sets `0o600` on the manifest after its atomic rename, and
  `revision.rs:377` sets `0o700` on the recovery archive root.
- On Windows all three are skipped, so the files inherit the ACL of their
  parent. Under `%APPDATA%` (the roaming profile folder) that's the
  user, SYSTEM and Administrators, which is the usual Windows equivalent of
  0700. The same holds for the `cfg(unix)` modes in `auth/crypto_device.rs`,
  `x509/mod.rs`, `desktop_logging.rs` and `desktop_file_transfer.rs`.
- Hardening, which is optional: strip inherited ACEs and grant only the
  current user's SID, via `SetNamedSecurityInfoW` in one small helper behind
  a `cfg(windows)`. **Effort: 1 day** if wanted. It's not required for parity
  with what most Windows apps do.

Two Windows filesystem behaviours in this code need a runtime check rather
than permission work:

- **Renames with handles open.** `revision.rs:218` renames whole store
  directories into the recovery archive. Windows refuses to rename a
  directory while any file in it is open (SQLite pools, the Tantivy index).
  Recovery and reset must run after every handle is dropped; on Unix they
  happen to work with handles open.
- **Paths from `canonicalize`.** On Windows `canonicalize` returns verbatim
  `\\?\C:\...` paths. The comparisons in `paths.rs` canonicalize both sides,
  so they're consistent. But any canonical path that reaches the renderer,
  logs or a non-Rust API will look odd.

**Effort: 1 day** to test and harden.

## 6. Bash-only build scripts

- **`npm run tauri`** is `cp config.json synara/ && scripts/with-rust-cache.sh
  tauri`. npm runs scripts through `cmd.exe` on Windows, and the probe failed
  with `'scripts' is not recognized as an internal or external command`.
  `cp` happened to work only because the runner has Git's `usr\bin` on
  `PATH`; a normal Windows machine fails there first.
  `scripts/with-rust-cache.sh` and `scripts/lib/rust-cache.sh` only set up
  the Kache compiler cache.
  - Fix: a small `scripts/tauri.mjs` that copies `config.json`, sets the
    cache environment when Kache is present, and runs `tauri`.
  - The `cargo` and `cache:*` npm scripts have the same problem.
- **`scripts/build-runtime.mjs`** (the Tauri `beforeBuildCommand`) fails on
  Windows before it does anything. It computes
  `resolve(new URL("..", import.meta.url).pathname)`, and `.pathname` is
  `/D:/a/...`, which resolves to `D:\D:\a\...\config.json` (`ENOENT`). Fix:
  `fileURLToPath(new URL("..", import.meta.url))`. Other `.mjs` scripts
  probably share this idiom and should get a sweep.
- **Release and packaging scripts** are bash and Linux or macOS specific by
  nature: `build-macos-local-release.sh`, `build-apt-repo.sh`,
  `build-pacman-repo.sh`, `build-arch-package.sh`, `sign-apt-repo.sh`,
  `install-linux-tauri-dependencies.sh`, `generate-ts-dto.sh`,
  `check-swift-api-snapshot.sh` and others. Windows needs its own release
  job, not a port of these.
- **CI:** `ci-scopes.sh` and `ci-rust-cache-identity.sh` run under `bash`,
  which GitHub's Windows runners provide, so CI can keep `shell: bash`. The
  Kache cache action is untested on Windows; this spike ran uncached.
  `build_support/build_inputs.rs` shells out to `git`, which worked on the
  runner.
- `package.json`'s description still says "for macOS and Linux".

**Effort: 1 day** for `tauri.mjs`, the `fileURLToPath` sweep and a Windows
smoke step.

## 7. Not covered by this spike

- **Packaging and distribution.** `tauri.conf.json` already has a WiX section
  (`wix/banner.bmp`, `wix/dialogImage.bmp`). Still needed:
  - an MSI or NSIS choice;
  - an Authenticode certificate and timestamp URL (`certificateThumbprint`
    is null), or Azure Trusted Signing;
  - SmartScreen reputation;
  - a WebView2 bootstrapper mode;
  - an AUMID and Start-menu shortcut, which toast activation depends on;
  - `tauri-plugin-updater` for Windows. It's currently built without the
    `zip` feature, macOS-only by design; enable that feature and publish
    `.msi.zip` or NSIS signatures.

  **Estimate: 3–5 days** plus certificate procurement.
- **`tauri-plugin-single-instance`** on Windows, and deep links or `matrix:`
  URI handler registration, if wanted (1 day).
- **A real app launch.** This spike only ran `cargo check` and unit tests.
  Login against a live homeserver, sync, media, notifications, tray and
  window behaviour on WebView2 still need a manual pass on a Windows machine
  or VM (2–3 days of testing and fixes).
- **Windows on ARM** (`aarch64-pc-windows-msvc`). It should compile the same
  way, but it's untested.

## Effort summary

| Item | Effort |
| --- | --- |
| 1a. Session locator fsync (product bug) | 0.5–1 day |
| 1b–1d. Test portability (`.gitattributes`, temp paths, dropped-file test) | 0.5 day |
| 1e. Windows warnings, to allow `clippy -D warnings` | 0.5 day |
| 2. Notifications (toasts, actions, routes, sound) | 4–6 days |
| 2. Tray, badge overlay, shortcuts, spellcheck, window chrome, platform flag | 3–5 days |
| 2. Renderer Windows pass (titlebar, Ctrl labels, fonts) | 2–3 days |
| 3. Secret store (Credential Manager plus DPAPI file) | 2–4 days |
| 4. Origin and CSP | 1 day |
| 5. Store ACLs (optional) and rename-with-open-handles check | 1–2 days |
| 6. Cross-platform build entry points | 1 day |
| CI: Windows lint and test lanes (from this workflow) | 0.5–1 day |
| 7. Packaging, signing, updater, single instance | 4–6 days plus certificate |
| 7. Manual end-to-end pass and fixes | 2–3 days |
| **Total** | **about 22–34 engineer-days (4.5–7 weeks)** to a signed, auto-updating Windows build at feature parity |

A minimum build that installs, signs in, stays signed in, syncs, shows media
and posts basic notifications needs only 1a–1e, 3, 4, 6, unsigned MSI
packaging and a manual pass: **roughly 8–12 days**.

## Re-running the probe

`.github/workflows/windows-spike.yml` is `workflow_dispatch` only. GitHub
can't dispatch a workflow until the file is on the default branch, so to
re-run it from a branch, either merge the workflow first or temporarily add a
`push` trigger scoped to that branch (which is how this run was started). The
run uploads `windows-spike-logs` with each step's log, `outcomes.txt` and a
grep summary.
