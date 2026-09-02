# ROE-01 Research Memo: Matrix SDK orchestration residual census

Status: accepted ownership recommendation; docs-only; not approved for implementation.

| Field              | Value                                                                                                                                                                                                 |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Workstream/cluster | ROE-01                                                                                                                                                                                                |
| Research owner     | Residual-census researcher (orchestration lane)                                                                                                                                                       |
| Reviewers          | Independent feature-branch review; `ACCEPT_WITH_NITS` on PR `#1081` at `afba8efb`                                                                                                                    |
| Source census      | 2026-09-01; worktree `5f9c4e718f858606a0934acb1c9c8fa3fde138ab` on `roe/memo-01-orchestration` (program docs on `main` `011cf39a`). Product paths re-read; they match the [CENSUS.md](../program/CENSUS.md) snapshot for this domain. |
| ADR baseline       | [0003](../../../adr/0003-shared-native-rust-core.md), [0004](../../../adr/0004-rust-language-boundaries.md), [0005](../../../adr/0005-native-media-handle-channel.md); last reviewed 2026-09-01 on the ADR index commit in this tree. |

## Observable problem

Users must not see two Matrix sessions, two sync supervisors, or two crypto/backup state machines across desktop and iOS. The residual question is whether any shipped TypeScript, Swift, or desktop-shell path still *owns* construction, restore, sync start/stop, reconnect, backup/cross-signing transitions, destructive wipe, or vault identity — or whether those paths only observe lifecycle, hold OS secrets, and present Core DTOs.

This memo does not ask whether P4 engine-ready or live-homeserver proof is complete. The [goal graph](../../../shared-native-core/13-language-boundary-goal-graph.md) still stops on paused iOS CI and live proof. That is a release gate, not a second engine.

## Current ownership census

Re-verification agrees with [CENSUS.md](../program/CENSUS.md) for sync/restore/crypto: Core owns the machines; desktop leftover secret commands and iOS Keychain/UniFFI wrappers stay shell-side. No snapshot-vs-source disagreement that changes the prior.

