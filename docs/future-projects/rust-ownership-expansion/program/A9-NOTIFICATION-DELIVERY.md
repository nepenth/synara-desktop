# A9 notification-delivery operating-path record

Status: deterministic iOS registration repair implemented; physical APNs/NSE
and desktop tray delivery are not confirmed. The desktop `failed` delivery
receipt is unreachable on macOS at the current source (see the 2026-09-08
desktop findings below).

Last updated: 2026-09-08 on `cursor/notification-core-push-rules-c25c`.

This record deliberately separates executable contract evidence from external
delivery evidence. A unit test, simulator run, compiled extension, or valid
pusher request is not evidence that a homeserver, push gateway, APNs, NSE, or
desktop notification center delivered a notification.

## Privacy-safe diagnostics contract

The app and NSE may persist only fixed stage codes, timestamps, and randomly
generated local correlation UUIDs. They must never persist message/event body,
formatted content, raw Matrix or MXC identifiers, room/display names, APNs
payloads, device or access tokens, gateway URLs, or raw errors. The diagnostic
buffer is bounded to 256 entries and remains device-local in the App Group.

The in-process lock does not serialize the app and NSE as separate processes.
Concurrent App Group read-modify-write operations can therefore lose an older
entry to last-writer-wins. The recorder is a bounded diagnostic aid, not an
audit log: absence of a stage is not proof that the stage did not occur. A
physical proof must pair it with homeserver/gateway and OS-visible readback.

NSE remains read-only: one event lookup from the shared Matrix store, no sync,
no media fetch, no notification-action mutation, and exactly one completion
handler delivery. Agent actions foreground the application before Core owns an
authenticated write.

## iOS operating path

1. The signed-in app observes notification authorization and asks UIKit to
   register for remote notifications.
2. UIKit returns an APNs device token.
3. `SynaraPushService` reconciles one desired `(authenticated session, token)`
   binding at a time. At session attach, Core creates a dedicated
   `NativeHttpPusherOwner` around that exact Matrix client; the UniFFI
   `HttpPusherOwner` capability retains it and its authenticated device ID
   across later Core account rotation. Owner writes do not accept device
   identity again, so a platform caller cannot register or enumerate another
   device through the capability. App ID remains an explicit argument because
   the shared Core is embedded by platform products with distinct application
   identities; each native bound service fixes its own app ID.
   The typed Core HTTP-pusher path sends `append = false` and `event_id_only`
   to the account's homeserver.
4. The homeserver applies the Core-written push rules and sends sparse event
   metadata to the configured gateway; the gateway sends a mutable alert to
   APNs without message content.
5. The NSE reads the device-local preview preference, shared Keychain session,
   and shared Matrix store. `SynaraNseCore` resolves only the referenced event.
6. When authorized, a decrypted `m.room.message` becomes a bounded local
   title/body preview. Missing session/store/event, undecryptable content,
   disabled previews, cancellation, and deadline expiry retain the nonblank
   generic alert. A missing App Group suite in the extension records the
   distinct `app-group-unavailable` stage before the preference check, so a
   sharing failure is never misread as a user-disabled `preferences-disabled`
   outcome. The completion handler fires once.
7. Foreground presentation is chosen by UIKit; background and terminated
   presentation are chosen by the OS. Tapping routes using sparse room/event
   metadata. Only the foreground application can execute an approval write.

Expected preview matrix:

| Event / preference                    | Foreground                        | Background                                                   | Terminated                                                   |
| ------------------------------------- | --------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------ |
| unencrypted, preview on               | locally resolved bounded preview  | same, subject to iOS Show Previews                           | same, subject to iOS Show Previews                           |
| encrypted and decryptable, preview on | locally decrypted bounded preview | same after first device unlock and shared-store availability | same after first device unlock and shared-store availability |
| encrypted but unavailable, preview on | generic nonblank alert            | generic nonblank alert                                       | generic nonblank alert                                       |
| any event, preview off                | generic nonblank alert            | generic nonblank alert                                       | generic nonblank alert                                       |

### Earliest owner-controlled defect and repair

