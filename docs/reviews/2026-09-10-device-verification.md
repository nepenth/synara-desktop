# Device verification investigation and proof

## Intended operating path

Goal: verify the existing Synara device with an already verified Element session
of the same account and retain that trust after process restart. Actor: the
signed-in user. Start: an unverified Synara session, a verified online Element
session, and the original encrypted Synara store. First action: Devices → Verify
from Another Device. Route: React → Tauri/Core → NativeVerificationOwner →
matrix-sdk 0.18.0 → homeserver → Element. States: request, acceptance, SAS
negotiation, matching displayed codes, explicit confirmation on both sides,
completion, SDK current-device trust, restart/readback. Side effects: public
device-key publication, verification to-device events, signatures and SDK secret
sharing; existing stores and identity remain intact. Authentication is owned by
the SDK/native vault, cryptographic state by the SDK, and match/mismatch by the
user. User explicitly authorized this account/client proof and dedicated test
accounts. Completion requires the current device to be verified in both clients
and still verified after restoring the exact persisted session. Authoritative
readback: homeserver device keys plus SDK verification state and Element's
session details. No retry, store reset, cross-signing reset, manual trust flag,
direct-peer substitute, or restart before completion qualifies as a clean run.

## Baseline observations

- Installed Synara 2.1.32 restored the user's session and offered verification.
  Element showed its own session as verified and Synara macOS as unverified.
- The normal Synara action remained at “Waiting for another device to accept”.
  Element's crypto log received that request and rejected it because it could
  not retrieve the requesting device's data. This run failed before SAS.
- An independent authenticated `/keys/query` read returned the three Element
  devices, but neither the macOS nor Linux Synara device. The account's public
  master and self-signing identities exist. The disposable diagnostic session
  was revoked immediately. No keys or credentials were printed or retained.
- Synara deliberately retains the encrypted crypto store after remote logout,
  then reuses its stored device ID during password login. SDK Account's
  `keys_for_upload` includes device identity keys only while `shared == false`.
  Thus an account that previously uploaded its device keys can retain that flag
  after the homeserver removes the device at logout. A live regression using
  the production encrypted client builder and SyncService confirmed this: fresh
  device publication succeeded; real logout, reopen, and password login with
  the retained device ID left device keys absent for the full 20-second check.
- Existing live SAS tests create fresh initiators, explicitly wait/query both
  participants' keys, and force additional identity queries while waiting for
  final trust. They do not exercise logout/relogin into a retained crypto store
  and do not prove that the product itself performs every needed query.

## SDK references