| Concern | Rust/Core | Desktop | iOS | Evidence/tests |
| ------- | --------- | ------- | --- | -------------- |
| Client construction / restore | Authority. `app/client_builder` builds the unauthenticated client. `app/lifecycle` persist/restore and `app/auth::login_with_password` are the shared login/restore functions. UniFFI `SharedCore::restore_persisted_session` / `login_with_password` open Core, restore from the S3a vault, and retain one Client. | Installation adapter plus accepted secret/vault boundary. `src-tauri/src/matrix/client_builder` and `lifecycle` re-export Core. `matrix_login_password` / `matrix_restore_session` construct Core owners, retain `ManagedMatrixSession`, then call `Core::attach_*` and `open_after_desktop_session_install`. This is thicker than a pure invoke adapter but remains one Core client. React `initMatrix.ts` / `nativeClientFacade.ts` invoke restore; no `matrix-js-sdk` `createClient`. | Thin adapter. Live `AppEnvironment` uses `SharedCoreAuthService` → `SharedCoreSessionLogin` and `SharedCoreSessionBootstrap` → `SharedCoreSessionRestore`. Product login writes `AuthenticatedSession.accessToken = ""`. `PlaceholderAuthService` is not wired live. | Core: `crates/synara-core/src/app/client_builder/`, `lifecycle/session_restore.rs`, `shared_core_ffi.rs` restore/login. Desktop: `product_commands.rs` inventory test in `session_lifecycle.rs`. Presenter: `synara/src/app/pages/auth/login/__tests__/desktopPasswordLoginNativeOnly.test.ts`. iOS: `SharedCoreSessionBootstrapTests.swift`. |
| Sync start / stop / cancellation | Authority. `SyncServiceOwner` is the single `matrix-sdk-ui` SyncService per generation. `Core::start_attached_sync` / `stop_attached_sync` start or stop that owner. UniFFI `start_sync` / `stop_sync` refuse NSE, require attach, and treat a second start as restart of the same owner. | Adapter. `src-tauri/src/matrix/sync` re-exports Core. Desktop `start_sync_owner` calls `build_sync_service` + `owner.start()`. Logout calls `sync.stop()` then `Core::close`. Presenter `startClient` / `retryImmediately` only refresh the facade cache; they do not start a JS sync. | Observation + typed start. `SharedCoreSyncStart` / `SharedCoreSyncStop` forward UniFFI. `SharedCoreMatrixClientService` gates start on UIKit foreground, pauses with `stop_sync`, and returns `false` from `syncForBackgroundNotification`. NSE `SharedCoreNseStore` never starts SyncService. | Core: `app/sync/service.rs`, `core.rs` `start_attached_sync`. Tests: `crates/synara-core/tests/p4_s12_start_sync.rs`. iOS: `SharedCoreSessionBootstrapTests.swift`, `MatrixLifecycleTests.swift`. |
| Reconnect / backoff / ordering | Authority. Pure `decide_reconnect` + `SyncServiceOwner::apply_intent`. Default `SyncServiceConfig.offline_mode = true` is SDK offline probe, not a second table. HTTP `DEFAULT_RETRY_LIMIT = 3` lives on the Core builder. | Observation. `shouldRetrySyncOnResume` is a two-state visibility predicate (`RECONNECTING` / `ERROR`) that calls facade `retryImmediately` (status poll). `watchSync` polls Core readiness. No JS backoff loop. | Observation / rendering. `ConnectionStatusStore` holds Lost chrome 4s. `SharedCoreSessionBootstrap` retries `start_sync` twice and observes Core readiness; idle is not live. Path-monitor reconnect re-enters `prepareLiveSession` (Core). `MatrixInteractiveFreshnessPolicy.shouldPerformSync` is test-only and returns false when SyncService is already active. | Core: `app/sync/reconnect.rs`. Desktop: `synara/src/app/utils/syncLifecycle.ts` + `synara/src/app/utils/__tests__/syncLifecycle.test.ts`, `ClientRoot.tsx`. iOS: `ConnectionStatus.swift`, `SharedCoreSessionBootstrap.swift`. |
| Crypto / backup / cross-signing | Authority. `app/backup` status + restore + `NativeBackupAction` projector. `app/cross_signing` status/setup start. `app/secret_storage` status projector. Leftover UniFFI `recover` is planted fail-closed and never performs secret-storage I/O. | Status through Core (`matrix_backup_status`, `matrix_cross_signing_status` / `setup`). Setup/repair/unlock leftovers stay in the shell because passphrases must not cross `Core::command` ([playbook §6](../../../shared-native-core/11-implementer-playbook.md)). They call the same `Client` and Core projectors. Presenter `nativeBackup.ts` / `nativeCrossSigning.ts` / `nativeSecretStorage.ts` are invoke wrappers. | Projection + dedicated restore. `SharedCoreSessionCrypto` maps leftover/status strings to UI enums. Product recover uses `SharedCoreBackupRestore.restoreBackup` → Core `restoreBackup`, not leftover `recover`. Leftover `recover` appears only in bindings tests. | Core: `app/backup/{status,live}.rs`, `app/cross_signing/`, `shared_core_ffi.rs` `recover` (always `LEFTOVER_UNAVAILABLE`). Desktop: `backup/live.rs` setup/repair. iOS: `SharedCoreProductServices.swift` `recover`, `SharedCoreLeftovers.swift`. |
| Destructive lifecycle | Authority. `app/lifecycle` logout/wipe, remote-logout machine, and `recovery_action_for` with hard `auto_wipe: false`. Store identity and wipe targets are Core. | Adapter + OS cleanup. `matrix_logout` best-effort remote logout, `sync.stop()`, Keyring clear, identity-file remove, then `Core::close`. Failed-store recovery arms an explicit archive; login never auto-wipes. Presenter `performLogout` invokes `matrix_logout` and clears local presenter caches. | Adapter + OS cleanup. `AppLocalWipeService` deletes Keychain identity first, then leftover `logout` (drops retained Client; no homeserver hit) and `stop_sync`. `resetLocalState` documents keeping the per-account crypto store. `MatrixSessionRestoreError.shouldDeletePersistedStore` is always `false`. | Core: `lifecycle/{logout,wipe,recovery}.rs`. Desktop: `product_commands.rs` `matrix_logout`. iOS: `LocalWipeService.swift`, `SharedCoreProductServices.swift` `resetLocalState`. |
| Store identity | Authority. `app/store/identity.rs` `AccountIdentity` derives path-safe roots. UniFFI restore/login validate the same identity before opening the store. | Adapter. Desktop `write_active_identity` is a non-secret file used to decide whether the native route mounts; restore still uses Core identity + vault. Store-key Keyring adapter implements Core's vault trait. | Observation. `AuthenticatedSession` carries user/device/homeserver for UI cold start. `sdkStoreID` is unused on the product login path. Live restore uses Core `AccountIdentity` + `storeRoot`, not a Swift store-identity policy. | Core: `app/store/identity.rs`. Desktop: `product_commands.rs` `matrix_session_identity`. iOS: `AppServices.swift` `AuthenticatedSession`, `SharedCoreSessionRestore.swift`. |
| Vault handoffs | Authority for envelope/trait. `SessionMaterialVault` + sealed envelope live in Core. Secrets never use `Core::command`. | Platform credential store. `KeyringSessionMaterialVault` and `KeyringStoreKeyVault` implement Core traits. Passwords/recovery keys stay on leftover Tauri commands. | Platform credential store. `KeychainIosSecretVault` is Core `SecretVault`. `KeychainSecureSessionStore` holds signed-in identity for the SwiftUI shell; it is not a Matrix client or crypto store. Product login does not persist an access token into that envelope. | Core: `lifecycle/session_material.rs`. Desktop: `src-tauri/src/matrix/lifecycle/session_material.rs`. iOS: `IosSecretVault.swift`, `SecureSessionStore.swift`. |
| Orchestration Matrix writes | Authority. Session, sync, crypto, and leftover status writes go through Core owners or dedicated secret FFI. | Leftover password/recovery commands write through the installed `Client` after Core construction; they are not a second client. | Product login/restore/start/backup-restore are UniFFI. Historical `MatrixRoomReadMarkerService` HTTP Bearer path is not the live `AppEnvironment` owner (`SharedCoreRoomReadMarkerService`); that file is out of this workstream (ROE-05). | Playbook §6 leftover table. iOS: `AppEnvironment.swift` live wiring. |

