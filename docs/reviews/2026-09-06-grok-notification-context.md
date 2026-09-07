# Grok4.6 High source review

I'll independently review this branch against the claimed notification-tap and NSE diagnostic fixes, starting from the review doc and the exact production/test diffs.I'll inspect the production and test files directly rather than deferring to the existing review notes.Next I’ll trace session readiness, room-title identity, Core NSE reason codes, and the test/cleanup paths that gate this change.**REQUEST_CHANGES**

The three ordinary notification-tap repairs are sound in source: the UN delegate is installed before launch returns; the focused timeline shares the existing Core readiness gate and then re-checks cancellation/account; room title is presentation-only from the room-list projection and does not sit in timeline identity. Opt-in, original-content fallback, encryption, and APNs payload shape are unchanged. I did not execute tests or a simulator.

The diagnostic allowlist added in this branch does not actually distinguish Keychain/vault unavailability. That is an actionable P2 on the claimed typed-reason path.

## P2

**`synara-ios/SynaraShared/SynaraNotificationPreviewSupport.swift:120`** (allowlist); arriving code **`crates/synara-nse-core/src/lib.rs:16`** and **`83-88`**; recorded at **`synara-ios/SynaraNotificationService/NotificationService.swift:271-276`**.

- **Lifecycle:** NSE `didReceive` → preference/session/store gates pass → `NsePreviewRequest.resolve` → Swift `NotificationKeychainNseSecretVault.get` throws `NseSecretVaultError.Unavailable(code: "p4-s3-secret-vault-unavailable")` → UniFFI → `SecretReaderAdapter::get` **discards that code** and always returns Core `"nse-secret-vault-unavailable"` → Swift `previewFailureStage(coreCode:)`.
- **Failure:** `previewFailureStage` maps `"p4-s3-secret-vault-unavailable"` to `.coreStoreUnavailable`, but that string never appears on `NseCoreError`. The code that does arrive falls through to `.coreResolutionFailed`. Filter / redacted / missing-event / decryption / missing-session still distinguish. Vault/Keychain unavailability does not. Description is not persisted (`Failed(code, _)`), so this is not a privacy leak and does not empty the notification.
- **Repair:** Map `"nse-secret-vault-unavailable"` to `.coreStoreUnavailable` (keep the p4 code if you want both). Add a unit assertion for the **arriving** code, not only the Swift vault’s unused code. Do not log or store the Failed description.

## Ordinary tap path (no defect)

**Cold/warm queue.** `UNUserNotificationCenter.current().delegate = self` is in `application(_:didFinishLaunchingWithOptions:)` (`SynaraApp.swift:278`). `bind(to:)` no longer races SwiftUI appearance. `didReceive` still queues when `push == nil` or `foregroundActive == false` (`322-328`); `drainPendingNotificationResponseIfReady` still requires both (`422-427`). `SynaraRootHost` can set `foregroundActive` before bind (`updateForegroundActive` returns early if `matrix == nil`), then `bind` drains. Warm: delegate already set, handle immediately.

**Session cancel / account switch.** After `waitUntilPrepared`, the timeline task returns unless the task is still live and `currentUserID` still matches (`RoomTimelineView.swift:548-553`). `waitUntilPrepared` already fails closed on supersede/cancel (`SignedInSessionReadiness.swift:66-82`). Login and wipe both `resetNavigationPathsForAccountChange()`, so this view is torn down with the account. `timelineTaskID` is only `roomID + eventID` (`1367-1369`); a same-view restart after `sessionEpoch` cancel is not an ordinary signed-in tap path.

**Title vs identity.** `displayRoomTitle` is `resolvedRoomTitle ?? roomDisplayName ?? route title ?? "Room"` (`351-353`). The title `.task` is keyed by user+room (`527-539`) and only writes `resolvedRoomTitle`. `RoutePlaceholderView` identity is `"\(id)-\(eventID ?? "")"` (`PlaceholderScreen.swift:39`). `timelineTaskID` does not include the title. Notification routes still replace the tab path with room/event IDs (`AppRouter.swift:36-46`). Product names come from `room.name ?? invite?.roomName ?? room.roomId`, so hydration can show a Matrix ID rather than `"Room"`; that is still the list owner.

**NSE product behavior.** Failed resolve still `deliver(content)` of the original mutable copy (`NotificationService.swift:113-119`). Preview text is applied only when `showPreview` is on (`91-95`). Defaults remain off. Encrypted timeline events still error as `p4-s11-nse-decryption-unavailable` (`nse_preview.rs:261-268`) and are not composed as preview (`SynaraMatrixEventPreviewComposer` still returns nil for `m.room.encrypted`). `synara-nse-core` production dep is `default-features = false, features = ["nse-preview"]`; `full-uniffi` is `[dev-dependencies]` only. I did not build the NSE staticlib.

## Proof limits (not defects)

- Simulator SpringBoard taps and host `NsePreviewRequest.resolve` are not physical APNs, extension Keychain/lock/memory, or the user’s missing-preview rate.
- Runner completion (`run-live-notification-tap.py:103-108`) fails closed on skipped XCTest, missing warm/cold confirmation, or any leave/logout failure; mocked regressions cover those four cases. Confirmed-phase matching is exact-line; ready-phase matching is substring. That is stricter, not a silent pass.
- I did not run the 92 Swift tests, 3 Core reason tests, live tap, or live NSE fixture. Those remain author-reported.

No P1. Fix the arriving vault code in the allowlist (and a test that uses that code) before this is ACCEPT.

## Arriving vault-code correction

I’ll re-check the mapping fix and the surrounding diagnostic path at the new head, without treating the previous proof notes as runtime facts.**ACCEPT**

The previous P2 is closed in source. `SecretReaderAdapter` still rewrites vault failures to `nse-secret-vault-unavailable` (`crates/synara-nse-core/src/lib.rs:16`, `83-88`). `previewFailureStage` now maps that arriving code, and still maps `p4-s3-secret-vault-unavailable`, to `.coreStoreUnavailable` (`SynaraNotificationPreviewSupport.swift:120`). Unknown strings remain `.coreResolutionFailed` (`128`; covered at `NotificationPreviewSupportTests.swift:9`). The NSE catch still binds `Failed(code, _)` and records only the allowlisted stage (`NotificationService.swift:271-276`); `record` stores `stage.rawValue` only (`SynaraNotificationPreviewSupport.swift:142`). The Swift vault still throws the p4 code (`NotificationService.swift:335-338`); that mapping is retained, not used as the FFI code.

`testPreviewFailureDiagnosticsRecognizeNseCoreVaultAdapterCode` asserts the arriving adapter string (`NotificationPreviewSupportTests.swift:12-19`). No preview body, opt-in, crypto owner, empty-content, or routing change in this delta.

No new P1/P2. I did not run tests. The review note that focused signed native validation is pending is a proof limit, not a pass. Physical APNs remains unclaimed.

Post-review native validation: all 14 signed NotificationPreviewSupportTests passed. Documentation-only commit `76ab6cdb` records that result; production remains identical to reviewed `cb745886`.
