# iOS verification presentation and completion

## Intended path

- Goal: complete an iOS verification comparison and dismiss its result once.
- Actor: the signed-in iOS user, accepting an incoming request or initiating
  verification from Security / a session row.
- Starting state: a live native session and an eligible peer; no acknowledged
  result should be presented again.
- First action: accept the request or request verification in the existing UI.
- Owner route: iOS presentation → typed SharedCore commands → Matrix SDK;
  native inbox observations drive the displayed flow.
- Transitions: request → comparison → confirmation → terminal result → user
  acknowledgement → closed. A subsequent distinct flow remains presentable.
- Side effects: existing verification messages and trust updates, then removal
  of exactly the acknowledged terminal inbox flow. UI tests use fixtures.
- Authority: the SDK owns protocol completion and device trust; iOS owns sheet
  presentation. A completed SAS exchange alone does not establish own-device
  cross-signing trust.
- Completion: Done or swipe acknowledges the displayed terminal flow and it
  remains closed through subsequent native updates.
- Readback: native inbox flow identity and session generation; own-device
  verification from the same device snapshot used by Security settings.
- Acceptance: compact sheets, consistent accessible actions, all seven ordered
  emojis, clear outcome, no result replay, no dismissal of another flow, and
  no unverified device labelled verified.
- Disqualifiers: automatic timer dismissal, reopened acknowledged results,
  swallowed dismissal failures, stale callbacks targeting another flow,
  clipping at large text sizes, or success inferred only from a heading.

## Initial evidence

The supplied screenshots show medium-height request/result sheets and a forced
large comparison sheet. The implementation expands a ScrollView between the
header and actions. The presenter sleeps for 1.5 seconds on terminal updates,
then dismisses, while also polling every 0.5 seconds and accepting stream
updates. This buffers repeated terminal observations and loses flow identity
for every phase except the incoming request. Dismissal clears the selected
flow even on an error. The completed heading claims own-device verification
from protocol `done` alone.

The reported first attempt failed the intended presentation path. Whether that
particular iOS device acquired trust is not confirmed by the screenshots.

## Repair

The presenter now receives the native flow ID and session generation throughout
its lifecycle. One stream includes authoritative removal; the timer and second
poller are removed. Done and swipe acknowledge the displayed terminal ID once,
retain dismissal errors, and reject buffered updates for acknowledged results.
A newer flow is not cleared by completion of an earlier acknowledgement.

Sheets measure their content height, scroll when the available screen is too
small, and share full-width primary/secondary actions. The emoji layout uses
four columns at ordinary text sizes and two at accessibility sizes. The result
observes own-device status from the same SDK device snapshot used in Security;
protocol completion alone displays “Verification complete,” not “Device verified.”

## Validation

Swift app, extension, and both test targets compile. Matrix boundary, generated
binding scaffold, documentation, version, and whitespace checks pass. Focused
regressions cover replayed completion, exact-flow acknowledgement, dismissal
failure/retry, concurrent dismissal, new requests, and session lifecycle changes.
UI cases exercise compact layout, Done, swipe, unverified completion, and large text.

The clean CI unit run on iPhone 17 / iOS 26.5 passed: 732 tests passed, three
skipped, and zero failed. All six new `CryptoVerificationPresentationTests`
passed. Evidence: [CI run 34544827922](https://github.com/nepenth/synara-desktop/actions/runs/34544827922),
`ios-test-results/test-20260911-000422-6154.xcresult`.

Local runtime execution was blocked before assertions: a fresh iOS 26.5
simulator stalled in CoreLocation migration; an isolated clone of an initialized
QA device booted but application launch/uninstall calls then hung in CoreSimulator.
No live account verification or trust change was performed for this UI work.
The clean macOS CI UI job also failed before assertions. Xcode reported
`Failed to send signal 19 to process ...: 3` while launching Synara, then
stalled until the 75-minute job timeout. Its test result bundle is incomplete
(no `Info.plist`), and it contains no completed UI test report or screenshot
evidence. The UI job was cancelled by timeout and the Quality gate failed.

The tested implementation is commit `27b6e021`. Presentation state transitions
are confirmed by unit tests. Actual sheet sizing, large-text scrolling, and
Done/swipe interactions remain **not confirmed** in runtime. The PR stays draft
pending execution on a functioning simulator or device; no merge or release
was performed. This does not establish the trust state of the user's historical
first attempt.

Temporary local DerivedData, failed result bundles, generated frameworks, and
the isolated QA simulator were removed after handing validation to CI. Other
active builds and the original QA simulator were left intact.