**Taxonomy.** Construction, restore, sync, reconnect, backup/cross-signing transitions, wipe policy, and store identity are **Core authority**. Foreground/background, visibility, path, Keychain/Keyring, and banner hold times are **platform observation** (and banner chrome is **rendering**). Constraints: one Matrix engine and no generic-envelope secrets are **hard invariants** (ADR 0004). Credential stores, NSE narrowness, and leftover secret commands are **accepted platform boundaries**. React vs SwiftUI vs Tauri command names are **technology preferences**, not engines.

**Earliest divergence.** There is no competing protocol owner. The earliest *appearance* of a second engine is desktop `ManagedMatrixSession` still holding the live `Client`, and iOS `AuthenticatedSession` still looking like a token-bearing session. Both are install/projection seams: desktop holds the Client because leftover secret I/O cannot cross the generic envelope; iOS Keychain identity is empty-token after Core login. Reconnect policy does not diverge — presenters poll or re-enter Core start.

## Boundary constraints

- ADR 0003: one Core; no JS or Swift Matrix engine for session, sync, crypto, or Matrix writes.
- ADR 0004: leftover passwords, recovery material, and paths stay off `Core::command`. NSE must not boot full sync.
- ADR 0005: media bytes are a dedicated channel; not an orchestration remainder.
- Playbook §5 / §6: the 21 unregistered leftover names stay desktop. Do not register `matrix_restore_session`, `matrix_logout`, or backup/secret commands tonight.
- Goal-graph stop: do not invent S38; do not claim P4 engine-ready; leftover secret/byte commands must not cross the envelope.
- Platform-side by design: Keyring/Keychain, UIKit foreground, connection banners, presenter cache wipe, and DTO mapping.