The previous registration owner launched independent unstructured tasks from
session configuration and APNs-token delivery. Main-actor reentrancy allowed
both tasks to pass `isRegistered == false` and issue duplicate pusher sets.
It retained only the bound token, so a session transition could delete the old
token with the new account's credentials or discard the old binding without
unregistering it. A registration completing after a token/session transition
could also be accepted as current.

The repaired owner uses a single reconciliation task and monotonic revision.
Every successful binding retains both the exact token and a dedicated Core
capability holding the authenticated Matrix client that created it. The
production adapter never resolves pusher writes through the mutable
`SharedCoreProductHost.core` session after bind. Token and account rotation
delete the old binding through its old capability before a new registration is
allowed; a failed old cleanup remains retryable and blocks the new set. Binding
the new account may fail without preventing old-account cleanup. An in-flight
result is committed only if its session and token are still desired; otherwise
it is deleted through its captured capability. Repeated identical
configure/token callbacks are idempotent.

Logout has a stricter destructive boundary. Remote pusher deletion runs before
local Keychain session deletion or Matrix session revocation. It does not
depend on UIKit redelivering the APNs token after process launch: Core first
enumerates the account's pushers, filters by exact Synara `app_id` and Matrix
device ID, and deletes every match without projecting enumerated push keys over
UniFFI. When this process still has the last registered token, Core also accepts
that exact key as a secondary match under the same app ID. This repairs a
homeserver readback whose device display name is empty or rewritten without
broadening cleanup to another application or arbitrary device. A missing token
does not weaken the app+device predicate.
That enumeration is used on every logout, including when an in-process binding
exists, so a same-device stale pusher left by an earlier crash is not skipped.
Exact push-key deletion is reserved for in-process token/account rotation.

Teardown is two-phase. A successful remote cleanup leaves registration gated
and retains the account-bound capability, current session, and token until the
Keychain deletion completes. Only then does the app finalize and discard them.
If Keychain deletion fails, the app remains signed in, cancels the teardown
gate, and reconciles the deleted pusher again (or requests a fresh APNs token
when this process has none). If remote deletion fails, the same authority is
retained, any token delivered during the suspended cleanup is applied, and the
still-signed-in registration reconciles; sign-out fails visibly, so a later
attempt can retry before the Matrix credential is deleted or revoked. After process re-instantiation, the
securely restored Matrix session binds a new exact Core capability; no Matrix
credential, access token, APNs token, or cleanup push key is duplicated into a
new persistence mechanism.

If session configuration retained an authenticated session but its initial
pusher-owner bind failed, logout makes one fresh bind attempt before failing
closed. A successful retry reaches device cleanup; a second bind failure still
blocks credential deletion and preserves the signed-in state for another
attempt.

The teardown gate also spans main-actor suspension: session/configuration,
registration-failure, and reconciliation callbacks cannot create a new pusher
while remote cleanup or the local Keychain handoff is in progress. An APNs
token callback retains only the latest token in process without reconciling;
successful logout discards it, while a failed Keychain handoff applies it
before push registration is restored for the still-signed-in session.

## Desktop operating path

Intended route:

1. Core receives an authoritative timeline event and room notification facts.
2. Core applies the single shared suppress/show/sound/privacy/dedup policy,
   incorporating platform-observed focus without transferring policy ownership.
3. A typed notification decision crosses the native boundary.
4. `desktop_notifications.rs` performs only sanitized OS mapping and delivery.
5. The OS presents the notification and returns an internal route/action.

Current-source result: **steps 1–3 implemented deterministically; steps 4–5
observed live on macOS only behind an injected observation step; the live
route into step 1 is broken at the renderer, and the macOS `failed` receipt is
unreachable** (2026-09-08 run below). PR #1097 registered the account-bound
`NativeNotificationDecisionOwner` with the product Core and routed every
renderer observation through `matrix_notification_decide`. The 2026-09-08
follow-on ([`docs/reviews/2026-09-08-notification-push-rule-owner.md`](../../../reviews/2026-09-08-notification-push-rule-owner.md))
completed step 1: Core now loads the exact observed event through the SDK and
reads its SDK-evaluated push actions, so room mode, account defaults,
intentional and legacy mentions, keywords, room-mention power levels, the
suppress-edits override, and rule ordering have one owner. The renderer sends
identity and product strings only; the wire rejects any renderer-supplied
mode, highlight, sender, or encryption verdict. A restored TypeScript
mute/mention matcher remains a wrong-owner workaround and is locked out by a
source guard. The same record's second pass closes the step-4 receipt: the
renderer awaits the OS answer and acknowledges with a closed `delivered` /
`failed` outcome, Core keeps an identifier-free per-session delivery ledger,
and a failed delivery is counted and released, never retried. Sound follows the
SDK tweak Core echoes. Desktop tray delivery must still be proven separately on
macOS and Linux; the ledger makes a refused delivery observable but is not
that proof.

