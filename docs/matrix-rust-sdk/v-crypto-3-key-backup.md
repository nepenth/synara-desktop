# V-CRYPTO.3 — native key-backup restore, setup, and repair

| Field                 | Value                                                                                 |
| --------------------- | ------------------------------------------------------------------------------------- |
| Status                | **DONE — native product owner; V-CRYPTO.3-D legacy deletion complete**                |
| Scope                 | Server-side key-backup status, setup, restore, repair, and restore after verification |
| Product path          | UI → Tauri IPC → managed `matrix_sdk::Client`                                         |
| Follow-up crypto rows | V-CRYPTO.4–V-CRYPTO.7 remain open                                                     |

## Product ownership

Native desktop sessions now source encryption-backup state from the live Matrix
Rust SDK client and expose the complete product decision:

- whether a server backup exists, with its opaque version and key count;
- whether this device has activated that backup;
- whether the device is connecting, downloading, uploading, ready, or
  disconnected;
- whether recovery is not set up, incomplete, or ready; and
- whether the next action is setup, restore, repair, or no action.

The native Devices screen renders this state and supports all three actions.
Setup creates secret storage and a server-side key backup through
`Recovery::enable`, requires a recovery passphrase, waits for locally available
room keys to upload, and never returns the generated recovery key to the
webview. Restore accepts a recovery key or passphrase and calls
`Recovery::recover`. Repair calls `Recovery::recover_and_fix_backup`, which
repairs inconsistent or missing backup material by recreating the backup when
matrix-sdk determines that is necessary.

Native clients use `BackupDownloadStrategy::OneShot`. Successful recovery and
backup material received from a verified device therefore download backed-up
room keys directly into the native crypto store. Matrix-sdk owns recovery after
verification through the same one-shot policy; the WebView does not attach a
crypto listener or issue a duplicate restore action.

`BackupRestore` and `useNativeKeyBackup` call only the native IPC owner and do
not import matrix-js-sdk `CryptoApi` or initialize JS crypto.
`LocalBackup` remains the separate native encrypted room-key file import/export
surface completed in **V-CRYPTO.5**. This slice does not change that retained
path or silently start JS crypto.

## Rust host and IPC

The registered and permissioned commands are:

- `matrix_backup_status`
- `matrix_backup_setup`
- `matrix_backup_restore`
- `matrix_backup_repair`

Status and operation responses contain only session generation, availability,
enabled state, opaque backup version, key count, device/recovery/action enums,
and operation outcome. They contain no tokens, recovery keys, passphrases,
private keys, session secrets, ciphertext, backup authentication data, or raw
SDK error strings.

Passphrases and recovery keys are one-way command inputs. The Tauri command
owns each input `String`, passes only a borrowed view to matrix-sdk, and
zeroizes it immediately after the awaited SDK operation. Setup's generated
recovery key is retained by matrix-sdk as required for the local crypto store
and is zeroized in the command implementation without crossing IPC. All
command errors use fixed product copy and stable diagnostic identifiers.

## V-CRYPTO.3 product-wiring criteria met

- Native-session status, setup, restore, and repair operate on the live managed
  Rust client.
- Recovery downloads backed-up room keys into the native crypto store; secret
  receipt after verification uses the same native one-shot restore policy.
- The native Devices product path calls no matrix-js-sdk `CryptoApi`,
  `restoreKeyBackup`, or `initRustCrypto` for this capability.
- Missing IPC, missing backup, invalid recovery input, and SDK failures fail
  closed with fixed privacy-safe errors.
- Commands, invoke registration, permissions, and generated schemas agree.
- Secrets are one-way inputs, are zeroized after SDK use, and never appear in
  IPC responses or logs.
- Pure Rust status projections and TypeScript privacy/ownership helpers have
  scoped tests.
- Rust formatting/tests/check and touched TypeScript formatting/typecheck pass.

## V-CRYPTO.3-D deletion complete

The matrix-js-sdk backup status/restore owner, crypto listeners, progress atom,
automatic restore listener, `CryptoApi`/backup types, and native-vs-legacy UI
branch are deleted. The retained settings tile owns setup, restore, repair, and
status only through native DTOs and `matrix_backup_*` IPC. Direct desktop-runtime
inventory moved from **222 files / 279 import declarations** to **219 / 276**;
production importers moved **211 → 208**, with component **40 → 39**, hook
**54 → 53**, and state **14 → 13**.

## Remaining named crypto residuals

Closing the deletion residual for this row does **not** close the V-CRYPTO vertical:

- **V-CRYPTO.4** — full secret-storage bootstrap/unlock and identity-recovery
  UX beyond the SDK calls required by backup setup/restore/repair here.
- **V-CRYPTO.5** — interactive key sharing and retained room-key flows,
  including native local encrypted room-key file import/export.
- **V-CRYPTO.6** — user-visible undecryptable-history recovery/retry controls.
- **V-CRYPTO.7** — full native device-list and trust presentation ownership.

The secret-storage calls in this row are only those needed to complete the
backup product path. They do not claim the broader V-CRYPTO.4 UX.
