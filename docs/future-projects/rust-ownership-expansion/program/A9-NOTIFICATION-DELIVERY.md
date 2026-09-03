# A9 notification-delivery operating-path record

Status: deterministic iOS registration repair implemented; physical APNs/NSE
and desktop tray delivery are not confirmed.

Last updated: 2026-09-02 on `feature/rust-ownership-follow-ons`.

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
   generic alert. The completion handler fires once.
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
device ID, and deletes every match without projecting push keys over UniFFI.
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

Current-source result: **Failed at step 1/2**. Product desktop
`NativeClientEmitter` emits `sync` and `session`, not `Room.timeline`, while
`MessageNotifications` and the immediate approval path listen for
`Room.timeline`. The existing `NotificationIndex` is a test harness and is not
registered with the product Core. A room-list polling delta or restored
TypeScript mute/mention matcher would be a wrong-owner workaround and is not an
acceptable repair. The route stays open until Core can emit a complete typed
decision from authoritative event, notification-mode/highlight, dedup, and
platform-focus inputs. Desktop tray delivery must then be proven separately on
macOS and Linux.

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

| Claim                                                    | Evidence required                                                                                                                                                                        | Verdict                                                                                       |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| HTTP pusher set is sparse and idempotent                 | Core contract tests assert `event_id_only` and `append = false`                                                                                                                          | passed: focused Core unit plus 6-test HTTP-pusher integration target; not live delivery       |
| duplicate callbacks create one desired binding           | focused iOS tests repeat session/token both before and during suspended registration                                                                                                     | passed in `E1` deterministic simulator suite; not physical APNs                               |
| token rotation removes old token                         | focused iOS test captures delete/set order and key                                                                                                                                       | passed in `E1` deterministic simulator suite; not live homeserver readback                    |
| session rotation uses the account-bound Core/client      | Core loopback observes old/new bearer credentials on distinct servers; production-adapter tests retain distinct owner capabilities                                                       | passed deterministically; not live homeserver readback                                        |
| tokenless and crash-stale logout cleanup                 | Core loopback enumerates and deletes only exact app+device matches and proves repeated empty cleanup is idempotent; re-instantiation and APNs-failure tests log out without a token      | Core 6-test integration target and `E1` passed; not live homeserver readback                  |
| logout cleanup remains reachable                         | failed remote cleanup blocks Keychain deletion and succeeds on retry; failed Keychain deletion cancels teardown and restores registration                                                | passed in `E1` deterministic simulator suite; not live homeserver readback                    |
| teardown rejects reentrant callbacks                     | delayed cleanup test injects token/configuration/failure callbacks both during remote await and before local finalization                                                                | passed in `E1` deterministic simulator suite; not physical APNs                               |
| stale in-flight set cannot become current                | delayed pusher test changes session while set is suspended                                                                                                                               | passed in `E1` deterministic simulator suite; not live homeserver readback                    |
| failed cleanup remains retryable                         | focused iOS test proves an old binding is retained and not overwritten                                                                                                                   | passed in `E1` deterministic simulator suite; not live homeserver readback                    |
| NSE privacy and exactly-once fallback                    | preview, coordinator, cancellation, timeout, deadline-winner, empty-deadline, and diagnostic allowlist tests; deadline completion occurs before one best-effort batched diagnostic write | passed in `E1` deterministic simulator suite; physical NSE delivery remains **Not confirmed** |
| unencrypted preview, foreground/background/terminated    | physical TestFlight device plus gateway/pusher/NSE stage readback                                                                                                                        | **Not confirmed**                                                                             |
| encrypted preview, foreground/background/terminated      | physical TestFlight device with shared store and decryptable event                                                                                                                       | **Not confirmed**                                                                             |
| preview disabled retains useful generic alert            | physical TestFlight device                                                                                                                                                               | **Not confirmed**                                                                             |
| token rotation and logout remove live homeserver pushers | disposable account/device plus authenticated pusher readback                                                                                                                             | **Not confirmed**                                                                             |
| desktop ordinary and approval tray delivery              | product Core decision stream plus macOS and Linux OS readback                                                                                                                            | **Failed at source; not implemented**                                                         |

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
