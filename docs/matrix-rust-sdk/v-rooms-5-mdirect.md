# V-ROOMS.5 — native `m.direct` map

| Field  | Value                                                                  |
| ------ | ---------------------------------------------------------------------- |
| Status | Implementation candidate; live authenticated runtime proof unclaimed   |
| Owner  | Managed Rust account-data `m.direct` → `mDirectAtom`                   |
| Queue  | `V-ROOMS.5`                                                            |
| Policy | Complete native replacement of DM room-id map ownership; no JS fallback |

## Retained product contract

Direct-tab / DM nav filters, room navigate classification, and other consumers
of `mDirectAtom` continue to read a `Set` of direct room ids. Writers that
create or mark DMs via MatrixClient remain residual until a later write slice.

## Operating path

```text
Desktop session logged in
  → client.account().account_data::<DirectEventContent>()
  → matrix_mdirect_snapshot (poll)
  → mDirectAtom PUT
  → DirectTab / useDirectRooms / navigate filters
```

Disqualifying deviations: binding `ClientEvent.AccountData` for `m.direct`;
falling back to `mx.getAccountData(AccountDataEvent.Direct)`.

## Explicitly out of scope

- `setAccountData('m.direct', …)` create/mark-DM writers (`utils/matrix.ts`,
  invite accept side effects)
- AccountDataEditor generic editing UI

## Deletion

- Removed `matrix-js-sdk` binder from `synara/src/app/state/mDirectList.ts`.
- Removed residual `MatrixClient` param/import from
  `synara/src/app/state/hooks/useBindAtoms.ts`.
- Dropped both paths from `p1.6-js-sdk-import-allowlist.json`.

## Inventory

From tip after V-SEND.1 (`90be0f4`, production **189** / repository-wide **202**):

- desktop-runtime production import files **189 → 187**
- repository-wide import files **202 → 200**
- allowlist **196 → 194**

## Evidence

- `cargo test --locked matrix::account_data`
- mDirect projection unit test + modernization suite subset
- `npm run check:matrix-rust-guardrails`
- Regenerated `desktop-sdk-usage.{json,md}`

Runtime proof remains **Not confirmed** until an authenticated disposable
session shows Direct-tab membership tracking native `m.direct` without JS
AccountData binders.
