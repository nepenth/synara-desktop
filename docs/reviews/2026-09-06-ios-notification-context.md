# iOS notification destination and preview operating paths

## Intended paths (before repair)

| Field | Notification destination | Local notification preview |
| --- | --- | --- |
| Goal | Open the actual room at the notified event with available surrounding history and the authoritative room name | Show a decrypted preview for an eligible new event only when the user enabled previews |
| Actor | Signed-in iOS reader | iOS notification service extension |
| Start | Valid room/event notification; app warm or restored from a cold launch | Valid room/event payload, shared session and protected store, explicit preview preference |
| First action | Tap the notification | iOS invokes the extension's didReceive |
| Owner route | App delegate → push route → AppRouter → RoomTimelineView → Core focused timeline; room-list projection supplies metadata | Extension preference gate → narrow NsePreviewRequest → SDK notification client using shared store → platform preview rendering |
| Transitions | Pending route → signed-in route → focused timeline; loading room metadata → resolved room title without replacing timeline identity | Received → preference checked → existing shared secrets/store → bounded event fetch/decryption → one delivered result |
| Side effects | Focused timeline session/subscription and normal viewport observations; no synthetic room or direct read write | Network event resolution and SDK crypto-store coordination; no new login, full sync owner, or message content sent to APNs |
| Authority | User authorized test accounts and simulator; Core owns Matrix identity/history | User preview preference, Apple Keychain/App Group, Core crypto owner |
| Completion | Named real room, exact focused event visible, surrounding messages available | Preview body delivered once with preference enabled; generic original notification with preference disabled |
| Readback | Actual app UI plus Core focused snapshot and test-server event IDs | Extension output, fixed diagnostic stages, narrow Core return value |
| Acceptance | Cold/warm route preserves room ID, event ID and focused session while room projection loads | Enabled/disabled cases behave distinctly; new encrypted event decrypts within the narrow request deadline |
| Disqualifiers | Generic title retained after room name becomes available; standalone synthetic one-event timeline; manual reopen to repair route | Personal preference changes, bypassing encryption/trust, full sync in extension, raw identifiers/secrets/content in diagnostics, treating simulator delivery as physical APNs proof |

## Initial evidence and ownership

ADR 0002/0003/0004 retain navigation, rendering and Apple services in Swift and
Matrix history/crypto in shared Core. Notification routes carry room/event IDs
without a room name. The room screen currently renders the immutable optional
route title, falling back to `Room`, and never consumes the room-list owner's
subsequent name projection. Core focused opening already requests event context;
the separate history workstream owns sparse history and pagination behavior.

Preview defaults are explicitly opt-in. The extension already uses the narrow
20-second Core notification request with shared secrets and a shared store.
Absent a live trace, a preview failure is not evidence of a verification defect.

Runtime verdict before testing: **Not confirmed**. The user's reported destination
path is **Failed** because it requires manual navigation to reach the actual room.

## Repair

The room screen now observes the existing room-list projection for its current
account and room, and uses that authoritative title in the header, navigation,
details and thread destinations. The room/event identity and timeline task ID do
not depend on this title, so late metadata cannot reset the focused target.

The notification owner now distinguishes policy-filtered, redacted, unavailable
and still-encrypted results with static reason codes. The extension maps only
allowlisted codes to fixed device-local diagnostic stages; unknown codes remain
generic. This is an evidence-boundary repair, not a decryption or trust bypass.
The original notification fallback and both explicit preview/approval opt-ins are
unchanged. A filtered result is not represented as a missing store entry, but
this does not claim iOS notification suppression without Apple's filtering
entitlement.

## Proof entrypoints and limits

- `synara-ios/scripts/run-live-notification-tap.py` prepares a private room and
  three fixture messages using dedicated test accounts, then runs the gated
  `testLiveNotificationTapContextWhenConfigured` on an explicitly selected
  simulator. The app logs in normally; the runner injects actual simulator remote
  notifications after warm/cold readiness markers. XCTest taps the SpringBoard
  notification and checks the room name, target and neighboring rows. Fixture
  API sessions leave the room and log out in teardown.
- The ignored `synara-nse-core` integration test
  `new_encrypted_event_resolves_after_parent_core_stops` uses normal Core login,
  sync and encrypted-room creation for fixture preparation, stops the reader's
  full Core before sending the new encrypted message, then invokes the actual
  narrow `NsePreviewRequest.resolve`. It verifies the returned plaintext against
  known fixture content and that the read-only secret vault was not mutated.
  Full Core support is enabled only as a dev-dependency for this fixture; the
  shipping extension dependency retains `default-features = false`.
- Existing preference tests cover default-off and explicit opt-in. The simulator
  push payload carries generic text and routing IDs, never a decrypted message
  body. Simulator notification delivery does not execute or prove physical APNs
  transport, NSE process memory/deadline behavior, or locked-device Keychain
  access. Those require device evidence.

