# iOS live automatic read acknowledgement

## Intended path

A dedicated Matrix test-account reader starts in the signed simulator app with a
private disposable room containing an unread message from the second test account.
The reader opens that room normally and reaches its visible live tail. UIKit
viewport geometry → RoomTimelineView acknowledgement policy → account-bound Core
read-state command → Matrix SDK receipt owner → homeserver receipt and room-list
projection must advance to the visible event and clear unread state. A subsequent
incoming message while the reader remains at the live edge must follow the same
route. Background or offscreen messages must remain unread until viewed.

The first action under test is opening the room through the app UI. Test-account
API clients may prepare a private room and messages and observe server state;
they must not send receipts or mark the room read on behalf of the app. The user
explicitly authorizes simulator and test-account use. Side effects are confined
to dedicated test logins/devices, a new disposable room, fixture messages,
receipts from the app, and cleanup of those test resources. Credentials and raw
private logs remain outside version control.

Completion requires independent server readback and a cleared app room row.
Manual Mark read, relaunch/reopen, or a receipt issued by the harness disqualifies
the normal-open proof. Each case begins with a new fixture event and records its
own result. A repaired clean rerun is distinct from the baseline failure.

## Evidence

Reported path: **Failed** on iOS after the prior release, according to the user.
Login/logout now works. The reproduced baseline source is
`eebd869de0427450473488a0f07a2bc179fe93f6`, branch `ios-live-read-ack-fix`.

## Live execution and repair

The simulator is a dedicated iPhone 17 running iOS 26.5. Xcode signed the
simulator build with the configured team and simulated application-group and
Keychain entitlements. The test logged in through the app UI using the configured
test account; a second account sent fixture messages in a new private,
unencrypted room. This isolates read-state behavior from decryption.
Animations remained enabled. No receipt was submitted by the harness.

The initial probe needed test-bundle metadata for signing and an accessibility
selector correction: message text is combined into an event row. These were
harness preparation issues, not product fixes or evidence of read-state success.

The released-code baseline then passed initial-room and active-live receipt
checks but **Failed** after backgrounding and returning: the new event was
visible and hittable, yet `m.fully_read` remained at the preceding event after
20 seconds. The background negative assertion had passed.

The earliest divergence was in `RoomTimelineView` lifecycle handling. Inactive
transitions canceled the queued acknowledgement; becoming active did not rearm
it. The viewport had already reported the new tail while backgrounded and had no
new geometry change to report on activation. A lifecycle-only diagnostic also
showed why reading the environment again inside the existing single-value
`onChange` closure was insufficient: the callback delivered `active`, UIKit
reported active, but the captured environment still reported `inactive`.

The view now stores the delivered scene activity in its read lifecycle state,
initializes it on appearance, closes it on disappearance, and reconsiders the
observed tail on activation. Both scheduling and delayed execution use that
current state. The existing visible-tail, application-state, interaction,
privacy, generation and Core compare-and-target checks remain authoritative.
Temporary diagnostic logging was removed before the final clean run.

## Clean proof verdict

**Confirmed**, scoped to the signed simulator and configured live test server.
The final 2.1.28 probe completed in 40.091 seconds with no failures:

- Normal room opening acknowledged the visible initial event.
- A subsequent incoming event while the room stayed open was acknowledged.
- An event received in the background stayed unread until foreground return,
  then was acknowledged without a gesture, reopen or manual Mark read.
- The server's fully-read marker and private receipt matched the visible event,
  its notification count was zero, and the app room row showed no unread count.
- A new event received while the room list was open stayed unread; opening that
  room acknowledged it through the normal app route.

The harness left its disposable room and logged out its API fixture devices.
The simulator retains only its dedicated test-account app session. Local logs
and result bundles are excluded from version control and contain no production
account activity. Physical-device behavior and other read-state scenarios are
not established by this scoped proof.

The gated live test is the preservation mechanism earned by this clean run. It
uses real app UI and server readback and skips unless both test accounts are
explicitly configured. CI and publication evidence will be linked from the PR.
