# V-AUTH.3 — UIA / login-flow discovery inventory + full-vertical slice plan

| Field | Value |
|-------|-------|
| Status | **Inventory + slice plan (docs only)** — no product code in this PR |
| Tip measured | `f9ed781fa1a287a1cf4cb71dbef2cf1ad2dd4b6b` (branch `matrix-rust/v-auth-3-inventory-ds2`) |
| Base | `feature/matrix-rust-sdk-full-replacement` |
| Policy | [full-vertical-policy.md](full-vertical-policy.md) — physical deletion inside the slice |
| Related | V-AUTH.1 #238 (SSO removed), V-AUTH.2 #262 (token non-retention), V-AUTH.4a #263 (password reset), V-AUTH.4b #266 (register, OPEN) |

> **Scope guard.** This is an **inventory + plan** document only. It does **not**
> implement product code in `product.rs` or register. It does not touch open
> **#266** (V-AUTH.4b register) or **#240** (V-TIMELINE, HOLD). No cutover.

---

## 1. Tip measured

- Branch: `matrix-rust/v-auth-3-inventory-ds2`
- Tip SHA: **`f9ed781fa1a287a1cf4cb71dbef2cf1ad2dd4b6b`** (merge of #272 docs-after-268)
- Working tree clean at measurement time.
- Base integration branch: `feature/matrix-rust-sdk-full-replacement`.

---

## 2. JS owners still on matrix-js-sdk for loginFlows / AuthFlowsLoader / UIA

The product auth surface still drives login-flow discovery and UIA through
`matrix-js-sdk`. The **single live-client residual** is `AuthFlowsLoader.tsx`
(`createClient` + `mx.loginFlows()`); the rest are type-only importers of
`matrix-js-sdk` auth/UIA types that must be made SDK-neutral or deleted.

| Path | Role | Notes |
|------|------|-------|
| `synara/src/app/components/AuthFlowsLoader.tsx` | **Live client residual.** `createClient({baseUrl})` + `mx.loginFlows()` + `mx.registerRequest({})`; builds `AuthFlows { loginFlows, registerFlows }` | The only remaining live `matrix-js-sdk` client in the auth vertical. `registerRequest` probe is #266's concern; V-AUTH.3 re-homes the **loginFlows** half. |
| `synara/src/app/hooks/useAuthFlows.ts` | Context provider for `AuthFlows`; types `ILoginFlowsResponse`, `IAuthData`, `MatrixError`; `parseRegisterErrResp` | Register-flow parsing is #266 scope. V-AUTH.3 replaces the `loginFlows` type with a Synara-owned DTO. |
| `synara/src/app/hooks/useParsedLoginFlows.ts` | `getPasswordFlow` + `useParsedLoginFlows` over `LoginFlow[]` | Pure helper; only imports `IPasswordFlow`/`LoginFlow` types. Make SDK-neutral (Synara `LoginFlow` DTO). |
| `synara/src/app/pages/auth/login/Login.tsx` | Consumes `useAuthFlows().loginFlows.flows` → `useParsedLoginFlows` | UI consumer; no direct js-sdk import but depends on the flow DTO shape. |
| `synara/src/app/pages/auth/login/PasswordLoginForm.tsx` | Password login form; imports `MatrixError` type | Password login is already native (`matrix_login_password`); js-sdk here is type-only for error mapping. |
| `synara/src/app/pages/auth/login/loginUtil.ts` | `login()` non-native fallback (`createClient` + `loginRequest`); `loginPassword` native path | Non-desktop fallback path retained; V-AUTH.3 does not remove it (password vertical, not UIA). |
| `synara/src/app/components/SupportedUIAFlowsLoader.tsx` | Filters `UIAFlow[]` by supported stages | Type-only `UIAFlow` import; used by register. SDK-neutralize. |
| `synara/src/app/hooks/useUIAFlows.ts` | UIA helpers over `IAuthData`/`UIAFlow`; `SUPPORTED_FLOW_TYPES` | Type-only; used by register + login UIA. SDK-neutralize. |
| `synara/src/app/utils/matrix-uia.ts` | Pure UIA helpers (`getSupportedUIAFlows`, `getUIAFlowForStages`, terms URL, …) | Type-only `AuthType`/`IAuthData`/`UIAFlow`. SDK-neutralize. |
| `synara/src/app/components/uia-stages/types.ts` | `StageComponentProps` uses `AuthDict` | Type-only; SDK-neutralize. |
| `synara/src/app/components/uia-stages/{Dummy,Email,ReCaptcha,RegistrationToken,Terms}Stage.tsx` | Stage UI components | Type-only; SDK-neutralize (register scope, but shared with login UIA). |
| `synara/src/app/hooks/useAuthMetadata.ts` | `ValidatedAuthMetadata` context | Type-only; not loginFlows/UIA — listed for completeness (auth-adjacent). |

**Register-specific owners** (`registerUtil.ts`, `PasswordRegisterForm.tsx`,
`Register.tsx`, `useRegisterEmail.ts`, `SupportedUIAFlowsLoader` usage in
`Register.tsx`) are **#266 V-AUTH.4b scope**, not V-AUTH.3. V-AUTH.3 must not
rewrite them; it only re-homes the **login-flow discovery** half of
`AuthFlowsLoader` and the shared UIA type layer that login also consumes.

---

## 3. What is already native

| Capability | Status | Evidence |
|------------|--------|----------|
| Password login | **Native** — `matrix_login_password` Tauri command; `loginPassword` in `loginUtil.ts` invokes it on desktop | `src-tauri/src/matrix/auth/product.rs:273`; registered in `src-tauri/src/lib.rs:282` |
| Password reset (V-AUTH.4a) | **Native / merged #263** — `matrix_password_reset_request_email_token` + `matrix_password_reset_complete` | `product.rs:366,385`; `nativePasswordReset.ts` |
| Register (V-AUTH.4b) | **OPEN #266** (not merged) — `matrix_register_flows` / `matrix_register_request_email_token` / `matrix_register` on that branch | #266 body; not on this tip |
| Login-flow discovery (P3.1) | **Rust foundation exists** — `discover_login_flows`, `LoginFlowTransport`, `HttpLoginFlowTransport`, `LoginFlowKind`/`LoginFlow` domain types | `src-tauri/src/matrix/auth/login_flow.rs`; `http_transport.rs` |
| UIA coordinator (P3.4) | **Rust foundation exists** — `UiaSession` state machine, `UiaStageKind`, `UiaFlowKind`; **no production Tauri command** | `src-tauri/src/matrix/auth/uia.rs` |
| Token login (V-AUTH.2) | **Closed as non-retention** #262 | `v-auth-2-token-login.md` |
| SSO (V-AUTH.1) | **Removed** #238 | — |

**Gap:** the Rust login-flow discovery + UIA foundation is **not wired to any
production Tauri command**. `AuthFlowsLoader.tsx` still calls the live
`matrix-js-sdk` client directly. V-AUTH.3 closes that gap for the **login-flow
discovery** half (UIA stage execution for login is a follow-on; see §5).

---

## 4. Proposed full-vertical V-AUTH.3 slice

### 4.1 Capability owned

Re-home **login-flow discovery** (the `loginFlows()` half of `AuthFlowsLoader`)
to native Rust IPC, and make the shared UIA type layer SDK-neutral so login no
longer depends on `matrix-js-sdk` auth types. Product decision: login-flow
discovery is **retained** and re-homed to native sole ownership (password-only
product login per V-AUTH.2; discovery still reports `m.login.token` /
`m.login.sso` as advertisement values the product ignores).

### 4.2 Proposed IPC names (Tauri commands, `matrix_*` prefix per existing convention)

| Command | Args | Returns | Notes |
|---------|------|---------|-------|
| `matrix_login_flows` | `{ homeserverUrl: string }` | `{ flows: LoginFlowDto[] }` where `LoginFlowDto = { kind, matrixType, getLoginToken? }` | Wraps `discover_login_flows` + `HttpLoginFlowTransport`; fail-closed on transport error. |
| `matrix_login_flows_discovery` | `{ serverNameOrUrl: string }` | `{ homeserverBaseUrl, flows }` | Optional combined well-known + flows (mirrors `discover_homeserver_and_login_flows`); may be deferred to keep slice minimal. |

**Fail-closed:** if the native command errors or is unavailable, the product
must **not** fall back to a `matrix-js-sdk` live client. `AuthFlowsLoader` shows
the existing error state (`Failed to get authentication flow information.`)
instead of silently constructing a JS client. No dual-backend selector.

### 4.3 Deletion list (physical, inside this slice)

- `AuthFlowsLoader.tsx`: remove `createClient` + `mx.loginFlows()` live call;
  replace with `invokeDesktopWithAvailability('matrix_login_flows', …)`.
- `useAuthFlows.ts`: replace `ILoginFlowsResponse` with a Synara-owned
  `LoginFlowsDto`; drop `matrix-js-sdk` import (keep register-flow parsing for
  #266 or move it to the register slice — do not delete register logic here).
- `useParsedLoginFlows.ts`: drop `IPasswordFlow`/`LoginFlow` js-sdk imports;
  consume Synara `LoginFlowDto`.
- `useUIAFlows.ts`, `matrix-uia.ts`, `uia-stages/types.ts`: replace
  `AuthType`/`IAuthData`/`UIAFlow`/`AuthDict` with Synara-owned UIA DTOs
  (mirroring `UiaStageKind`/`UiaStage` in `uia.rs`).
- `Login.tsx`: consume the new DTO shape (no js-sdk import).
- Remove stale allowlist entries for already-deleted files if present
  (`SSOLogin.tsx`, `TokenLogin.tsx` — verify they are gone on tip).

**Not deleted here (other verticals):** `loginUtil.ts` non-native fallback
(password vertical), register files (#266), `useAuthMetadata.ts` (auth-adjacent,
not UIA).

### 4.4 Tests

- Rust: `cargo test --locked matrix::auth` — extend `login_flow.rs`/`product.rs`
  tests for `matrix_login_flows` command (mock transport, fail-closed on error,
  no secrets in DTO).
- TypeScript: `AuthFlowsLoader`/`useAuthFlows` unit tests asserting the native
  command is invoked and no `matrix-js-sdk` live client is constructed
  (mirror `tokenLoginAbsence.test.ts` pattern).
- Guardrails: `npm run check:matrix-rust-guardrails` — allowlist must **not
  increase**; expect it to drop as auth files leave the allowlist.
- `cargo clippy --lib -- -D warnings` clean.

---

## 5. Explicit non-goals / residual after this slice

| Item | Status |
|------|--------|
| UIA **stage execution** for login (multi-stage password/recaptcha/terms submit over IPC) | **Residual** — V-AUTH.3 re-homes discovery only; stage execution needs a `matrix_uia_*` command family (follow-on, named V-AUTH.3b or folded into a later auth slice). |
| Register (V-AUTH.4b) | #266, separate |
| Password login non-native fallback (`loginUtil.login`) | Password vertical, separate |
| `useAuthMetadata.ts` | Auth-adjacent, not UIA |
| Live Synapse e2e for login-flow discovery | Required PR CI, not claimed here |
| Cutover / dual-backend removal | #240 HOLD; no cutover |

---

## 6. Self-eval

**Confidence: high** for the inventory. I traced the full auth surface: the only
live `matrix-js-sdk` client in the auth vertical is `AuthFlowsLoader.tsx`
(`createClient` + `loginFlows` + `registerRequest`); all other auth/UIA files
are type-only importers. The Rust foundation (`login_flow.rs`, `uia.rs`,
`http_transport.rs`) is present but unwired to production Tauri commands, which
is exactly the V-AUTH.3 gap.

**Possible missed files:**
- `synara/src/app/hooks/useAuthMetadata.ts` (auth-adjacent, listed for
  completeness).
- `synara/src/app/components/SpecVersionsLoader.tsx` / `useSpecVersions` —
  auth-layout version discovery, not loginFlows/UIA; not in scope but worth a
  glance during implementation.
- `synara/src/app/cs-api.ts` — well-known discovery helper (already has a
  REST-exception allowlist entry); not loginFlows.
- Any `matrix-js-sdk` import hidden behind a barrel re-export in the auth tree
  (e.g. `hooks/types.ts` `RequestEmailTokenCallback`) — verify during
  implementation with a full `grep -rn "matrix-js-sdk" synara/src/app/pages/auth synara/src/app/components synara/src/app/hooks`.

**Caveat:** the allowlist (`p1.6-js-sdk-import-allowlist.json`) is at 191 paths
and still lists some deleted files (`SSOLogin.tsx`, `TokenLogin.tsx`,
`useRegisterEmail.ts`); the guardrails script should be re-run to confirm
whether stale entries are tolerated or must be pruned in this slice.
