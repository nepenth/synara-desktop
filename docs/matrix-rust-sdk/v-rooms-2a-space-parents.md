# V-ROOMS.2a — native space parent map

| Field  | Value                                                                  |
| ------ | ---------------------------------------------------------------------- |
| Status | Implementation candidate; live authenticated runtime proof unclaimed   |
| Owner  | Managed Rust `m.space.child` projection → `roomToParentsAtom`          |
| Queue  | `V-ROOMS.2a` (first slice of V-ROOMS.2)                                |
| Policy | Complete native replacement of parent-map ownership; no JS SDK fallback |

## Retained product contract

Nav filters, space-child selectors, and unread parent rollup continue to read
`roomToParentsAtom` as child→parents. Cycle-safe edges from joined spaces'
valid `m.space.child` events must match the previous JS binder.

## Operating path

```text
Desktop session logged in
  → joined space rooms (RoomState::Joined + is_space)
  → m.space.child state (via-present / valid)
  → matrix_space_parents_snapshot
  → roomToParentsAtom INITIALIZE
  → unread rollup / SpaceTabs / home/orphan filters
```

Disqualifying deviations: binding `RoomStateEvent` / `ClientEvent.Room` for
parent map; falling back to `getRoomToParents(mx)`.

## Explicitly out of scope (V-ROOMS.2b)

Lobby UI, `getRoomHierarchy` pagination, DnD reorder / `sendStateEvent` child
mutations, SynaraSpaces account-data pins, create/join-from-lobby.

## Deletion

- Removed `matrix-js-sdk` binder from
  `synara/src/app/state/room/roomToParents.ts`.
- Dropped that path from `p1.6-js-sdk-import-allowlist.json`.

## Inventory

From tip after V-ROOMS.4 (`151948c`, production **190** / repository-wide **203**):

- desktop-runtime production import files **190 → 189**
- repository-wide import files **203 → 202**
- allowlist **197 → 196**

## Evidence

- `cargo test --locked matrix::spaces`
- parent-map projection unit test + modernization suite
- `npm run check:matrix-rust-guardrails`
- Regenerated `desktop-sdk-usage.{json,md}`

Runtime proof remains **Not confirmed** until an authenticated disposable
session shows space nav/unread rollup tracking native parent edges without JS
room-state binders.
