# Notification previews and critical approval alerts

Review date: 2026-10-04. Branch: `feature/2026-10-04-notification-previews-critical-approvals`.

The reported iOS preview failure is intermittent and remains unresolved. The
user enabled the disabled content preference and saw one successful preview,
then reported generic “New activity” notifications on both Apple Watch and an
unlocked iPhone at approximately 20:09–20:12 on October 4. The screenshot confirms
generic delivered content. The subsequent diagnostic screenshot establishes
two divergences: Core initialization/event fetching at 20:11, and payload parsing
at 20:12. The infrastructure owner's subsequent evidence confirms the 20:12
gateway defect: a counts-only update became a visible alert. The 20:11 push had
valid event identifiers; its underlying Core failure remains unknown. At review start, both
Apple clients already exposed Approve once and Deny, but neither requested
Critical Alert permission nor posted locally verified approvals at Critical level.
Time Sensitive and Critical are different Apple notification capabilities. The
repairs and validation sections describe the resulting branch behavior.

## Intended delivery routes

| Contract                | iOS preview                                                                                                               | iOS approval                                                                                                    | macOS approval                                                                                                   |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Actor and goal          | A Matrix sender causes a useful message preview                                                                           | Hermes causes an urgent, actionable command approval alert                                                      | Hermes causes an urgent, actionable command approval alert                                                       |
| Starting state          | Signed-in account, push registered, preview opt-in enabled, shared session/store accessible                               | Signed-in account, push registered, approval alerts enabled, fresh prompt                                       | Signed-in desktop session, notifications authorized, fresh prompt                                                |
| First action            | Send a message to the account                                                                                             | Send a Hermes approval prompt                                                                                   | Send a Hermes approval prompt                                                                                    |
| Owner route             | Homeserver → event-ID-only gateway → APNs → NSE → narrow Matrix notification client → Notification Center                 | Same route, then shared approval classifier and freshness check                                                 | Matrix SDK candidate → shared notification policy → renderer adapter → native UserNotifications                  |
| Transitions             | Alert received → identifiers parsed → shared session restored → event fetched/decrypted → title/body composed → delivered | Alert received → exact event resolved/classified → freshness checked → category/urgency assigned → delivered    | SDK candidate classified → generation-bound native payload → OS submission accepted → banner/actions             |
| Side effects            | One notification; device-local stage diagnostics; bounded SDK store access                                                | Same; an explicit action can send one Matrix reaction                                                           | One notification; an explicit action can send one Matrix reaction                                                |
| Authority               | Preview opt-in and iOS Show Previews control message disclosure                                                           | Local event classification, user permission, signed capability, and shared-core decision validation             | Shared classification, live account generation, OS authorization, shared-core decision validation                |
| Completion and readback | Physical Lock Screen or Notification Center shows the expected bounded text                                               | Physical OS alert shows expected urgency and actions; action resolves against original event                    | Bundled app's OS alert shows expected urgency/actions; action resolves against original event                    |
| Acceptance              | Useful preview with opt-in; generic content without it; exactly one completion                                            | Fresh verified prompt escalates to Critical when authorized, otherwise Time Sensitive; approve/deny work        | Same urgency behavior on supported macOS, with generation-bound actions                                          |
| Disqualifiers           | Manual app opening needed to obtain preview; APNs carrying private plaintext; simulator treated as physical proof         | Gateway hints grant actions; expired prompt escalates; duplicate decisions; Critical claimed without permission | Renderer-only receipt treated as visible-banner proof; stale account action; legacy subtitle claimed as Critical |

## Findings

1. **Critical support is absent on both Apple clients.** iOS requests only alert,
   badge and sound permission; the NSE sets `.timeSensitive`. Bundled macOS also
   requests basic permission and sets `TimeSensitive`. Signing configuration has
   no critical-alert entitlement. Apple requires a granted entitlement and user
   permission. The native notification owners can implement selection and
   permission handling; signing/release owners must obtain approved profiles.
   The standard iOS entitlement files and macOS bundle configuration also omit
   the Time Sensitive entitlement. This is a separate signing-capability defect;
   setting an interruption level alone is not the current Apple capability
   configuration.
