# iOS E2EE Validation

Reviewed: 2026-05-27

Status: Matrix Rust SDK live E2EE probe validated against a disposable
encrypted room, and the iOS app target now builds with SDK-backed auth, session
restore, room list, timeline, and send service adapters. Production E2EE is
still not release-complete until app-level live encrypted-room validation,
recovery, verification, key backup, and encrypted media are completed.

## Current Behavior

- App login/session restore, room list, timeline pagination, and message send
  are wired through the Matrix Rust SDK service boundary.
- `m.room.encrypted` and UTD states are still mapped defensively.
- Encrypted events render as a safe unavailable-message placeholder when the SDK
  cannot decrypt or the app does not yet support that event surface.
- Reply, edit, redact, and reaction actions are disabled for encrypted
  placeholders so the app does not pretend to act on undecrypted content.
- Unit tests cover encrypted placeholder mapping and action availability.

## Not Yet Supported

- Key backup restore.
- Device verification and cross-signing.
- Recovery from undecryptable history.
- Encrypted media decryption.
- Gated app-level live simulator E2EE smoke is not yet passing because room-list
  selection/routing does not reach the timeline reliably under UI automation.

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
   SDK-backed message sender; app-level live validation is still pending.
8. Validate encrypted room sync and send through a gated simulator smoke using a
   disposable encrypted test room. Blocked on the UI routing issue above.
9. Add recovery, verification, and key-backup UX or explicitly block release
   until a conservative minimum is implemented.
10. Add encrypted media decryption/download before claiming media parity in
    encrypted rooms.
11. Re-run live-smoke flows in encrypted and unencrypted rooms.

## Acceptance Gate

Phase 3 plus the SDK-backed app integration build gate are complete enough to
continue Phase 4 local work. External TestFlight and App Store release remain
blocked until the app-level encrypted-room live smoke passes and recovery,
verification, key backup, and encrypted media requirements are closed.
