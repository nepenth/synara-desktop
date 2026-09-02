# ROE-02 Research Memo: Device verification and iOS continuity

Status: draft research; docs-only; not approved for implementation.

| Field              | Value                                                                                                                                                                                                                                                                                                                                 |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Workstream/cluster | ROE-02                                                                                                                                                                                                                                                                                                                                |
| Research owner     | Residual-census researcher (this memo)                                                                                                                                                                                                                                                                                                |
| Reviewers          | Unassigned                                                                                                                                                                                                                                                                                                                            |
| Source census      | 2026-09-01 on worktree `roe/memo-02-verification` at `5f9c4e718f858606a0934acb1c9c8fa3fde138ab`. [CENSUS.md](../program/CENSUS.md) is a `main` `011cf39a` snapshot only; source below was re-read on this commit.                                                                                                                      |
| ADR baseline       | [ADR 0001](../../../adr/0001-ios-repository-layout.md), [0002](../../../adr/0002-ios-architecture.md), [0003](../../../adr/0003-shared-native-rust-core.md), [0004](../../../adr/0004-rust-language-boundaries.md), [0005](../../../adr/0005-native-media-handle-channel.md); last reviewed 2026-09-01; source commit as above. |

## Observable problem

Users complete interactive device verification (request, SAS comparison, confirm, durable trust readback) from desktop and iOS. The portfolio prior is that Core already owns that state machine. The only open residual question is whether iOS still omits a **device-key continuity** or **app-lifecycle** input that `NativeVerificationOwner` needs to resume, especially around `KeysExchanged`, presentability, comparison, confirmation, and trust propagation.

This memo asks whether such a missing **Core input** exists. It does not treat leftover Swift helpers, sheet navigation, or emoji layout as a second verification engine.

## Current ownership census

Re-verified against current source. Where this table disagrees with [CENSUS.md](../program/CENSUS.md), **source wins**. The snapshot’s note that “extra device-key continuity policy may live in `MatrixClientPolicies.swift`” is stale as a *product owner*: those types exist, but the product session and verification paths do not call them.

