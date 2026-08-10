# Desktop password login — fail-closed native only

| Field   | Value                                                                                        |
| ------- | -------------------------------------------------------------------------------------------- |
| Status  | **Active this PR** — residual after V-AUTH.3                                                 |
| Policy  | [full-vertical-policy.md](full-vertical-policy.md)                                           |
| Related | V-AUTH.2 token non-retention; V-AUTH.3 loginFlows DONE #276; P3.2 native password foundation |

## Capability owned

Desktop product password login must use **only** the native Tauri command
`matrix_login_password` (already implemented). Remove the superseded
matrix-js-sdk path in `loginUtil.ts` (`createClient` + `loginRequest`) so it
cannot remain a live fallback on desktop.

## Inventory (pre-change)

| Owner                     | Pre-state                                                      |
| ------------------------- | -------------------------------------------------------------- |
| `PasswordLoginForm.tsx`   | Calls `loginPassword`                                          |
| `loginUtil.loginPassword` | Desktop → `matrix_login_password`; **non-desktop → `login()`** |
| `loginUtil.login`         | Live `createClient` + `mx.loginRequest` (matrix-js-sdk)        |

## This slice

| Layer                   | Change                                                                                                                  |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| TS owner                | `loginPassword` always requires desktop + native IPC; fail-closed otherwise                                             |
| Deletion                | Remove `login()` / `createClient` / `loginRequest` from `loginUtil.ts`                                                  |
| SDK-neutral             | Drop `matrix-js-sdk` imports from `loginUtil.ts` and `PasswordLoginForm.tsx`; local `PasswordLoginError` / request DTOs |
| Allowlist               | **171→169** (drop both files)                                                                                           |
| Production import files | **169→167**                                                                                                             |
| Tests                   | `desktopPasswordLoginNativeOnly.test.ts` (source absence + fail-closed unit behavior)                                   |

## Explicit non-goals

| Item                                | Status                                       |
| ----------------------------------- | -------------------------------------------- |
| UIA stage execution for login       | Residual (follow-on after V-AUTH.3)          |
| Register / password-reset owners    | Already native (#266 / #263) — not rewritten |
| Live Synapse e2e for password login | Not claimed here                             |
| V-TIMELINE / #240                   | **HOLD** — no cutover                        |
| Umbrella #39                        | Do not merge                                 |

## Fail-closed

- Not Synara desktop → `PasswordLoginError` (no js client)
- Native command unavailable → `PasswordLoginError`
- **No** dual_backend / createClient fallback on desktop

## Native desktop-session bootstrap (envelope dual-write) — DONE

The native password-login / register session now rehydrates the frontend
bootstrap from the host-side desktop session envelope, so route guards and
ClientRoot see an active **native** session after login/register without any
token appearing on the login/register IPC return path.

| Layer          | Change                                                                                                                                                                                                                                                                                                                                                          |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust host      | `matrix_login_password` + register session install dual-write the hybrid `DesktopSessionEnvelope` (access/refresh tokens) into the OS credential store via `desktop_set_session_in_store`; `matrix_logout` clears it. `MatrixLoginIdentity` stays token-free. Fail-closed: an unusable store rolls the login back.                                              |
| TS             | `completeNativeLoginBootstrap` re-reads `desktop_get_session` and sets `sessionBootstrap` (`source: 'native'`) after native login (`useLoginComplete`) and register (`PasswordRegisterForm` complete branch). Missing envelope → fail-closed (no navigation, no JS fallback). `desktop.ts` gained `formatDesktopInvokeError` for structured invoke diagnostics. |
| Tests          | Rust: `envelope_from_auth_session_*` + `v_auth_native_session_envelope_host_dual_write_is_wired`; TS: `nativeLoginBootstrap.test.ts` + `nativeRegister.test.ts` source guard                                                                                                                                                                                    |
| Gates          | `cargo test --lib` **835** green; `npm run test:modernization` **695** green; typecheck + eslint + prettier + clippy `-D warnings` green; p1.6 allowlist **114** unchanged (no importer delta)                                                                                                                                                                  |
| UX/UI          | **No visual change** — bootstrap/route-gate only; rendering, layout, copy untouched                                                                                                                                                                                                                                                                             |
| Public hygiene | No real secrets in fixtures/tests; token values are placeholders                                                                                                                                                                                                                                                                                                |