Stale comment only (not a remainder): `src-tauri/src/bridge/session_lifecycle.rs` still says the desktop is the sole SDK-client owner. Current install path builds via Core, attaches owners, and mirrors a credential-free snapshot. Treat that comment as historical P3 wording.

## Alternatives

1. **No ownership change (recommended).** Leave leftover secret commands, Keyring/Keychain adapters, and presenter observation in place. Falsified if a shipped TS/Swift path constructs a Matrix client, runs its own SyncService, applies a second reconnect table, or performs backup/cross-signing without the Core client/projector.

2. **Bounded extraction / shared fixture.** Move desktop backup setup/repair or iOS leftover `recover` wrappers into Core commands. Falsified as necessary tonight because setup/repair carry recovery secrets (ADR 0004 / playbook §6) and leftover `recover` is already fail-closed and unused by product. A fixture corpus would not change ownership.

3. **Broader Core model.** Register leftover restore/logout/backup on `Core::command` or collapse desktop `ManagedMatrixSession` into UniFFI-only construction. Falsified as a residual-census action: it would move secrets onto the generic envelope or invent S38, both forbidden by D3/D7 and the goal-graph stop.

Strongest stay-put case: the thick-looking desktop install function is the accepted secret/vault boundary around one Core client, not a second lifecycle engine. Thinning comments later does not require a product move.

## Recommendation

**Already correctly owned.** Confidence: high.

No shipped desktop or iOS path is a second Matrix lifecycle or crypto engine. TypeScript has no `matrix-js-sdk` importer and no `createClient`. Product Swift imports `SynaraCore`; the probe uses `MatrixRustSDK`, and `synara-ios/Synara.xcodeproj/project.pbxproj` still contains a leftover Frameworks entry for it, but no shipped product source uses it as an engine. Desktop `src-tauri/src/matrix/*` modules for sync, lifecycle, client builder, backup status, and cross-signing live-status are Core re-exports plus the accepted installation/OS-vault seam. iOS session/sync/crypto product services are UniFFI forwards plus chrome.

Concrete remainder for ROE-01: **none.** Device-key continuity in `MatrixClientPolicies.swift` is the ROE-02 question, not a second sync/crypto engine. Agent-approval planners remain ROE-08. HTTP read-marker leftovers are unused live and belong to ROE-05 if reopened.

Unresolved (explicitly not this lane): P4 engine-ready / live homeserver / hosted iOS CI remain blocked on the goal graph. That does not re-open orchestration ownership.

Regression proof to keep the close stable:

- Desktop: `session_lifecycle.rs` install/clear inventory; login tests forbidding `createClient` / `matrix-js-sdk`; `synara/src/app/utils/__tests__/syncLifecycle.test.ts` staying a resume predicate.
- Core: `p4_s12_start_sync.rs` (NSE cannot start; missing attach fail-closes); lifecycle recovery never auto-wipes.
- iOS: `SharedCoreSessionBootstrapTests.swift` (idle is not live; restore→attach→start); leftover `recover` remains fail-closed and off the product recover path.

## Next gate

Close the ownership question. No implementation plan or ADR change is needed.
Optional stale-comment and Xcode-linkage hygiene is tracked as
[A11](../program/ACTIONS.md); remove linkage only after proving it is not a
build dependency.
