# D0 — Dogfood epic: break the branch, Rust owns Matrix

| Field | Value |
| --- | --- |
| Status | **Active execution** (2026-07-28) |
| Branch | `feature/matrix-rust-sdk-full-replacement` |
| Policy | Clean-break; **no dual-backend**; product on this branch may be broken until D0.1–D0.2 work |
| Related | [cutover-operating-model.md](cutover-operating-model.md), [delivery-layers.md](delivery-layers.md), [PROGRESS.md](PROGRESS.md) |

## Pivot decision

Stop expanding L1-only harness foundations as the main effort.  
**Execute product replacement:** rip js-sdk ownership capability-by-capability and re-implement via **UI → Tauri IPC → Rust matrix-sdk**.

L1 modules already on tip are **parts** to wire, not the end product.

## Priority list (serial)

| ID | Name | Done when |
| --- | --- | --- |
| **D0.1** | **Login + session sole owner** | Password login (and restore if vault present) goes through Rust Tauri commands; product login path does **not** call `matrix-js-sdk` `createClient`/`loginRequest` for the happy path; session tokens live in host vault / SDK client only |
| **D0.2** | **Sync + room list sole owner** | After login, room list UI is driven by Rust projections/IPC; no js `client.getRooms()` as source of truth |
| **D0.3** | **Timeline read** | Opened room timeline rows come from Rust timeline IPC |
| **D0.4** | **Send text** | Composer send uses Rust send path |
| **D0.5** | **Crypto minimum** | Encrypted rooms usable enough for dogfood (or document unencrypted-only dogfood gate) |
| **D0.6** | **Burn-down** | `matrix-js-sdk` product imports → 0 (or approved residual zero) |

## Freeze / park

L1-only open PRs (notify polish, call-state, extra media orthogona**l**, MiniMax helper, etc.) are **parked** unless they block a D0 slice. Comment on each PR points here.

## Success metrics (replace “tasks/112” as primary)

1. Product import count of `matrix-js-sdk` under `synara/src` **decreasing** each D0 PR  
2. **Can log in** on a desktop build of this branch via Rust (D0.1)  
3. Can see rooms / messages / send (D0.2–D0.4)  
4. Ledger (`program-status`) stays truthful for L1; L2/L3 tracked in this epic + PROGRESS  

## D0.1 implementation sketch

### Rust host

- Tauri commands (names illustrative — keep privacy-safe errors, no tokens in returns):
  - `matrix_discover` / login flows (may reuse discovery)
  - `matrix_login_password` → build unauthenticated client, `auth::login`, optional `persist_session_after_login`
  - `matrix_session_status` → logged-out | logged-in identity (user_id, device_id, homeserver) **no tokens**
  - `matrix_restore_session` if vault material exists
  - `matrix_logout` → lifecycle wipe hooks
- Hold live `matrix_sdk::Client` in process state (supervisor / `OnceCell` / managed state) — **sole Matrix client for product on this branch after D0.1**

### Product (break js auth)

- `loginUtil.ts` / password login: call Tauri invoke instead of `createClient` + `loginRequest`
- After success: set session identity for UI routing **without** inventing dual-backend flag; if rest of app still expects js client, **fail closed or stub** with clear “D0 incomplete” rather than spinning js Matrix client for the same session
- Accept broken post-login until D0.2

### Explicit non-goals for D0.1

- Full SSO/UIA parity in first PR (password path first; SSO follow-up OK)
- Timeline/rooms
- Keeping js-sdk login working on this branch
- Dual-backend selector

## Codex / Grok roles

| Role | Owner |
| --- | --- |
| Implement D0 slices | **Codex** `gpt-5.6-sol` medium (high if crypto/session edges need it) |
| Review + tip-merge + focused tests | Codex preferred |
| Dispatch + merge when green | Grok (thin) |

## Update map for other docs

| Doc | Change |
| --- | --- |
| cutover-operating-model | D0 is active execution mode on integration branch |
| delivery-layers | L2 dogfood vertical is current priority over new L1 |
| PROGRESS | Snapshot = D0 priority list |
| program-status | next_task / inventory stay L1-truthful; cutover_state remains not_started until formal P11 (checker constraint) — D0 is tracked here and PROGRESS |

## Orchestration

Recurring loop contract: [d0-orchestrator-loop.md](d0-orchestrator-loop.md).
