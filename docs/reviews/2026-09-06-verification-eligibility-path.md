# Verification eligibility investigation

## Intended operating path

Goal: a signed-in client opening verification settings receives the SDK's actual
current-device trust and eligible-peer result, with an unavailable lookup kept
separate from a successfully checked account with no eligible peer.

Actor: an authorized dedicated test account through the production SharedCore
session interface. Start: fresh encrypted store and in-memory platform vault.
First action: normal password login; attach the product owners and start sync;
read the device snapshot once. Close the Core session without revoking it,
restore the persisted account into a new Core, attach/start, and read once again.

The owner route is platform session bridge → SharedCore → NativeDeviceOwner →
Matrix SDK crypto store and authenticated homeserver devices endpoint. Meaningful
states are authenticated, owners attached, syncing, snapshot returned, closed,
restored, and snapshot returned. Authoritative readback is the typed snapshot,
including own-device verification, eligibility, and the current-device row.

Side effects are a disposable test-account session, its encrypted local SDK
store, and normal SDK sync/key traffic. The proof must not bootstrap or replace
cross-signing, trust devices, initiate SAS, or change room content. Cleanup revokes
only its newly created session and removes only its temporary directory. The
user authorized dedicated test accounts; personal accounts remain outside scope.

Completion requires a single successful snapshot on both fresh and restored
paths, with one current device, known eligibility, and no fabricated verified
status. Retry, trust reset, server bypass, or local fallback substituted for a
successful authority query disqualifies clean proof. Healthy API probes alone
cannot establish this Core/session route.

## Initial evidence

The pinned SDK's eligibility operation first ensures the own-key query and then
uses its crypto device set. Core concurrently queries the homeserver session
list. A server-list failure rejects the snapshot even if eligibility succeeded.
An unavailable crypto machine/store can produce unknown eligibility. The current
Swift session-status projection uses `try?`; the desktop settings projection also
omits the query error. Consequently their connection/retry wording does not
establish that the connection is the failed boundary.

The reported path is Failed: the user cannot discover eligible verification
sessions. The underlying cause on the affected installed clients remains Not
confirmed; no trust reset or extra fallback is justified by the message alone.

## Live Core proof

The ignored `live_device_eligibility` integration test exercised the production
SharedCore password-login and persisted-session restore interfaces against two
authorized dedicated accounts. Each proof creates an encrypted temporary store,
uses normal attached Core owners and sync, and revokes only the device it created.
No personal-account store or cross-signing authority was opened or changed.

| Account condition | Fresh snapshot | Restored snapshot | Result |
| --- | --- | --- | --- |
| Existing eligible cross-signed peer | unverified, eligible=true, one current device | same | Confirmed, 17.43 seconds |
| No eligible cross-signed peer | unverified, eligible=false, one current device | same | Confirmed, 16.29 seconds |

Both queries reported `sessions=available` and `crypto=available`. Their authority
queries returned `eligible` or `none` directly; neither depended on the existing
local fallback, a retry, trust bootstrap, or SAS. Session revocation and temporary
store removal succeeded before the assertions completed. This confirms the
fresh/restored Core route for these accounts, not the affected user's persisted
device state or physical-device presentation.

## Confirmed observation defect and repair

The iOS settings observer refreshed from `verificationUpdates()`. That stream
only yields a Core owner wakeup if a verification inbox request exists. Ordinary
device/key-query updates therefore never refreshed settings when the inbox was
empty. An initially unavailable eligibility result could remain on screen after
the authority became available.

Settings now consumes the existing `sessionDeviceUpdates()` stream. That stream
forwards device and verification owner signals without requiring a SAS flow.
The subscription is created before the initial snapshot, and cancellation stops
further reads. Core continues to own every eligibility/trust decision. No new
poller, retry, fallback, or synthetic verified state was added.

Desktop already listens directly to both owner families and window-focus
refreshes; it does not share this specific dropped-update defect. Its persistent
error remains unreproduced and is not claimed fixed here.

## Bounded diagnostics

`SYNARA_VERIFICATION_DIAGNOSTICS=1` additionally records the authority-query,
homeserver-session-list, and crypto-store outcomes at the owning snapshot
boundary. Categories are source constants; no SDK Display/Debug text, account,
device, room, URL, or credential is emitted. This separates transport, server
response, crypto readiness/store, timeout, and healthy no-peer outcomes without
changing snapshot behavior. The missing affected-device evidence makes this
bounded observation necessary; it is not an alternative verification mechanism.

## Validation

- CI-aligned Rust 1.93 toolchain (1.93.1): production lifecycle proof, two
  dedicated accounts, fresh and restored paths as above.
- Core diagnostic categorization/privacy unit: passed.
- Native observer regression with the original subscription: Failed as expected;
  after a device-authority update with no SAS flow, eligibility stayed nil and
  the verification action stayed disabled (two failed assertions, one test).
- Repaired observer: 25 MatrixLifecycleTests passed, including that regression.
- Existing encrypted Settings UI route: passed (one test, 23.281 seconds).
- Final combined native rerun after synchronous subscription ordering: passed,
  25 unit tests and one encrypted Settings UI test, zero failures.

The first local build selected 1.93.0 instead of CI's 1.93.1 and was stopped
before proof. A diagnostic helper type mismatch and local Xcode artifact-path
setup were corrected during harness preparation. None was a product-route proof.

Run the opt-in proof only with explicitly authorized dedicated test credentials:

```sh
SYNARA_VERIFICATION_DIAGNOSTICS=1 cargo +1.93 test -p synara-core \
  --test live_device_eligibility -- --ignored --nocapture
```

Supply `SYNARA_LIVE_HOMESERVER`, `SYNARA_LIVE_USERNAME`, and
`SYNARA_LIVE_PASSWORD` through a protected environment, never command-line literals
or committed files. The test changes only its newly created session and encrypted
temporary store. A failing cleanup must be treated as a failed proof, not ignored.