### 2026-09-08 live macOS run at commit `3ace2188fd6859a4cc6c5456ad956c0c86d96118`

Rig: macOS 26.6.2, `tauri dev` debug binary of the exact commit above,
launched through Launch Services inside a throwaway `/tmp` bundle with the
isolated identifier `com.whylandcreative.synara.desktop.a9qa`, Vite dev server
for the renderer. Receiver: authorized test account signed in through the
normal password form. Sender: a second authorized test account on the same
homeserver posting through the client-server API. Evidence channels:
identifier-free renderer beacons to a localhost listener (decision, outcome,
ledger before/after ack), the `matrix_notification_pending_snapshot` ledger,
and the `usernoted` unified log. Uncommitted local instrumentation only
(beacons, an auto-login helper, an event-injection command, a Keychain
service-name suffix, one `eprintln!` of the OS send result); none of it changes
policy, and none is part of the commit under test. No screenshot or audio
capture was available; "sound" below means the renderer's `playSound()` call
was observed, not that audio was heard.

Earliest divergence, **Failed** at commit `3ace2188` — the product path never
reaches Core for a live message. `MessageNotifications` has two observation
pumps and both are dead on the native client:

- Both pumps return early unless `mx.getSyncState() === 'SYNCING'`. The native
  facade maps `running` readiness to `PREPARED` and has no `SYNCING` state
  (`readinessToSyncState` in `nativeClientFacade.ts`). Live: the focus probe
  read `syncState: "PREPARED"` while signed in and synced, and 2,872
  consecutive scan ticks reported `gatePasses: false`.
- The `Room.timeline` pump has no emitter: no production code emits
  `Room.timeline` on the facade (only test mocks do). Live: 2 listeners, 0
  events.
- The sync-driven scan reads `getLiveTimeline().getEvents()`, which the facade
  room stubs to a constant `[]`. Live: `loadedLiveEvents: 0` across 11 rooms.

Result: a plain group message and a DM sent by the second account produced
zero `decide` calls, zero OS notifications, and a ledger of
`delivered 0 / failed 0 / unreported 0`. The SDK push-rule owner is correct
but is not invoked by the shipped renderer; this is the wrong-owner residue the
review did not catch, and it predates this PR (the pumps were already gated on
`SYNCING`).

Downstream path, proven live by injecting the identity of each real,
already-synced event past the dead gate (the injection replaces only the
observation step; Core still loaded the event itself and evaluated the SDK
push actions):

| Case                                                  | Core decision                           | OS (`usernoted`)                                               | Ledger after ack                        |
| ----------------------------------------------------- | --------------------------------------- | -------------------------------------------------------------- | --------------------------------------- |
| (a) message in the 2-member "group" room, app on Home | `show`, `highlight false`, `sound true` | record delivered to `[.alert .lockScreen .notificationCenter]` | `delivered 1 / failed 0 / unreported 0` |
| (b) DM, app on Home                                   | `show`, `highlight false`, `sound true` | record delivered to `[.alert .lockScreen .notificationCenter]` | `delivered 2 / failed 0 / unreported 0` |
| (c) `m.mentions` of the receiver, app on Home         | `show`, `highlight true`, `sound true`  | record delivered to `[.alert .lockScreen .notificationCenter]` | `delivered 3 / failed 0 / unreported 0` |
| (d) mention while that room was selected and focused  | `suppress`, `reason focused-room`       | no record                                                      | unchanged `3 / 0 / 0`, `pending 0`      |

Verdicts against the requested gates:

