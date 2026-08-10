# V-CRYPTO.4 — native secret-storage bootstrap and unlock

| Field                 | Value                                                                         |
| --------------------- | ----------------------------------------------------------------------------- |
| Status                | **DONE — native owner retained; V-CRYPTO.4-D legacy deletion complete**       |
| Scope                 | Secret-storage readiness, bootstrap, unlock/import, and recovery-key rotation |
| Product path          | UI → Tauri IPC → managed `matrix_sdk::Client`                                 |
| Follow-up crypto rows | V-CRYPTO.6–V-CRYPTO.7 remain open                                             |

## Product ownership

Native desktop sessions now source secret-storage readiness from the live
Matrix Rust SDK recovery and secret-storage managers. The privacy-safe product
projection reports:

- whether a valid default secret-storage key and its description exist;
- whether this device has imported the protected account secrets;
- whether a recovery passphrase is configured;
- which required secret categories are absent: cross-signing master,
  self-signing, user-signing, or encryption backup; and
- whether the next action is bootstrap, unlock/import, or none.

The native Devices security surface renders setup, unlock/import, ready, and
key-rotation states. Unlock accepts either the Matrix recovery key or recovery
passphrase and calls `Recovery::recover`, which opens secret storage and imports
the known cross-signing and backup secrets into the managed native crypto
store. Rotation uses `Recovery::reset_key` after requiring an unlocked local
identity.

Fresh verification setup completes native cross-signing first, then requires
secret-storage bootstrap before the dialog can finish. This order lets
matrix-sdk export the newly created private cross-signing identity into the new
secret store. The standalone recovery tile also refuses bootstrap until the
native private cross-signing identity is complete. Existing secret storage is
unlocked through the recovery tile before verification and backup readiness
can proceed.

`SecretStorage.tsx`, `useNativeSecretStorage.ts`, and the Devices and
Verification gates use this native path unconditionally. The legacy
account-data hooks, browser recovery-key derivation/checking components, and JS
secret-key cache are deleted. A missing desktop command or unavailable SDK
status fails closed with fixed recovery guidance; it never starts JS crypto.

## Recovery document and secret boundary

The registered and permissioned commands are:

- `matrix_secret_storage_status`
- `matrix_secret_storage_bootstrap`
- `matrix_secret_storage_unlock`
- `matrix_secret_storage_reset`

Recovery keys and passphrases are transient one-way IPC command inputs. They may
be present in the WebView while the user supplies them to the native action, but
they are not persisted, returned, or logged. Each command owns its input
`String`, passes only a borrowed view to matrix-sdk, and zeroizes the buffer
immediately after the awaited operation. Status and operation responses contain
only booleans, enums, missing-secret categories, session generation, operation
outcome, and recovery-document save metadata.

Bootstrap and rotation necessarily create a new Matrix recovery key. That key
never enters an IPC response, event, diagnostic, or log. Rust writes it
directly to a uniquely named `synara-recovery-key.txt` document in Downloads
with owner-only `0600` creation mode on Unix, syncs the file, and zeroizes the
in-memory string. IPC receives only that the document was saved and its fixed
display filename. A recovery passphrase is required, so a file-save failure
does not strand the account; the command returns fixed privacy-safe guidance
that the passphrase remains valid.

Raw encrypted account-data bodies are inspected only inside Rust to project
presence. Key IDs, key descriptions, encrypted secret bodies, private identity
material, and raw SDK errors never cross IPC.

## V-CRYPTO.4 closure criteria met

- Native-session readiness comes from the live managed Rust client.
- Native bootstrap creates secret storage and exports the complete local
  cross-signing and backup secret set supported by matrix-sdk.
- Native unlock imports protected secrets and makes the device recovery-ready.
- Native key rotation retains the product reset path without returning new key
  material to the webview.
- Devices and verification setup gates do not call matrix-js-sdk `CryptoApi`,
  `secretStorage`, recovery-key derivation, or the JS key cache for native
  sessions.
- Missing IPC, invalid input, locked reset, incomplete identity, server backup
  conflicts, and SDK failures fail closed with fixed privacy-safe errors.
- Commands, invoke registration, permissions, and generated schemas agree.
- Scoped Rust projection/privacy tests and TypeScript projection/privacy tests
  cover the boundary.
- Rust formatting/tests/check and touched TypeScript formatting/typecheck pass.
- Browser recovery passphrase/key components, their dead manual-verification
  owner, JS key cache, JS-only cache test, and account-data compatibility types
  are physically deleted.
- Direct desktop-runtime imports move from 219 files / 276 import lines to 218 / 275;
  production importers move 208 → 207, component importers 39 → 38, the import
  allowlist 208 → 207, and repository-wide importers 222 → 221.

## Remaining named crypto residuals

Closing this row does **not** close the V-CRYPTO vertical. Continue with
V-CRYPTO.6:

- **V-CRYPTO.6** — full user-visible undecryptable-history recovery and retry
  controls.
- **V-CRYPTO.7** — full native device-list and trust presentation ownership.
