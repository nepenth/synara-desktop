# V-CRYPTO.3 — native key-backup restore, setup, and repair

| Field | Value |
| --- | --- |
| Status | **Done on `matrix-rust/v-crypto-3-key-backup`** |
| Scope | Server-side key-backup status, setup, restore, repair, and restore after verification |
| Product path | UI → Tauri IPC → managed `matrix_sdk::Client` |
| Follow-up crypto rows | V-CRYPTO.4–V-CRYPTO.7 remain open |

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
room keys directly into the native crypto store. The
`useRestoreBackupOnVerification` native path does not attach matrix-js-sdk
crypto listeners or invoke `restoreKeyBackup`; matrix-sdk owns that transition.

For native sessions, `BackupRestore`, `useKeyBackup`, `backupRestore.ts`, and
`LocalBackup` do not call matrix-js-sdk `CryptoApi` or initialize JS crypto.
`LocalBackup` fails closed on the native path because encrypted room-key file
import/export is the separately named retained room-key transfer surface in
**V-CRYPTO.5**. It points users to the fully native server backup path rather
than silently starting JS crypto. Legacy web/non-native sessions retain their
existing components.

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

## V-CRYPTO.3 done criteria

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

## Remaining named crypto residuals

Closing this row does **not** close the V-CRYPTO vertical:

- **V-CRYPTO.4** — full secret-storage bootstrap/unlock and identity-recovery
  UX beyond the SDK calls required by backup setup/restore/repair here.
- **V-CRYPTO.5** — interactive key sharing and retained room-key flows,
  including native local encrypted room-key file import/export.
- **V-CRYPTO.6** — user-visible undecryptable-history recovery/retry controls.
- **V-CRYPTO.7** — full native device-list and trust presentation ownership.

The secret-storage calls in this row are only those needed to complete the
backup product path. They do not claim the broader V-CRYPTO.4 UX.
