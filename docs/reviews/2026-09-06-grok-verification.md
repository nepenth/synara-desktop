# Grok4.6 High source review

I'll review the verification-eligibility changes independently from the provided docs and source, starting with the review skill and the exact files in the diff.I'll inspect the observer, stream, and lifecycle call sites next so the review can challenge the path document against the actual code.**ACCEPT**

The iOS settings observer was reading `verificationUpdates()`, which only yields when a verification inbox request exists. Device/key-query updates with an empty inbox never refreshed settings, so an initial `hasDevicesToVerifyAgainst == nil` (“could not check”) could stick. Switching that observer to `sessionDeviceUpdates()`, and subscribing to `devices`+`verification` before the first snapshot, is the right repair. Core still owns eligibility. Diagnostics are env-gated closed vocabulary. No desktop speculative fix.

This review is source-only. It does not re-run tests, live credentials, or git, and it does not treat the path document’s runtime table as independent proof.

## Findings

### P2 — Live proof treats Core fallback `Some(false)` as success
- **File:** `crates/synara-core/tests/live_device_eligibility.rs:48`
- **Route:** ignored `fresh_and_restored_core_report_device_eligibility` with `SYNARA_LIVE_*`, after `attach_session_owners` → `start_sync` → `device_snapshot`
- **Broken behavior:** `observe` only rejects `has_devices_to_verify_against.is_none()`. On authority timeout/error, `snapshot` already projects `eligible_local_authority` over the local set (`crates/synara-core/src/app/devices/live.rs:517`). A fresh empty store becomes `Some(false)`. Fresh and restored can both “succeed” and `assert_eq!` at line 117 without ever getting `Ok(Ok(_))` from `has_devices_to_verify_against()`. That violates the path document’s own rule that local fallback disqualifies clean proof. Diagnostics can show `authority=timeout` vs `none`, but the test never asserts them.
- **Repair:** Fail unless the raw authority query is `Ok(Ok(_))`, or assert the diagnostic categories (`eligible`/`none` vs `timeout`/error). Do not treat snapshot `Some(false)` as proof of a healthy none result.

### P2 — Restore/logout failure can leave a live server session
- **File:** `crates/synara-core/tests/live_device_eligibility.rs:92`
- **Route:** production `login_with_password` succeeds, then `logout().expect` panics, or `restore_persisted_session` fails
- **Broken behavior:** Fresh `logout` is local-only. Revoke runs only on the restored Core (`line 105`). `revoke_server_session` returns `Ok(false)` when there is no retained client, and the test still `remove_dir_all`s the store before asserting revoke. A failed restore therefore deletes the local encrypted store and drops the in-memory vault while the homeserver device from login remains. `logout().expect` at line 93 panics before any revoke. There is no cleanup scope.
- **Repair:** Keep the logged-in Core until revoke succeeds, or revoke from the fresh Core if restore fails. Use a cleanup path that always attempts `revoke_server_session` + store removal, and treat leftover remote session as a failed proof without depending on `expect` panics.

### P2 — Device invalidations are unbounded and replayed as full snapshots
- **File:** `synara-ios/Synara/Services/SharedCoreProductServices.swift:1966`
- **Route:** Security/Account settings `.task` → `SessionCryptoStatusObserver.start` → `sessionDeviceUpdates()` while `sessionStatus()`/`deviceSnapshot()` is in flight (eligibility lookup can take up to 8s)
- **Broken behavior:** `ownerSignals` is subscribed synchronously (good), but both the inner owner stream and the outer `AsyncStream` use default unbounded buffering. Timeline already uses `.bufferingNewest(1)` for the same “invalidation, then re-read snapshot” pattern. Each buffered device/verification signal then drives another serial full snapshot, including `/keys/query` and `/devices`.
- **Repair:** Create `sessionDeviceUpdates()` with `bufferingPolicy: .bufferingNewest(1)` (and the same on `ownerSignals` for this family, or coalesce in the observer) so only the latest wakeup is retained across the initial read.

## Assessment

**Stream subscription / cancellation / session / concurrency.**  
Subscribe-before-read is real at both layers: `sessionDeviceUpdates()` registers `ownerSignals(["devices","verification"])` before returning, and `SessionCryptoStatusObserver.start` (`SettingsView.swift:1194`) holds that stream across the first `refresh`. That matches the empty-inbox bug: `verificationUpdates()` already listened to both families, but only yielded when `currentVerificationState()` found an inbox request (`SharedCoreProductServices.swift:1802` and `2050`). Cancellation is adequate: SwiftUI `.task` cancellation drops the stream, `onTermination` cancels the forwarding task, and both loops check `Task.isCancelled`. Session generation is not filtered (desktop `useDeviceList` is); a stale waiter after logout/login can refresh once against the new Core. Retry (`SettingsView.swift:1247`) can still race `start()`’s loop and apply an older `sessionStatus()`; not new, more likely now that device signals wake the observer.

**Healthy none vs unavailable.**  
Presenters still distinguish `Some(false)` (“No eligible verified session”) from `nil` (“could not check”). Diagnostics additionally split raw authority `none` / `eligible` / `timeout` / store categories. The snapshot itself is unchanged: authority failure plus a loaded local set still becomes `Some(bool)`, including `Some(false)` for an empty fresh store. That is a pre-existing product collapse of unavailable into healthy none, not introduced here; the live test does not catch it (P2 above).

**Diagnostics privacy.**  
`lookup_failure_category` / `http_failure_category` return static labels only. `eprintln` prints `authority` / `sessions` / `crypto` categories. The unit test at `live.rs:601` locks an `Error::Io` carrying private text to `"other"`. Live-test stderr is closed vocabulary plus `DeviceCommandError` source constants. No SDK Display/Debug, MXID, device id, URL, or credential in these new lines. Flag is exact `SYNARA_VERIFICATION_DIAGNOSTICS=1`.

**Live fixture cleanup.**  
Happy path: revoke the login `device_id`, local `logout`, drop Cores, `remove_dir_all` of the `0700` temp store, in-memory vault dropped. Failure path is P2 above. The test does not bootstrap cross-signing, SAS, or trust.

**Proof claims.**  
The ignored test can show fresh/restored snapshots with one current device and `Some(eligibility)` on whatever account is in the environment. It cannot, by itself, prove eligible vs none, and it cannot prove the authority query succeeded. I did not execute it. The path document’s two-account table and “neither depended on fallback” remain operator claims, not locked assertions.

## Limits (not defects)

- Both-clients “could not check” on the affected installed devices is still unreproduced. This iOS inbox-filter bug does not explain desktop, which already listens to device-list + verification and window-focus refreshes.
- iOS `sessionStatus()` still uses `try?` (`SharedCoreProductServices.swift:1769`). A server-list snapshot failure still presents as nil eligibility even if the authority query succeeded.
- The new native test proves the observer now follows `sessionDeviceUpdates` with an empty verification inbox. It does not prove cancellation, session-generation filtering, or retention of a wakeup that arrives *during* `sessionStatus()`.
- Diagnostics are observation only; they are not a verification mechanism and do not replace affected-device evidence.
- No desktop change, as intended.
