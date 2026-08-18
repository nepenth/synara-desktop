# Desktop Secure Secret Storage Plan

Reviewed: 2026-05-25

## Decision

Use a Rust-owned native credential-store adapter for desktop Matrix session
secrets. For macOS this means Keychain. For Linux this means a Secret
Service-backed store when available, with an explicit unsupported or migration
fallback state when no user credential service exists.

Do not make the Tauri Stronghold JavaScript plugin the default access-token
store. Stronghold is useful for app-managed encrypted snapshots, but it pushes
password/snapshot lifecycle concerns into the app and exposes a broader secret
management API to the frontend. Synara's immediate need is narrower: persist
and clear the Matrix session credential through an operating-system credential
store.

The desktop shell now has the native credential adapter, and the runtime uses
native-first session persistence. Existing legacy fallback sessions migrate to
the native store only after Matrix client initialization succeeds. Login and
registration write native credentials first when available, with legacy
localStorage kept only as a fallback when the native store is unavailable or a
write fails.

## Current State

- Runtime session fallback is in
  `synara/src/app/state/sessions.ts`.
- Login and registration write `synara_access_token`, `synara_device_id`,
  `synara_user_id`, and `synara_hs_base_url` into browser `localStorage`.
- Startup reads that fallback session synchronously from `src/index.tsx`,
  `src/app/pages/Router.tsx`, and client boot paths.
- The Tauri shell exposes the scoped secret-store command surface and a
  keyring-backed native credential adapter.
- macOS reports `macos-keychain` and can persist the Matrix session envelope.
- Linux reports `linux-secret-service` when a D-Bus Secret Service session is
  detected. `linux-keyutils` is detected as session-scoped and is not used for
  persistent session migration by default.
- Tauri capabilities currently expose clipboard, notifications, opener,
  window-state, and global shortcuts only.

## Evaluated Options

### Option A: Keep localStorage

Rejected for production credential storage.

This is the lowest implementation cost, but it keeps bearer access tokens in the
WebView data store. That is not aligned with the iOS Keychain plan, nor with an
App Store-grade credential model.

### Option B: Tauri Stronghold Plugin

Rejected as the default Matrix access-token store for now.

Tauri's Stronghold plugin stores secrets and keys through the IOTA Stronghold
engine and supports desktop and mobile platforms, but the official plugin
currently requires Rust 1.77.2 or newer and must be enabled through explicit
Tauri permissions. The repo's `src-tauri/Cargo.toml` now advertises
`rust-version = "1.77.2"` to match the Tauri 2 floor.

Stronghold may be useful later for a separate encrypted application vault, but
it is not the simplest fit for a single OS-backed Matrix credential. It also
does not remove the need to decide how the Stronghold password is generated,
stored, rotated, and recovered.

### Option C: Rust Native Credential Store

Accepted, then superseded by the native-only credential boundary in 2.1.0.

The desktop shell uses native credential APIs, but the renderer no longer has
any credential read, write, or clear command. Matrix access and refresh tokens
are owned exclusively by the per-account native session vault.

The Rust `keyring` 3.x crate is the adapter because it connects to native
credential stores and exposes platform-specific backends such as Apple Keychain
and Linux Secret Service/keyutils stores. The repo pins the 3.x API line because
the latest 4.x crate has reorganized away from the simple `Entry` API this
adapter needs.

## Current Command Surface

The renderer can probe storage capability and read non-secret identity only:

```text
desktop_secret_store_status() -> DesktopSecretStoreStatus
matrix_session_identity() -> Option<MatrixLoginIdentity>
```

Do not expose generic commands such as `get_secret(key)` or `set_secret(key,
value)` to the WebView.

The former `desktop_get_session`, `desktop_set_session`, and
`desktop_remove_session` commands were removed in 2.1.0. Legacy renderer
credential envelopes are deleted after a successful native login or restore,
and retired localStorage credential keys are purged during bootstrap.

Status: native adapter implemented. macOS uses Keychain.
Linux treats Secret Service as the persistent backend and reports keyutils as a
session-scoped capability until we explicitly decide to accept non-persistent
Linux session restore semantics.

Suggested status payload:

```ts
type DesktopSecretStoreStatus = {
  available: boolean;
  backend:
    | "none"
    | "macos-keychain"
    | "linux-secret-service"
    | "linux-keyutils"
    | "unknown";
  canPersistSession: boolean;
  reason?: string;
};
```

Suggested session envelope:

