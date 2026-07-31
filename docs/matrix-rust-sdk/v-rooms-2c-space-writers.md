# V-ROOMS.2c — native space child writers (mutations)

| Field  | Value                                                                 |
| ------ | --------------------------------------------------------------------- |
| Status | **Partial** — mutation commands + product owners; local graph residual |
| Owner  | Managed Rust `matrix_space_child_set` / `remove` / `matrix_room_join_rules_set` |
| Queue  | `V-ROOMS.2c` (writers follow-on to 2a parent map + 2b hierarchy reads) |
| Policy | Fail-closed native mutations when desktop session live; no dual_backend |

## Retained product contract

- Lobby DnD reorder of spaces/rooms updates `m.space.child` order keys.
- Hierarchy item menu toggles `suggested` and removes children.
- Add-existing and create-room-with-parent add child edges with `via`.
- Moving a restricted room between spaces rewrites join-rule `allow`.
- Room access settings write `m.room.join_rules` (including restricted allow).

## Operating path

```text
Lobby / HierarchyItemMenu / AddExisting / createRoom / RoomJoinRules
  → matrix_session_snapshot (logged_in required)
  → matrix_space_child_set | matrix_space_child_remove | matrix_room_join_rules_set
  → matrix-sdk Room::send_state_event_for_key / send_state_event_raw / send_state_event
  → privacy-safe mutation result { parentId|roomId, childId?, status: "updated" }
```

Disqualifying deviations: `mx.sendStateEvent(..., m.space.child | m.room.join_rules)` for
these product owners when desktop is available; dual-backend fallback.

## IPC surface

| Command | Args (camelCase) | Notes |
| --- | --- | --- |
| `matrix_space_child_set` | `parentId`, `childId`, `via[]`, `order?`, `suggested?` | Full content replace |
| `matrix_space_child_remove` | `parentId`, `childId` | Empty content clears edge |
| `matrix_room_join_rules_set` | `roomId`, `joinRule`, `allow?[{type,roomId}]` | Restricted allow lists |

## Deletion / rewiring

- Rewired writers away from JS `sendStateEvent` in:
  - `features/lobby/Lobby.tsx`
  - `features/lobby/HierarchyItemMenu.tsx`
  - `features/add-existing/AddExisting.tsx`
  - `components/create-room/utils.ts`
  - `features/common-settings/general/RoomJoinRules.tsx`
- No net production `matrix-js-sdk` importer deletion expected (call sites still
  use JS for permissions/power-levels/state reads and hierarchy listeners).

## Residual (honest)

Not done for full V-ROOMS.2c closure:

- **Local graph / listeners** still JS: `useSpaceHierarchy` builds child lists from
  JS room state (`getStateEvents(SpaceChild)`) and `useStateEventCallback`.
- `utils/room.ts` `getSpaceChildren` / `isValidChild` still JS state helpers.
- Parent map snapshot remains V-ROOMS.2a; no new local children graph snapshot IPC.
- Live Synapse proof unclaimed.

## Evidence

- `cargo test --locked matrix::spaces` — 17 passed (includes mutations).
- `npm --prefix synara run typecheck:modernization`
- `node synara/scripts/run-modernization-tests.mjs` (includes
  `nativeSpaceChildOwner.test.ts`)