- OS notification appears for (a), (b), (c): **Confirmed** (via injection; see
  the caveat below). Ledger `delivery.delivered` advances 1→2→3 with
  `failed 0` and `pending` returning to 0 after each ack: **Confirmed**.
  Focused room suppresses with no OS record and no ledger movement:
  **Confirmed**.
- Sound only for (b) and (c): **Not tested as specified.** Both authorized test
  accounts share only 2-member rooms, so case (a) is evaluated by the SDK as
  `.m.rule.room_one_to_one` and correctly carries `sound true`. A ≥3-member
  room with no mention is required to observe the silent default; no such room
  was available to these accounts.
- Deny notification permission → ack arrives as `failed`, counted once, no
  repeat: **Failed** at commit `3ace2188` (structural). On macOS, when a
  notification carries a `route` or actions, `desktop_notify` spawns
  `show_notification_with_route_click_handler`, which returns `Ok(true)` as
  soon as the send task is spawned; the actual `notification.send()` (with
  `wait_for_click(true)`) runs later on a blocking task and any error is
  dropped by `if let Ok(Ok(response))`. The renderer therefore acknowledges
  `delivered` before the OS has answered. Live ordering proof: all three
  `delivered` acks landed within ~1 s of the decision, while the `eprintln!`
  placed after `notification.send()` had still not fired more than five
  minutes later. A refused OS delivery on this path is recorded as
  `delivered`, never `failed`; the review's "delivery receipt instead of a
  blind acknowledgement" claim holds for the Linux/no-route paths only. System
  Settings was not toggled for this reason: the receipt cannot change with it.
- Caveat on the OS evidence: the unbundled debug binary registered with
  `usernoted` as `com.apple.Terminal` (the `mac-notification-sys` 0.6.15
  fallback identity when the process has no real bundle), which also triggered
  a one-time Terminal permission prompt. The signed product bundle would use
  its own identity; OS presentation was observed, but under the harness's
  identity, not the product's.

Not done and why: no ≥3-member group (accounts); no Linux run (macOS-only
scope); no System Settings deny toggle (moot, see above); no live proof of the
agent-approval kind (out of the requested message cases). No retry was added,
no TypeScript matcher was restored, and no credential or event payload was
written into the repository or this record.

Correction to the identity caveat above: the `com.apple.Terminal` identity is
not a crate fallback. `configure_macos_notification_application` in
`desktop_notifications.rs` deliberately registers debug builds
(`tauri::is_dev()`) as `com.apple.Terminal` and release builds as
`com.whylandcreative.synara.desktop`. Every debug-binary run below therefore
presents under Terminal's identity by product design.

### 2026-09-08 live macOS rerun at commit `466e41e25c593d554c22fd019bbedd543b316c49`

This commit closes both failures recorded above: Core now pushes each live
message-like event to the renderer (`matrix-notification-observed`, from an
SDK event handler held by `NativeNotificationObservationOwner` in the managed
session), the renderer's `SYNCING` gate, `Room.timeline` listener, and
30-second scan are deleted (source-guarded), and the macOS route/action path
awaits Notification Center's `deliveredNotifications` record before returning
so the acknowledgement carries the OS answer.

Rig: same as the `3ace2188` run (macOS 26.6.2, throwaway `/tmp` bundle with
identifier `com.whylandcreative.synara.desktop.a9qa`, Vite dev server, the
same two authorized accounts and the same 2-member rooms). The debug binary is
the exact commit above plus uncommitted local instrumentation only: four
`eprintln!` lines (observation, decision, OS receipt, acknowledgement with
ledger), an env-driven auto-login helper, one harness effect that selects the
proof room for the focused case, and the Keychain service-name suffix. None
of it changes policy; **no event injection was used**. Every `decide` below
was reached through the shipped observation stream.

