# Desktop password login — fail-closed native only

| Field | Value |
|-------|-------|
| Status | **Active this PR** — residual after V-AUTH.3 |
| Policy | [full-vertical-policy.md](full-vertical-policy.md) |
| Related | V-AUTH.2 token non-retention; V-AUTH.3 loginFlows DONE #276; P3.2 native password foundation |

## Capability owned

Desktop product password login must use **only** the native Tauri command
`matrix_login_password` (already implemented). Remove the superseded
matrix-js-sdk path in `loginUtil.ts` (`createClient` + `loginRequest`) so it
cannot remain a live fallback on desktop.

## Inventory (pre-change)

| Owner | Pre-state |
|-------|-----------|
| `PasswordLoginForm.tsx` | Calls `loginPassword` |
| `loginUtil.loginPassword` | Desktop → `matrix_login_password`; **non-desktop → `login()`** |
| `loginUtil.login` | Live `createClient` + `mx.loginRequest` (matrix-js-sdk) |

## This slice

| Layer | Change |
|-------|--------|
| TS owner | `loginPassword` always requires desktop + native IPC; fail-closed otherwise |
| Deletion | Remove `login()` / `createClient` / `loginRequest` from `loginUtil.ts` |
| SDK-neutral | Drop `matrix-js-sdk` imports from `loginUtil.ts` and `PasswordLoginForm.tsx`; local `PasswordLoginError` / request DTOs |
| Allowlist | **171→169** (drop both files) |
| Production import files | **169→167** |
| Tests | `desktopPasswordLoginNativeOnly.test.ts` (source absence + fail-closed unit behavior) |

## Explicit non-goals

| Item | Status |
|------|--------|
| UIA stage execution for login | Residual (follow-on after V-AUTH.3) |
| Register / password-reset owners | Already native (#266 / #263) — not rewritten |
| Live Synapse e2e for password login | Not claimed here |
| V-TIMELINE / #240 | **HOLD** — no cutover |
| Umbrella #39 | Do not merge |

## Fail-closed

- Not Synara desktop → `PasswordLoginError` (no js client)
- Native command unavailable → `PasswordLoginError`
- **No** dual_backend / createClient fallback on desktop