## Cold-launch divergence discovered by the real tap proof

The first complete tap attempt on the title repair confirmed warm routing to the
named room, exact event and both neighbors. Its cold phase failed: the visible
notification opened Synara's room list and no notified timeline appeared. The
local notification recorder contained `response-received` for the warm tap but
none for the cold tap. This is independent of encryption: the fixture room was
private and unencrypted.

The earliest owner-controlled divergence was notification delegate installation.
It occurred in `bind(to:)`, invoked from SwiftUI's appearance callback. Apple
requires the notification center delegate to be assigned before application
launch finishes. The delegate is now installed in
`application(_:didFinishLaunchingWithOptions:)`; the existing pending-response
queue handles the interval before SwiftUI binds product services. No second
notification route, replay polling or Matrix engine was added. [Apple's delegate
contract](https://developer.apple.com/documentation/usernotifications/unusernotificationcenter/delegate).

The fixture originally had two preparation defects, recorded separately from
product proof: opening a settings URL relaunched with the test reset flag; and
SpringBoard reported its visible banner container as non-hittable. Preparation
now uses the real Settings tab and taps the observed banner frame. The final
readback identifies `TimelineRoomTitle` specifically so a room-list row cannot
satisfy the title assertion.

After early delegate registration, a second clean run delivered the cold
`response-received` callback and opened the correctly named room. It then failed
with `Could Not Load Timeline` / `Sign in again to load this timeline.` The
focused timeline request raced the root shell's Core preparation. The room screen
now waits on the same `SignedInSessionReadiness` owner already used by the room
list before opening timeline, typing and crypto observers. It checks cancellation
and the captured account/route after that wait. The root shell still owns Core
startup; the screen does not start a second engine or poll/retry failed opens.

## Final observed results

| Scope | Verdict | Evidence |
| --- | --- | --- |
| Real simulator warm and cold notification taps → named room, exact event, surrounding messages | **Confirmed** | Final signed `testLiveNotificationTapContextWhenConfigured` passed in 50.312 seconds. Two actual remote notifications were injected and tapped; both target rows were hittable, neighboring rows existed, and the header matched the authoritative room name. Fixture cleanup completed. |
| New encrypted event → narrow NSE Core request with parent stopped | **Confirmed** | `new_encrypted_event_resolves_after_parent_core_stops` passed on the configured test homeserver. Narrow resolution returned the expected plaintext in 10.762 seconds; full test took 37.39 seconds. Encrypted room projection, unchanged vault, fixture device revocation, room leave and store deletion assertions passed. |
| Core diagnostic distinctions and pre-secret validation | **Confirmed** | Three focused `app::nse_preview::tests` passed, including distinct policy-filtered/redacted/missing reasons and secret-store access bounds. |
| Swift route, startup, projection, preview preference and binding boundaries | **Confirmed** | 92 focused signed iOS unit tests passed: AppRoute (19), NotificationPreviewSupport (13), RoomListService (51), SessionCoordinator (7), and narrow NSE binding cancellation (2). Startup tests include superseded-account and cancelled preparation behavior. |
| Physical APNs/NSE process, locked-device Keychain, user's missing-preview frequency | **Not confirmed** | These routes were not exercised by simulator injection or host Core integration. Existing preview opt-in remains unchanged. The new fixed diagnostic reasons support a device-side follow-up without collecting message content. |

Local execution records are retained in private Xcode result bundles and private
runner logs. The live fixture credential values, actual room/event identifiers,
private homeserver and decrypted fixture output are intentionally absent from
this document. No physical-device or production-account proof is claimed.

Fresh review tightened the fixture runner's completion boundary: a successful
XCTest exit now requires both phase confirmation markers and successful cleanup
of every API fixture session. Teardown attempts continue after an individual
leave/logout failure, and any cleanup failure prevents a zero exit. Four mocked
runner regressions passed, including successful UI plus failed logout, skipped
XCTest, missing cold readback, and full success. This reporting-only correction
preserves the prior live run, whose two readbacks and complete cleanup were
independently observed; no new production or notification routing behavior changed.

### Grok diagnostic mapping correction

Grok found that the Core NSE vault adapter converts vault failures to
`nse-secret-vault-unavailable` before returning across FFI. The diagnostic
allowlist previously recognized only the originating Swift vault code and
therefore recorded the generic failure stage for this Core error. The fixed
allowlist now maps the actual arriving Core code to `core-store-unavailable`,
while preserving the existing Swift code mapping. Added a focused regression
for the Core adapter code; arbitrary error descriptions still map to the fixed
generic stage. No preview content, crypto ownership, privacy or notification
routing behavior changes. Focused signed native validation is pending behind
the active live-read run.
