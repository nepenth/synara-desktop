# V-AUTH.4b — native desktop registration

| Field        | Value                                                                 |
| ------------ | --------------------------------------------------------------------- |
| Status       | **DONE on this branch** — native sole owner for retained register flows |
| Capability   | Password registration + product UIAA stages                           |
| Decision     | **Retained** on desktop; re-homed to Rust (not non-retention)         |
| Prior owner  | `registerUtil` / `PasswordRegisterForm` / `useRegisterEmail` (js-sdk) |
| Native path  | `matrix_register_flows`, `matrix_register_request_email_token`, `matrix_register` |

## Product decision

Synara Desktop **retains** account registration. Users can create accounts from
the desktop register route (`/register/:server?/`). This slice re-homes that
capability to the Matrix Rust SDK host so the product happy path no longer uses
a live `matrix-js-sdk` client for registration.

This is full residual closure under [full-vertical-policy.md](full-vertical-policy.md)
via **native sole ownership + physical JS deletion**, not dual-backend and not
an approved residual plateau.

## Owned capability

1. Registration flow probe (empty `/register` → UIAA flows / disabled / rate-limit)
2. Registration email token request
3. Multi-stage register submit for product-supported stages:
   - `m.login.registration_token`
   - `m.login.terms`
   - `m.login.recaptcha`
   - `m.login.email.identity`
   - `m.login.dummy`
4. On complete: install native product session (tokens never cross IPC)

Unsupported UIAA-only homeserver flows fail closed (`Unsupported`).

## JS deletion

- Deleted `registerUtil.ts`, `useRegisterEmail.ts`
- Rewrote `PasswordRegisterForm.tsx` / `Register.tsx` to native IPC only
- Made register-owned UIA helpers / stage dialogs SDK-neutral (no matrix-js-sdk)
- `AuthFlowsLoader` no longer probes register via js-sdk (login-flow probe remains)

## Residuals (explicit, not this slice)

| Item | Status |
| ---- | ------ |
| V-AUTH.4a password reset | Prior/adjacent slice (#263); tip-merged here |
| V-AUTH.3 login-flow discovery / AuthFlowsLoader login `createClient` | Residual — login still uses js-sdk for `loginFlows()` |
| Password login `loginUtil` residual js types / createClient fallbacks | Separate residual |
| Live Synapse registration e2e | Required PR CI / not claimed as local proof |

## Import / deletion accounting

| Metric | Value |
| ------ | ----- |
| Allowlist | **191 → 175** (on tip after V-AUTH.4a) |
| Production inventory import files | **172** (snapshot refreshed) |
| Dual backend | Forbidden / not introduced |

## Privacy

- No access/refresh tokens over register IPC
- Password, client secret, captcha response, registration token never logged
- Ephemeral unauthenticated client for probe/submit until complete, then product identity restore