2. **NSE deadline delivery can restore unvalidated gateway presentation.**
   `NotificationService.didReceive` registers original content with the delivery
   coordinator before removing the provisional category. Expiration therefore
   delivers an unsanitized category. The extension owns the fix: register the
   sanitized generic fallback before asynchronous work.
3. **The preview route has external and device boundaries not proven here.**
   The gateway implementation is outside this repo. Apple runs the extension
   only for alert pushes containing `mutable-content: 1`. App Group settings,
   shared Keychain items, the store-ready marker, SDK event policy, decryption,
   memory ceiling, and the 20-second resolver deadline can each prevent previews.
   Existing unit tests and prior host integration evidence do not identify this
   device's intermittent failure. Preview content is intentionally opt-in, default
   off. Commit `3fe40dc8` on August 18 changed the registered default from true to
   false. An unset preference therefore changed behavior; registering defaults
   does not overwrite an explicitly persisted true value. The user's report
   does not establish that a saved opt-in was reset.
4. **Actions already have authoritative validation.** Both platforms map Approve
   once to ✅ and Deny to ❌ through shared Core. Five-minute expiry, same-event
   deduplication, and account binding protect decisions. iOS actions foreground
   the app and require authentication; these are OS notification actions, but
   they are not silent background command execution. Permanent approval remains
   an explicit in-app action.
5. **Platform scope differs.** Linux already uses Critical D-Bus urgency for
   agent approvals, subject to the desktop notification server's behavior. The
   browser's Notification API provides no Apple-style Critical Alert capability.
   Bare macOS development executables use a legacy path; a subtitle there does
   not provide modern interruption-level behavior. Bundled macOS is the release
   path that must be tested.
6. **A store migration conflict can keep iOS previews unavailable.** If both
   legacy and shared stores are populated without the ready marker,
   `SharedCoreProductHost.resolvedLiveStoreRoot` deliberately keeps the legacy
   store. Publishing NSE readiness then returns false, and the extension can
   only see `shared-store-not-ready`. The app can otherwise sync normally. This
   is a concrete conditional failure route, not a confirmed diagnosis of the
   affected device. The storage owner must preserve the active account's crypto
   material when resolving a conflict; selecting or deleting a populated store
   blindly would be an unsafe fix. The captured 20:11 route did not fail at the
   readiness marker; no migration change is justified by these records.

## Evidence and next verification

Runtime verdict: **Not confirmed**. The user's preview path works intermittently
and still has confirmed generic fallback deliveries. The device records show:

| Time (America/New_York) | Correlation prefix | Observed route                                                       | Conclusion                                                                                                                                                                                                   |
| ----------------------- | ------------------ | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 20:08:10                | `34098a66`         | `resolved-preview`                                                   | The installed path can compose a preview.                                                                                                                                                                    |
| 20:11:19–20             | `d8b84883`         | `received` → `resolution-queued` → `core-fetch-failed` → `delivered` | Valid event push accepted by APNs after the 20:09:23 gateway request, delivered about 116 seconds later. Old diagnostics merge client initialization and SDK fetch errors; about one second elapsed locally. |
| 20:12:11                | `1674a278`         | `received` → `payload-invalid` → `delivered`                         | Legitimate counts-only request incorrectly became an alert without room/event IDs; Core never ran.                                                                                                           |

