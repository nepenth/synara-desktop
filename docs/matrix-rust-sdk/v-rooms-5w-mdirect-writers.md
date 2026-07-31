# V-ROOMS.5w — native `m.direct` writers

| Field  | Value                                                                  |
| ------ | ---------------------------------------------------------------------- |
| Status | Implementation candidate; live authenticated runtime proof unclaimed   |
| Owner  | Managed Rust `matrix_mdirect_add` / `matrix_mdirect_remove`            |
| Queue  | `V-ROOMS.5w` (writer follow-on to V-ROOMS.5 read #249)                 |
| Policy | Complete native replacement of DM map mutations; no JS SDK fallback    |

## Retained product contract

`/startdm`, CreateChat, `/converttodm`, and `/converttoroom` continue to mark or
unmark rooms as direct. A room remains associated with at most one user key in
`m.direct`.

## Operating path

```text
CreateChat / /startdm / /converttodm
  → matrix_mdirect_add { roomId, userId }
  → fetch m.direct → exclusive add → set_account_data

/converttoroom
  → matrix_mdirect_remove { roomId }
  → fetch m.direct → remove room → set_account_data
```

Disqualifying deviations: `mx.setAccountData('m.direct', …)` or
`mx.getAccountData` for writer ownership; dual-backend fallback when desktop
session is available.

## Explicitly out of scope

- `useDirectUsers` JS AccountData reader (separate residual)
- `mx.createRoom` / room creation itself
- AccountDataEditor generic editing UI

## Deletion

- Removed `addRoomIdToMDirect` / `removeRoomIdFromMDirect` from
  `synara/src/app/utils/matrix.ts`.
- Rewired CreateChat + slash commands to native owner helpers.

## Inventory

From tip after V-ROOMS.5 read (`d17ab2c`, production **187** / repository-wide **200**):

- desktop-runtime production import files **187 → 187** (writers lived inside
  retained `matrix.ts` / command files; capability deletion is the helpers)
- repository-wide import files **200 → 200**
- allowlist **194 → 194**

## Evidence

- `cargo test --locked matrix::account_data`
- nativeMDirectOwner unit tests + modernization suite
- `npm run check:matrix-rust-guardrails`

Runtime proof remains **Not confirmed** until an authenticated disposable
session shows CreateChat/`/converttodm`/`/converttoroom` updating Direct-tab
membership without JS `setAccountData('m.direct')`.