| Case                                                            | Observation → Core decision                                               | macOS receipt                | Ledger after ack                               |
| --------------------------------------------------------------- | ------------------------------------------------------------------------- | ---------------------------- | ---------------------------------------------- |
| (a) message in the 2-member "group" room, app on Home           | observed once; `show`, `highlight false`, `sound true`                    | `delivered` 64 ms after send | `1 / 0 / 0`                                    |
| (b) DM, app on Home                                             | observed once; `show`, `highlight false`, `sound true`                    | `delivered` 52 ms after send | `2 / 0 / 0`                                    |
| (c) `m.mentions` of the receiver, app on Home                   | observed once; `show`, `highlight true`, `sound true`                     | `delivered` 53 ms after send | `3 / 0 / 0`                                    |
| (d) mention while that room was selected and the window focused | observed once; `suppress`, `reason focused-room`                          | none attempted               | unchanged `3 / 0 / 0`                          |
| (e) DM while a different room was selected and focused          | observed once; `show`, `sound true`                                       | `delivered` 53 ms after send | `4 / 0 / 0`                                    |
| (f) fresh sign-in ≤5 min after (d) and (e) were sent            | each observed twice (initial sliding-sync re-delivery); decided once each | `delivered` 54 ms and 55 ms  | `1 / 0 / 0` → `2 / 0 / 0` (new session ledger) |

`usernoted` logged a `Delivering <NotificationRecord app:"com.apple.Terminal"
…>` record about 50 ms after each `Record` for (a)–(c) and (e), matching the
receipt timings above; the receipt now lands after the OS record exists
rather than at spawn time.

Verdicts against the requested gates at `466e41e`:

- Live observation reaches Core without instrumentation: **Confirmed.** Each
  message from the second account produced exactly one `matrix_notification_decide`
  call in the running app; the renderer no longer scans.
- OS notification appears for (a), (b), (c): **Confirmed** (Terminal identity,
  by design for debug builds). Ledger `delivery.delivered` advances 1→2→3→4
  with `failed 0` and `unreported 0`: **Confirmed.** Focused room suppresses
  with no OS attempt and no ledger movement, while a DM to another room still
  notifies: **Confirmed.**
- Sound only for (b) and (c): **Not tested as specified**, unchanged reason
  (only 2-member rooms are shared by the authorized accounts, so (a) is
  `.m.rule.room_one_to_one` and correctly sounds).
- macOS receipt is real: **Confirmed** for the accepted case. The route path
  now returns only after Notification Center reports the record delivered
  (52–64 ms), and a send error or a 2.5 s silence returns `failed`. The
  deterministic `select_new_delivery` tests cover the refused case.
- Deny notification permission → ack arrives as `failed`, counted once, no
  repeat: **Pending an operator toggle.** The debug identity is
  `com.apple.Terminal`, so the deny switch lives under System Settings →
  Notifications → Terminal and cannot be flipped from the harness without
  editing system notification preferences; that toggle was not performed
  autonomously. The rig is left ready to record the result.

Observed, not in scope, worth knowing:

- On a fresh sign-in, the SDK's initial sliding sync delivered the two most
  recent messages twice to the event handler (once with the narrow first
  timeline, once with the widened one). Core emitted both observations; the
  renderer decided each event once and Core's dedup covers the rest. Because
  the recency window is 5 minutes and the ledger is per session, messages
  received in the last 5 minutes are notified again after a re-login.
- Two concurrent first notifications both called
  `configure_macos_notification_application`; the second logged
  `Application 'com.apple.Terminal' can only be set once` and delivery was
  unaffected.

## Evidence ledger

Executable evidence `E1` (simulator/contract evidence only): on 2026-09-02,
the exact candidate working tree was validated with
`RUN_IOS_TESTS=1 IOS_TEST_DESTINATION='platform=iOS Simulator,id=EAB7A3B4-B57C-4BF1-8CB0-CD7F9753CD7F' DERIVED_DATA_PATH=/private/tmp/synara-a9-resumed-derived IOS_PACKAGE_CACHE_PATH=/private/tmp/synara-a9-resumed-package-cache IOS_RESULT_BUNDLE_DIR=/private/tmp/synara-a9-exact-head-results IOS_RESULT_STAMP=a9-exact-head-20260902-231028 synara-ios/scripts/ci-build.sh`.
The script transactionally regenerated the canonical all-slice `SynaraCore`
and `SynaraNseCore` Apple artifacts, passed `build-for-testing`, and passed the
complete unit and UI run. Console results were 704 unit tests executed with 3
skipped and 0 failed, plus 73 UI tests executed with 14 skipped and 0 failed.
The xcresult summary reports 777 total, 760 passed, 17 skipped, and 0 failed at
`/private/tmp/synara-a9-exact-head-results/test-a9-exact-head-20260902-231028.xcresult`.
The generated room-encryption enum/DTO and forward-confirmation interfaces
compiled through the application and tests. A case-insensitive scan of the
fresh log for background-publishing, `ObservedObject`, main-actor-isolation,
invalid-frame, and non-finite-frame warnings returned no matches. `E1` does
not exercise a homeserver, push gateway, physical APNs delivery, an NSE under
the device deadline, or OS notification presentation.

