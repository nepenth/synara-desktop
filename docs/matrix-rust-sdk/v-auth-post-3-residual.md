# V-AUTH post-3 residual — remaining auth edges after V-AUTH.3 #276 (login-flow discovery)

| Field | Value |
|-------|-------|
| Status | **Inventory (docs only)** — no product code in this PR |
| Tip measured | `e6db76c71e65b0d08e637473bc62bc41596bd54b` (integration `feature/matrix-rust-sdk-full-replacement` after #276 V-AUTH.3 + #277 V-SEND residual docs) |
| Base | `feature/matrix-rust-sdk-full-replacement` |
| Policy | [full-vertical-policy.md](full-vertical-policy.md) — physical deletion inside each owning slice |
| Related | V-AUTH.1 #238 (SSO removed), V-AUTH.2 #262 (token non-retention), V-AUTH.3 #276 (login-flow discovery DONE), V-AUTH.4a #263 (password reset), V-AUTH.4b #266 (register DONE) |

> **Scope guard.** This is an **inventory** document only. It does **not**
> implement product code in `product.rs` or any TS. It does not touch open
> **#240** (V-TIMELINE, HOLD). No cutover. No SESSION-HANDOFF docs are produced.
> It does **not** rewrite register (#266, DONE) or reset-password (#263, DONE).

---

## 1. Tip measured

- Branch: `matrix-rust/v-auth-loginutil-residual`
- Tip SHA: **`e6db76c71e65b0d08e637473bc62bc41596bd54b`** (integration tip after #276 V-AUTH.3 + #277 V-SEND residual docs)
- Working tree clean at measurement time.
- Base integration branch: `feature/matrix-rust-sdk-full-replacement`.
- Allowlist at tip: **171** paths (`p1.6-js-sdk-import-allowlist.json`); only two auth-tree files remain on it (`loginUtil.ts`, `PasswordLoginForm.tsx`).

---

## 2. What V-AUTH.3 #276 already closed

| Capability | Status | Evidence |
|------------|--------|----------|
| Login-flow **discovery** (`GET /login` flows list) | **DONE #276** at `4d33227f` | Native `matrix_login_flows` + `HttpLoginFlowTransport`; `AuthFlowsLoader.tsx` live `createClient`/`mx.loginFlows()` deleted; SDK-neutral `LoginFlowDto`/`LoginFlowsDto` in `nativeLoginFlows.ts`; allowlist **175→171**; production import files **172→169** |
| `useAuthFlows.ts` / `useParsedLoginFlows.ts` | **SDK-neutral** | No `matrix-js-sdk` import; consume `LoginFlowsDto`/`LoginFlowDto` from `nativeLoginFlows.ts` |
| `Login.tsx` | **SDK-neutral** | Consumes `useAuthFlows().loginFlows` + `useParsedLoginFlows`; no `matrix-js-sdk` import |

---

## 3. Residual inventory — remaining auth edges after #276

Each row: **path** | **current owner** | **native gap** | **proposed residual ID**.

### 3.1 UIA stage execution for login — residual

V-AUTH.3 re-homed login-flow **discovery** only. **Stage execution** (submitting
`m.login.password` / `m.login.recaptcha` / `m.login.terms` / `m.login.dummy`
stages against a 401 UIA challenge during login) is **not** wired to any native
Tauri command. The Rust `UiaSession` state machine exists (`uia.rs`) but is
**not** exposed as a `matrix_uia_*` production command, and `matrix_login_password`
is a single-shot password login that does **not** handle a UIA challenge
(verified: it calls `login_with_password` directly and returns
`MatrixLoginIdentity` or an error — no `UiaRequired` outcome).

The register vertical (#266) is the reference model: `matrix_register` returns
`MatrixRegisterOutcome::UiaRequired { session, flows, completed, params, … }`
and the TS side submits a `RegisterAuthStage`; login UIA would mirror this with
a `matrix_uia_*` command family.

| Path | Current owner | Native gap | Proposed residual ID |
|------|---------------|------------|----------------------|
| `src-tauri/src/matrix/auth/uia.rs` (`UiaSession`) | Rust state machine exists; **no production Tauri command** | No `matrix_uia_*` command family for login stage submit/advance/cancel | **V-AUTH.R-UIA-LOGIN** |
| `src-tauri/src/matrix/auth/product.rs` (`matrix_login_password`) | Single-shot password login; no UIA challenge handling | No `UiaRequired` outcome / stage-submit path for login | **V-AUTH.R-UIA-LOGIN** |
| `synara/src/app/hooks/useUIAFlows.ts` | SDK-neutral UIA helpers (`useUIAFlow`, `useUIACompleted`, `useUIAParams`) | Only consumed by register (`PasswordRegisterForm.tsx`); no login UIA submit path | **V-AUTH.R-UIA-LOGIN** |
| `synara/src/app/utils/matrix-uia.ts` | SDK-neutral UIA helpers (`getSupportedUIAFlows`, `getUIAFlowForStages`, terms URL, …) | No login UIA consumer; shared with register | **V-AUTH.R-UIA-LOGIN** |
| `synara/src/app/components/uia-stages/*` (`DummyStage`, `EmailStage`, `ReCaptchaStage`, `RegistrationTokenStage`, `TermsStage`, `types.ts`) | SDK-neutral stage UI components | Only wired to register; no login UIA stage UI | **V-AUTH.R-UIA-LOGIN** |
| `synara/src/app/components/SupportedUIAFlowsLoader.tsx` | Filters `UIAFlow[]` by supported stages | Only used by register (`Register.tsx`); no login UIA usage | **V-AUTH.R-UIA-LOGIN** |

**Note:** the TS UIA surface is already SDK-neutral (no `matrix-js-sdk` import),
so the residual is the **native command family + login wiring**, not a type
deletion. The `uia-stages` components are shared with register and must not be
deleted; they are listed because login UIA would consume them.

### 3.2 `loginUtil` non-native fallback — residual

`loginUtil.ts` `login()` is the **only remaining live `matrix-js-sdk` client in
the auth tree** after #276 (`createClient` + `mx.loginRequest(data)`). It is the
non-desktop fallback path: `loginPassword` invokes native `matrix_login_password`
on desktop but falls back to `login()` when `!isSynaraDesktop()`.

| Path | Current owner | Native gap | Proposed residual ID |
|------|---------------|------------|----------------------|
| `synara/src/app/pages/auth/login/loginUtil.ts` (`login()`) | `createClient({ baseUrl })` + `mx.loginRequest(data)` — live JS client | Non-native fallback retained; no native equivalent for non-desktop | **V-AUTH.R-LOGINUTIL-FALLBACK** |
| `synara/src/app/pages/auth/login/loginUtil.ts` (`loginPassword()`) | Native `matrix_login_password` on desktop; `login()` fallback off-desktop | Fallback branch (`!isSynaraDesktop()`) still constructs a JS client | **V-AUTH.R-LOGINUTIL-FALLBACK** |

**Decision note (password vertical, not UIA):** this is the password-login
non-native fallback, explicitly out of scope for V-AUTH.3 (#276) and for the UIA
residual above. It is a separate residual because it is a **live JS client**
(`createClient` + `loginRequest`) that must be deleted or made native before
V-BURN can claim zero live JS clients in the auth vertical.

### 3.3 Other auth `matrix-js-sdk` live clients after #276 — none

There are **no other live `matrix-js-sdk` clients** in the auth tree after #276.
The only remaining auth-tree `matrix-js-sdk` imports are:

| Path | Import | Kind |
|------|--------|------|
| `synara/src/app/pages/auth/login/loginUtil.ts` | `LoginRequest, LoginResponse, MatrixError, createClient` | **Live client** (§3.2) |
| `synara/src/app/pages/auth/login/PasswordLoginForm.tsx` | `MatrixError` | **Type-only** (error mapping for `loginPassword`) |

`PasswordLoginForm.tsx`'s `MatrixError` is type-only (used to type the
`loginPassword` callback / error mapping) and is **not** a live client. It will
leave the allowlist when `loginUtil.ts` is re-homed (the two are coupled through
`loginPassword`'s `MatrixError` error surface).

**Auth-adjacent (not UIA / not login):** `useAuthMetadata.ts` is listed for
completeness — it is auth-adjacent context, not loginFlows/UIA, and is not part
of this residual.

---

## 4. Residual ID summary

| ID | Capability | Owner today | Native gap |
|----|------------|-------------|------------|
| **V-AUTH.R-UIA-LOGIN** | UIA **stage execution** for login (multi-stage password/recaptcha/terms/dummy submit over IPC) | `uia.rs` `UiaSession` (no command); `matrix_login_password` single-shot; `useUIAFlows.ts`, `matrix-uia.ts`, `uia-stages/*`, `SupportedUIAFlowsLoader.tsx` (register-only) | No `matrix_uia_*` command family for login; `matrix_login_password` has no `UiaRequired` outcome |
| **V-AUTH.R-LOGINUTIL-FALLBACK** | Password login non-native fallback | `loginUtil.ts` `login()` (`createClient` + `mx.loginRequest`) | Non-desktop fallback still constructs a live JS client; no native equivalent |

---

## 5. Deletion list (per owning slice, not this PR)

When **V-AUTH.R-UIA-LOGIN** lands:

- Add `matrix_uia_*` command family (mirror `matrix_register`'s
  `MatrixRegisterOutcome::UiaRequired` + `RegisterAuthStage` submit model).
- Extend `matrix_login_password` (or add a login-UIA submit command) to return a
  `UiaRequired` outcome and accept a login auth stage.
- Wire `useUIAFlows.ts` / `matrix-uia.ts` / `uia-stages/*` to the login submit
  path (they are already SDK-neutral; no type deletion needed).
- `PasswordLoginForm.tsx`: drop the type-only `MatrixError` import once the
  native login-UIA error surface is SDK-neutral.

When **V-AUTH.R-LOGINUTIL-FALLBACK** lands:

- `loginUtil.ts`: remove `createClient` + `mx.loginRequest` (`login()`); make
  `loginPassword` native-only (or provide a native non-desktop path).
- Remove `loginUtil.ts` and `PasswordLoginForm.tsx` from the allowlist
  (allowlist **171 → 169**).

**Not deleted here (other verticals):** register files (#266 DONE),
reset-password files (#263 DONE), `useAuthMetadata.ts` (auth-adjacent, not
UIA/login), and all non-auth `matrix-js-sdk` importers (media, client, room,
call, lobby, settings — V-TIMELINE / other verticals).

---

## 6. Explicit non-goals / out of scope

| Item | Status |
|------|--------|
| Login-flow **discovery** | **DONE #276** — not residual |
| Register (V-AUTH.4b) | **DONE #266** — not rewritten here |
| Password reset (V-AUTH.4a) | **DONE #263** — not rewritten here |
| SSO (V-AUTH.1) / token login (V-AUTH.2) | Removed / non-retention — closed |
| Non-auth `matrix-js-sdk` importers (media, room, call, lobby, settings) | V-TIMELINE / other verticals — not auth |
| Cutover / dual-backend removal | #240 HOLD; no cutover |
| Live Synapse e2e for login UIA / loginUtil | Required per owning slice, not claimed here |

---

## 7. Self-eval

**Confidence: high** for the inventory. I traced the full auth tree after #276:

- The only remaining **live** `matrix-js-sdk` client in the auth vertical is
  `loginUtil.ts` `login()` (`createClient` + `mx.loginRequest`), reached only on
  the non-desktop fallback branch of `loginPassword`.
- `PasswordLoginForm.tsx` imports `MatrixError` **type-only** (no live client).
- `AuthFlowsLoader.tsx`, `useAuthFlows.ts`, `useParsedLoginFlows.ts`, `Login.tsx`
  are all SDK-neutral post-#276.
- There is **no** `matrix_uia_*` production Tauri command; `matrix_login_password`
  is single-shot with no `UiaRequired` outcome; the Rust `UiaSession` state
  machine exists but is unwired to production IPC. Register (#266) is the
  reference model for the login-UIA command family.

**Possible missed files:**
- `synara/src/app/hooks/useAuthMetadata.ts` — auth-adjacent context, not
  loginFlows/UIA; listed for completeness.
- `synara/src/app/pages/auth/login/__tests__/*` — test files assert the
  post-#276 SDK-neutral state; not product residual.
- Any `matrix-js-sdk` import hidden behind a barrel re-export in the auth tree —
  verify during implementation with a full
  `grep -rn "matrix-js-sdk" synara/src/app/pages/auth synara/src/app/hooks/useAuthFlows.ts synara/src/app/hooks/useParsedLoginFlows.ts synara/src/app/components/AuthFlowsLoader.tsx`.

**Caveat:** the allowlist (`p1.6-js-sdk-import-allowlist.json`, 171 paths) still
lists `loginUtil.ts` and `PasswordLoginForm.tsx`; these leave the allowlist when
the two residuals above land.