| Concern | Rust/Core | Desktop | iOS | Evidence/tests |
| ------- | --------- | ------- | --- | -------------- |
| Request discovery, SAS phases, allowed actions, cancel/timeout/complete | Authority: `NativeVerificationOwner` retains SDK request/SAS handles; projects privacy-safe phases; owner-accepts every transitioned SAS; confirm requires `sas.can_be_presented()` | Thin Tauri `matrix_verification_*` bridges | Thin UniFFI `verification_*` + `SharedCoreVerificationList` / `SharedCoreVerificationSas` | `crates/synara-core/src/app/verification/live.rs`; `src-tauri/src/bridge/verification_*.rs`; `synara-core.udl` `verification_start(string? device_id)`; `crates/synara-core/src/app/verification/tests.rs`; `crates/synara-core/tests/p4_s8_verification_list.rs`, `p4_s9_verification_sas.rs`; `synara-ios/SynaraTests/SynaraCoreBindingsTests.swift` fail-closed without session |
| Own-identity vs peer start | Authority: `start(None)` uses `query_own_identity`; `start(Some(device_id))` is direct peer | Observation: `startNativeVerification(deviceId?)`; Settings “Verify from Another Device” omits `deviceId` | Observation: Settings “Verify This Device” calls `requestDeviceVerification()` → `deviceId: nil`; explicit session-row IDs remain peer | `live.rs` `start` / `start_self_verification`; `synara/src/app/features/settings/devices/Verification.tsx`; `synara-ios/Synara/Features/SettingsView.swift`; `SharedCoreProductServices.swift` comments on nil vs session-row |
| `KeysExchanged` / presentability / confirm | Authority: `SasState::KeysExchanged` plus `can_be_presented()` → phase `sas_ready` with emoji/decimals; confirm before presentable → `v-crypto.1-confirm-before-sas` | Rendering: confirm UI only on `sas_ready` with codes | Rendering: `sas_ready` + emoji/decimals → comparison; `sas_ready` without payload → `.failed` (contract, not a second machine) | `live.rs` `project_request` / `confirm`; `nativeVerification.ts` `verificationRequestHasSasCodes`; `NativeDeviceVerification.tsx`; `SharedCoreVerificationLive.swift`; `RootShellView.swift` “They Match” only on `.emojis` / `.decimals` |
| Trust readback / eligibility | Authority: `NativeDeviceSnapshot.own_verification` from `Encryption::verification_state()`; `has_devices_to_verify_against` from the SDK | Presentation of the snapshot | Presentation: `sessionStatus()` maps `ownVerification` + `hasDevicesToVerifyAgainst`; `SecuritySettingsVerificationPolicy` only enables the button | `crates/synara-core/src/app/devices/live.rs`; snapshot JSON forbids `ed25519` / `curve25519` / `device_key` (`devices/mod.rs` test); `SharedCoreProductServices.swift` `sessionStatus` |
| Device-key continuity (curve25519/ed25519 vs `/keys/query`) | Not a `NativeVerificationOwner` input. Session restore binds vault user/homeserver/device id (`p3.6-session-*`). Device snapshots omit keys. | Desktop store-continuity leftovers are session/store, not SAS | Leftover `MatrixDeviceKeyContinuityValidator` exists in `MatrixClientPolicies.swift` and is **unit-tested only**. Product restore is `SharedCoreSessionBootstrap` → Core restore/attach/start. No product caller. | `session_restore.rs`; `MatrixClientPolicies.swift`; `MatrixStoreLifecycleTests.swift`; `SharedCoreSessionBootstrap.swift`; `SharedCoreSessionRestore.swift` |
| App / process lifecycle vs in-flight SAS | Owner is in-process; no foreground/background command. Attach is once per live client (`p4-s3d-already-attached`). `start_sync` resumes the same owner; `stop_sync` quiesces sync/stores and does **not** detach verification. | Desktop process stays attached | Observation: `pauseForBackground` → `stopSync`; `resumeFromForeground` → `prepareLiveSession` (skip restore/attach if already live). Unused leftover `MatrixVerificationLifecyclePolicy` says background/foreground must **not** reset. | `shared_core_ffi.rs` `start_sync` / `stop_sync`; `SharedCoreProductServices.swift`; `MatrixLifecycleTests.swift`; 2026-08-25 iOS note that presented SAS survived background/foreground |
| Inbox projection / sheet restore | Authority: list + wake-only `NativeVerificationUpdateSignal` | Presentation helpers + window event | Presentation: `SharedCoreVerificationLive` maps DTO → `CryptoVerificationState`; `CryptoVerificationPresentationPolicy.restoredStateIfCleared` re-shows a non-terminal Core row if the sheet was cleared | `nativeVerification.ts`; `SharedCoreVerificationLive.swift`; `RootShellView.swift` |
| Unused leftover Swift reducers | None | None | `MatrixVerificationStateReducer` and `MatrixVerificationContinuationRegistrationTracker` are **not** product-called (tests only). Not a competing engine. | `MatrixClientPolicies.swift`; `MatrixLifecycleTests.swift` |

**Classification.** Verification request/SAS/trust authority is Core (hard invariant: no second Matrix/crypto engine — ADR 0003/0004). Viewport, sheet, emoji/decimal layout, accessibility, and button enablement are platform rendering (accepted platform boundary). Foreground/background and “sheet was swiped away” are platform observations that may *wake* Core (sync resume, re-list) without Core owning UI lifecycle. Device keys and recovery secrets must not cross the generic envelope (ADR 0004 hard invariant 3); they are not a missing verification DTO.

**Earliest actual divergence.** Historical iOS owned an independent Swift Matrix/crypto path, including a local `/keys/query` continuity check and UI-phase reducers. After P4-S8/S9/S20 (`#947`, `#948`, `#1001`), product verification consumes Core list/SAS and the attached owner. The leftover Swift continuity/lifecycle/reducer types were not deleted; they were also not re-homed into the product restore or SAS path. That is leftover unused policy, not an omitted Core input.

Playbook §5 and the [goal-graph stop conditions](../../../shared-native-core/13-language-boundary-goal-graph.md) treat P4-S20 as landed and P5 / live-homeserver / iOS-on-engine as blocked. This memo does not invent S38 or a verification implementation slice.

