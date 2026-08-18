# iOS E2EE Validation

Reviewed: 2026-08-17

Status: Matrix Rust SDK live E2EE probe and app-level live encrypted-room UI
smoke validated against a disposable encrypted room. The iOS app target now
builds with SDK-backed auth, session restore, room list, timeline, send, crypto
status, recovery, and verification-request service adapters. Encrypted media is
explicitly detected and safely blocked until decrypted media bytes are
available. Production E2EE is still not release-complete until full
recovery/verification flows, key backup restore UX, encrypted media decryption,
and broader encrypted-room regression coverage are completed.

## Current Behavior

- App login/session restore, room list, timeline pagination, and message send
  are wired through the Matrix Rust SDK service boundary.
- Decrypted SDK timeline content renders normally when keys are available.
- `m.room.encrypted` and UTD states are still mapped defensively when keys are
  unavailable.
- Encrypted events render as a safe unavailable-message placeholder when the SDK
  cannot decrypt or the app does not yet support that event surface.
- Reply, edit, redact, and reaction actions are disabled for encrypted
  placeholders so the app does not pretend to act on undecrypted content.
- Timeline headers and Settings now surface room/session crypto state:
  encrypted, unverified, key backup unavailable, recovery needed, and
  unable-to-decrypt counts.
- Settings exposes SDK-backed device verification request and recovery-key
  submission actions. Recovery keys are used only for the request and are not
  stored by Synara.
- Unit/UI tests cover encrypted placeholder mapping, crypto-status
  presentation, recovery-state decisions, recovery controls, and action
  availability.
- Encrypted Matrix media events that expose `content.file.url` are mapped to
  encrypted media placeholders. Thumbnail/download loading and timeline actions
  are blocked with a recovery-oriented safe message until media decryption is
  implemented.

## Not Yet Supported

- Complete key backup restore UX.
- Complete device verification and cross-signing UX.
- Complete recovery from undecryptable history.
- Encrypted media decryption and viewer/download support.

These unsupported items describe the remaining release blockers. The SDK probe
below proves the pinned Matrix Rust SDK Swift package can perform the underlying
login, room encryption inspection, encrypted timeline, and encrypted send work.

## Matrix Rust SDK Live Probe

Date: 2026-05-27.

Probe: `synara-ios/spikes/matrix-sdk-probe`.

Environment contract:

```sh
SYNARA_MATRIX_PROBE=live-e2ee
SYNARA_E2EE_HOMESERVER=<test homeserver>
SYNARA_E2EE_USERNAME=<test username>
SYNARA_E2EE_PASSWORD=<test password>
SYNARA_E2EE_ROOM=<encrypted room id, alias, or display name>
SYNARA_E2EE_SEND=1
```

Validation result:

- Password login succeeded against the disposable test homeserver account.
- Matrix Rust SDK E2EE initialization completed.
- The encrypted test room was discovered from the account's joined room list.
- Room encryption reported `current=encrypted`, `latest=encrypted`, and
  `isEncrypted=true`.
- Timeline pagination observed 21 events.
- Encrypted send path accepted a probe message.
- Timeline listener reported `unableToDecrypt=0`.
- Unable-to-decrypt delegate reported `0` callbacks.

The test room reference supplied as a Matrix alias was not public, so the probe
also matches joined or invited rooms by room id, canonical alias, alternative
alias, and display name/local alias. This reflects the likely production need to
handle encrypted private rooms that do not publish room aliases.

## App-Level Live E2EE Smoke

Date: 2026-05-27.

Gated XCTest:
`SynaraUITests/SynaraUITests/testLiveEncryptedRoomSmokeWhenConfigured`.

Environment contract:

```sh
SYNARA_LIVE_E2EE_SMOKE=1
SYNARA_LIVE_HOMESERVER=<test homeserver>
SYNARA_LIVE_USERNAME=<test username>
SYNARA_LIVE_PASSWORD=<test password>
SYNARA_LIVE_E2EE_ROOM_ID=<encrypted room id>
```

Validation result:

- App launched with a clean local session and SDK store.
- Password login succeeded through the app UI.
- The encrypted room opened directly through the live route hook.
- Composer became available in the encrypted room.
- Room header surfaced an app-visible crypto status.
- Sending from the composer created a visible encrypted-room message.
- Relaunch without resetting local state restored session/crypto state and
  re-opened the room without re-login.
- The sent message rendered after relaunch.
- No visible undecrypted placeholder appeared on the smoke path.

### 2026-08-17 Revalidation

The signed app-level encrypted-room test passed again as part of the
seven-scenario consolidated live simulator suite. It covered password login,
encrypted room open, encrypted send, app termination/relaunch, secure session
and crypto-store restore, sent-message visibility after relaunch, and no visible
unexpected unable-to-decrypt placeholder. The run used disposable credentials
provided out of band, disabled XCTest retries, and did not write credentials or
tokens to the repository or result summary.

## Required Production Work

1. Matrix Rust SDK Swift package integration to the app target. Done.
2. Introduce app-owned `SynaraMatrix` protocols for auth, session restore, room
   list, timeline, media, and crypto status.
3. Move login/session restore from REST services to SDK-backed services. Done
   for auth/session metadata.
4. Store SDK session and crypto state in SDK-approved storage plus Keychain for
   app-owned session metadata. Done for login/session; logout now clears SDK
   persisted stores.
5. Initialize SDK crypto before sync and before encrypted room timelines load.
   Done in the SDK client store.
6. Replace encrypted placeholders with decrypted SDK timeline content when
   available, while preserving safe placeholders for UTD states.
7. Send encrypted text messages through SDK timeline APIs. Implemented in the
   SDK-backed message sender; app-level live validation now passes.
8. Validate encrypted room sync and send through a gated simulator smoke using a
   disposable encrypted test room. Done for open, send, relaunch restore, and
   visible no-UTD smoke path.
9. Add recovery, verification, and key-backup UX or explicitly block release
   until a conservative minimum is implemented. First conservative UI is in
   place; full verification/recovery flows remain release blockers.
10. Add encrypted media decryption/download before claiming media parity in
    encrypted rooms. Encrypted media is now detected and safely blocked; actual
    decryption remains the release blocker.
11. Re-run live-smoke flows in encrypted and unencrypted rooms.

## Acceptance Gate

The first Phase 7 production E2EE slice is validated: app-level encrypted room
open, send, relaunch restore, crypto status UI, conservative recovery UI, and
safe UTD handling are implemented and tested. External TestFlight and App Store
release remain blocked until complete recovery, verification/cross-signing, key
backup restore, encrypted media, and broader encrypted-room regression
requirements are closed.
