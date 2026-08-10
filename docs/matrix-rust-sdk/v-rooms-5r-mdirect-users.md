# V-ROOMS.5r — native `m.direct` user list

| Field  | Value                                                                  |
| ------ | ---------------------------------------------------------------------- |
| Status | Implementation candidate; live authenticated runtime proof unclaimed   |
| Owner  | Extended `matrix_mdirect_snapshot.userIds` → `mDirectUsersAtom`         |
| Queue  | `V-ROOMS.5r` (reader follow-on after #249/#251)                        |
| Policy | Complete native replacement of DM user-list ownership; no JS fallback  |

## Retained product contract

Invite/create-room autocomplete continues to suggest users that still have at
least one joined room listed under their `m.direct` key.

## Operating path

```text
Desktop session logged in
  → DirectEventContent + joined rooms
  → matrix_mdirect_snapshot { roomIds, userIds }
  → mDirectUsersAtom
  → useDirectUsers / InviteUserPrompt / AdditionalCreatorInput
```

Disqualifying deviations: `useAccountData(AccountDataEvent.Direct)` for the
DM user list; reading `mx.getAccountData('m.direct')` for product suggestions.

## Deletion

- Removed JS AccountData ownership from
  `synara/src/app/hooks/useDirectUsers.ts` (re-exports native atom hook).
- Binder extended in `synara/src/app/state/mDirectList.ts`.

## Inventory

From tip after #243 (`31b4a30`, production **187** / repository-wide **200**):

- desktop-runtime production import files **187 → 187** (`useDirectUsers` was
  not a direct `matrix-js-sdk` importer)
- repository-wide import files **200 → 200**
- allowlist **194 → 194**

## Evidence

- `cargo test --locked matrix::account_data`
- mDirectList projection unit tests + modernization suite
- `npm run check:matrix-rust-guardrails`

Runtime proof remains **Not confirmed** until an authenticated disposable
session shows invite/create autocomplete tracking native DM users without JS
AccountData binders.