Executable evidence `E2` (post-review simulator/contract evidence only): on
2026-09-03, the corrected working tree passed the complete iOS unit target
(703 passed, 3 intentionally skipped, 0 failed) and the serial UI target (59
passed, 14 intentionally skipped, 0 failed) after regenerating both four-slice
Apple packages. The added logout-rebind and exact-last-key cases are part of
this run. `E2` does not supersede any live gate: it does not read a homeserver's
pusher fields or exercise a push gateway, physical APNs delivery, an NSE under
the device deadline, or OS notification presentation.

Executable evidence `E3` (simulator/contract evidence only): on 2026-09-08 at
commit `3ace2188fd6859a4cc6c5456ad956c0c86d96118`, the local simulator unit
lane `RUN_IOS_TESTS=1 IOS_TEST_SUITE=unit SYNARA_CORE_APPLE_SLICES=simulator-arm64 scripts/ci-build.sh`
(run from `synara-ios` with only a result-stamp variable added) passed the
scaffold and NSE-isolation checks, regenerated the simulator-slice `SynaraCore`
artifact, and reported `** TEST BUILD SUCCEEDED **` and
`** TEST EXECUTE SUCCEEDED **` with 720 test cases passed and 0 failed on the
`iPhone 17` simulator clone. No intermittent failure reproduced in this single
run; no test named as intermittent could be located in the 2026-09-06 records
in this repository, so there is nothing further to report against that flag.
`E3` does not exercise APNs, an NSE under the device deadline, or OS
presentation.

