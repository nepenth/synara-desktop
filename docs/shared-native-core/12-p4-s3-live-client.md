# 12 — P4-S3 live Client on iOS (written plan)

This is the plan playbook §9.4 requires before guessing at password FFI.
It does **not** accept P4 or claim iOS is on the shared engine.

Evidence tip when written: `feature/shared-native-core` `ee896416` (#935 S3a).
S3a landed in #935. S3b restore landed in #937 (`4edfc1f5`).
S3c login (Option A) landed in #938 (`9b4ec54f`).
S3d attach landed in #939 (`ad63d56d`).
S4 room-list snapshot landed in #940 (`5a6f3e9b`).
S5 invites snapshot landed in #942 (`17c29afd`).
S6 timeline open/close/paginate landed in #944 (`1a77815a`).
S7 typing/presence landed in #945 (`ee884ef3`).
S8 verification list landed in #947 (`52874c20`).
S9 verification SAS landed in #948 (`c56a82aa`).
S9-2 devices landed in #950 (`1539ed6a`).
**S9-3 join-rule snapshot lands on `agent/snc-p4-s9-join-rules`.**
Playbook recipe: §9.5. `matrix_room_join_rule_snapshot` only. Next is image packs.

## Decision

S3a/S3b do not put a password on a UniFFI `string`. **S3c chooses
Option A:** a dedicated `SharedCore.login_with_password` FFI. The
password is a method argument only — not stored, not in the DTO, never
echoed. It never rides `Core.command` and does not register
`matrix_login_password`.

S3 is four serial product PRs. Each one merges only to
`feature/shared-native-core`.

| Slice | What lands | What must not land |
|---|---|---|
| **S3a** LANDED #935 | Swift `IosSecretVault` callback. `SharedCore` can be constructed with that vault. Rust `IosFailClosedPlatform` uses it instead of `UnavailableSecretVault`. In-memory + Keychain key/value adapters. | Live `Client`, `command`, `attach_*`, password, restore, APNs |
| **S3b** landed #937 | Restore an already-persisted session through Core persist/restore using the vault. No password. Not `matrix_restore_session`. | Password login, owner attach, `command` families, leftover registration |
| **S3c** landed #938 | Dedicated `SharedCore.login_with_password` FFI. Persist via `persist_session_after_login` so S3b restore can find `matrix-session:{segment}`. Password is a dedicated UniFFI argument only. Dual-engine: login+restore on SharedCore; room list/timeline/crypto stay `MatrixRustSDK` until S3d. | Register, email token, recovery passphrase, `attach_*`, `Core.command`, leftover registration |
| **S3d** landed #939 | After a session exists, `SharedCore.attach_session_owners` attaches the desktop owner set on the retained Client: typing, presence, verification, devices, join-rules, image-packs, timelines, sync. No-op emit sinks. SyncService is attached but not started (no dual live sync while `MatrixRustSDKService` still owns product room list). | Retiring `MatrixRustSDKService`, leftover registration |

## S3c Option A (chosen)

Core already owns `login_with_password` (decision 3B); desktop already
calls it from the shell. S3b restore only works if login persists into
the same vault + `StoreKeyId`. Deferring to Swift `MatrixRustSDK` login
would write a different session store and leave S3b unrestorable from
product login.

Password may cross UniFFI as the dedicated `login_with_password`
argument. It is not stored, not copied into a DTO, and never echoed in
errors. Rust zeroizes the password `String` on drop after the Core call.
Do not attach owners. Do not add `Core.command`. Live product login may
stay on `MatrixRustSDKAuthService` for this slice; the helper + XCTest
are the iOS surface.

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

## Desktop attach set (S3d)

From `src-tauri/src/matrix/auth/product_commands.rs` after login, in
this order:

`attach_typing`, `attach_presence`, `attach_verification`,
`attach_devices`, `attach_join_rules`, `attach_image_packs`,
`attach_timelines`, `attach_sync`.

S3d calls those Core APIs from `SharedCore.attach_session_owners`.
Missing attach → later commands fail closed. That is correct.
Do not start `SyncService` here: desktop starts it because desktop is
already on-engine. iOS stays dual-engine until S4 consumes
`matrix_room_list_snapshot`.

## Non-goals until later slices

- `Platform::emit` for product events
- Invented 7B desktop routes
- Deleting `MatrixRustSDKService`
- NSE / APNs
