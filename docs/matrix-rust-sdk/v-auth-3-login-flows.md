# V-AUTH.3 — native login-flow discovery (implementation note)

| Field | Value |
|-------|-------|
| Status | **Implementation** (this PR) — discovery only |
| Residual | UIA **stage execution** for login → **closed as non-retention** ([v-auth-3b-uia-login-stage.md](v-auth-3b-uia-login-stage.md)) |
| Policy | [full-vertical-policy.md](full-vertical-policy.md) |
| Inventory | [v-auth-3-uia-inventory.md](v-auth-3-uia-inventory.md) |

## Capability owned

Re-home **login-flow discovery** (`AuthFlowsLoader` → `GET /_matrix/client/v3/login` flows list) to native Tauri IPC with live HTTP transport. Shared login DTO / UIA type surfaces used by the loader are SDK-neutral.

## Native owners

| Layer | Owner |
|-------|-------|
| Tauri command | `matrix_login_flows` in `src-tauri/src/matrix/auth/product.rs` |
| Domain | `discover_login_flows` + `HttpLoginFlowTransport` (`login_flow.rs`, `http_transport.rs`) |
| TS IPC | `synara/src/app/pages/auth/login/nativeLoginFlows.ts` |
| UI loader | `synara/src/app/components/AuthFlowsLoader.tsx` (no `createClient`) |

DTO: `{ flows: [{ kind, matrixType, getLoginToken? }] }` — no tokens/passwords.

## Physical JS deletion (this slice)

- Live `createClient` + `mx.loginFlows()` removed from `AuthFlowsLoader.tsx`
- `useAuthFlows.ts` / `useParsedLoginFlows.ts` no longer import `matrix-js-sdk`
- Allowlist **175→171** (also drops stale `Login.tsx` entry with no js-sdk import)
- Production import files **172→169**

## Fail-closed

If `matrix_login_flows` is unavailable or errors, AuthFlowsLoader shows the existing error UI. **No** matrix-js-sdk fallback. No dual_backend.

## Explicit non-goals / residual

| Item | Status |
|------|--------|
| UIA stage execution for login (`matrix_uia_*`) | **DONE non-retention #280** — [v-auth-3b-uia-login-stage.md](v-auth-3b-uia-login-stage.md) |
| `loginUtil.ts` non-native password fallback | **Active residual this PR** (password fail-closed) — [#279](https://github.com/nepenth/synara-desktop/pull/279) |
| Register product owners | DONE #266 V-AUTH.4b — not rewritten here |
| Live Synapse e2e for login-flow discovery | Not claimed |
| V-TIMELINE cutover / #240 | HOLD |