## Boundary constraints

- ADR 0003: one Core for crypto/verification; Swift adapters stay thin.
- ADR 0004: verification states are Core-shaped authority; app lifecycle and SwiftUI comparison screens stay platform-owned. No second Matrix engine. No generic-envelope secrets or device keys.
- ADR 0005: not in scope (no media bytes/paths).
- `NativeVerificationOwner` required platform inputs are: optional start `device_id` (`nil` = own identity), `flow_id` for later actions, and user match/mismatch/cancel/dismiss after presentability. Incoming requests are registered from SDK to-device events after attach+sync. There is **no** device-key or background-pause parameter on the owner.
- iOS `stop_sync` is an OS-suspension observation (release SQLite locks). It must not be read as “drop the verification state machine.”
- Current-device live proof remaining “Not confirmed” in `synara-ios/docs/ios-validation-status.md` is an operator/P5 evidence gap, not a missing Core input. Goal graph: do not start P5 from this portfolio.

## Alternatives

1. **No ownership change (stay-put).** Keep Core as the only SAS/trust authority. Leave leftover unused Swift continuity/reducer types as dead policy (or a later hygiene deletion outside this census). Keep sheet restore and Settings enablement platform-side. **Falsified if** a current product path still feeds curve25519/ed25519 (or a Swift phase machine) into verification, or if iOS “Verify This Device” substitutes a peer device id, or if background/foreground detaches the owner so Core cannot resume an in-flight flow that the SDK still holds *and* Core has no attach/list path for that case.
2. **Bounded extraction / shared contract.** Only justified if both clients omitted the same typed input Core already requires (for example, losing `nil` own-identity, or confirming before `sas_ready`). Current source does not show that. Shared fixtures for SAS *display* would be rendering, not authority.
3. **Broader Core model (Swift verification engine, or Core-owned device-key continuity as a verification input).** Rejected: it would create a second engine or put keys on the verification envelope. Core restore already binds session identity; the SDK crypto store is the continuity owner after attach.

## Recommendation

**Already correctly owned.**

Confidence: high for ownership; live current-device proof remains operator-gated and is not treated as a residual Core input.

Supporting evidence:

- iOS product start preserves `deviceId: nil` for own-identity; UniFFI and Core `start(None)` agree with desktop omitting `deviceId`.
- Confirm is Core-gated on presentability; both presenters only offer match/mismatch after `sas_ready` codes.
- Trust eligibility is Core snapshot fields, not a Swift trust flag.
- Foreground resume skips re-attach when owners are live; background stop does not replace `NativeVerificationOwner`.
- `MatrixDeviceKeyContinuityValidator` is not a product caller and is not an owner input. CENSUS snapshot overstated it as the open remainder.

Strongest stay-put objection: unused Swift continuity/lifecycle types *look* like a second policy owner, and the current-device live run is still “Not confirmed.” Those facts do not prove iOS omits a device-key or lifecycle **input required by** `NativeVerificationOwner`. Unused tests-only policy is leftover, not a missing Core argument. Unconfirmed live proof is P5/operator, not a state-machine hole.

Unresolved questions (explicit, not assumed):

- Operator-gated current-device Synapse proof after relaunch (`own_verification == verified` on a nil-target run) is still not recorded. That does not authorize a Swift engine or a new Core input.
- Later dead-code deletion of unused `MatrixClientPolicies.swift` verification/restore types is hygiene, not a residual-ownership extract.

Regression proof to keep the boundary stable:

- Product iOS “Verify This Device” continues to pass `nil` device id; session-row verify remains a distinct peer route.
- Core confirm still rejects before `can_be_presented()`; presenters still hide match actions until `sas_ready` with codes.
- Device snapshots still omit key material.
- iOS background/foreground still does not detach the verification owner or reset Core phases.
- No product caller of `MatrixDeviceKeyContinuityValidator` / `MatrixVerificationStateReducer` as a live SAS authority.

## Next gate

Already owned: close the research item. No implementation plan, no Core command, no Swift verification engine. Do not treat leftover unused Swift validators as a missing Core input. Do not start P5 or invent S38 from this memo.
