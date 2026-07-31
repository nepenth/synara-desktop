# V-AUTH.2 — desktop token login (product non-retention)

| Field        | Value                                                                 |
| ------------ | --------------------------------------------------------------------- |
| Status       | **DONE — product does not retain `m.login.token` desktop login**      |
| Capability   | One-time login-token (`m.login.token`) product login                  |
| Decision     | **Not retained** on desktop; capability closed without re-home        |
| Prior owner  | `TokenLogin.tsx` + `login()` JS (`m.login.token`) — deleted in V-AUTH.1 |
| Native path  | None (no `matrix_login_token` IPC; `login_with_token` foundation removed) |

## Product decision

Synara Desktop **does not retain** `m.login.token` as a product login method.

Historical product use of token login was **SSO redirect completion**
(`loginToken` query param → `TokenLogin.tsx` → matrix-js-sdk `loginRequest` with
`type: 'm.login.token'`). **V-AUTH.1** removed desktop SSO entry, callback, and
token-completion UI. After that removal there is no remaining desktop entry
point for a standalone token-login form, and password remains the only product
login method (`Login.tsx` + `matrix_login_password`).

V-AUTH.2 makes that non-retention **explicit** and finishes residual closure:

1. No product UI for token login (already deleted with V-AUTH.1).
2. No production Tauri command for token login.
3. Remove the unused Rust `login_with_token` foundation that was held only pending
   this decision.
4. Keep login-flow **discovery** of `m.login.token` as a domain value when a
   homeserver advertises it; the product UI ignores non-password flows.
5. Focused negative/privacy evidence for the closed boundary.

This is full residual closure under [full-vertical-policy.md](full-vertical-policy.md)
via **explicit product deletion / non-retention**, not a dual-backend or
“approved residual plateau.”

## What remains (not V-AUTH.2)

| Item | Status |
| ---- | ------ |
| Password login | Retained; native `matrix_login_password` |
| Login-flow discovery `LoginFlowKind::Token` | Retained for HS advertisement parsing only |
| Registration token (`m.login.registration_token` UIA) | Unrelated; register residual is V-AUTH.4 |
| Access/refresh token session material | Lifecycle/vault; not login-token login |
| V-AUTH.3 UIA | Separate residual |
| V-AUTH.4 register / reset-password | Separate residual |

## Privacy

- No one-time login token crosses product IPC.
- Password login identity/snapshot DTOs never include access/refresh tokens or password.
- SDK `.login_token` remains guardrail-banned outside `matrix/auth/`; the auth
  module no longer calls it after non-retention.

## Evidence

- Rust: `matrix::auth` unit tests including
  `v_auth_2_product_has_no_token_login_command_or_login_token_sdk_call`
- TypeScript: `synara/src/app/pages/auth/login/__tests__/tokenLoginAbsence.test.ts`
- Product files: `Login.tsx` password-only; `useParsedLoginFlows` password-only

## Import / deletion accounting

| Metric | Value |
| ------ | ----- |
| Capability owner deleted this slice | Rust `login_with_token` foundation (+ Token `LoginMethodKind`) |
| JS product owner | Already deleted in V-AUTH.1 (`TokenLogin.tsx`); no additional production importers this slice |
| Direct `matrix-js-sdk` import delta | **0** (capability was already absent from product TS) |
| Dual backend | Forbidden / not introduced |

## Related

- Residual row: [d0-residual-completion.md](d0-residual-completion.md) **V-AUTH.2**
- Password/token foundation history: [p3.2-password-token-login.md](p3.2-password-token-login.md)
- SSO removal: V-AUTH.1 / PR #238
