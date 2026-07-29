# V-CRYPTO.2 — native cross-signing readiness and setup

| Field                 | Value                                                                                 |
| --------------------- | ------------------------------------------------------------------------------------- |
| Status                | **Product wiring merged #224; physical deletion open (V-CRYPTO.2-D)**                 |
| Scope                 | Cross-signing publication, local readiness, own-identity status, and first-time setup |
| Product path          | UI → Tauri IPC → managed `matrix_sdk::Client`                                         |
| Follow-up crypto rows | V-CRYPTO.3–V-CRYPTO.7 remain open                                                     |

## Product ownership

Native desktop sessions now source the device-settings cross-signing gate from
the managed Matrix Rust SDK session. The native path projects:

- whether the master, self-signing, and user-signing public identity is
  published;
- whether the private cross-signing identity in the host crypto store is
  missing, partial, or complete;
- whether first-time bootstrap is needed;
- whether the user's own identity is missing, unverified, or verified; and
- whether the next product action is setup, recovery, identity verification,
  or no action.

When no public identity exists, the Devices settings screen offers the complete
first-time setup flow. The host creates and uploads the cross-signing identity
with `matrix-sdk`. It automatically completes a dummy UIA flow or asks for the
account password when password UIA is required by the homeserver. The UIA
session stays in the managed Rust session. The password is a one-way command
input, is never returned or logged, and is zeroized after the SDK call.

An existing published identity is not reset merely because this device lacks
its private signing material. That state is reported as recovery required,
preserving the existing identity for verification or the V-CRYPTO.4 secret
storage recovery path.

For native sessions, `useCrossSigning` and the device-settings setup gate do not
call matrix-js-sdk `CryptoApi`, `bootstrapCrossSigning`, or `initRustCrypto`.
Missing IPC, unavailable SDK crypto, unsupported UIA, and failed status queries
fail closed with fixed product errors; none starts JS crypto.

## Rust host and IPC

The registered and permissioned commands are:

- `matrix_cross_signing_status`
- `matrix_cross_signing_setup`
- `matrix_cross_signing_setup_password`

Generated desktop and macOS schemas contain the same command permissions.
Status and setup responses contain only session generation and enum/status
strings for publication, bootstrap need, private-identity readiness,
own-identity verification, readiness, and setup outcome. They never contain
access or refresh tokens, public key values,
private cross-signing keys, recovery material, secret-storage material,
ciphertext, UIA session values, or raw SDK errors.

Cross-signing private keys are generated, retained, and read by matrix-sdk
inside the encrypted native crypto store. The store encryption key remains in
the OS credential store through the existing native client/store foundation.

## V-CRYPTO.2 product-wiring criteria met

- Native-session status comes from the live managed Rust client.
- Fresh cross-signing bootstrap completes through Rust, including password or
  dummy UIA supported by the currently managed native password sessions.
- The native device-settings setup gate has a working setup and authentication
  flow rather than a permanent unavailable message.
- Existing published identities are preserved and projected honestly when
  local private material needs recovery.
- Native status/setup paths call no matrix-js-sdk `CryptoApi` and do not start
  JS crypto.
- Commands, invoke registration, permissions, and generated schemas agree.
- IPC responses and errors contain no secret, key, token, recovery, or
  ciphertext material.
- Pure Rust projection/auth-selection tests and TypeScript ownership helper
  tests cover the scoped behavior.
- Rust formatting, tests/check, and touched TypeScript formatting/typecheck
  pass.

## Blocking deletion residual — V-CRYPTO.2-D

Delete the superseded matrix-js-sdk cross-signing status/setup implementation,
`CryptoApi` ownership, and native-vs-legacy branches. Keep shared setup UI only
after it consumes native DTOs/actions. Record the deleted-file/import delta
before marking V-CRYPTO.2 done.

## Remaining named crypto residuals

Closing the deletion residual for this row does **not** close the V-CRYPTO vertical:

- **V-CRYPTO.3** — key-backup setup, restore, repair, and recovery UI.
- **V-CRYPTO.4** — full secret-storage bootstrap and unlock UX, including
  recovery of an existing cross-signing identity on a new device.
- **V-CRYPTO.5** — interactive key-share and retained room-key flows.
- **V-CRYPTO.6** — undecryptable-history recovery and retry UX.
- **V-CRYPTO.7** — full native device-list and trust presentation ownership.

V-CRYPTO.3 and V-CRYPTO.4 are not hidden inside this row. First-time
cross-signing setup completes without exposing recovery material, while backup
and secret-storage product ownership remain their named full verticals.
