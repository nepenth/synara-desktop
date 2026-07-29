# V-CRYPTO.5 — native room-key export and import

| Field                 | Value                                                                        |
| --------------------- | ---------------------------------------------------------------------------- |
| Status                | **Implemented on PR #227; integration acceptance pending**                   |
| Scope                 | Retained local encrypted room-key export/import product path                 |
| Product path          | UI → privacy-safe Tauri IPC → managed `matrix_sdk::Client` → host filesystem |
| Follow-up crypto rows | V-CRYPTO.6–V-CRYPTO.7 remain open                                            |

## Product ownership

Native desktop sessions now retain the complete Local Backup product surface.
Export calls the live Matrix Rust SDK room-key exporter on the managed client.
The SDK encrypts the room-key export with the one-way passphrase and writes it
directly to a uniquely named `synara-room-keys.txt` file in Downloads. Import
opens a native host file chooser and calls the SDK encrypted room-key importer
against the selected host path.

The webview receives only operation phase, progress percentage, imported or
exported counts, room count where the SDK can project it, and a display
basename. The native file chooser returns an opaque selection number and the
basename; the full import path remains in the managed Rust session. A new
selection replaces the prior one. Import exclusively reserves the selection
while it is in flight. Success consumes it; failure restores it for retry only
when the same managed-session generation is still active and no newer
selection occupies the slot. Logout or generation replacement discards the
reservation.

`LocalBackup.tsx` renders the native export/import UI and
calls only the room-key IPC adapter. It does not call matrix-js-sdk
`exportRoomKeysAsJson` / `importRoomKeysAsJson`, the browser megolm keyfile
helpers, browser file reads, Blob export, or FileSaver. The superseded WebView
owner and browser megolm keyfile helper are physically deleted; there is no
native/legacy runtime branch. Missing IPC and SDK/file failures fail closed
with fixed privacy-safe product errors.

## Capability deletion and import delta

The PR records the physical ownership change separately from the repository's
direct-import inventory:

| Evidence                                      | Before                | After                 | Delta |
| --------------------------------------------- | --------------------- | --------------------- | ----: |
| Legacy conditional WebView room-key owner     | 1                     | 0                     |    -1 |
| Browser room-key crypto helper files          | 1                     | 0                     |    -1 |
| `useMatrixClient` calls in `LocalBackup.tsx`  | 2                     | 0                     |    -2 |
| `getCrypto` calls in `LocalBackup.tsx`        | 2                     | 0                     |    -2 |
| JavaScript room-key API calls in Local Backup | 2                     | 0                     |    -2 |
| Repository direct `matrix-js-sdk` inventory   | 232 files / 292 lines | 232 files / 292 lines |     0 |

The global direct-import count is unchanged because the deleted owner reached
the JavaScript client indirectly through `useMatrixClient`; the negative
capability-owner and helper-file deltas are the binding deletion evidence.
V-CRYPTO.5 is not accepted or complete in the integration ledger until PR #227
passes reviewed-SHA gates and lands.

Synara has no retained inbound room-key-request approval prompt or accept/deny
surface. V-CRYPTO.5 therefore does not invent one. Matrix device verification,
backup recovery, and secret-storage recovery remain owned by V-CRYPTO.1–.4.

## Secret and filesystem boundary

The registered and permissioned commands are:

- `matrix_room_key_transfer_status`
- `matrix_room_key_export`
- `matrix_room_key_import_select`
- `matrix_room_key_import`

Export and import passphrases are one-way IPC inputs. Each command owns the
input `String`, passes only a borrowed view to matrix-sdk, and zeroizes the
buffer immediately after the awaited SDK operation. Rust and matrix-sdk hold
decrypted room-key objects only inside the native crypto operation. No room-key
JSON, megolm session, encrypted file bytes, ciphertext, passphrase, access
token, raw file path, or raw SDK error enters an IPC response, event,
diagnostic, or log.

Rust pre-creates export files with owner-only `0600` mode on Unix before the SDK
writes them. Failed exports remove the incomplete destination. Import paths are
retained behind opaque, session-local selection IDs and never returned by the
room-key commands.

`RoomKeyTransferFlow` is the operation coordinator. Its projection contains
only transfer kind, phase, percentage, counts, basename, session generation,
and allow-listed diagnostic identifiers. A completed or failed transfer resets
cleanly when the next operation starts; concurrent transfers fail closed.

## V-CRYPTO.5 done criteria

- Native Local Backup keeps both encrypted export and encrypted import.
- Native happy paths call the live managed Matrix Rust SDK client and never
  call matrix-js-sdk room-key export/import or browser keyfile crypto.
- Encryption/decryption and all keyfile reads/writes happen in Rust against
  host paths.
- Passphrases are zeroized after use, and secret/key/file material never
  appears in IPC responses or logs.
- Export uses a private host-created Downloads file; import uses a host picker
  and opaque session-local selection ID.
- Import success consumes its selection. Import failure retains it for retry
  only in the same session generation and never overwrites a newer selection.
- Status and outcomes expose only privacy-safe phases, counts, labels, and
  diagnostic identifiers.
- Commands, invoke registration, permissions, and generated schemas agree.
- Scoped Rust flow/projection/privacy tests and TypeScript projection/privacy
  tests cover the boundary.
- Rust formatting/tests/check and touched TypeScript formatting/typecheck pass.

## Remaining named crypto residuals

Closing this row does **not** close the V-CRYPTO vertical:

- **V-CRYPTO.6** — full user-visible undecryptable-history recovery and retry
  controls.
- **V-CRYPTO.7** — full native device-list and trust presentation ownership.