The exact resolved 0.18.0 SDK source is the implementation authority:
`matrix-sdk/src/authentication/matrix/login_builder.rs` (device ID reuse requires
the matching keys), `matrix-sdk-crypto/src/olm/account.rs` (`shared` gates device
key publication), `matrix-sdk/src/encryption/identities/users.rs` (own-identity
verification), and `matrix-sdk/src/encryption/mod.rs` (key query and verification
state updates). Upstream documentation:
[encryption](https://docs.rs/matrix-sdk/0.18.0/matrix_sdk/encryption/index.html),
[verification](https://docs.rs/matrix-sdk/0.18.0/matrix_sdk/encryption/verification/index.html).

## Ownership and SDK comparison

| Concern | Synara path | SDK behavior and finding |
| --- | --- | --- |
| Login and restoration | Shared auth/lifecycle; native vault holds session material; encrypted SQLite crypto store survives logout | Reusing a device ID requires the matching stored private keys. Synara retains those keys correctly, but remote logout removes the public device registration. |
| Key publication | Product SyncService drives SDK outgoing crypto traffic | SDK 0.18 persists the account's `shared` flag. Retaining that account does not make it upload device identity keys a second time. An entry in `/devices` does not establish a usable E2EE device in `/keys/query`. |
| Request initiation | Both desktop and iOS call shared `NativeVerificationOwner::start`; Devices uses own-identity verification | SDK own-identity verification is the correct route for trusting the current device. A direct peer-verification substitute would not prove this device's cross-signing state. |
| Incoming request and SAS | Native owner retains SDK request/SAS handles and watches their streams; incoming registration runs outside sync dispatch | The receiver must first resolve the sender's public crypto device. Element rejected the baseline before Synara's SAS handling could matter. |
| Human confirmation | React displays SDK emoji/decimals and calls native match/mismatch actions | SAS negotiation and confirmation stay SDK-owned. No automatic confirmation or manual trust flag is added. |
| Trust and recovery | Devices reads SDK verification state, crypto-device trust, cross-signing/secret-storage status, and backup status | SAS completion and current-device trust are distinct evidence. Final proof must inspect authoritative trust and persisted restoration, not just the modal's Done phase. |

The production client builder enables SDK encryption and its OneShot backup
settings. Device, verification, cross-signing, secret-storage, and backup owners
all share the same native client. The repair belongs in the shared native core,
so desktop and iOS use the same precondition without shell-specific crypto code.

## Repair

Before sending an interactive verification request:

1. Read the SDK's own signed public device-key object and check it belongs to
   the authenticated user and device.
2. Query the homeserver for that exact device. Treat failed/incomplete responses,
   malformed or invalidly signed objects, and conflicting identity keys as
   errors. A lookup failure is not evidence that keys are missing.
3. If absent, upload the **existing** SDK-signed public device object with the
   public Matrix `/keys/upload` request through `Client::send`.
4. Read back and validate the same public identity keys before proceeding.
5. Refresh the user's identity through the SDK's `request_user_identity`, then
   use `UserIdentity::request_verification_with_methods([SasV1])`.

The pinned SDK exposes the signed public object and Matrix request transport,
but has no public force-republish method for an already-shared account. This
small reconciliation does not edit the crypto database, mutate the SDK's shared
flag, generate replacement keys, or submit one-time/fallback keys. The SDK
continues to own those queues, signatures, trust, and secret sharing. The check
also repairs already-restored installations when the user invokes verification;
a new login or store reset is unnecessary.

## Status updates after verification

The first repaired desktop handshake also exposed a separate presentation gap:
verification became successful, but the recovery/backup panels still displayed
their earlier locked/disconnected state. After reopening the original session,
they read Ready and Connected from the SDK. Their React hooks had only listened
to renderer-initiated operations, and the shared native owner only observed
crypto-device-list changes. SDK secret sharing can finish after those events.

`NativeDeviceOwner` now merges the supported SDK device, current-device trust,
backup, and recovery streams into its existing generation-only invalidation
signal. Desktop device, cross-signing, secret-storage, and backup hooks subscribe
to that signal and re-read their normal native status commands. iOS already
consumes this shared device signal for session status. No second crypto state
cache, timer, or inferred ready flag is introduced.

## Validation

- Baseline live retained-store regression: **failed as expected** after successful
  fresh publication, real logout, store reopen, device-ID reuse, and sync.
- Updated live regression: **passed**. The actual native verification action
  republishes the retained device's keys. Disposable test sessions are revoked
  and their encrypted stores removed after revocation.
- HTTP/SDK regression tests: passed. They check original signed public JSON,
  readback ordering, no repeated upload for existing keys, failed/incomplete
  queries, malformed objects, mismatched identities, different keys for the same
  device ID, invalid signatures, failed uploads, and missing readback. All error
  paths stop before sending verification.
- Native status-stream regression: passed. Creating a backup through the real
  SDK against a mock homeserver emits completion without a crypto-device-list
  change; dropping the owner releases its subscriptions.
- Frontend subscription regression: passed, including callbacks queued during
  asynchronous registration and cleanup after the view is retired.
- Shared-core library suite: **896 passed, 4 ignored** (live tests are opt-in).
- Complete frontend modernization suite: **966 passed**. Full TypeScript check,
  Matrix boundary checks, SDK inventory, documentation hygiene, runtime asset
  consistency, and whitespace checks passed.
- Live own-device authority proof on the dedicated secondary test account:
  **passed**, including matching SAS, authoritative current-device trust, and
  rebuilding/restoring the encrypted store. Removed the test's receiver-side
  device pre-query and its polling `request_user_identity` after confirmation;
  the SDK and product now have to converge without those test-only helpers.
  The account had no public cross-signing identity before the fixture's normal
  password-authorized bootstrap. Its encrypted authority fixture is retained
  locally; the disposable initiator was revoked and removed.

### Desktop and Element proof

On September 10, 2026, the locally built and project-signed desktop restored the
same existing macOS session and original crypto store used for the failed
baseline. It still showed Unverified before the action.

The first repaired **Devices → Verify from Another Device** attempt reached
Element immediately. Element displayed Synara macOS with the same device ID as
the baseline. After Start Verification, both clients displayed the same seven
emoji in the same order. Explicit match confirmation on both sides completed
SAS. Element displayed Device verified, and Synara's separate Devices snapshot
changed to Verified while that process was still running.

After a normal process quit/relaunch, Synara restored the same session and showed
**Verified**, recovery secrets **Ready**, and backup **Connected** (existing
backup version 2). No retry, store deletion, cross-signing reset, recovery-key
entry, or replacement personal device was needed to achieve device trust.
The status-panel subscription change was then validated separately as described
above. The final rebuilt desktop again restored Verified / Ready / Connected,
and Element’s Sessions list independently showed that same Synara macOS device
as Verified after the final restart.

### Local build details and limits

The local proof uses bundled production UI routing, the project's existing
Developer ID signing identity, and the original macOS data location. With the
standard Rust 1.93 toolchain, the unoptimized desktop build command is:

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_TARGET_DIR="$PWD/target" \
  cargo build --locked --manifest-path src-tauri/Cargo.toml \
  --config 'profile.dev.package.synara.debug-assertions=false' \
  --config 'profile.dev.package.matrix-sdk-store-encryption.opt-level=3'
```

Run `npm run build:runtime` first. Package/sign the executable normally before
using the existing macOS Keychain session. Debug shell routing otherwise selects
the development URL; ad-hoc signing differs from the installed application's
Keychain identity. Neither build detail requires changing verification code.

The live UI proof is macOS Synara ↔ Element Desktop. The repair and SDK status
subscriptions are shared with iOS, but an iOS UI proof and a Linux desktop UI
proof were not performed in this change. The Linux Synara device observed in the
baseline will reconcile its own keys when verification is started in a build
containing this repair; this macOS run does not modify another device's keys.
