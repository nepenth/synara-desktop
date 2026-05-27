# iOS E2EE Validation

Reviewed: 2026-05-27

Status: Phase 3 REST boundary validated, and Matrix Rust SDK live E2EE probe
validated against a disposable encrypted room. Production E2EE is not complete
in the current REST-backed iOS MVP and must be delivered by moving app-facing
Matrix services onto the Matrix Rust SDK crypto path before TestFlight or App
Store release.

## Current Behavior

- `m.room.encrypted` events are detected by the timeline mapper.
- Encrypted events render as a safe unavailable-message placeholder.
- Reply, edit, redact, and reaction actions are disabled for encrypted
  placeholders so the app does not pretend to act on undecrypted content.
- Unit tests cover encrypted placeholder mapping and action availability.

## Not Yet Supported

- Megolm decryption.
- Encrypted message sending.
- Key backup restore.
- Device verification and cross-signing.
- Recovery from undecryptable history.
- Encrypted media decryption.

These unsupported items describe the shipping app surface. The SDK probe below
proves the pinned Matrix Rust SDK Swift package can perform the underlying
login, room encryption inspection, encrypted timeline, and encrypted send work
outside the app service layer.

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

1. Add Matrix Rust SDK Swift package integration to the app target.
2. Introduce app-owned `SynaraMatrix` protocols for auth, session restore, room
   list, timeline, media, and crypto status.
3. Move login/session restore from REST services to SDK-backed services.
4. Store SDK session and crypto state in SDK-approved storage plus Keychain for
   app-owned session metadata.
5. Initialize SDK crypto before sync and before encrypted room timelines load.
6. Replace encrypted placeholders with decrypted SDK timeline content when
   available, while preserving safe placeholders for UTD states.
7. Send encrypted text messages through SDK timeline APIs.
8. Validate encrypted room sync and send through a gated simulator smoke using a
   disposable encrypted test room.
9. Add recovery, verification, and key-backup UX or explicitly block release
   until a conservative minimum is implemented.
10. Add encrypted media decryption/download before claiming media parity in
    encrypted rooms.
11. Re-run live-smoke flows in encrypted and unencrypted rooms.

## Acceptance Gate

Phase 3 is complete for unencrypted live messaging, safe encrypted-event
fallback, and SDK-level E2EE feasibility. Phase 4 can proceed for local
development, but external TestFlight and App Store release remain blocked until
the app itself can send, receive, decrypt, and recover encrypted room content
according to the production E2EE requirements above.