| Claim                                                    | Evidence required                                                                                                                                                                                                                           | Verdict                                                                                                                                                                                                                                                                 |
| -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| HTTP pusher set is sparse and idempotent                 | Core contract tests assert `event_id_only` and `append = false`                                                                                                                                                                             | passed: focused Core unit plus 6-test HTTP-pusher integration target; not live delivery                                                                                                                                                                                 |
| duplicate callbacks create one desired binding           | focused iOS tests repeat session/token both before and during suspended registration                                                                                                                                                        | passed in `E1` deterministic simulator suite; not physical APNs                                                                                                                                                                                                         |
| token rotation removes old token                         | focused iOS test captures delete/set order and key                                                                                                                                                                                          | passed in `E1` deterministic simulator suite; not live homeserver readback                                                                                                                                                                                              |
| session rotation uses the account-bound Core/client      | Core loopback observes old/new bearer credentials on distinct servers; production-adapter tests retain distinct owner capabilities                                                                                                          | passed deterministically; not live homeserver readback                                                                                                                                                                                                                  |
| tokenless and crash-stale logout cleanup                 | Core loopback enumerates and deletes only exact app+device matches; unit coverage proves an exact last-known key is a secondary same-app match; re-instantiation, bind-retry, and APNs-failure tests exercise logout without broad deletion | passed in `E2` deterministic simulator/Core suites; live pusher-field readback remains **Not confirmed**                                                                                                                                                                |
| logout cleanup remains reachable                         | failed remote cleanup blocks Keychain deletion and succeeds on retry; failed Keychain deletion cancels teardown and restores registration                                                                                                   | passed in `E1` deterministic simulator suite; not live homeserver readback                                                                                                                                                                                              |
| teardown rejects reentrant callbacks                     | delayed cleanup test injects token/configuration/failure callbacks both during remote await and before local finalization                                                                                                                   | passed in `E1` deterministic simulator suite; not physical APNs                                                                                                                                                                                                         |
| stale in-flight set cannot become current                | delayed pusher test changes session while set is suspended                                                                                                                                                                                  | passed in `E1` deterministic simulator suite; not live homeserver readback                                                                                                                                                                                              |
| failed cleanup remains retryable                         | focused iOS test proves an old binding is retained and not overwritten                                                                                                                                                                      | passed in `E1` deterministic simulator suite; not live homeserver readback                                                                                                                                                                                              |
| NSE privacy and exactly-once fallback                    | preview, coordinator, cancellation, timeout, deadline-winner, empty-deadline, and diagnostic allowlist tests; deadline completion occurs before one best-effort batched diagnostic write                                                    | passed in `E1` deterministic simulator suite; physical NSE delivery remains **Not confirmed**                                                                                                                                                                           |
| unencrypted preview, foreground/background/terminated    | physical TestFlight device plus gateway/pusher/NSE stage readback                                                                                                                                                                           | **Not confirmed**                                                                                                                                                                                                                                                       |
| encrypted preview, foreground/background/terminated      | physical TestFlight device with shared store and decryptable event                                                                                                                                                                          | **Not confirmed**                                                                                                                                                                                                                                                       |
| preview disabled retains useful generic alert            | physical TestFlight device                                                                                                                                                                                                                  | **Not confirmed**                                                                                                                                                                                                                                                       |
| token rotation and logout remove live homeserver pushers | disposable account/device plus authenticated pusher readback                                                                                                                                                                                | **Not confirmed**                                                                                                                                                                                                                                                       |
| desktop decision uses SDK push rules as the single owner | `p4_s39_notification_push_rules` mock-homeserver proof: default rules, `m.mentions`, mentions-only room rule, mute override, own-event, dedup, `/event` fallback, fail-closed diagnostics; source guard locks the renderer out of matching  | passed deterministically on 2026-09-08; not live delivery                                                                                                                                                                                                               |
| desktop delivery receipt reaches Core                    | `matrix_notification_dismiss` closed `outcome`; per-session ledger counted once per released candidate; renderer awaits the OS answer and never retries; SDK-path proof for undecryptable encrypted-room events and focus changes           | `delivered` counted live on macOS (1→2→3→4, `failed 0`) at `466e41e` with the receipt landing after the OS record (52–64 ms); refused-delivery `failed` receipt deterministic only, live deny toggle **Pending an operator toggle** (Terminal identity in debug builds) |
| desktop live observation reaches Core                    | a message received by the signed-in native client produces one `matrix_notification_decide` call without instrumentation                                                                                                                    | **Confirmed** at `466e41e`: Core push stream → one `decide` per live message, no renderer scanning (was **Failed** at `3ace2188`)                                                                                                                                       |
| desktop ordinary and approval tray delivery              | product Core decision stream plus macOS and Linux OS readback                                                                                                                                                                               | macOS OS presentation **Confirmed** for DM, mention, and 2-member room through the shipped path at `466e41e` (debug builds present as Terminal by design); silent ≥3-member case, Linux, and approval kind **Not confirmed**                                            |

## Clean rerun protocol

Use a disposable account/device or explicitly authorized test account. Never
send production messages as a proof side effect.

1. Clear local stage diagnostics; record build, OS, preview preference, and app
   lifecycle outside the privacy-safe app log.
2. Authorize notifications and confirm the fixed stages advance through APNs
   token capture and pusher registration success.
3. Read the homeserver pusher list with authorized tooling and confirm exactly
   one current `event_id_only` HTTP pusher. Do not capture its raw push key.
4. Send one controlled unencrypted message and one controlled encrypted
   message for each lifecycle state. Confirm visible title/body behavior and
   correlate NSE stage UUIDs without recording content or Matrix IDs.
5. Disable previews and repeat one case; confirm a nonblank generic alert.
6. Rotate the APNs token if the test rig can do so, then sign out. Confirm all
   Synara pushers for that exact Matrix device are absent by opaque counts only.
   Relaunch once before token delivery and repeat sign-out to exercise the
   tokenless cleanup route.
7. Run desktop proof independently after the Core decision stream exists.

Any path that succeeds only after reopening the app, manually re-registering,
retrying a failed event, or retaining two pushers is not a pass. Record the
earliest divergent fixed stage and keep the live verdict open.
