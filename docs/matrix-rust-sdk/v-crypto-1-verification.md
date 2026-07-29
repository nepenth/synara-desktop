# V-CRYPTO.1 — native device verification product ownership

| Field                 | Value                                                                 |
| --------------------- | --------------------------------------------------------------------- |
| Status                | **Product wiring merged #223; physical deletion open (V-CRYPTO.1-D)** |
| Scope                 | Device-verification request, SAS comparison, and completion UX        |
| Product path          | UI → Tauri IPC → managed `matrix_sdk::Client`                         |
| Follow-up crypto rows | V-CRYPTO.2–V-CRYPTO.7 remain open                                     |

## Product ownership

Native desktop sessions now use the Matrix Rust SDK for the complete interactive
device-verification flow:

- receive and retain incoming self-verification requests from live sync;
- list the session-generation-stamped verification inbox;
- request verification from all eligible own devices or one selected own device;
- accept and cancel requests;
- negotiate and start `m.sas.v1`;
- present emoji and decimal short authentication strings;
- confirm matching codes or report a mismatch;
- observe confirmed, completed, and cancelled outcomes; and
- dismiss terminal requests from the product inbox.

The app-level incoming request renderer, current-device verification action,
other-device verification action, verification status checks, and cross-signing
readiness display select this native owner for a native session. The legacy
matrix-js-sdk components remain reachable only for a legacy web/non-native
session. Under the clarified full-vertical policy, that retained implementation
is a blocking deletion residual rather than an accepted steady state.

Native client bootstrap explicitly skips `MatrixClient.initRustCrypto()` and
does not install the matrix-js-sdk verification inbox. It first restores or
confirms the managed Rust session. Failure to reach the native commands is a
closed failure with fixed product copy; it does not start JS crypto.

## Rust host and IPC

The host retains all `VerificationRequest` and `SasVerification` handles in a
per-session owner registered before native sync starts. The following Tauri
commands are registered, permissioned, and present in generated schemas:

- `matrix_verification_list`
- `matrix_verification_start`
- `matrix_verification_accept`
- `matrix_verification_begin_sas`
- `matrix_verification_confirm`
- `matrix_verification_mismatch`
- `matrix_verification_cancel`
- `matrix_verification_dismiss`
- `matrix_device_verification_status`

IPC returns only product DTOs: flow ID, other user/device ID, incoming/outgoing
direction, projected phase, start timestamp, and the emoji or decimal values
that users must compare. Command errors use fixed messages and diagnostic IDs.
Tokens, device or cross-signing keys, MAC data, recovery material, ciphertext,
and raw SDK error text never cross IPC and are not logged.

## Cross-signing dependency

Interactive verification itself is fully implemented here. An account with an
existing cross-signing identity can request and complete verification. If the
identity has not been configured, starting own-device verification returns the
privacy-safe `v-crypto.1-own-identity-unavailable` error and the UI explains
that cross-signing setup is required. The host does not create or reset a
cross-signing identity implicitly because that is the separate V-CRYPTO.2
product setup and authentication workflow.

This is not a permanent verification-unavailable shell: incoming requests,
verification of selected known devices, and own-identity verification on
configured accounts all execute against the live SDK.

## V-CRYPTO.1 product-wiring criteria met

- Native-session verification happy paths use no matrix-js-sdk `CryptoApi`.
- Native-session bootstrap does not call `initRustCrypto` or install the JS
  verification inbox.
- Incoming and outgoing SAS flows are backed by the managed live Rust client.
- Settings and global incoming-request entry points select the Rust commands.
- Missing IPC/SDK capability fails closed with privacy-safe copy.
- Commands, permissions, invoke registration, and generated ACL schemas agree.
- Pure host projection and product ownership helpers have scoped tests.
- Rust formatting/check/tests and touched TypeScript formatting/typecheck pass.

## Blocking deletion residual — V-CRYPTO.1-D

Delete the legacy matrix-js-sdk verification implementation, inbox/listeners,
`CryptoApi`/verification type imports, and native-vs-legacy product branches.
Preserve reusable presentation as SDK-neutral UI over native DTOs. Record the
deleted-file/import delta before marking V-CRYPTO.1 done.

## Remaining named crypto residuals

Closing the deletion residual for this row does **not** close the V-CRYPTO vertical:

- **V-CRYPTO.2** — cross-signing readiness/setup/reset product UX.
- **V-CRYPTO.3** — key backup setup, restore, and repair.
- **V-CRYPTO.4** — secret storage bootstrap and unlock.
- **V-CRYPTO.5** — interactive key-share and room-key flows.
- **V-CRYPTO.6** — undecryptable-history recovery and retry UX.
- **V-CRYPTO.7** — full device list and trust presentation ownership.

The settings wire in this change performs the narrow device-status queries
needed to expose verification actions. Full device-list sourcing and trust
badge ownership remain V-CRYPTO.7.
