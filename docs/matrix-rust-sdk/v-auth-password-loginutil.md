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

## Native desktop-session bootstrap (identity-only IPC) — DONE

Native password login and registration persist credentials only in the
per-account host vault. The renderer rehydrates route state through an
identity-only command; SDK restore and sync start after the client loading UI
mounts. Retired renderer envelopes and localStorage credentials are purged.

| Layer          | Change                                                                                                                                                                                                                                                                                                                                                          |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust host      | `matrix_login_password`, registration, refresh rotation, restore, and logout own all credentials through the native per-account vault. `matrix_session_identity` returns only user, device, and homeserver identity and does not build or restore an SDK client. |
| TS             | `completeNativeLoginBootstrap` reads `matrix_session_identity`; missing identity fails closed. No renderer credential write, localStorage fallback, service-worker token channel, or JS login fallback remains. |
| Tests          | Rust source guards prove identity bootstrap cannot restore SDK state or contain tokens; TS bootstrap and storage tests prove native-only identity and one-way legacy credential cleanup. |
| Gates          | Covered by the repository Rust, renderer modernization, boundary, and release validation suites. |
| UX/UI          | **No visual change** — bootstrap/route-gate only; rendering, layout, copy untouched                                                                                                                                                                                                                                                                             |
| Public hygiene | No real secrets in fixtures/tests; token values are placeholders                                                                                                                                                                                                                                                                                                |