These are distinct failures. They do not establish a global missing extension,
a 20-second timeout, a reset preference, or a decryption failure. The
[Matrix Push Gateway specification](https://spec.matrix.org/latest/push-gateway-api/#homeserver-behaviour)
permits counts-only push requests. The infrastructure owner supplied sanitized
request shapes, accepted APNs responses, deployed-source reconstruction and
matching server timestamps. The captured gateway version unconditionally adds `alert`,
`mutable-content` and, when priority is absent, sound. It also omits `aps.badge`
when the authoritative count is zero, which can leave an old badge uncleared.
These are gateway defects, not preview visibility settings. See the concrete
[gateway repair handoff](../agent-approval-notification-proxy-spec.md#confirmed-counts-only-gateway-repair).

The earlier failed push was an ordinary `m.room.message` / `m.text` event in the
same room as the successful preview, with exact matching room/event IDs and no
edit relation. It used APNs priority 5 and no sound because Matrix priority was
low. APNs accepted it, but the device received it about 116 seconds later. This
does not establish why delivery was delayed or why subsequent SDK resolution
failed. Changing client sound or adding a fetch retry would not explain that
boundary. The gateway repository is on the infrastructure host, outside this
checkout; the maintainer subsequently reports its repair completed. No server
fix, deployment or post-repair traffic verification was performed from this
checkout.
The pinned SDK already attempts a context fallback, so no speculative retry was
added. Some Sliding Sync failures can be masked by a subsequent room-unavailable
result; that diagnostic does not prove missing membership.

The installed macOS client (2.1.43) reported active
notification permission and enabled desktop notifications/sound; diagnostic
capture was off. No Mac settings were changed or test messages sent. Repairs
and local validation are recorded below. No production APNs traffic, agent approvals,
release uploads, or signing changes are performed by this review.

On the affected device, keep the content preference enabled, send a new message through
the normal sender, and inspect Settings → Notifications → Local Delivery
Diagnostics. No `received` entry calls for checking shared diagnostic storage,
extension invocation and packaging before attributing the issue to the push
gateway. A fixed failure stage points to the named session/store/fetch/decryption
boundary. `resolved-preview` followed by delivery points to OS preview visibility.
The current release only displays stage rows: screenshots of the relevant
timestamps are usable evidence. This branch adds Copy Diagnostic Report and
Share Diagnostic Report with app/OS settings, registration and store readiness,
and up to 256 fixed stage codes with opaque correlation IDs. It excludes message
text, Matrix IDs, payloads, tokens, gateway URLs, paths and raw errors.

Then rerun cleartext and encrypted cases, preview on/off, locked/unlocked, and a
fresh approval with approval permission allowed/denied. Check the actual visible
notification and Matrix reaction readback; OS submission acceptance alone does
not prove banner visibility. Physical-device memory and deadline evidence remain
necessary for the NSE release path.

Apple references: [Critical Alert authorization](https://developer.apple.com/documentation/usernotifications/unauthorizationoptions/criticalalert),
[Critical Alerts entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.usernotifications.critical-alerts),
and [notification service extension invocation](https://developer.apple.com/documentation/usernotifications/unnotificationserviceextension).

## Repairs on this branch

- Routing identifiers now use a bounded, deterministic raw-field walker.
  It rejects conflicting, wrongly typed or invalid identifiers rather than
  allowing flattened dictionary enumeration order to choose a route. Foreground
  filtering shares the same parser with strict whitespace handling. Domainless
  [room version 12 IDs](https://spec.matrix.org/latest/rooms/v12/#client-considerations)
  remain supported; legacy domain suffixes are not assumed for all rooms. New fixed
  stages distinguish missing references, malformed fields and resolver queue
  entry from Core work. Typed Core errors distinguish initialization, network,
  authentication, authorization, store/lock, crypto, room/event, response and
  API/version boundaries without exporting raw SDK error text.
- NSE fallback content is sanitized before the coordinator takes its immutable
  deadline snapshot. Gateway approval hints, critical sounds and plaintext never
  survive into the generic fallback. Routing IDs and badge counts are retained.
- The shared Swift policy composes preview text only with opt-in, classifies
  approval presentation only after local resolution, and requires freshness and
  the existing approval-alert preference. Fresh approvals use Critical plus a
  critical sound only when the restricted build capability and OS `criticalAlertSetting` are enabled; otherwise they
  use Time Sensitive. The diagnostic stage `resolved-critical-approval` records
  critical selection without identifiers or message text.
- iOS requests Critical permission only in an explicitly enabled restricted
  build; bundled macOS probes its actual signed entitlement before requesting it. Builds without the capability still request
  ordinary alert, badge and sound permission. Existing users can request the
  additional authorization through notification settings. The iOS button now
  describes that permission request explicitly.
- Bundled macOS uses the same OS authorization boundary for Critical level and
  critical sound. An explicit silent payload never becomes an audible Critical
  alert. Existing session binding, submission receipts and action routing remain
  in place.
- Account-synced [agent notification preferences](../agent-notification-preferences.md)
  apply to explicit agent IDs, using anchored Hermes Tool activity/Commentary
  headers and a final/other category. All defaults preserve notifications;
  verified approval prompts bypass exclusions. Desktop and foreground iOS
  suppress excluded events. Complete background iOS suppression requires
  Apple's approved NSE Filtering entitlement; standard builds use a generic
  fallback for excluded remote events.
- Standard Apple entitlement files now include Time Sensitive. Separate optional
  Critical entitlement files allow release owners to enable the granted
  capability without making unapproved profiles unusable by default.

## Signing and permission follow through

Enable Time Sensitive Notifications for the app and notification-service App IDs,
and regenerate the manually selected iOS distribution profiles before archiving.
Check the final embedded executable entitlements, rather than treating a source
plist as proof that the installed app supports the capability.

After Apple grants Critical Alerts for the intended App IDs and the profiles are
updated, archive iOS with these Xcode build-setting overrides:

```text
SYNARA_CRITICAL_ALERTS_ENABLED=YES
SYNARA_APP_ENTITLEMENTS=Synara/Resources/SynaraCriticalAlerts.entitlements
SYNARA_NSE_ENTITLEMENTS=SynaraNotificationService/Resources/SynaraNotificationServiceCriticalAlerts.entitlements
```

For approved NSE Filtering profiles, also set
`SYNARA_NOTIFICATION_FILTERING_ENABLED=YES` and select
`SynaraNotificationServiceFiltering.entitlements` or the combined
`SynaraNotificationServiceCriticalAlertsFiltering.entitlements` as
`SYNARA_NSE_ENTITLEMENTS`. Both restricted flags default to NO. The release archive
checker rejects enabled flags without their final signed entitlements, mismatched
app/extension flags, and unresolved placeholders.

For macOS, select `src-tauri/Entitlements.critical-alerts.plist` using Tauri's
`bundle.macOS.entitlements` configuration override (path relative to `src-tauri`).
The normal bundle uses `Entitlements.plist`. Retain any other entitlements needed
by the release configuration when combining files. Verify the signed bundle,
request permission, and inspect OS Critical Alert authorization before testing
with Focus/mute enabled. Entitlement templates and local tests do not establish
Apple approval or production Critical Alert delivery.

Apple now deprecates the Time Sensitive authorization option in favor of the
[Time Sensitive entitlement](https://developer.apple.com/documentation/usernotifications/unauthorizationoptions/timesensitive).

## Local validation

- Core and NSE compilation, 10 SDK-backed notification/account-data integration
  tests, six preference/FFI tests, one NSE policy test and one static diagnostic
  privacy test passed. Generated Swift signatures match the iOS consumers.
- `cargo check --locked -p synara --lib` passed on macOS, including the full
  desktop library and actual UserNotifications APIs.
- 36 existing and new Swift tests passed using the production shared helpers and
  XCTest sources in a temporary macOS Swift package. These cover composition,
  preferences, freshness, critical authorization selection, sanitized deadline
  delivery, cancellation, exactly-once completion, foreground account guards,
  restricted suppression, and bounded diagnostic export privacy. This is
  shared-helper validation, not a full iOS application test run.
- A further focused Swift regression passed for silent event payloads with
  badge zero: fallback, ordinary preview and expired approval preserve both
  silence and the zero badge. Together these cover 37 shared Swift tests.
- The two new native macOS policy tests passed in an isolated crate containing
  the exact production policy functions and test bodies with the repository's
  `objc2-user-notifications` version. No OS notifications were posted.
- The complete iOS application and notification-service Swift sources
  typechecked against the iOS 16 Simulator SDK using freshly generated
  project-owned Core/NSE UniFFI modules. Extension API restrictions were enabled
  for the NSE check. This verifies Swift API integration without linking Rust
  libraries. Entitlement plists and documentation links validated;
  `git diff --check` passed.
- Seven archive-capability fixture tests passed, including absent or wrongly
  typed entitlements, combined Critical/Filtering profiles and
  unresolved/mismatched flags. These test archive validation, not Apple approval.
- Full Xcode package resolution could not proceed because this clean checkout
  has no generated `SynaraCore.xcframework` or `SynaraNseCore.xcframework`.
  Regenerate the Apple artifacts before a full app/NSE build and signed device
  test. No release archive or physical-device result is claimed.

Clean end-to-end rerun verdict remains **Not confirmed**. The new tests preserve
the local authorization/privacy/completion boundaries; they do not replace
physical APNs delivery, preview visibility, or approval reaction readback.

## macOS live message overlap

The user also supplied a screenshot of overlapping Hermes Tool activity,
Commentary, metadata and code blocks. The installed client uses Page Zoom 100%,
Normal message spacing and Bright message text. No appearance setting was changed.
The overlap was no longer present in the currently visible production messages,
so the persistent screenshot state was not directly replayed there.

The harness reproduces stale row offsets immediately after growing formatted
message edits commit: stable keyed rows retain their previous measured height
until ResizeObserver runs. Large ordered lists and code blocks can therefore
paint across adjacent rows. The repair remeasures the bounded mounted rows in a
layout effect through the existing virtualizer adapter and again when scrolling
becomes idle. It retains the existing measurement cache and scroll-anchor owner,
and avoids direct lifecycle resize calls that bypass the adapter's commit guard.

Four new Chromium/WebKit edit-layout tests passed, including edits above a parked
viewport and commit-warning checks. Seventeen existing Chromium navigation,
scroll and jank cases passed; observed p95 frame gaps were approximately
16.7–16.8 ms. Full frontend typechecking, lint, formatting and quality gates
passed. CI now runs the narrow WebKit regressions. These checks prove the harness
repair; the user's installed-client case still needs a rebuilt native client and
live edited-feed smoke test.

## Direct Messages unread indicator

The user reported Home's unread number working while Direct Messages showed no
equivalent indicator. Both rail components already contained badge markup.
The defect was in their inputs: a separately refreshed `m.direct` room-ID
projection and effect-mirrored unread map could disagree with the native room
summaries used for current per-room unread state. An actual rail-component
harness reproduced a missing Direct Messages badge when a new native DM unread
snapshot arrived while Home was selected and the `m.direct` projection lagged.
The opposite Home unread case passed before the repair.

The intended route is native room-list revision → joined Home/DM scope → shared
mute, unread and mention policy → rail count/dot → matching destination list.
Explicit Mark as Read must use that same scope, and a native read update must
clear the badge. Route selection must not control whether another area's
unread indicator renders. Sign-out must clear the projection. The client repair
uses the native `isDirect` classification and current unread data for these
navigation scopes; it does not introduce another unread store or room owner.
Home/DM list hooks, header and rail Mark as Read actions, and generic room/thread
navigation now share that classification. Space navigation retains precedence.

Four scope tests, ten Chromium/WebKit rail/navigation regressions and six
existing Chromium room-list/avatar/live-call regressions passed.
Coverage includes new DMs while Home is selected, the reverse direction,
aggregate updates and clearing, muted rooms, mention styling, marked unread,
membership and space boundaries, lagging `m.direct`, matching destination room
IDs, scoped Mark as Read, and generic room/thread links. Frontend typechecking,
scoped lint/format checks, repository quality gates and `git diff --check`
passed. Independent review found and verified the generic-route alignment;
no actionable source issues remained. CI includes the narrow WebKit cases.

Harness proof and installed-client proof remain distinct: a native rebuild and
live incoming DM are required to confirm the reported release behavior.

## Release route

The maintainer authorized verification, CI, merging and tagging a new release
on October 4. The release goal is v2.1.44 containing this reviewed feature, using
the [repository release runbook](../build-and-release.md).

- Actor and starting state: the release agent, with explicit maintainer
  authorization; uncommitted feature changes based on main `74c34538`, latest
  published version v2.1.43. The maintainer reports the external gateway repair
  deployed; physical preview verification remains open.
- First action and owners: complete local validation, commit and push the
  feature PR; GitHub CI owns the acceptance gate. Merge the green feature PR,
  then create `release/v2.1.44` from main with version/build and notes updates.
- Transitions: reviewed feature commit → green feature PR → merged main →
  versioned release PR → full green Quality gate, including iOS unit/UI suites →
  merged main → matching tag → Release workflow → published client artifacts.
- Side effects: Git commits, remote branches, PRs, merges, a public release tag,
  signed desktop artifacts, updater/repository metadata and internal TestFlight
  delivery. Production publication stays owned by the tag-triggered workflow.
- Authority: the user's release instruction authorizes this sequence. Restricted
  Critical/Filtering flags remain disabled unless approved profiles and
  entitlements are independently available; release authorization does not
  establish Apple capability approval.
- Completion and authoritative readback: merged feature/release PRs and their
  actual checks; the version-matching tag reachable from main; the Release run,
  published artifact inventory/updater version and exact TestFlight promotion
  state where available.
- Acceptance and disqualifiers: required CI must pass on the candidate source,
  and the tag must target merged main with consistent version metadata. Do not
  bypass failed checks, tag the feature branch, manually replace production
  assets or claim physical notification proof from simulator/harness results.

Execution is pending at this packet's creation. Signing, publication and device
notification results must be read back separately before claiming those outcomes.

Release preflight passed the production runtime build and 1,208 modernization
tests on pinned Node 24.13.1; 410 release-tooling tests; full frontend
typechecking, lint and formatting; both Rust all-target Clippy gates with
warnings denied; shared/desktop checks; 1,119 Core unit tests (four ignored),
ten notification integration tests and two NSE wrapper tests. The full desktop
and shared integration suites remain required in GitHub CI because the local
host has insufficient disk for all linked test artifacts. Archive fixture
metadata and the command registry expectation were updated to represent the
new capability flags and preference commands; production validation was retained.

Signing preflight found that the active July iOS profiles lacked Time Sensitive
support. It also proved a macOS launch failure with that entitlement but no
profile: the same small executable launched without the entitlement and was
killed with it, under both ad-hoc and the existing Developer ID signing identity.
The standard capability was enabled in the Apple Developer portal for the iOS
app, notification service and exact desktop App ID. Release signing must use
fresh profiles, and macOS must embed its matching Developer ID profile before
signing. Critical and Filtering remain separate, disabled restricted opt-ins.

Release preflight completed: 422 root tooling tests pass, including 12 Developer ID profile fixtures. Fresh iOS app, NSE, and desktop profiles were decoded and checked against exact bundle IDs, existing signing certificates, Time Sensitive authorization, and absence of restricted Critical/Filtering claims before updating the matching GitHub secrets. An embedded-profile Developer ID scratch app passes signature validation and launches successfully. Full release package notarization and physical-device delivery remain downstream evidence.

CI correction evidence: the WebKit mutation-history fixture sampled a 62-update edit during active scrolling before ResizeObserver consumed its new height. Separate idle-commit and matching-ResizeObserver frame cases retain strict geometry and parked-anchor assertions; removing only the production remeasurement effect makes the idle case fail. Restored Chromium/WebKit coverage passes six cases. NSE read-only integration fixtures now assert the specific static room-unavailable cause and real typed mapper wiring while retaining no-owner, no-sync, and secret-isolation checks. Developer ID final signer validation also binds the extracted leaf to the profile certificate list, covering multiple same-name keychain certificates.
