# 12 — P4-S3 live Client on iOS (written plan)

This is the plan playbook §9.4 requires before guessing at password FFI.
It does **not** accept P4 or claim iOS is on the shared engine.

Evidence tip when written: `feature/shared-native-core` `4c080c43` (#934).

## Decision

Keep password login on `MatrixRustSDK` until **S3c**. Do not put a
password on a UniFFI `string` in S3a or S3b.

S3 is four serial product PRs. Each one merges only to
`feature/shared-native-core`.

| Slice | What lands | What must not land |
|---|---|---|
| **S3a** (this PR) | Swift `IosSecretVault` callback. `SharedCore` can be constructed with that vault. Rust `IosFailClosedPlatform` uses it instead of `UnavailableSecretVault`. In-memory + Keychain key/value adapters. | Live `Client`, `command`, `attach_*`, password, restore, APNs |
| **S3b** | Restore an already-persisted session through Core persist/restore using the vault. No password. | Password login, owner attach, `command` families |
| **S3c** | Dedicated authenticated login FFI **or** keep Swift `MatrixRustSDK` login. Design in that PR body. Password never rides a generic `Core.command` string. | Register, email token, recovery passphrase |
| **S3d** | After a session exists, attach the same owner set desktop attaches (typing, presence, verification, devices, join-rules, image-packs, sync, timelines). | Retiring `MatrixRustSDKService` |

## Why S3a first

`Platform::secret_store()` is the shell vault trait. Desktop persist/restore
already expects a `SecretVault`. iOS today uses
`KeychainSecureSessionStore` for a Swift `AuthenticatedSession` JSON blob,
which is **not** that trait. S3a adds the trait adapter only.

Session tokens may cross this vault (that is the vault's job). Passwords,
recovery passphrases, `client_secret`, file paths, and media bytes still
must not cross `Core::command`.

## S3a surface

UDL:

- `callback interface IosSecretVault` with `get` / `put` / `delete`
- `IosSecretVaultError::Unavailable { code, description }` — static
  source constants only. Never echo key, value, or Keychain status.
- `SharedCore` keeps `constructor()` (fail-closed vault) and adds
  `constructor(IosSecretVault store)` named `new_with_secret_store`.

Rust:

- `ForeignSecretVault` implements `SecretVault` by calling the callback.
- `IosFailClosedPlatform::with_secret_store(Arc<dyn SecretVault>)`.
- No `keyring` crate. No Tauri. No `command`.

Swift:

- In-memory test double.
- Keychain key/value adapter (generic bytes, not `AuthenticatedSession`).
- Do not migrate `SecureSessionStore` in S3a.

## Desktop attach set (S3d later)

From `src-tauri/src/matrix/auth/product_commands.rs` after login:

`attach_typing`, `attach_presence`, `attach_verification`,
`attach_devices`, `attach_join_rules`, `attach_image_packs`, then
timelines and sync (same file). Missing attach → later commands
fail closed. That is correct.

## Non-goals until later slices

- `Platform::emit` for product events
- Invented 7B desktop routes
- Deleting `MatrixRustSDKService`
- NSE / APNs
