# iOS E2EE Validation

Reviewed: 2026-05-27

Status: Phase 3 boundary validated. Production E2EE is not complete in the
current REST-backed iOS MVP and must be delivered through the Matrix Rust SDK
crypto path before TestFlight or App Store release.

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

## Required Production Work

1. Integrate Matrix Rust SDK crypto into the app-facing Matrix client service.
2. Store SDK crypto state in SDK-approved secure storage.
3. Add login/session restore that initializes the crypto machine before sync.
4. Validate encrypted room sync with a disposable encrypted test room.
5. Add recovery, verification, and key-backup UX or explicitly block release
   until a conservative minimum is implemented.
6. Re-run the live-smoke flow in encrypted and unencrypted rooms.

## Acceptance Gate

Phase 3 is complete for unencrypted live messaging and safe encrypted-event
fallback. External TestFlight and App Store release remain blocked until
encrypted rooms can send, receive, decrypt, and recover according to the
production E2EE requirements above.
