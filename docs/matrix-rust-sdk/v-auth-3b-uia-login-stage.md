# V-AUTH.3b — login multi-stage UIA (product non-retention)

| Field | Value |
| ----- | ----- |
| Status | **DONE — product does not retain multi-stage UIA on the login route** |
| Capability | Multi-stage interactive auth submit for **login** (`matrix_uia_*` session start/submit/cancel) |
| Decision | **Not retained** for login; closed without inventing unused IPC |
| Inventory | [v-auth-3-uia-inventory.md](v-auth-3-uia-inventory.md) (post-#276 residual row) |
| Policy | [full-vertical-policy.md](full-vertical-policy.md) |
| Related | V-AUTH.2 token non-retention; V-AUTH.3 loginFlows #276; V-AUTH.3p password `loginUtil` residual (parallel [#279](https://github.com/nepenth/synara-desktop/pull/279)); V-AUTH.4a/4b; V-CRYPTO.7 device delete UIAA |

## Product decision

Synara Desktop **does not retain multi-stage UIA for password login**.

Product login is a **single-shot password** path:

| Layer | Owner |
| ----- | ----- |
| UI | `Login.tsx` → password-only `PasswordLoginForm` (no stage overlay) |
| TS | `loginPassword` → `matrix_login_password` |
| Native | `login_with_password` / `matrix_login_password` |
| UIAA on login | Fail-closed: `map_login_sdk_error` → `AuthError::InteractiveAuthRequired` (`p3.2-login-uiaa-required`); product login error mapper does **not** open a multi-stage challenge UI |

There is **no** product entry that starts a multi-stage login challenge (recaptcha/terms/email/msisdn/dummy chain) after password submit. The historical product surface matched that shape: login never mounted `UIAFlowOverlay` / `uia-stages/*`; multi-stage UIA lived on **register** and (chrome-only) password-reset.

V-AUTH.3b makes that non-retention **explicit** so residual tracking stops asking for a generic `matrix_uia_*` login-stage family that no product path would call.

This is residual closure under [full-vertical-policy.md](full-vertical-policy.md) via **explicit product non-retention** (same pattern as V-AUTH.2), **not** dual-backend and **not** an approved residual plateau that leaves a live js multi-stage login client.

## Inventory evidence (tip `e6db76c7`)

### Login route — no multi-stage owner

| Path | Finding |
| ---- | ------- |
| `synara/src/app/pages/auth/login/Login.tsx` | Password form only; no UIA overlay/stages |
| `synara/src/app/pages/auth/login/PasswordLoginForm.tsx` | Single `loginPassword` submit; no stage dialogs |
| `synara/src/app/pages/auth/login/loginUtil.ts` | Password IPC (+ residual js fallback owned by #279); no interactive-auth stage loop |
| `src-tauri/src/matrix/auth/login.rs` | UIAA response → `InteractiveAuthRequired` (no stage continuation) |
| `src-tauri/src/matrix/auth/product.rs` | **No** `matrix_uia_*` product commands |

### Where multi-stage / UIAA **is** product-used (already native)

| Surface | Status | Native owners |
| ------- | ------ | ------------- |
| Register multi-stage UIA | **DONE #266** V-AUTH.4b | `matrix_register_flows` / `matrix_register_request_email_token` / `matrix_register` + SDK-neutral stages |
| Password reset | **DONE #263** V-AUTH.4a | `matrix_password_reset_request_email_token` / `matrix_password_reset_complete` (`UIAFlowOverlay` chrome only) |
| Other-device delete UIAA | **DONE** V-CRYPTO.7 | Password-only `matrix_device_delete_start` / `_password` / `_cancel` |
| Shared stage UI helpers | SDK-neutral; **register/reset consumers only** | `matrix-uia.ts`, `useUIAFlows.ts`, `uia-stages/*`, `UIAFlowOverlay.tsx` |

### What is **not** invented

| Item | Why |
| ---- | --- |
| `matrix_uia_session_start` / `submit` / `cancel` | No login product consumer; register/reset/device-delete already have specialized native stage/UIAA paths |
| Rewire login overlay to generic UIA IPC | Would invent unused product surface |
| Delete register/reset stage UI | Still product-used (native) |
| Delete P3.4 `UiaSession` harness | Foundation/tests only; not a live login client; not claimed as product login wiring |

## Fail-closed behavior (login)

If a homeserver answers password login with multi-stage UIAA that cannot complete via single password:

1. Native host maps to privacy-safe `InteractiveAuthRequired` / unknown login failure (no secrets in DTO/logs).
2. Product shows the existing password-login error path.
3. **No** matrix-js-sdk interactive-auth client is constructed for stage continuation on desktop.

Homeservers that complete password login in one step (the product-supported case) continue to work via `matrix_login_password`.

## Explicit non-goals / adjacent residual

| Item | Status |
| ---- | ------ |
| Desktop password `loginUtil` js fallback / type neutralization | **Parallel residual** — open [#279](https://github.com/nepenth/synara-desktop/pull/279) (`V-AUTH.3p`); **not** this PR |
| Register / password-reset multi-stage | Already native; not reworked here |
| Device-delete password UIAA | Already native; not reworked here |
| Live Synapse multi-stage login e2e | Not product-retained; unclaimed |
| V-TIMELINE cutover / #240 | **HOLD** |
| Umbrella #39 | Do not merge |

## Evidence

- TypeScript: `synara/src/app/pages/auth/login/__tests__/loginUiaStageAbsence.test.ts`
- Rust: `v_auth_3b_product_has_no_matrix_uia_login_stage_commands` in `product.rs` tests
- Design mirrors: [v-auth-2-token-login.md](v-auth-2-token-login.md)

## Import / deletion accounting

| Metric | Value |
| ------ | ----- |
| Live js multi-stage login client deleted this slice | **N/A** — none existed on the login route |
| Production `matrix-js-sdk` import delta | **0** (docs + absence tests only) |
| Allowlist delta | **0** (no claimed allowlist burn-down; password files remain #279) |
| Dual backend | Forbidden / not introduced |
| Invented unused IPC | **None** |

## Residual ledger impact

| Row | After this close |
| --- | ---------------- |
| V-AUTH.3 discovery | DONE #276 |
| V-AUTH.3b login multi-stage UIA | **DONE — non-retention** (this PR) |
| V-AUTH.3p password `loginUtil` | Open #279 until merged |
| V-AUTH.4a/4b | DONE |