```ts
type DesktopSessionEnvelope = {
  baseUrl: string;
  userId: string;
  deviceId: string;
  accessToken: string;
  refreshToken?: string;
  expiresInMs?: number;
};
```

Validation rules:

- Reject empty `baseUrl`, `userId`, `deviceId`, and `accessToken`.
- Require `baseUrl` to be `https://` unless explicitly allowed for development.
- Do not log session payloads or token-like values.
- Store a single current-user session first; multi-account storage should be a
  later extension.
- Namespace the credential service/account names under the app identifier,
  currently `com.whylandcreative.synara.desktop`.

## Runtime Migration Plan

The runtime currently expects synchronous session reads. Native credential
stores require asynchronous IPC. The migration should therefore happen in two
steps.

### Step 1: Async Session Bootstrap

Status: implemented in the runtime directory. Startup now initializes an async
session bootstrap before mounting the router, caches the selected session for
existing synchronous route/client consumers, and keeps legacy localStorage
fallback behavior.

Add a startup bootstrap that resolves the session before the app/router decides
whether to show auth or the client:

1. Ask `platform` for secure secret-store availability.
2. If available, read the native desktop session.
3. If no native session exists, read the legacy localStorage fallback.
4. Hydrate an in-memory session cache used by existing boot paths.
5. Keep localStorage writes unchanged until the migration step is ready.

Acceptance criteria:

- App startup still routes logged-in and logged-out users correctly.
- Existing localStorage users still launch.
- Tests cover native session present, legacy session present, and no session.

### Step 2: Credential Migration

Status: implemented in the runtime directory. When startup used a legacy
fallback session, successful Matrix client initialization now writes the
sanitized session envelope to the native store before removing only the legacy
fallback token fields.

After bootstrap is stable:

1. On first launch with a legacy localStorage token and available native store,
   write the session envelope to the native credential store.
2. Start Matrix client init using the migrated session.
3. After successful client init, remove legacy token fields from localStorage.
4. Keep non-secret local settings and drafts untouched.
5. On logout, clear native credential store, legacy token keys, Matrix stores,
   and service-worker in-memory token state.

Acceptance criteria:

- Access tokens are not left in localStorage after successful migration.
- Failed migration does not delete the only valid session.
- Logout clears both native and legacy session locations.
- Downgrade behavior is documented before release.

### Step 3: Write New Sessions Securely

Status: implemented in the runtime directory. Login and registration now use
the platform session persistence helper, which writes native credentials first
when available and only writes legacy fallback fields when the native write is
unavailable or fails.

Login and registration should write the native credential store first when
available. localStorage token writes should remain development-only fallback.

Acceptance criteria:

- Successful login/register stores the access token in native credentials on
  macOS/Linux when available.
- If native credentials are unavailable, the UI presents a clear unsupported or
  fallback state.
- Failed login/register does not persist partial credentials.

## Testing Requirements

Runtime tests:

- Session bootstrap chooses native session over legacy localStorage.
- Legacy fallback remains readable until migration.
- Migration removes only token keys, not settings/drafts.
- Logout clears native and legacy stores.

Rust tests:

- Session payload validation rejects empty or malformed fields.
- Store key/service names are stable and scoped.
- Commands never include token values in errors.
- Missing credential backend returns a typed unavailable status.

Manual smoke:

- macOS: install, login, quit, relaunch, verify session restores from Keychain.
- macOS: logout, relaunch, verify no session restores.
- Linux with Secret Service: same login/relaunch/logout smoke.
- Linux without Secret Service: verify explicit fallback/unsupported behavior.

## Acceptance Criteria Status

- Decision names a primary desktop credential strategy.
- Stronghold and OS credential-store tradeoffs are recorded.
- Migration steps protect existing users from losing sessions.
- Command surface is scoped and does not expose arbitrary secret access.
- Runtime capability contract can represent available, unavailable, and
  non-persistent secret-store states.
- Runtime credential persistence now writes native credentials first when
  available and keeps legacy fallback only when needed.

## Sources

- Tauri Stronghold plugin: https://v2.tauri.app/plugin/stronghold/
- Tauri plugin permissions: https://v2.tauri.app/learn/security/using-plugin-permissions/
- Rust keyring crate: https://docs.rs/keyring/latest/keyring/
- Current shell config: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`,
  `src-tauri/capabilities/main.json`
- Current runtime session store:
  `synara/src/app/state/sessions.ts`
